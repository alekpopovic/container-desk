//! Separate local PTY owner. No terminal bytes enter logs, storage or structured command runners.
use crate::domain::*;
use portable_pty::{CommandBuilder, PtySize};
use std::{
    collections::{HashMap, VecDeque},
    ffi::OsString,
    io::{Read, Write},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
pub(crate) type Current = Arc<dyn Fn() -> Result<(), AppError> + Send + Sync>;
pub(crate) struct Launch {
    pub executable: PathBuf,
    pub arguments: Vec<OsString>,
    pub lease: Box<dyn Send>,
    pub shutdown: tokio::sync::watch::Receiver<bool>,
}
const OUTPUT_LIMIT: usize = 256 * 1024;
const OUTPUT_BATCH: usize = 32 * 1024;
const INPUT_LIMIT: usize = 16 * 1024;
const CONSUMER_LEASE: Duration = Duration::from_secs(10);
const INPUT_IDLE: Duration = Duration::from_secs(600);
struct Buffer {
    output: VecDeque<u8>,
    state: TerminalState,
    exit_code: Option<u32>,
    error: Option<ErrorCode>,
    output_sequence: u32,
    input_sequence: u32,
    accepting: bool,
    last_poll: Instant,
    last_input: Instant,
    size: (u16, u16),
}
struct Entry {
    scope: SessionScope,
    shared: Arc<Mutex<Buffer>>,
    close: Arc<AtomicBool>,
    input: SyncSender<Vec<u8>>,
    current: Current,
    thread: Option<JoinHandle<()>>,
}
#[derive(Clone, Default)]
pub(crate) struct Terminals {
    entries: Arc<Mutex<HashMap<SubscriptionId, Entry>>>,
}
pub(crate) struct Reservation {
    owner: Terminals,
    id: SubscriptionId,
    receiver: Option<mpsc::Receiver<Vec<u8>>>,
    committed: bool,
}
fn failure(code: ErrorCode) -> AppError {
    AppError::new(code)
}
fn size(columns: u16, rows: u16) -> Result<PtySize, AppError> {
    if !(20..=500).contains(&columns) || !(5..=300).contains(&rows) {
        return Err(failure(ErrorCode::InvalidLimits));
    }
    Ok(PtySize {
        rows,
        cols: columns,
        pixel_width: 0,
        pixel_height: 0,
    })
}
impl Terminals {
    pub fn reserve(
        &self,
        request: &TerminalRequest,
        current: Current,
    ) -> Result<Reservation, AppError> {
        current()?;
        let dimensions = size(
            u16::try_from(request.spec.columns).map_err(|_| failure(ErrorCode::InvalidLimits))?,
            u16::try_from(request.spec.rows).map_err(|_| failure(ErrorCode::InvalidLimits))?,
        )?;
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| failure(ErrorCode::Internal))?;
        // Finished abandoned sessions can be retired; live native children continue to own admission.
        entries.retain(|_, entry| {
            !(entry.thread.as_ref().is_some_and(|t| t.is_finished())
                && entry
                    .shared
                    .lock()
                    .is_ok_and(|b| b.last_poll.elapsed() > CONSUMER_LEASE))
        });
        if entries.len() >= 3
            || entries
                .values()
                .any(|e| e.scope.selection.host_id == request.scope.selection.host_id)
        {
            return Err(failure(ErrorCode::ResourceLimit));
        }
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| failure(ErrorCode::Internal))?;
        let id = SubscriptionId(format!(
            "sub_{}",
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ));
        if entries.contains_key(&id) {
            return Err(failure(ErrorCode::Internal));
        }
        let (input, receiver) = mpsc::sync_channel(16);
        entries.insert(
            id.clone(),
            Entry {
                scope: request.scope.clone(),
                current,
                input,
                thread: None,
                close: Arc::new(AtomicBool::new(false)),
                shared: Arc::new(Mutex::new(Buffer {
                    output: VecDeque::new(),
                    state: TerminalState::Starting,
                    exit_code: None,
                    error: None,
                    output_sequence: 0,
                    input_sequence: 0,
                    accepting: false,
                    last_poll: Instant::now(),
                    last_input: Instant::now(),
                    size: (dimensions.cols, dimensions.rows),
                })),
            },
        );
        Ok(Reservation {
            owner: self.clone(),
            id,
            receiver: Some(receiver),
            committed: false,
        })
    }
    fn checked<'a>(
        entries: &'a HashMap<SubscriptionId, Entry>,
        request: &TerminalHandleRequest,
    ) -> Result<&'a Entry, AppError> {
        request.scope.validate()?;
        request.terminal_id.validate()?;
        let entry = entries
            .get(&request.terminal_id)
            .ok_or_else(|| failure(ErrorCode::TerminalClosed))?;
        if entry.scope != request.scope {
            return Err(failure(ErrorCode::StaleSession));
        }
        Ok(entry)
    }
    pub fn read(&self, request: TerminalHandleRequest) -> Result<TerminalOutput, AppError> {
        let entries = self
            .entries
            .lock()
            .map_err(|_| failure(ErrorCode::Internal))?;
        let entry = Self::checked(&entries, &request)?;
        let mut state = entry
            .shared
            .lock()
            .map_err(|_| failure(ErrorCode::Internal))?;
        state.last_poll = Instant::now();
        state.output_sequence = state
            .output_sequence
            .checked_add(1)
            .ok_or_else(|| failure(ErrorCode::ResourceLimit))?;
        let count = OUTPUT_BATCH.min(state.output.len());
        Ok(TerminalOutput {
            scope: request.scope,
            terminal_id: request.terminal_id,
            sequence: state.output_sequence,
            bytes: state.output.drain(..count).collect(),
            state: state.state.clone(),
            exit_code: state.exit_code,
            error: state.error.clone(),
        })
    }
    pub fn input(&self, request: TerminalInputRequest) -> Result<(), AppError> {
        if request.bytes.is_empty() || request.bytes.len() > INPUT_LIMIT {
            return Err(failure(ErrorCode::InvalidLimits));
        }
        let entries = self
            .entries
            .lock()
            .map_err(|_| failure(ErrorCode::Internal))?;
        let entry = Self::checked(
            &entries,
            &TerminalHandleRequest {
                scope: request.scope,
                terminal_id: request.terminal_id,
            },
        )?;
        (entry.current)()?;
        let mut state = entry
            .shared
            .lock()
            .map_err(|_| failure(ErrorCode::Internal))?;
        if !state.accepting || entry.close.load(Ordering::SeqCst) {
            return Err(failure(ErrorCode::TerminalClosed));
        }
        if Some(request.sequence) != state.input_sequence.checked_add(1) {
            return Err(failure(ErrorCode::InvalidIntent));
        }
        entry.input.try_send(request.bytes).map_err(|e| {
            failure(match e {
                mpsc::TrySendError::Full(_) => ErrorCode::ResourceLimit,
                mpsc::TrySendError::Disconnected(_) => ErrorCode::TerminalClosed,
            })
        })?;
        state.input_sequence = request.sequence;
        state.last_input = Instant::now();
        Ok(())
    }
    pub fn resize(&self, request: TerminalResizeRequest) -> Result<(), AppError> {
        size(request.columns, request.rows)?;
        let entries = self
            .entries
            .lock()
            .map_err(|_| failure(ErrorCode::Internal))?;
        let entry = Self::checked(
            &entries,
            &TerminalHandleRequest {
                scope: request.scope,
                terminal_id: request.terminal_id,
            },
        )?;
        (entry.current)()?;
        let mut state = entry
            .shared
            .lock()
            .map_err(|_| failure(ErrorCode::Internal))?;
        if !state.accepting {
            return Err(failure(ErrorCode::TerminalClosed));
        }
        state.size = (request.columns, request.rows);
        Ok(())
    }
    pub async fn close(&self, request: TerminalHandleRequest) -> Result<(), AppError> {
        {
            let entries = self
                .entries
                .lock()
                .map_err(|_| failure(ErrorCode::Internal))?;
            Self::checked(&entries, &request)?
                .close
                .store(true, Ordering::SeqCst);
        }
        for _ in 0..150 {
            {
                let mut entries = self
                    .entries
                    .lock()
                    .map_err(|_| failure(ErrorCode::Internal))?;
                let entry = Self::checked(&entries, &request)?;
                if entry.thread.as_ref().is_some_and(|t| t.is_finished()) {
                    let mut entry = entries.remove(&request.terminal_id).expect("checked entry");
                    if let Some(thread) = entry.thread.take() {
                        let _ = thread.join();
                    }
                    return Ok(());
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        Err(failure(ErrorCode::OperationTimedOut))
    }
    pub async fn shutdown(&self) {
        let requests = if let Ok(entries) = self.entries.lock() {
            entries
                .iter()
                .map(|(id, e)| {
                    e.close.store(true, Ordering::SeqCst);
                    TerminalHandleRequest {
                        scope: e.scope.clone(),
                        terminal_id: id.clone(),
                    }
                })
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
        for request in requests {
            let _ = self.close(request).await;
        }
    }
}
impl Reservation {
    pub fn launch(mut self, launch: Launch) -> Result<TerminalResponse, AppError> {
        let mut entries = self
            .owner
            .entries
            .lock()
            .map_err(|_| failure(ErrorCode::Internal))?;
        let entry = entries
            .get_mut(&self.id)
            .ok_or_else(|| failure(ErrorCode::Internal))?;
        (entry.current)()?;
        let shared = entry.shared.clone();
        let close = entry.close.clone();
        let current = entry.current.clone();
        let receiver = self.receiver.take().expect("one launch");
        entry.thread = Some(
            std::thread::Builder::new()
                .name("containerdesk-pty".into())
                .spawn(move || {
                    let result = run(launch, &shared, &close, current, receiver);
                    if let Ok(mut state) = shared.lock() {
                        state.accepting = false;
                        state.state = TerminalState::Exited;
                        match result {
                            Ok(code) => state.exit_code = code,
                            Err(code) => state.error = Some(code),
                        }
                    }
                })
                .map_err(|_| failure(ErrorCode::ResourceLimit))?,
        );
        self.committed = true;
        Ok(TerminalResponse {
            scope: entry.scope.clone(),
            terminal_id: self.id.clone(),
        })
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        if !self.committed
            && let Ok(mut entries) = self.owner.entries.lock()
        {
            entries.remove(&self.id);
        }
    }
}
impl Drop for Entry {
    fn drop(&mut self) {
        self.close.store(true, Ordering::SeqCst);
    }
}
struct ChildOwner {
    child: Option<Box<dyn portable_pty::Child + Send + Sync>>,
}
impl Drop for ChildOwner {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            // portable-pty's Unix spawn creates a new session with setsid. Never signal a user's master.
            if let Some(pid) = child.process_id() {
                unsafe {
                    libc::kill(-(pid as libc::pid_t), libc::SIGKILL);
                }
            }
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
fn run(
    launch: Launch,
    shared: &Arc<Mutex<Buffer>>,
    close: &AtomicBool,
    current: Current,
    input: mpsc::Receiver<Vec<u8>>,
) -> Result<Option<u32>, ErrorCode> {
    current().map_err(|e| e.code)?;
    let initial = shared.lock().map_err(|_| ErrorCode::Internal)?.size;
    let pair = portable_pty::native_pty_system()
        .openpty(size(initial.0, initial.1).map_err(|e| e.code)?)
        .map_err(|_| ErrorCode::TransportUnavailable)?;
    let fd = pair
        .master
        .as_raw_fd()
        .ok_or(ErrorCode::TransportUnavailable)?;
    // One worker owns nonblocking PTY I/O, so cancellation never waits on a blocked reader/writer.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(ErrorCode::TransportUnavailable);
    }
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|_| ErrorCode::TransportUnavailable)?;
    let mut writer = pair
        .master
        .take_writer()
        .map_err(|_| ErrorCode::TransportUnavailable)?;
    let mut command = CommandBuilder::new(&launch.executable);
    command.args(&launch.arguments);
    command.env("TERM", "xterm-256color");
    command.env("SSH_ASKPASS_REQUIRE", "never");
    command.env_remove("SSH_ASKPASS");
    current().map_err(|e| e.code)?;
    if close.load(Ordering::SeqCst) || *launch.shutdown.borrow() {
        return Err(ErrorCode::OperationCancelled);
    }
    let _lease = launch.lease;
    let mut child = ChildOwner {
        child: Some(
            pair.slave
                .spawn_command(command)
                .map_err(|_| ErrorCode::TransportUnavailable)?,
        ),
    };
    drop(pair.slave);
    shared.lock().map_err(|_| ErrorCode::Internal)?.state = TerminalState::Running;
    shared.lock().map_err(|_| ErrorCode::Internal)?.accepting = true;
    let mut current_size = initial;
    let mut pending = Vec::new();
    let mut offset = 0;
    let mut sent_input = false;
    let mut exited = None;
    let mut full_since = None;
    loop {
        if close.load(Ordering::SeqCst) {
            return Ok(exited);
        }
        if *launch.shutdown.borrow() {
            return Err(ErrorCode::Disconnected);
        }
        current().map_err(|e| e.code)?;
        let (target_size, space, last_poll, last_input) = {
            let state = shared.lock().map_err(|_| ErrorCode::Internal)?;
            (
                state.size,
                OUTPUT_LIMIT - state.output.len(),
                state.last_poll,
                state.last_input,
            )
        };
        if last_poll.elapsed() > CONSUMER_LEASE || last_input.elapsed() > INPUT_IDLE {
            return Err(ErrorCode::OperationTimedOut);
        }
        if target_size != current_size {
            pair.master
                .resize(size(target_size.0, target_size.1).map_err(|e| e.code)?)
                .map_err(|_| ErrorCode::TransportUnavailable)?;
            current_size = target_size;
        }
        if exited.is_none() {
            if pending.is_empty() {
                match input.try_recv() {
                    Ok(bytes) => {
                        pending = bytes;
                        offset = 0;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => return Ok(None),
                    Err(mpsc::TryRecvError::Empty) => (),
                }
            }
            if !pending.is_empty() {
                current().map_err(|e| e.code)?;
                match writer.write(&pending[offset..]) {
                    Ok(0) => return Err(ErrorCode::Disconnected),
                    Ok(n) => {
                        sent_input = true;
                        offset += n;
                        if offset == pending.len() {
                            pending.clear();
                        }
                    }
                    Err(e)
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                        ) => {}
                    Err(_) => return Err(ErrorCode::Disconnected),
                }
            }
        }
        let mut eof = false;
        if space > 0 {
            full_since = None;
            let mut bytes = [0u8; 8192];
            let length = space.min(bytes.len());
            match reader.read(&mut bytes[..length]) {
                Ok(0) => eof = true,
                Ok(n) => shared
                    .lock()
                    .map_err(|_| ErrorCode::Internal)?
                    .output
                    .extend(&bytes[..n]),
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                    ) => {}
                Err(e) if e.raw_os_error() == Some(libc::EIO) => eof = true,
                Err(_) => return Err(ErrorCode::Disconnected),
            }
        } else if full_since.get_or_insert_with(Instant::now).elapsed() > Duration::from_secs(5) {
            return Err(ErrorCode::ResourceLimit);
        }
        if let Some(owner) = child.child.as_mut()
            && let Some(status) = owner.try_wait().map_err(|_| ErrorCode::Internal)?
        {
            exited = Some(status.exit_code());
            child.child = None;
            shared.lock().map_err(|_| ErrorCode::Internal)?.accepting = false;
        }
        if eof && let Some(code) = exited {
            if code == 127 && !sent_input {
                return Err(ErrorCode::TerminalShellUnavailable);
            }
            if code == 255 {
                return Err(ErrorCode::TransportUnavailable);
            }
            return Ok(Some(code));
        }
        let mut descriptor = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        unsafe {
            libc::poll(&mut descriptor, 1, 10);
        }
        // A full output queue must not spin on a continuously readable PTY.
        if space == 0 || eof {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> TerminalRequest {
        TerminalRequest {
            scope: crate::contract_tests::scope(),
            intent_id: IntentId(format!("i_{}", "a".repeat(32))),
            spec: TerminalSpec {
                container_id: ContainerId("b".repeat(64)),
                shell: TerminalShell::Sh,
                columns: 80,
                rows: 24,
            },
        }
    }
    #[test]
    fn terminal_admission_sequences_queues_and_scope_are_bounded() {
        let manager = Terminals::default();
        let request = request();
        let current: Current = Arc::new(|| Ok(()));
        let reservation = manager.reserve(&request, current.clone()).unwrap();
        assert!(manager.reserve(&request, current.clone()).is_err());
        let key = TerminalHandleRequest {
            scope: request.scope.clone(),
            terminal_id: reservation.id.clone(),
        };
        let mut foreign = key.clone();
        foreign.scope.daemon_id = "foreign".into();
        assert_eq!(
            manager.read(foreign).unwrap_err().code,
            ErrorCode::StaleSession
        );
        manager
            .entries
            .lock()
            .unwrap()
            .get(&key.terminal_id)
            .unwrap()
            .shared
            .lock()
            .unwrap()
            .accepting = true;
        for sequence in 1..=16 {
            manager
                .input(TerminalInputRequest {
                    scope: key.scope.clone(),
                    terminal_id: key.terminal_id.clone(),
                    sequence,
                    bytes: vec![0; INPUT_LIMIT],
                })
                .unwrap();
        }
        for (sequence, bytes, expected) in [
            (16, vec![1], ErrorCode::InvalidIntent),
            (17, vec![1], ErrorCode::ResourceLimit),
            (17, vec![1; INPUT_LIMIT + 1], ErrorCode::InvalidLimits),
        ] {
            assert_eq!(
                manager
                    .input(TerminalInputRequest {
                        scope: key.scope.clone(),
                        terminal_id: key.terminal_id.clone(),
                        sequence,
                        bytes
                    })
                    .unwrap_err()
                    .code,
                expected
            );
        }
        assert_eq!(
            manager
                .resize(TerminalResizeRequest {
                    scope: key.scope.clone(),
                    terminal_id: key.terminal_id.clone(),
                    columns: 0,
                    rows: 24
                })
                .unwrap_err()
                .code,
            ErrorCode::InvalidLimits
        );
        let mut reservations = Vec::new();
        for digit in ['c', 'd'] {
            let mut next = request.clone();
            next.scope.selection.host_id = HostId(format!("h_{}", digit.to_string().repeat(32)));
            reservations.push(manager.reserve(&next, current.clone()).unwrap());
        }
        let mut extra = request.clone();
        extra.scope.selection.host_id = HostId(format!("h_{}", "e".repeat(32)));
        assert!(manager.reserve(&extra, current.clone()).is_err());
        drop(reservations);
        drop(reservation);
        assert!(manager.entries.lock().unwrap().is_empty());
    }
    fn launch(program: &str, args: &[&str]) -> (Launch, tokio::sync::watch::Sender<bool>) {
        let (shutdown, receiver) = tokio::sync::watch::channel(false);
        (
            Launch {
                executable: program.into(),
                arguments: args.iter().map(OsString::from).collect(),
                lease: Box::new(()),
                shutdown: receiver,
            },
            shutdown,
        )
    }
    #[tokio::test]
    async fn full_output_closes_with_bounded_memory_and_releases_admission() {
        let manager = Terminals::default();
        let request = request();
        let (launch, _sender) = launch("/usr/bin/yes", &["bounded-terminal-fixture"]);
        let response = manager
            .reserve(&request, Arc::new(|| Ok(())))
            .unwrap()
            .launch(launch)
            .unwrap();
        let key = TerminalHandleRequest {
            scope: response.scope,
            terminal_id: response.terminal_id,
        };
        tokio::time::sleep(Duration::from_secs(7)).await;
        {
            let entry = manager.entries.lock().unwrap();
            let state = entry.get(&key.terminal_id).unwrap().shared.lock().unwrap();
            assert_eq!(state.state, TerminalState::Exited);
            assert_eq!(state.error, Some(ErrorCode::ResourceLimit));
            assert!(state.output.len() <= OUTPUT_LIMIT);
        }
        manager.close(key).await.unwrap();
        assert!(manager.entries.lock().unwrap().is_empty());
    }
    #[tokio::test]
    async fn revocation_reaps_the_actual_owned_process_and_never_exposes_bytes_in_debug() {
        let allowed = Arc::new(AtomicBool::new(true));
        let gate = allowed.clone();
        let current: Current = Arc::new(move || {
            if gate.load(Ordering::SeqCst) {
                Ok(())
            } else {
                Err(failure(ErrorCode::PermissionDenied))
            }
        });
        let manager = Terminals::default();
        let request = request();
        let (launch, _sender) = launch("/bin/sh", &["-c", "printf '%s\\n' $$; exec /bin/sleep 60"]);
        let response = manager
            .reserve(&request, current)
            .unwrap()
            .launch(launch)
            .unwrap();
        let key = TerminalHandleRequest {
            scope: response.scope,
            terminal_id: response.terminal_id,
        };
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut bytes = Vec::new();
        let pid: libc::pid_t = loop {
            let output = manager.read(key.clone()).unwrap();
            bytes.extend(output.bytes);
            if let Ok(text) = std::str::from_utf8(&bytes)
                && let Ok(pid) = text.trim().parse::<libc::pid_t>()
            {
                break pid;
            }
            assert!(Instant::now() < deadline);
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        assert!(pid > 1);
        allowed.store(false, Ordering::SeqCst);
        manager.close(key).await.unwrap();
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
        let input = TerminalInputRequest {
            scope: request.scope,
            terminal_id: SubscriptionId(format!("sub_{}", "1".repeat(32))),
            sequence: 1,
            bytes: b"private-terminal-input".to_vec(),
        };
        assert!(!format!("{input:?}").contains(&format!("{:?}", input.bytes)));
    }
}
