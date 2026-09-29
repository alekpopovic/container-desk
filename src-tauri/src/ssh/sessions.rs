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
        effective: Option<Box<EffectiveSshConfig>>,
    },
    Authenticated(SshTransportMode),
    #[cfg(test)]
    Ready,
    Probed(Box<DockerProbeReport>),
    Failed(ConnectionDiagnosticCode),
}
pub(crate) trait StageDriver: Send + Sync {
    fn run<'a>(
        &'a self,
        stage: ConnectionStage,
        selection: &'a SshSelection,
    ) -> StageFuture<'a, StageOutcome>;
    fn idle(&self) -> StageFuture<'_, Option<ConnectionDiagnosticCode>> {
        Box::pin(std::future::pending())
    }
    fn list<'a>(
        &'a self,
        scope: &'a SessionScope,
    ) -> StageFuture<'a, Result<ListContainersResponse, AppError>> {
        Box::pin(async move { Err(AppError::new(ErrorCode::FeatureUnavailable).in_scope(scope)) })
    }
    fn networks<'a>(
        &'a self,
        request: &'a ListNetworksRequest,
    ) -> StageFuture<'a, Result<ListNetworksResponse, AppError>> {
        Box::pin(async move {
            Err(AppError::new(ErrorCode::FeatureUnavailable).in_scope(&request.scope))
        })
    }
    fn inspect_network<'a>(
        &'a self,
        request: &'a InspectNetworkRequest,
    ) -> StageFuture<'a, Result<NetworkDetail, AppError>> {
        Box::pin(async move {
            Err(AppError::new(ErrorCode::FeatureUnavailable).in_scope(&request.scope))
        })
    }
    fn volumes<'a>(
        &'a self,
        request: &'a ListVolumesRequest,
    ) -> StageFuture<'a, Result<ListVolumesResponse, AppError>> {
        Box::pin(async move {
            Err(AppError::new(ErrorCode::FeatureUnavailable).in_scope(&request.scope))
        })
    }
    fn inspect_volume<'a>(
        &'a self,
        request: &'a InspectVolumeRequest,
    ) -> StageFuture<'a, Result<VolumeDetail, AppError>> {
        Box::pin(async move {
            Err(AppError::new(ErrorCode::FeatureUnavailable).in_scope(&request.scope))
        })
    }
    fn images<'a>(
        &'a self,
        request: &'a ListImagesRequest,
    ) -> StageFuture<'a, Result<ListImagesResponse, AppError>> {
        Box::pin(async move {
            Err(AppError::new(ErrorCode::FeatureUnavailable).in_scope(&request.scope))
        })
    }
    fn inspect_image<'a>(
        &'a self,
        request: &'a InspectImageRequest,
    ) -> StageFuture<'a, Result<ImageDetail, AppError>> {
        Box::pin(async move {
            Err(AppError::new(ErrorCode::FeatureUnavailable).in_scope(&request.scope))
        })
    }
    fn compose<'a>(
        &'a self,
        scope: &'a SessionScope,
    ) -> StageFuture<'a, Result<ListComposeResponse, AppError>> {
        Box::pin(async move { Err(AppError::new(ErrorCode::FeatureUnavailable).in_scope(scope)) })
    }
    fn inspect<'a>(
        &'a self,
        request: &'a InspectContainerRequest,
    ) -> StageFuture<'a, Result<ContainerDetail, AppError>> {
        Box::pin(async move {
            Err(AppError::new(ErrorCode::FeatureUnavailable).in_scope(&request.scope))
        })
    }
    fn stats<'a>(
        &'a self,
        request: &'a ContainerStatsRequest,
    ) -> StageFuture<'a, Result<StatsSample, AppError>> {
        Box::pin(async move {
            Err(AppError::new(ErrorCode::FeatureUnavailable).in_scope(&request.scope))
        })
    }
    fn logs<'a>(
        &'a self,
        request: &'a ContainerLogsRequest,
    ) -> StageFuture<'a, Result<LogSnapshot, AppError>> {
        Box::pin(async move {
            Err(AppError::new(ErrorCode::FeatureUnavailable).in_scope(&request.scope))
        })
    }
    fn follow_events<'a>(
        &'a self,
        _request: &'a FollowEventsRequest,
        _sink: super::runner::streaming::LineSink,
    ) -> StageFuture<'a, Result<super::runner::Job, AppError>> {
        Box::pin(async { Err(AppError::new(ErrorCode::FeatureUnavailable)) })
    }
    fn follow_logs<'a>(
        &'a self,
        _request: &'a FollowLogsRequest,
        _sink: super::runner::streaming::LineSink,
    ) -> StageFuture<'a, Result<super::runner::Job, AppError>> {
        Box::pin(async { Err(AppError::new(ErrorCode::FeatureUnavailable)) })
    }
    fn verify_compose<'a>(
        &'a self,
        request: &'a VerifyComposeRequest,
    ) -> StageFuture<'a, Result<crate::docker::compose_actions::VerifiedRead, AppError>> {
        Box::pin(async move {
            Err(AppError::new(ErrorCode::FeatureUnavailable).in_scope(&request.scope))
        })
    }
    fn mutate_compose<'a>(
        &'a self,
        _request: ComposeMutationRequest,
        _expected: crate::docker::compose_actions::VerifiedRead,
        _owner: crate::activity::Operation,
        _current: crate::docker::mutations::Current,
    ) -> StageFuture<'a, Result<ComposeMutationResponse, AppError>> {
        Box::pin(async { Err(AppError::new(ErrorCode::FeatureUnavailable)) })
    }
    fn mutate<'a>(
        &'a self,
        _request: MutationRequest,
        _owner: crate::activity::Operation,
        _current: crate::docker::mutations::Current,
    ) -> StageFuture<'a, Result<MutationResponse, AppError>> {
        Box::pin(async { Err(AppError::new(ErrorCode::FeatureUnavailable)) })
    }
    fn quiesce(&self) -> StageFuture<'_, ()>;
}
pub(crate) struct NativeDriver {
    pub runner: super::runner::Runner,
    pub executable: String,
    pub _permit: tokio::sync::OwnedSemaphorePermit,
    pub connection: AsyncMutex<Option<super::multiplex::Connection>>,
    pub docker_options: DockerOptions,
    pub docker_binding: AsyncMutex<Option<crate::docker::probe::VerifiedDocker>>,
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
                        effective: Some(Box::new(config)),
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
                    let connection = match super::multiplex::Connection::new(
                        &self.executable,
                        selection.clone(),
                    ) {
                        Ok(connection) => connection,
                        Err(_) => {
                            return StageOutcome::Failed(
                                ConnectionDiagnosticCode::ConnectionFailed,
                            );
                        }
                    };
                    let mut owner = self.connection.lock().await;
                    *owner = Some(connection);
                    let connection = owner.as_mut().expect("connection owner");
                    match connection.start().await {
                        Ok(report) => match report.status {
                            SshAccessStatus::Verified => {
                                StageOutcome::Authenticated(connection.mode())
                            }
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
                        Err(error) => {
                            StageOutcome::Failed(if error.code == ErrorCode::OperationTimedOut {
                                ConnectionDiagnosticCode::TimedOut
                            } else {
                                ConnectionDiagnosticCode::ConnectionFailed
                            })
                        }
                    }
                }
                ConnectionStage::Probe => {
                    let owner = self.connection.lock().await;
                    let Some(connection) = owner.as_ref() else {
                        return StageOutcome::Failed(ConnectionDiagnosticCode::ConnectionLost);
                    };
                    let (report, binding) =
                        crate::docker::probe::run(connection, &self.docker_options).await;
                    *self.docker_binding.lock().await = binding;
                    StageOutcome::Probed(Box::new(report))
                }
            }
        })
    }
    fn verify_compose<'a>(
        &'a self,
        request: &'a VerifyComposeRequest,
    ) -> StageFuture<'a, Result<crate::docker::compose_actions::VerifiedRead, AppError>> {
        Box::pin(async move {
            let client = self
                .connection
                .lock()
                .await
                .as_ref()
                .map(|c| c.client())
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected))?;
            let binding = self
                .docker_binding
                .lock()
                .await
                .as_ref()
                .cloned()
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected))?;
            crate::docker::compose_actions::verify(&client, &self.docker_options, &binding, request)
                .await
        })
    }
    fn mutate_compose<'a>(
        &'a self,
        request: ComposeMutationRequest,
        expected: crate::docker::compose_actions::VerifiedRead,
        owner: crate::activity::Operation,
        current: crate::docker::mutations::Current,
    ) -> StageFuture<'a, Result<ComposeMutationResponse, AppError>> {
        Box::pin(async move {
            let client = self
                .connection
                .lock()
                .await
                .as_ref()
                .map(|c| c.client())
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected))?;
            let binding = self
                .docker_binding
                .lock()
                .await
                .as_ref()
                .cloned()
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected))?;
            crate::docker::compose_actions::run(
                &client,
                &self.docker_options,
                &binding,
                request,
                expected,
                owner,
                current,
            )
            .await
        })
    }
    fn mutate<'a>(
        &'a self,
        request: MutationRequest,
        owner: crate::activity::Operation,
        current: crate::docker::mutations::Current,
    ) -> StageFuture<'a, Result<MutationResponse, AppError>> {
        Box::pin(async move {
            let client = self
                .connection
                .lock()
                .await
                .as_ref()
                .map(|c| c.client())
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected))?;
            let binding = self
                .docker_binding
                .lock()
                .await
                .as_ref()
                .cloned()
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected))?;
            crate::docker::mutations::run(
                &client,
                &self.docker_options,
                &binding,
                request,
                owner,
                current,
            )
            .await
        })
    }
    fn idle(&self) -> StageFuture<'_, Option<ConnectionDiagnosticCode>> {
        Box::pin(async move {
            let client = self
                .connection
                .lock()
                .await
                .as_ref()
                .map(|owner| owner.client());
            if let Some(client) = client {
                client.wait_lost().await;
            }
            Some(ConnectionDiagnosticCode::ConnectionLost)
        })
    }
    fn list<'a>(
        &'a self,
        scope: &'a SessionScope,
    ) -> StageFuture<'a, Result<ListContainersResponse, AppError>> {
        Box::pin(async move {
            let client = self
                .connection
                .lock()
                .await
                .as_ref()
                .map(|owner| owner.client())
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
            let binding = self
                .docker_binding
                .lock()
                .await
                .as_ref()
                .cloned()
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
            crate::docker::listing::read(&client, &self.docker_options, &binding, scope).await
        })
    }
    fn networks<'a>(
        &'a self,
        request: &'a ListNetworksRequest,
    ) -> StageFuture<'a, Result<ListNetworksResponse, AppError>> {
        Box::pin(async move {
            let scope = &request.scope;
            let client = self
                .connection
                .lock()
                .await
                .as_ref()
                .map(|owner| owner.client())
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
            let binding = self
                .docker_binding
                .lock()
                .await
                .as_ref()
                .cloned()
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
            crate::docker::networks::list(&client, &self.docker_options, &binding, scope).await
        })
    }
    fn inspect_network<'a>(
        &'a self,
        request: &'a InspectNetworkRequest,
    ) -> StageFuture<'a, Result<NetworkDetail, AppError>> {
        Box::pin(async move {
            let scope = &request.scope;
            let client = self
                .connection
                .lock()
                .await
                .as_ref()
                .map(|owner| owner.client())
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
            let binding = self
                .docker_binding
                .lock()
                .await
                .as_ref()
                .cloned()
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
            crate::docker::networks::inspect(&client, &self.docker_options, &binding, request).await
        })
    }
    fn volumes<'a>(
        &'a self,
        request: &'a ListVolumesRequest,
    ) -> StageFuture<'a, Result<ListVolumesResponse, AppError>> {
        Box::pin(async move {
            let scope = &request.scope;
            let client = self
                .connection
                .lock()
                .await
                .as_ref()
                .map(|owner| owner.client())
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
            let binding = self
                .docker_binding
                .lock()
                .await
                .as_ref()
                .cloned()
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
            crate::docker::volumes::list(&client, &self.docker_options, &binding, scope).await
        })
    }
    fn inspect_volume<'a>(
        &'a self,
        request: &'a InspectVolumeRequest,
    ) -> StageFuture<'a, Result<VolumeDetail, AppError>> {
        Box::pin(async move {
            let scope = &request.scope;
            let client = self
                .connection
                .lock()
                .await
                .as_ref()
                .map(|owner| owner.client())
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
            let binding = self
                .docker_binding
                .lock()
                .await
                .as_ref()
                .cloned()
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
            crate::docker::volumes::inspect(&client, &self.docker_options, &binding, request).await
        })
    }
    fn images<'a>(
        &'a self,
        request: &'a ListImagesRequest,
    ) -> StageFuture<'a, Result<ListImagesResponse, AppError>> {
        Box::pin(async move {
            let scope = &request.scope;
            let client = self
                .connection
                .lock()
                .await
                .as_ref()
                .map(|owner| owner.client())
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
            let binding = self
                .docker_binding
                .lock()
                .await
                .as_ref()
                .cloned()
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
            crate::docker::images::list(&client, &self.docker_options, &binding, request).await
        })
    }
    fn inspect_image<'a>(
        &'a self,
        request: &'a InspectImageRequest,
    ) -> StageFuture<'a, Result<ImageDetail, AppError>> {
        Box::pin(async move {
            let scope = &request.scope;
            let client = self
                .connection
                .lock()
                .await
                .as_ref()
                .map(|owner| owner.client())
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
            let binding = self
                .docker_binding
                .lock()
                .await
                .as_ref()
                .cloned()
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
            crate::docker::images::inspect(&client, &self.docker_options, &binding, request).await
        })
    }
    fn compose<'a>(
        &'a self,
        scope: &'a SessionScope,
    ) -> StageFuture<'a, Result<ListComposeResponse, AppError>> {
        Box::pin(async move {
            let client = self
                .connection
                .lock()
                .await
                .as_ref()
                .map(|owner| owner.client())
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
            let binding = self
                .docker_binding
                .lock()
                .await
                .as_ref()
                .cloned()
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
            crate::docker::compose::read(&client, &self.docker_options, &binding, scope).await
        })
    }
    fn inspect<'a>(
        &'a self,
        request: &'a InspectContainerRequest,
    ) -> StageFuture<'a, Result<ContainerDetail, AppError>> {
        Box::pin(async move {
            let client = self
                .connection
                .lock()
                .await
                .as_ref()
                .map(|owner| owner.client())
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(&request.scope))?;
            let binding = self
                .docker_binding
                .lock()
                .await
                .as_ref()
                .cloned()
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(&request.scope))?;
            crate::docker::inspect::read(&client, &self.docker_options, &binding, request).await
        })
    }
    fn stats<'a>(
        &'a self,
        request: &'a ContainerStatsRequest,
    ) -> StageFuture<'a, Result<StatsSample, AppError>> {
        Box::pin(async move {
            let client = self
                .connection
                .lock()
                .await
                .as_ref()
                .map(|owner| owner.client())
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(&request.scope))?;
            let binding = self
                .docker_binding
                .lock()
                .await
                .as_ref()
                .cloned()
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(&request.scope))?;
            crate::docker::stats::read(&client, &self.docker_options, &binding, request).await
        })
    }
    fn logs<'a>(
        &'a self,
        request: &'a ContainerLogsRequest,
    ) -> StageFuture<'a, Result<LogSnapshot, AppError>> {
        Box::pin(async move {
            let client = self
                .connection
                .lock()
                .await
                .as_ref()
                .map(|owner| owner.client())
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(&request.scope))?;
            let binding = self
                .docker_binding
                .lock()
                .await
                .as_ref()
                .cloned()
                .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(&request.scope))?;
            crate::docker::logs::read(&client, &self.docker_options, &binding, request).await
        })
    }
    fn follow_logs<'a>(
        &'a self,
        request: &'a FollowLogsRequest,
        sink: super::runner::streaming::LineSink,
    ) -> StageFuture<'a, Result<super::runner::Job, AppError>> {
        Box::pin(async move {
            tokio::time::timeout(std::time::Duration::from_secs(30), async {
                let client = self
                    .connection
                    .lock()
                    .await
                    .as_ref()
                    .map(|owner| owner.client())
                    .ok_or_else(|| AppError::new(ErrorCode::Disconnected))?;
                let binding = self
                    .docker_binding
                    .lock()
                    .await
                    .as_ref()
                    .cloned()
                    .ok_or_else(|| AppError::new(ErrorCode::Disconnected))?;
                let (fresh, _) = crate::docker::probe::run(&client, &self.docker_options).await;
                if fresh.daemon_id.as_deref() != Some(request.scope.daemon_id.as_str()) {
                    return Err(AppError::new(ErrorCode::StaleSession));
                }
                let plan = crate::policy::registry::read(
                    &crate::policy::registry::ReadOperation::FollowLogs {
                        container_id: request.container_id.clone(),
                        tail: request.tail,
                        since: request.since.clone(),
                    },
                )?;
                let command = binding.prepare(plan, &fresh)?;
                client.start_log_stream(command.encoded().into(), sink)
            })
            .await
            .map_err(|_| AppError::new(ErrorCode::OperationTimedOut))?
            .map_err(|e| e.in_scope(&request.scope))
        })
    }
    fn follow_events<'a>(
        &'a self,
        request: &'a FollowEventsRequest,
        sink: super::runner::streaming::LineSink,
    ) -> StageFuture<'a, Result<super::runner::Job, AppError>> {
        Box::pin(async move {
            tokio::time::timeout(std::time::Duration::from_secs(30), async {
                let client = self
                    .connection
                    .lock()
                    .await
                    .as_ref()
                    .map(|owner| owner.client())
                    .ok_or_else(|| AppError::new(ErrorCode::Disconnected))?;
                let binding = self
                    .docker_binding
                    .lock()
                    .await
                    .as_ref()
                    .cloned()
                    .ok_or_else(|| AppError::new(ErrorCode::Disconnected))?;
                let (fresh, _) = crate::docker::probe::run(&client, &self.docker_options).await;
                if fresh.daemon_id.as_deref() != Some(request.scope.daemon_id.as_str()) {
                    return Err(AppError::new(ErrorCode::StaleSession));
                }
                let plan = crate::policy::registry::read(
                    &crate::policy::registry::ReadOperation::FollowEvents {
                        since: request.since.clone(),
                    },
                )?;
                let command = binding.prepare(plan, &fresh)?;
                client.start_log_stream(command.encoded().into(), sink)
            })
            .await
            .map_err(|_| AppError::new(ErrorCode::OperationTimedOut))?
            .map_err(|e| e.in_scope(&request.scope))
        })
    }
    fn quiesce(&self) -> StageFuture<'_, ()> {
        Box::pin(async move {
            self.docker_binding.lock().await.take();
            if let Some(connection) = self.connection.lock().await.take() {
                connection.close().await;
            }
            self.runner.wait_idle().await;
        })
    }
}
#[derive(Default)]
struct State {
    generation: u32,
    snapshot: Option<ConnectionSnapshot>,
}
struct Worker {
    driver: std::sync::Weak<dyn StageDriver>,
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
        docker_options: DockerOptions,
        factory: impl FnOnce() -> Result<Arc<dyn StageDriver>, AppError>,
    ) -> Result<ConnectionSnapshot, AppError> {
        self.begin_owned(selection, docker_options, None, factory)
            .await
    }
    pub(crate) async fn begin_owned(
        &self,
        selection: SshSelection,
        docker_options: DockerOptions,
        host_id: Option<HostId>,
        factory: impl FnOnce() -> Result<Arc<dyn StageDriver>, AppError>,
    ) -> Result<ConnectionSnapshot, AppError> {
        if let Some(id) = &host_id {
            id.validate()?;
        }
        super::resolver::arguments(&selection)?;
        crate::docker::DockerCommandConfig::from_options(&docker_options)?;
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
                && snapshot.host_id == host_id
                && snapshot.selection == selection
                && snapshot.docker_options == docker_options
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
                host_id,
                effective: None,
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
                transport_mode: SshTransportMode::Unconnected,
                docker_options,
                docker: None,
            };
            state.snapshot = Some(snapshot.clone());
            snapshot
        };
        let (cancel, cancelled) = oneshot::channel();
        let state = self.state.clone();
        let initial = snapshot.clone();
        let weak = Arc::downgrade(&driver);
        let task = tokio::spawn(async move {
            drive(state, initial, driver, cancelled).await;
        });
        *control = Some(Worker {
            cancel,
            task,
            driver: weak,
        });
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
            snapshot.transport_mode = SshTransportMode::Unconnected;
            snapshot.docker = None;
        }
        Ok(())
    }
    pub fn current(&self) -> Result<Option<ConnectionSnapshot>, AppError> {
        Ok(self
            .state
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .snapshot
            .clone())
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
    pub fn require_scope(&self, scope: &SessionScope) -> Result<(), AppError> {
        scope.validate()?;
        let current = self.snapshot(&ConnectionToken {
            session_id: scope.session_id.clone(),
            session_generation: scope.session_generation,
        })?;
        if current.host_id.as_ref() != Some(&scope.selection.host_id)
            || current.state != ConnectionState::Ready
            || current.docker.as_ref().and_then(|d| d.daemon_id.as_deref())
                != Some(scope.daemon_id.as_str())
        {
            return Err(AppError::new(ErrorCode::StaleSession).in_scope(scope));
        }
        Ok(())
    }
    pub(crate) async fn verify_compose(
        &self,
        request: &VerifyComposeRequest,
    ) -> Result<crate::docker::compose_actions::VerifiedRead, AppError> {
        self.require_scope(&request.scope)?;
        let driver = self
            .control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?
            .as_ref()
            .and_then(|w| w.driver.upgrade())
            .ok_or_else(|| AppError::new(ErrorCode::Disconnected))?;
        let result = driver.verify_compose(request).await;
        self.require_scope(&request.scope)?;
        if result
            .as_ref()
            .is_err_and(|e| e.code == ErrorCode::StaleSession)
        {
            drop(driver);
            let _ = self
                .disconnect(&ConnectionToken {
                    session_id: request.scope.session_id.clone(),
                    session_generation: request.scope.session_generation,
                })
                .await;
        }
        result
    }
    pub(crate) async fn mutate_compose(
        &self,
        request: ComposeMutationRequest,
        expected: crate::docker::compose_actions::VerifiedRead,
        owner: crate::activity::Operation,
        current: crate::docker::mutations::Current,
    ) -> Result<ComposeMutationResponse, AppError> {
        self.require_scope(&request.scope)?;
        let driver = self
            .control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?
            .as_ref()
            .and_then(|w| w.driver.upgrade())
            .ok_or_else(|| AppError::new(ErrorCode::Disconnected))?;
        let scope = request.scope.clone();
        let result = driver
            .mutate_compose(request, expected, owner, current)
            .await;
        if result
            .as_ref()
            .is_err_and(|e| e.code == ErrorCode::StaleSession)
            || result.as_ref().is_ok_and(|r| {
                r.results
                    .iter()
                    .any(|r| r.error == Some(ErrorCode::StaleSession))
            })
        {
            drop(driver);
            let _ = self
                .disconnect(&ConnectionToken {
                    session_id: scope.session_id,
                    session_generation: scope.session_generation,
                })
                .await;
        }
        result
    }
    pub(crate) async fn mutate(
        &self,
        request: MutationRequest,
        owner: crate::activity::Operation,
        current: crate::docker::mutations::Current,
    ) -> Result<MutationResponse, AppError> {
        self.require_scope(&request.scope)?;
        let driver = self
            .control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?
            .as_ref()
            .and_then(|worker| worker.driver.upgrade())
            .ok_or_else(|| AppError::new(ErrorCode::Disconnected))?;
        let scope = request.scope.clone();
        // After dispatch, preserve Unknown even if disconnect invalidated this scope.
        let result = driver.mutate(request, owner, current).await;
        if result
            .as_ref()
            .is_err_and(|error| error.code == ErrorCode::StaleSession)
            || result.as_ref().is_ok_and(|response| {
                response
                    .results
                    .iter()
                    .any(|r| r.error == Some(ErrorCode::StaleSession))
            })
        {
            drop(driver);
            let _ = self
                .disconnect(&ConnectionToken {
                    session_id: scope.session_id,
                    session_generation: scope.session_generation,
                })
                .await;
        }
        result
    }
    pub async fn list(&self, scope: &SessionScope) -> Result<ListContainersResponse, AppError> {
        self.require_scope(scope)?;
        let driver = self
            .control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?
            .as_ref()
            .and_then(|worker| worker.driver.upgrade())
            .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
        let result = driver.list(scope).await;
        self.require_scope(scope)?;
        if result
            .as_ref()
            .is_err_and(|error| error.code == ErrorCode::StaleSession)
        {
            // Actual daemon/context drift revokes this read session. Foreign requests fail before dispatch.
            drop(driver);
            let _ = self
                .disconnect(&ConnectionToken {
                    session_id: scope.session_id.clone(),
                    session_generation: scope.session_generation,
                })
                .await;
        }
        result
    }
    pub async fn networks(
        &self,
        request: &ListNetworksRequest,
    ) -> Result<ListNetworksResponse, AppError> {
        let scope = &request.scope;
        self.require_scope(scope)?;
        let driver = self
            .control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?
            .as_ref()
            .and_then(|worker| worker.driver.upgrade())
            .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
        let result = driver.networks(request).await;
        self.require_scope(scope)?;
        if result
            .as_ref()
            .is_err_and(|error| error.code == ErrorCode::StaleSession)
        {
            drop(driver);
            let _ = self
                .disconnect(&ConnectionToken {
                    session_id: scope.session_id.clone(),
                    session_generation: scope.session_generation,
                })
                .await;
        }
        result
    }
    pub async fn inspect_network(
        &self,
        request: &InspectNetworkRequest,
    ) -> Result<NetworkDetail, AppError> {
        let scope = &request.scope;
        self.require_scope(scope)?;
        let driver = self
            .control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?
            .as_ref()
            .and_then(|worker| worker.driver.upgrade())
            .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
        let result = driver.inspect_network(request).await;
        self.require_scope(scope)?;
        if result
            .as_ref()
            .is_err_and(|error| error.code == ErrorCode::StaleSession)
        {
            drop(driver);
            let _ = self
                .disconnect(&ConnectionToken {
                    session_id: scope.session_id.clone(),
                    session_generation: scope.session_generation,
                })
                .await;
        }
        result
    }
    pub async fn volumes(
        &self,
        request: &ListVolumesRequest,
    ) -> Result<ListVolumesResponse, AppError> {
        let scope = &request.scope;
        self.require_scope(scope)?;
        let driver = self
            .control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?
            .as_ref()
            .and_then(|worker| worker.driver.upgrade())
            .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
        let result = driver.volumes(request).await;
        self.require_scope(scope)?;
        if result
            .as_ref()
            .is_err_and(|error| error.code == ErrorCode::StaleSession)
        {
            drop(driver);
            let _ = self
                .disconnect(&ConnectionToken {
                    session_id: scope.session_id.clone(),
                    session_generation: scope.session_generation,
                })
                .await;
        }
        result
    }
    pub async fn inspect_volume(
        &self,
        request: &InspectVolumeRequest,
    ) -> Result<VolumeDetail, AppError> {
        let scope = &request.scope;
        self.require_scope(scope)?;
        let driver = self
            .control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?
            .as_ref()
            .and_then(|worker| worker.driver.upgrade())
            .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
        let result = driver.inspect_volume(request).await;
        self.require_scope(scope)?;
        if result
            .as_ref()
            .is_err_and(|error| error.code == ErrorCode::StaleSession)
        {
            drop(driver);
            let _ = self
                .disconnect(&ConnectionToken {
                    session_id: scope.session_id.clone(),
                    session_generation: scope.session_generation,
                })
                .await;
        }
        result
    }
    pub async fn images(
        &self,
        request: &ListImagesRequest,
    ) -> Result<ListImagesResponse, AppError> {
        let scope = &request.scope;
        self.require_scope(scope)?;
        let driver = self
            .control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?
            .as_ref()
            .and_then(|worker| worker.driver.upgrade())
            .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
        let result = driver.images(request).await;
        self.require_scope(scope)?;
        if result
            .as_ref()
            .is_err_and(|error| error.code == ErrorCode::StaleSession)
        {
            drop(driver);
            let _ = self
                .disconnect(&ConnectionToken {
                    session_id: scope.session_id.clone(),
                    session_generation: scope.session_generation,
                })
                .await;
        }
        result
    }
    pub async fn inspect_image(
        &self,
        request: &InspectImageRequest,
    ) -> Result<ImageDetail, AppError> {
        let scope = &request.scope;
        self.require_scope(scope)?;
        let driver = self
            .control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?
            .as_ref()
            .and_then(|worker| worker.driver.upgrade())
            .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
        let result = driver.inspect_image(request).await;
        self.require_scope(scope)?;
        if result
            .as_ref()
            .is_err_and(|error| error.code == ErrorCode::StaleSession)
        {
            drop(driver);
            let _ = self
                .disconnect(&ConnectionToken {
                    session_id: scope.session_id.clone(),
                    session_generation: scope.session_generation,
                })
                .await;
        }
        result
    }
    pub async fn compose(&self, scope: &SessionScope) -> Result<ListComposeResponse, AppError> {
        self.require_scope(scope)?;
        let driver = self
            .control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?
            .as_ref()
            .and_then(|worker| worker.driver.upgrade())
            .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
        let result = driver.compose(scope).await;
        self.require_scope(scope)?;
        if result
            .as_ref()
            .is_err_and(|error| error.code == ErrorCode::StaleSession)
        {
            // Actual daemon/context drift revokes this read session. Foreign requests fail before dispatch.
            drop(driver);
            let _ = self
                .disconnect(&ConnectionToken {
                    session_id: scope.session_id.clone(),
                    session_generation: scope.session_generation,
                })
                .await;
        }
        result
    }
    pub async fn inspect(
        &self,
        request: &InspectContainerRequest,
    ) -> Result<ContainerDetail, AppError> {
        let scope = &request.scope;
        self.require_scope(scope)?;
        let driver = self
            .control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?
            .as_ref()
            .and_then(|worker| worker.driver.upgrade())
            .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
        let result = driver.inspect(request).await;
        self.require_scope(scope)?;
        if result
            .as_ref()
            .is_err_and(|error| error.code == ErrorCode::StaleSession)
        {
            // Actual daemon/context drift revokes this read session. Foreign requests fail before dispatch.
            drop(driver);
            let _ = self
                .disconnect(&ConnectionToken {
                    session_id: scope.session_id.clone(),
                    session_generation: scope.session_generation,
                })
                .await;
        }
        result
    }
    pub async fn logs(&self, request: &ContainerLogsRequest) -> Result<LogSnapshot, AppError> {
        let scope = &request.scope;
        self.require_scope(scope)?;
        let driver = self
            .control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?
            .as_ref()
            .and_then(|worker| worker.driver.upgrade())
            .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
        let result = driver.logs(request).await;
        self.require_scope(scope)?;
        if result
            .as_ref()
            .is_err_and(|error| error.code == ErrorCode::StaleSession)
        {
            // Actual daemon/context drift revokes this read session. Foreign requests fail before dispatch.
            drop(driver);
            let _ = self
                .disconnect(&ConnectionToken {
                    session_id: scope.session_id.clone(),
                    session_generation: scope.session_generation,
                })
                .await;
        }
        result
    }
    pub async fn stats(&self, request: &ContainerStatsRequest) -> Result<StatsSample, AppError> {
        let scope = &request.scope;
        self.require_scope(scope)?;
        let driver = self
            .control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?
            .as_ref()
            .and_then(|worker| worker.driver.upgrade())
            .ok_or_else(|| AppError::new(ErrorCode::Disconnected).in_scope(scope))?;
        let result = driver.stats(request).await;
        self.require_scope(scope)?;
        if result
            .as_ref()
            .is_err_and(|error| error.code == ErrorCode::StaleSession)
        {
            // Actual daemon/context drift revokes this read session. Foreign requests fail before dispatch.
            drop(driver);
            let _ = self
                .disconnect(&ConnectionToken {
                    session_id: scope.session_id.clone(),
                    session_generation: scope.session_generation,
                })
                .await;
        }
        result
    }
    pub(crate) async fn follow_logs(
        &self,
        request: &FollowLogsRequest,
        sink: super::runner::streaming::LineSink,
    ) -> Result<super::runner::Job, AppError> {
        self.require_scope(&request.scope)?;
        let driver = self
            .control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?
            .as_ref()
            .and_then(|w| w.driver.upgrade())
            .ok_or_else(|| AppError::new(ErrorCode::Disconnected))?;
        let result = driver.follow_logs(request, sink).await;
        self.require_scope(&request.scope)?;
        if result
            .as_ref()
            .is_err_and(|error| error.code == ErrorCode::StaleSession)
        {
            drop(driver);
            let _ = self
                .disconnect(&ConnectionToken {
                    session_id: request.scope.session_id.clone(),
                    session_generation: request.scope.session_generation,
                })
                .await;
        }
        result
    }
    pub(crate) async fn follow_events(
        &self,
        request: &FollowEventsRequest,
        sink: super::runner::streaming::LineSink,
    ) -> Result<super::runner::Job, AppError> {
        self.require_scope(&request.scope)?;
        let driver = self
            .control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?
            .as_ref()
            .and_then(|w| w.driver.upgrade())
            .ok_or_else(|| AppError::new(ErrorCode::Disconnected))?;
        let result = driver.follow_events(request, sink).await;
        self.require_scope(&request.scope)?;
        if result
            .as_ref()
            .is_err_and(|error| error.code == ErrorCode::StaleSession)
        {
            drop(driver);
            let _ = self
                .disconnect(&ConnectionToken {
                    session_id: request.scope.session_id.clone(),
                    session_generation: request.scope.session_generation,
                })
                .await;
        }
        result
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
    pub async fn shutdown(&self) {
        let mut control = self.control.lock().await;
        let _ = self.invalidate();
        stop(&mut control).await;
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
            (
                ConnectionStage::Resolve,
                StageOutcome::Resolved {
                    has_jump,
                    effective,
                },
            ) => {
                current.has_jump = has_jump;
                current.effective = effective.map(|config| *config);
                current.state = ConnectionState::Connecting;
            }
            (ConnectionStage::Authenticate, StageOutcome::Authenticated(mode)) => {
                current.transport_mode = mode;
                current.state = ConnectionState::Probing
            }
            #[cfg(test)]
            (ConnectionStage::Probe, StageOutcome::Ready) => current.state = ConnectionState::Ready,
            (ConnectionStage::Probe, StageOutcome::Probed(report)) => {
                current.state = if report.status == DockerProbeStatus::Ready {
                    ConnectionState::Ready
                } else {
                    ConnectionState::Degraded
                };
                current.diagnostic =
                    (report.status != DockerProbeStatus::Ready).then_some(ConnectionDiagnostic {
                        stage,
                        code: ConnectionDiagnosticCode::DockerUnavailable,
                    });
                current.docker = Some(*report);
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
    let hold = state.lock().is_ok_and(|state| {
        state.snapshot.as_ref().is_some_and(|current| {
            current.token == initial.token
                && matches!(
                    current.state,
                    ConnectionState::Ready | ConnectionState::Degraded
                )
        })
    });
    if hold {
        let diagnostic =
            tokio::select! { biased; _ = &mut cancelled => None, code = driver.idle() => code };
        if let Some(code) = diagnostic
            && let Ok(mut state) = state.lock()
            && let Some(current) = &mut state.snapshot
            && current.token == initial.token
        {
            current.state = ConnectionState::Degraded;
            current.diagnostic = Some(ConnectionDiagnostic {
                stage: ConnectionStage::Probe,
                code,
            });
            current.transport_mode = SshTransportMode::Unconnected;
        }
    }
    // Dropped stage futures cancel Runner Jobs. Hold the native gate until their owners reap.
    driver.quiesce().await;
    if let Ok(mut state) = state.lock()
        && let Some(current) = &mut state.snapshot
        && current.token == initial.token
    {
        current.transport_mode = SshTransportMode::Unconnected;
    }
}
#[cfg(test)]
mod tests;
