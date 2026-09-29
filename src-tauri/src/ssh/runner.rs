//! Bounded process ownership for internal SSH operations. No arbitrary-command IPC is exposed.
use crate::{diagnostics::validate_executable, domain::SshSelection};
use std::{
    ffi::OsString,
    fmt,
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    process::{ExitStatus, Stdio},
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::{Child, Command},
    sync::{Semaphore, oneshot},
    time::sleep,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stream {
    Stdout,
    Stderr,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunError {
    InvalidInput,
    Unavailable,
    Busy,
    Io,
    Cancelled,
    TimedOut,
    OutputLimit(Stream),
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub deadline: Duration,
    pub stdout_bytes: usize,
    pub stderr_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            deadline: Duration::from_secs(30),
            stdout_bytes: 16 * 1024 * 1024,
            stderr_bytes: 256 * 1024,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), RunError> {
        if self.deadline.is_zero()
            || self.deadline > Duration::from_secs(30)
            || self.stdout_bytes > 16 * 1024 * 1024
            || self.stderr_bytes > 256 * 1024
        {
            Err(RunError::InvalidInput)
        } else {
            Ok(())
        }
    }
}
pub struct Captured {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
impl fmt::Debug for Captured {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Debug is safe even when a caller logs an unexpected Result in a test/failure path.
        f.debug_struct("Captured")
            .field("status", &self.status)
            .field("stdout_bytes", &self.stdout.len())
            .field("stderr_bytes", &self.stderr.len())
            .finish()
    }
}
#[derive(Clone)]
pub struct Runner {
    slots: Arc<Semaphore>,
}
impl Default for Runner {
    fn default() -> Self {
        Self {
            slots: Arc::new(Semaphore::new(4)),
        }
    }
}
pub struct Job {
    cancel: Option<oneshot::Sender<()>>,
    result: oneshot::Receiver<Result<Captured, RunError>>,
}
impl Job {
    pub fn cancel(&mut self) {
        if let Some(sender) = self.cancel.take() {
            let _ = sender.send(());
        }
    }
    pub async fn finished(&mut self) -> Result<Captured, RunError> {
        (&mut self.result).await.unwrap_or(Err(RunError::Io))
    }
    pub async fn wait(self) -> Result<Captured, RunError> {
        let Self { cancel, result } = self;
        let result = result.await.unwrap_or(Err(RunError::Io));
        drop(cancel);
        result
    }
}
impl Runner {
    pub(crate) async fn wait_idle(&self) {
        // Snapshot owners retain slots until child/group cleanup and reaping finish.
        let permit = self.slots.acquire_many(4).await;
        drop(permit);
    }
    /// Caller must supply arguments from a fixed validated operation builder, never renderer shell text.
    /// Dropping Job or its wait future cancels; the owner retains its slot until direct-child reaping.
    pub fn start(
        &self,
        executable: &str,
        args: Vec<OsString>,
        limits: Limits,
    ) -> Result<Job, RunError> {
        self.start_owned(executable, args, limits, Box::new(()))
    }
    pub(crate) fn start_owned(
        &self,
        executable: &str,
        args: Vec<OsString>,
        limits: Limits,
        resource: Box<dyn Send>,
    ) -> Result<Job, RunError> {
        self.start_for_session(executable, args, limits, resource, None)
    }
    pub(crate) fn start_for_session(
        &self,
        executable: &str,
        args: Vec<OsString>,
        limits: Limits,
        resource: Box<dyn Send>,
        session: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> Result<Job, RunError> {
        limits.validate()?;
        self.start_validated_for_session(executable, args, limits, resource, session)
    }
    /// Only log snapshots treat stderr as data. Ordinary diagnostics retain their 256 KiB cap.
    pub(crate) fn start_log_snapshot_for_session(
        &self,
        executable: &str,
        args: Vec<OsString>,
        deadline: Duration,
        resource: Box<dyn Send>,
        session: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> Result<Job, RunError> {
        let limits = Limits {
            deadline,
            stdout_bytes: 8 * 1024 * 1024,
            stderr_bytes: 8 * 1024 * 1024,
        };
        Limits {
            stderr_bytes: 0,
            ..limits
        }
        .validate()?;
        self.start_validated_for_session(executable, args, limits, resource, session)
    }
    fn start_validated_for_session(
        &self,
        executable: &str,
        args: Vec<OsString>,
        limits: Limits,
        resource: Box<dyn Send>,
        session: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> Result<Job, RunError> {
        if args.len() > 64
            || args
                .iter()
                .any(|a| a.as_bytes().len() > 128 * 1024 || a.as_bytes().contains(&0))
        {
            return Err(RunError::InvalidInput);
        }
        let executable = validate_executable(executable).map_err(|_| RunError::Unavailable)?;
        let permit = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| RunError::Busy)?;
        let (cancel, cancelled) = oneshot::channel();
        let (result, receive) = oneshot::channel();
        tokio::spawn(async move {
            let outcome = execute(executable, args, limits, cancelled, session).await;
            drop(resource);
            drop(permit); // Capacity becomes available only after cleanup, before acknowledgment.
            let _ = result.send(outcome);
        });
        Ok(Job {
            cancel: Some(cancel),
            result: receive,
        })
    }
}
struct OwnedChild(Child);
impl OwnedChild {
    fn stop_group(&self) {
        if let Some(pid) = self.0.id() {
            // SAFETY: setsid() creates a dedicated session/group; id exists only until reaping.
            unsafe {
                libc::kill(-(pid as libc::pid_t), libc::SIGKILL);
            }
        }
    }
    async fn stop_and_reap(&mut self) {
        self.stop_group();
        let _ = self.0.kill().await;
        let _ = self.0.wait().await;
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        self.stop_group();
    }
}
async fn capture(
    mut input: impl AsyncRead + Unpin,
    limit: usize,
    stream: Stream,
) -> Result<Vec<u8>, RunError> {
    let mut bytes = Vec::with_capacity(limit.min(64 * 1024));
    let mut buffer = [0; 8192];
    loop {
        let length = input.read(&mut buffer).await.map_err(|_| RunError::Io)?;
        if length == 0 {
            return Ok(bytes);
        }
        if length > limit.saturating_sub(bytes.len()) {
            return Err(RunError::OutputLimit(stream));
        }
        bytes.extend_from_slice(&buffer[..length]);
    }
}
fn owned_command(executable: PathBuf, args: Vec<OsString>) -> Command {
    let mut command = Command::new(executable);
    command
        .args(args)
        .env("LC_ALL", "C")
        .env("SSH_ASKPASS_REQUIRE", "never")
        .env_remove("SSH_ASKPASS")
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    // SAFETY: setsid is async-signal-safe; no allocation/locks in the fork child.
    // Detach the controlling terminal as well as creating an owned process group.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
    command
}
async fn execute(
    executable: PathBuf,
    args: Vec<OsString>,
    limits: Limits,
    mut cancel: oneshot::Receiver<()>,
    mut session: Option<tokio::sync::watch::Receiver<bool>>,
) -> Result<Captured, RunError> {
    match cancel.try_recv() {
        Ok(()) | Err(oneshot::error::TryRecvError::Closed) => return Err(RunError::Cancelled),
        Err(oneshot::error::TryRecvError::Empty) => (),
    }
    if session
        .as_ref()
        .is_some_and(|receiver| *receiver.borrow() || receiver.has_changed().is_err())
    {
        return Err(RunError::Cancelled);
    }
    let mut command = owned_command(executable, args);
    let child = command.spawn().map_err(|_| RunError::Unavailable)?;
    let mut owned = OwnedChild(child);
    let stdout = owned.0.stdout.take().expect("piped stdout");
    let stderr = owned.0.stderr.take().expect("piped stderr");
    let outcome = tokio::select! {
        biased;
        _ = &mut cancel => Err(RunError::Cancelled),
        _ = async {
            if let Some(receiver) = &mut session {
                while !*receiver.borrow() {
                    if receiver.changed().await.is_err() { break; }
                }
            } else { std::future::pending::<()>().await; }
        } => Err(RunError::Cancelled),
        _ = sleep(limits.deadline) => Err(RunError::TimedOut),
        result = async {
            // Keep the leader unreaped while inherited pipe handles remain, preserving group identity.
            let (stdout, stderr) = tokio::try_join!(capture(stdout, limits.stdout_bytes, Stream::Stdout), capture(stderr, limits.stderr_bytes, Stream::Stderr))?;
            let status = owned.0.wait().await.map_err(|_| RunError::Io)?;
            Ok(Captured {status, stdout, stderr})
        } => result,
    };
    if outcome.is_err() {
        owned.stop_and_reap().await;
    }
    outcome
}

/// Base argv for fixed structured remote operations. The centralized quoting builder must append
/// exactly one POSIX-quoted remote command after these args; never use this for terminal sessions.
pub fn structured_arguments(
    selection: &SshSelection,
) -> Result<Vec<OsString>, crate::domain::AppError> {
    let selected = super::resolver::arguments(selection)?;
    let mut arguments = vec![OsString::from("-T"), OsString::from("-n")];
    for option in [
        "BatchMode=yes",
        "ConnectTimeout=10",
        "ServerAliveInterval=15",
        "ServerAliveCountMax=2",
        "StrictHostKeyChecking=yes",
        "UpdateHostKeys=no",
        "CheckHostIP=no",
        "VerifyHostKeyDNS=no",
        "ForwardAgent=no",
        "ForwardX11=no",
        "ForwardX11Trusted=no",
        "Tunnel=no",
        "ClearAllForwardings=yes",
        "PermitLocalCommand=no",
        "ControlMaster=no",
        "ControlPath=none",
        "ControlPersist=no",
        "ForkAfterAuthentication=no",
        "RemoteCommand=none",
        "SessionType=default",
    ] {
        arguments.extend([OsString::from("-o"), OsString::from(option)]);
    }
    arguments.extend(selected.into_iter().skip(1)); // omit diagnostic -G; retain exact config policy and alias
    Ok(arguments)
}
#[cfg(test)]
mod tests;

pub(crate) mod streaming;
