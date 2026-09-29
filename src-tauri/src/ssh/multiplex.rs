//! Each selected alias/config/executable owns a private master. No dispatched command is retried.
use super::{
    auth::PolicyConfig,
    runner::{Job, Limits, RunError, Runner},
};
use crate::domain::*;
use std::{ffi::OsString, fs, sync::Arc, time::Duration};
use std::{sync::Mutex, time::Instant};
use tokio::sync::watch;
const PERSIST_SECONDS: u32 = 60;

struct Activity {
    active: usize,
    last: Instant,
}
struct ChannelLease {
    _policy: Arc<PolicyConfig>,
    activity: Arc<Mutex<Activity>>,
}
impl ChannelLease {
    fn new(policy: Arc<PolicyConfig>, activity: Arc<Mutex<Activity>>) -> Self {
        if let Ok(mut state) = activity.lock() {
            state.active += 1;
        }
        Self {
            _policy: policy,
            activity,
        }
    }
}
impl Drop for ChannelLease {
    fn drop(&mut self) {
        if let Ok(mut state) = self.activity.lock() {
            state.active -= 1;
            state.last = Instant::now();
        }
    }
}
pub(crate) struct Connection {
    policy: Arc<PolicyConfig>,
    selection: SshSelection,
    executable: String,
    channels: Runner,
    control: Runner,
    shutdown: watch::Sender<bool>,
    mode: SshTransportMode,
    launched_master: bool,
    closed: bool,
    activity: Arc<Mutex<Activity>>,
    persistence_seconds: u32,
}
impl Connection {
    pub fn new(executable: &str, selection: SshSelection) -> Result<Self, AppError> {
        let executable = crate::diagnostics::validate_executable(executable)
            .map_err(|_| AppError::new(ErrorCode::SshUnavailable))?
            .to_string_lossy()
            .into_owned();
        let policy = Arc::new(PolicyConfig::create(&selection)?);
        let (shutdown, _) = watch::channel(false);
        Ok(Self {
            policy,
            selection,
            executable,
            channels: Runner::default(),
            control: Runner::default(),
            shutdown,
            mode: SshTransportMode::Unconnected,
            launched_master: false,
            closed: false,
            activity: Arc::new(Mutex::new(Activity {
                active: 0,
                last: Instant::now(),
            })),
            persistence_seconds: PERSIST_SECONDS,
        })
    }
    pub fn mode(&self) -> SshTransportMode {
        self.mode.clone()
    }
    pub async fn start(&mut self) -> Result<SshAccessReport, AppError> {
        tokio::time::timeout(
            Duration::from_secs(30),
            self.start_with_persistence(PERSIST_SECONDS),
        )
        .await
        .map_err(|_| AppError::new(ErrorCode::OperationTimedOut))?
    }
    async fn start_with_persistence(&mut self, seconds: u32) -> Result<SshAccessReport, AppError> {
        if self.closed || self.mode != SshTransportMode::Unconnected {
            return Err(AppError::new(ErrorCode::StaleSession));
        }
        // OpenSSH ControlPersist deliberately suppresses ProxyJump stderr. A bounded strict
        // preflight preserves actionable hop authentication/trust errors before creating the master.
        // This is setup only; dispatched application commands are never replayed.
        let access =
            super::auth::probe(&self.channels, &self.executable, self.selection.clone()).await?;
        if access.status != SshAccessStatus::Verified {
            return Ok(access);
        }
        self.persistence_seconds = seconds;
        let socket = self.policy.runtime.socket_path();
        if fs::symlink_metadata(&socket).is_ok() {
            // Unexpected/stale location: never ask an unknown listener to exit or unlink it.
            self.mode = SshTransportMode::DirectFallback;
        } else {
            let mut args = vec![
                "-o".into(),
                "ControlMaster=yes".into(),
                "-o".into(),
                format!("ControlPath={}", socket.display()).into(),
                "-o".into(),
                format!("ControlPersist={seconds}").into(),
                "-N".into(),
            ];
            args.extend(super::runner::structured_arguments(&self.policy.selection)?);
            // Let ControlPersist background only after successfully opening its socket.
            // An unconditional -f could orphan an unaddressable process when mux setup fails.
            self.launched_master = true;
            let result = self
                .control
                .start_owned(
                    &self.executable,
                    args,
                    Limits {
                        deadline: Duration::from_secs(15),
                        stdout_bytes: 4096,
                        stderr_bytes: 64 * 1024,
                    },
                    Box::new(self.policy.clone()),
                )
                .map_err(map_start)?
                .wait()
                .await;
            // A cancelled starter may already have forked the daemon. quiesce/Drop still owns its path.
            if fs::symlink_metadata(&socket).is_ok() {
                self.policy.runtime.record_socket()?;
            }
            match result {
                Ok(output) if output.status.success() && self.policy.runtime.owns_socket() => {
                    self.mode = SshTransportMode::Multiplexed;
                    if !self.healthy().await {
                        return Err(AppError::new(ErrorCode::TransportUnavailable));
                    }
                }
                Ok(output)
                    if mux_unavailable(&output.stderr) && !self.policy.runtime.owns_socket() =>
                {
                    self.mode = SshTransportMode::DirectFallback
                }
                other => return super::auth::report(self.selection.clone(), other),
            }
        }
        let command =
            super::quoting::command(&["printf".into(), "containerdesk-access-ok".into()])?;
        let result = self
            .start_fixed(
                command,
                Limits {
                    deadline: Duration::from_secs(15),
                    stdout_bytes: 1024,
                    stderr_bytes: 64 * 1024,
                },
            )?
            .wait()
            .await;
        super::auth::report(self.selection.clone(), result)
    }
    pub fn client(&self) -> Client {
        Client {
            policy: self.policy.clone(),
            executable: self.executable.clone(),
            channels: self.channels.clone(),
            control: self.control.clone(),
            shutdown: self.shutdown.clone(),
            mode: self.mode.clone(),
            activity: self.activity.clone(),
            persistence_seconds: self.persistence_seconds,
        }
    }
    pub fn start_fixed(&self, encoded: String, limits: Limits) -> Result<Job, AppError> {
        self.client().start_fixed(encoded, limits)
    }
    pub async fn healthy(&self) -> bool {
        self.client().healthy().await
    }
    #[cfg(test)]
    pub async fn wait_lost(&self) {
        self.client().wait_lost().await
    }
    #[cfg(test)]
    fn start_channel(
        &self,
        encoded: String,
        limits: Limits,
        terminal: bool,
    ) -> Result<Job, AppError> {
        self.client().start_channel(encoded, limits, terminal)
    }
    pub async fn close(mut self) {
        self.closed = true;
        self.shutdown.send_replace(true);
        cleanup(
            self.policy.clone(),
            self.channels.clone(),
            self.control.clone(),
            self.executable.clone(),
            self.launched_master,
        )
        .await;
    }
}
/// Cloned command/health capability. Only Connection owns and closes the master.
#[derive(Clone)]
pub(crate) struct Client {
    policy: Arc<PolicyConfig>,
    executable: String,
    channels: Runner,
    control: Runner,
    shutdown: watch::Sender<bool>,
    mode: SshTransportMode,
    activity: Arc<Mutex<Activity>>,
    persistence_seconds: u32,
}
impl Client {
    pub(crate) fn terminal_launch(
        &self,
        command: crate::docker::PreparedCommand,
    ) -> Result<super::terminal::Launch, AppError> {
        if *command.category() != crate::policy::registry::OperationCategory::Terminal {
            return Err(AppError::new(ErrorCode::PermissionDenied));
        }
        let executable = crate::diagnostics::validate_executable(&self.executable)
            .map_err(|_| AppError::new(ErrorCode::TransportUnavailable))?;
        Ok(super::terminal::Launch {
            executable,
            arguments: self.channel_arguments(command.encoded().into(), true)?,
            lease: Box::new(ChannelLease::new(
                self.policy.clone(),
                self.activity.clone(),
            )),
            shutdown: self.shutdown.subscribe(),
        })
    }
    pub fn start_fixed(&self, encoded: String, limits: Limits) -> Result<Job, AppError> {
        self.start_channel(encoded, limits, false)
    }
    pub(crate) fn start_log_stream(
        &self,
        encoded: String,
        sink: super::runner::streaming::LineSink,
    ) -> Result<Job, AppError> {
        let args = self.channel_arguments(encoded, false)?;
        self.channels
            .start_stream(
                &self.executable,
                args,
                sink,
                Box::new(ChannelLease::new(
                    self.policy.clone(),
                    self.activity.clone(),
                )),
                Some(self.shutdown.subscribe()),
            )
            .map_err(map_start)
    }
    pub(crate) fn start_log_snapshot(
        &self,
        encoded: String,
        deadline: Duration,
    ) -> Result<Job, AppError> {
        let args = self.channel_arguments(encoded, false)?;
        self.channels
            .start_log_snapshot_for_session(
                &self.executable,
                args,
                deadline,
                Box::new(ChannelLease::new(
                    self.policy.clone(),
                    self.activity.clone(),
                )),
                Some(self.shutdown.subscribe()),
            )
            .map_err(map_start)
    }
    fn start_channel(
        &self,
        encoded: String,
        limits: Limits,
        terminal: bool,
    ) -> Result<Job, AppError> {
        let args = self.channel_arguments(encoded, terminal)?;
        self.channels
            .start_for_session(
                &self.executable,
                args,
                limits,
                Box::new(ChannelLease::new(
                    self.policy.clone(),
                    self.activity.clone(),
                )),
                Some(self.shutdown.subscribe()),
            )
            .map_err(map_start)
    }
    /// Structured snapshots/streams and the later PTY owner use the same private connection identity.
    /// Returning argv does not grant terminal permission; that remains a backend operation-policy check.
    fn channel_arguments(
        &self,
        encoded: String,
        terminal: bool,
    ) -> Result<Vec<OsString>, AppError> {
        if *self.shutdown.borrow() || self.mode == SshTransportMode::Unconnected {
            return Err(AppError::new(ErrorCode::Disconnected));
        }
        let mut args = Vec::new();
        if self.mode == SshTransportMode::Multiplexed {
            if !self.policy.runtime.owns_socket() {
                return Err(AppError::new(ErrorCode::Disconnected));
            }
            args.extend([
                "-o".into(),
                format!(
                    "ControlPath={}",
                    self.policy.runtime.socket_path().display()
                )
                .into(),
                "-o".into(),
                "ControlMaster=no".into(),
                "-o".into(),
                "ProxyCommand=/usr/bin/false".into(),
            ]);
            // Native OpenSSH normally reconnects if mux disappears. The fixed failing proxy prevents
            // that implicit fallback; no application shell or user-supplied proxy text is spawned.
        }
        let mut base = super::runner::structured_arguments(&self.policy.selection)?;
        if terminal {
            base.retain(|arg| arg != "-T" && arg != "-n");
            args.extend([
                "-tt".into(),
                "-o".into(),
                "EscapeChar=none".into(),
                "-o".into(),
                "StdinNull=no".into(),
            ]);
        }
        args.extend(base);
        args.push(encoded.into());
        Ok(args)
    }
    pub async fn healthy(&self) -> bool {
        if *self.shutdown.borrow() {
            return false;
        }
        if self.mode == SshTransportMode::DirectFallback {
            return !*self.shutdown.borrow();
        }
        self.policy.runtime.owns_socket()
            && control_command(&self.control, &self.executable, &self.policy, "check").await
    }
    pub async fn wait_lost(&self) {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let idle = self.activity.lock().is_ok_and(|state| {
                state.active == 0
                    && state.last.elapsed()
                        >= Duration::from_secs(u64::from(self.persistence_seconds))
            });
            if idle {
                // Even health-control requests can reset native ControlPersist's idle timer.
                // Bound app idleness independently, counting only actual channel leases.
                self.shutdown.send_replace(true);
                let _ =
                    control_command(&self.control, &self.executable, &self.policy, "exit").await;
                return;
            }
            if !self.healthy().await {
                return;
            }
        }
    }
}
impl Drop for Connection {
    fn drop(&mut self) {
        self.shutdown.send_replace(true);
        if !self.closed
            && let Ok(runtime) = tokio::runtime::Handle::try_current()
        {
            runtime.spawn(cleanup(
                self.policy.clone(),
                self.channels.clone(),
                self.control.clone(),
                self.executable.clone(),
                self.launched_master,
            ));
        }
        // If no runtime remains (abrupt shutdown), native ControlPersist bounds an idle daemon.
        // RuntimeLease retains any socket directory for conservative recovery on the next launch.
    }
}
fn map_start(error: RunError) -> AppError {
    AppError::new(match error {
        RunError::Busy => ErrorCode::ResourceLimit,
        RunError::Unavailable => ErrorCode::SshUnavailable,
        _ => ErrorCode::TransportUnavailable,
    })
}
fn mux_unavailable(stderr: &[u8]) -> bool {
    let text = String::from_utf8_lossy(stderr);
    text.contains("unix_listener:")
        || text.contains("ControlSocket ")
        || text.contains("cannot listen to path")
}
async fn control_command(
    runner: &Runner,
    executable: &str,
    policy: &Arc<PolicyConfig>,
    operation: &str,
) -> bool {
    if !["check", "exit"].contains(&operation) || !policy.runtime.owns_socket() {
        return false;
    }
    // Control messages address only the verified app socket. No source config/Match/ProxyCommand is evaluated.
    let args = vec![
        "-F".into(),
        "/dev/null".into(),
        "-S".into(),
        policy.runtime.socket_path().into_os_string(),
        "-O".into(),
        operation.into(),
        "--".into(),
        policy.selection.alias.clone().into(),
    ];
    match runner.start_owned(
        executable,
        args,
        Limits {
            deadline: Duration::from_secs(3),
            stdout_bytes: 4096,
            stderr_bytes: 4096,
        },
        Box::new(policy.clone()),
    ) {
        Ok(job) => job.wait().await.is_ok_and(|output| output.status.success()),
        Err(_) => false,
    }
}
async fn cleanup(
    policy: Arc<PolicyConfig>,
    channels: Runner,
    control: Runner,
    executable: String,
    launched: bool,
) {
    channels.wait_idle().await;
    control.wait_idle().await;
    if launched
        && !policy.runtime.owns_socket()
        && fs::symlink_metadata(policy.runtime.socket_path()).is_ok()
    {
        // The private location was absent before our starter; retain ownership on cancelled startup too.
        let _ = policy.runtime.record_socket();
    }
    if !launched || !policy.runtime.owns_socket() {
        return;
    }
    let _ = control_command(&control, &executable, &policy, "exit").await;
    for _ in 0..20 {
        if !policy.runtime.owns_socket() {
            return;
        }
        match tokio::time::timeout(
            Duration::from_millis(50),
            tokio::net::UnixStream::connect(policy.runtime.socket_path()),
        )
        .await
        {
            Ok(Err(e))
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound
                ) =>
            {
                policy.runtime.remove_dead_socket();
                return;
            }
            _ => tokio::time::sleep(Duration::from_millis(25)).await,
        }
    }
}
#[cfg(test)]
mod tests;
