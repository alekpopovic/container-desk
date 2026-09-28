//! Transient connection attempts; no persistence or Docker authorization is granted here.
use crate::domain::*;
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::sync::{Mutex as AsyncMutex, oneshot};
type StageFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub(crate) enum StageOutcome {
    Resolved {
        has_jump: bool,
    },
    Authenticated,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Native Docker readiness arrives in prompt 016; the controlled driver exercises this state now."
        )
    )]
    Ready,
    Degraded(ConnectionDiagnosticCode),
    Failed(ConnectionDiagnosticCode),
}
pub(crate) trait StageDriver: Send + Sync {
    fn run<'a>(
        &'a self,
        stage: ConnectionStage,
        selection: &'a SshSelection,
    ) -> StageFuture<'a, StageOutcome>;
    fn quiesce(&self) -> StageFuture<'_, ()>;
}
pub(crate) struct NativeDriver {
    pub runner: super::runner::Runner,
    pub executable: String,
    pub _permit: tokio::sync::OwnedSemaphorePermit,
}
impl StageDriver for NativeDriver {
    fn run<'a>(
        &'a self,
        stage: ConnectionStage,
        selection: &'a SshSelection,
    ) -> StageFuture<'a, StageOutcome> {
        Box::pin(async move {
            match stage {
                ConnectionStage::Resolve => match super::resolver::resolve(
                    &self.runner,
                    &self.executable,
                    selection.clone(),
                )
                .await
                {
                    Ok(config) => StageOutcome::Resolved {
                        has_jump: config.proxy_jump.is_some() || config.has_proxy_command,
                    },
                    Err(error) => {
                        StageOutcome::Failed(if error.code == ErrorCode::OperationTimedOut {
                            ConnectionDiagnosticCode::TimedOut
                        } else {
                            ConnectionDiagnosticCode::ResolutionFailed
                        })
                    }
                },
                ConnectionStage::Authenticate => {
                    match super::auth::probe(&self.runner, &self.executable, selection.clone())
                        .await
                    {
                        Ok(report) => match report.status {
                            SshAccessStatus::Verified => StageOutcome::Authenticated,
                            SshAccessStatus::RemoteCommandFailed => {
                                StageOutcome::Failed(ConnectionDiagnosticCode::RemoteCommandFailed)
                            }
                            SshAccessStatus::UnknownHostKey => {
                                StageOutcome::Failed(ConnectionDiagnosticCode::UnknownHostKey)
                            }
                            SshAccessStatus::ChangedHostKey => {
                                StageOutcome::Failed(ConnectionDiagnosticCode::ChangedHostKey)
                            }
                            SshAccessStatus::HostKeyRejected => {
                                StageOutcome::Failed(ConnectionDiagnosticCode::HostKeyRejected)
                            }
                            SshAccessStatus::AuthenticationFailed => {
                                StageOutcome::Failed(ConnectionDiagnosticCode::AuthenticationFailed)
                            }
                            SshAccessStatus::TimedOut => {
                                StageOutcome::Failed(ConnectionDiagnosticCode::TimedOut)
                            }
                            SshAccessStatus::OutputLimit => {
                                StageOutcome::Failed(ConnectionDiagnosticCode::OutputLimit)
                            }
                            SshAccessStatus::ConnectionFailed => {
                                StageOutcome::Failed(ConnectionDiagnosticCode::ConnectionFailed)
                            }
                        },
                        Err(_) => StageOutcome::Failed(ConnectionDiagnosticCode::ConnectionFailed),
                    }
                }
                // Prompt 016 supplies the Docker capability probe. Never fabricate readiness now.
                ConnectionStage::Probe => {
                    StageOutcome::Degraded(ConnectionDiagnosticCode::ProbeUnavailable)
                }
            }
        })
    }
    fn quiesce(&self) -> StageFuture<'_, ()> {
        Box::pin(self.runner.wait_idle())
    }
}
#[derive(Default)]
struct State {
    generation: u32,
    snapshot: Option<ConnectionSnapshot>,
}
struct Worker {
    cancel: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<()>,
}
#[derive(Default)]
pub struct Sessions {
    state: Arc<Mutex<State>>,
    control: AsyncMutex<Option<Worker>>,
}
impl Sessions {
    pub(crate) async fn begin(
        &self,
        selection: SshSelection,
        factory: impl FnOnce() -> Result<Arc<dyn StageDriver>, AppError>,
    ) -> Result<ConnectionSnapshot, AppError> {
        super::resolver::arguments(&selection)?;
        let mut control = self
            .control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
        {
            let state = self
                .state
                .lock()
                .map_err(|_| AppError::new(ErrorCode::Internal))?;
            if let Some(snapshot) = &state.snapshot
                && snapshot.selection == selection
                && matches!(
                    snapshot.state,
                    ConnectionState::Resolving
                        | ConnectionState::Connecting
                        | ConnectionState::Probing
                        | ConnectionState::Ready
                )
            {
                return Ok(snapshot.clone()); // Repeated clicks share the same attempt.
            }
        }
        // Invalidate before requesting cancellation, so a racing completion cannot publish.
        self.invalidate()?;
        stop(&mut control).await;
        let driver = factory()?;
        let snapshot = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| AppError::new(ErrorCode::Internal))?;
            let mut bytes = [0; 16];
            getrandom::fill(&mut bytes).map_err(|_| AppError::new(ErrorCode::Internal))?;
            let snapshot = ConnectionSnapshot {
                token: ConnectionToken {
                    session_id: SessionId(format!(
                        "s_{}",
                        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
                    )),
                    session_generation: state.generation,
                },
                selection,
                state: ConnectionState::Resolving,
                durations: vec![],
                diagnostic: None,
                has_jump: false,
            };
            state.snapshot = Some(snapshot.clone());
            snapshot
        };
        let (cancel, cancelled) = oneshot::channel();
        let state = self.state.clone();
        let initial = snapshot.clone();
        let task = tokio::spawn(async move {
            drive(state, initial, driver, cancelled).await;
        });
        *control = Some(Worker { cancel, task });
        Ok(snapshot)
    }
    fn invalidate(&self) -> Result<(), AppError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?;
        state.generation = state
            .generation
            .checked_add(1)
            .ok_or_else(|| AppError::new(ErrorCode::ResourceLimit))?;
        let generation = state.generation;
        if let Some(snapshot) = &mut state.snapshot {
            snapshot.token.session_generation = generation;
            snapshot.state = ConnectionState::Disconnected;
            snapshot.diagnostic = None;
        }
        Ok(())
    }
    pub fn snapshot(&self, token: &ConnectionToken) -> Result<ConnectionSnapshot, AppError> {
        token.validate()?;
        let state = self
            .state
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?;
        let current = state
            .snapshot
            .as_ref()
            .ok_or_else(|| AppError::new(ErrorCode::SessionNotFound))?;
        if current.token != *token {
            return Err(AppError::new(ErrorCode::StaleSession));
        }
        Ok(current.clone())
    }
    pub async fn disconnect(
        &self,
        token: &ConnectionToken,
    ) -> Result<ConnectionSnapshot, AppError> {
        let mut control = self
            .control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
        self.snapshot(token)?;
        self.invalidate()?;
        stop(&mut control).await;
        self.state
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .snapshot
            .clone()
            .ok_or_else(|| AppError::new(ErrorCode::SessionNotFound))
    }
    pub fn reset_idle(&self) -> Result<(), AppError> {
        // Caller holds the diagnostic gate: there can be no native stage/cleanup in flight.
        self.invalidate()
    }
}
async fn stop(control: &mut Option<Worker>) {
    if let Some(worker) = control.take() {
        let _ = worker.cancel.send(());
        let _ = worker.task.await;
    }
}
async fn drive(
    state: Arc<Mutex<State>>,
    initial: ConnectionSnapshot,
    driver: Arc<dyn StageDriver>,
    mut cancelled: oneshot::Receiver<()>,
) {
    for stage in [
        ConnectionStage::Resolve,
        ConnectionStage::Authenticate,
        ConnectionStage::Probe,
    ] {
        let started = Instant::now();
        let outcome = tokio::select! {
            biased;
            _ = &mut cancelled => None,
            outcome = driver.run(stage, &initial.selection) => Some(outcome),
        };
        let Some(outcome) = outcome else { break };
        let Ok(mut state) = state.lock() else { break };
        let Some(current) = &mut state.snapshot else {
            break;
        };
        if current.token != initial.token {
            break;
        }
        current.durations.push(StageDuration {
            stage,
            duration_ms: started.elapsed().as_millis().min(u32::MAX as u128) as u32,
        });
        match (stage, outcome) {
            (ConnectionStage::Resolve, StageOutcome::Resolved { has_jump }) => {
                current.has_jump = has_jump;
                current.state = ConnectionState::Connecting;
            }
            (ConnectionStage::Authenticate, StageOutcome::Authenticated) => {
                current.state = ConnectionState::Probing
            }
            (ConnectionStage::Probe, StageOutcome::Ready) => current.state = ConnectionState::Ready,
            (ConnectionStage::Probe, StageOutcome::Degraded(code)) => {
                current.state = ConnectionState::Degraded;
                current.diagnostic = Some(ConnectionDiagnostic { stage, code });
            }
            (_, StageOutcome::Failed(code)) => {
                current.state = ConnectionState::Error;
                current.diagnostic = Some(ConnectionDiagnostic { stage, code });
                break;
            }
            _ => {
                current.state = ConnectionState::Error;
                current.diagnostic = Some(ConnectionDiagnostic {
                    stage,
                    code: ConnectionDiagnosticCode::RemoteCommandFailed,
                });
                break;
            }
        }
    }
    // Dropped stage futures cancel Runner Jobs. Hold the native gate until their owners reap.
    driver.quiesce().await;
}
#[cfg(test)]
mod tests;
