use crate::policy::{PolicyEngine, registry::ReadOperation};
use crate::transport::{WorkspaceTransport, demo_host, fixtures::FixtureTransport};
use crate::{
    domain::*,
    storage::{FileStorage, SettingsStore},
};
use std::sync::{Arc, Mutex};

/// Resource operations remain behind identity gates. Local version probes are separately bounded.
pub struct Backend {
    process_runner: crate::ssh::runner::Runner,
    sessions: crate::ssh::sessions::Sessions,
    config_home: std::path::PathBuf,
    policy: Mutex<PolicyEngine>,
    settings: Mutex<Result<SettingsStore, AppError>>,
    diagnostic_slot: Arc<tokio::sync::Semaphore>,
    workspace: Mutex<WorkspaceTransport>,
    read_slots: tokio::sync::Semaphore,
}
impl Backend {
    pub fn new(app_data: &std::path::Path, config_home: std::path::PathBuf) -> Self {
        let settings = FileStorage::open(app_data)
            .map_err(|_| AppError::new(ErrorCode::StorageUnavailable))
            .and_then(|adapter| SettingsStore::load(Box::new(adapter)));
        Self {
            process_runner: crate::ssh::runner::Runner::default(),
            sessions: crate::ssh::sessions::Sessions::default(),
            config_home,
            policy: Mutex::new(PolicyEngine::default()),
            settings: Mutex::new(settings),
            diagnostic_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            workspace: Mutex::new(WorkspaceTransport::default()),
            read_slots: tokio::sync::Semaphore::new(4),
        }
    }
    pub fn config_path(&self, path: Option<&str>) -> Result<SshConfigPath, AppError> {
        let preferences = self.preferences()?;
        let path = path.or(preferences.preferences.trusted_config_path.as_deref());
        Ok(SshConfigPath {
            path: crate::ssh::discovery::config_path(&self.config_home, path)?,
        })
    }
    pub fn select_alias(&self, request: SelectSshAliasRequest) -> Result<SshSelection, AppError> {
        self.require_live_mode()?;
        crate::ssh::validate_alias(&request.alias)?;
        let preferences = self.preferences()?.preferences;
        let configured = request
            .config_path
            .as_deref()
            .or(preferences.trusted_config_path.as_deref());
        Ok(SshSelection {
            use_default_config: configured.is_none(),
            config_path: crate::ssh::discovery::config_path(&self.config_home, configured)?,
            alias: request.alias,
        })
    }
    pub async fn resolve_ssh(
        &self,
        request: ResolveSshRequest,
    ) -> Result<EffectiveSshConfig, AppError> {
        let permit = self
            .diagnostic_slot
            .clone()
            .try_acquire_owned()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
        self.require_live_mode()?;
        crate::ssh::validate_alias(&request.selection.alias)?;
        let path = crate::ssh::discovery::config_path(
            &self.config_home,
            Some(&request.selection.config_path),
        )?;
        if request.selection.use_default_config
            && path != crate::ssh::discovery::config_path(&self.config_home, None)?
        {
            return Err(AppError::new(ErrorCode::InvalidConfigPath));
        }
        let preferences = self.preferences()?.preferences;
        let executable = preferences
            .ssh_executable_override
            .unwrap_or_else(|| crate::diagnostics::DEFAULT_SSH.into());
        let runner = self.process_runner.clone();
        // The owner keeps its permit through bounded completion/reaping even if the IPC caller disappears.
        tauri::async_runtime::spawn(async move {
            let _permit = permit;
            crate::ssh::resolver::resolve(&runner, &executable, request.selection).await
        })
        .await
        .map_err(|_| AppError::new(ErrorCode::Internal))?
    }
    pub async fn check_ssh_access(
        &self,
        request: ResolveSshRequest,
    ) -> Result<SshAccessReport, AppError> {
        let permit = self
            .diagnostic_slot
            .clone()
            .try_acquire_owned()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
        self.require_live_mode()?;
        crate::ssh::validate_alias(&request.selection.alias)?;
        let path = crate::ssh::discovery::config_path(
            &self.config_home,
            Some(&request.selection.config_path),
        )?;
        if request.selection.use_default_config
            && path != crate::ssh::discovery::config_path(&self.config_home, None)?
        {
            return Err(AppError::new(ErrorCode::InvalidConfigPath));
        }
        let preferences = self.preferences()?.preferences;
        let executable = preferences
            .ssh_executable_override
            .unwrap_or_else(|| crate::diagnostics::DEFAULT_SSH.into());
        let runner = self.process_runner.clone();
        // The owner keeps its permit through bounded completion/reaping even if the IPC caller disappears.
        tauri::async_runtime::spawn(async move {
            let _permit = permit;
            crate::ssh::auth::probe(&runner, &executable, request.selection).await
        })
        .await
        .map_err(|_| AppError::new(ErrorCode::Internal))?
    }

    pub async fn begin_ssh_session(
        &self,
        request: ResolveSshRequest,
    ) -> Result<ConnectionSnapshot, AppError> {
        self.require_live_mode()?;
        crate::ssh::resolver::arguments(&request.selection)?;
        if request.selection.use_default_config
            && request.selection.config_path
                != crate::ssh::discovery::config_path(&self.config_home, None)?
        {
            return Err(AppError::new(ErrorCode::InvalidConfigPath));
        }
        self.sessions
            .begin(request.selection, || {
                let permit = self
                    .diagnostic_slot
                    .clone()
                    .try_acquire_owned()
                    .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
                self.require_live_mode()?;
                let executable = self
                    .preferences()?
                    .preferences
                    .ssh_executable_override
                    .unwrap_or_else(|| crate::diagnostics::DEFAULT_SSH.into());
                Ok(Arc::new(crate::ssh::sessions::NativeDriver {
                    runner: self.process_runner.clone(),
                    executable,
                    _permit: permit,
                }))
            })
            .await
    }
    pub fn ssh_session(&self, request: ConnectionRequest) -> Result<ConnectionSnapshot, AppError> {
        self.require_live_mode()?;
        self.sessions.snapshot(&request.token)
    }
    pub async fn disconnect_ssh_session(
        &self,
        request: ConnectionRequest,
    ) -> Result<ConnectionSnapshot, AppError> {
        self.require_live_mode()?;
        self.sessions.disconnect(&request.token).await
    }
    pub async fn discover_hosts(
        &self,
        request: DiscoverHostsRequest,
    ) -> Result<HostDiscovery, AppError> {
        // Share the native probe gate so switching to demo cannot race discovery.
        let permit = self
            .diagnostic_slot
            .clone()
            .try_acquire_owned()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
        self.require_live_mode()?;
        let path = self.config_path(request.config_path.as_deref())?.path;
        let home = self.config_home.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let _permit = permit;
            crate::ssh::discovery::discover(&home, &path)
        })
        .await
        .map_err(|_| AppError::new(ErrorCode::Internal))?
    }
    pub fn preferences(&self) -> Result<PreferencesSnapshot, AppError> {
        self.settings
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .as_ref()
            .map(SettingsStore::snapshot)
            .map_err(Clone::clone)
    }
    pub fn set_theme(&self, request: SetThemeRequest) -> Result<PreferencesSnapshot, AppError> {
        self.settings
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .as_mut()
            .map_err(|e| e.clone())?
            .set_theme(request)
    }
    pub async fn diagnostics(&self) -> Result<DependencyDiagnostics, AppError> {
        let _permit = self
            .diagnostic_slot
            .try_acquire()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
        self.require_live_mode()?;
        let preferences = self.preferences()?;
        let path = preferences
            .preferences
            .ssh_executable_override
            .as_deref()
            .unwrap_or(crate::diagnostics::DEFAULT_SSH);
        let socket = std::env::var_os("SSH_AUTH_SOCK").map(std::path::PathBuf::from);
        let (ssh, agent) = tokio::join!(
            crate::diagnostics::inspect_ssh(path),
            crate::diagnostics::inspect_agent(socket.as_deref())
        );
        Ok(DependencyDiagnostics {
            app_version: env!("CARGO_PKG_VERSION").into(),
            platform: std::env::consts::OS.into(),
            architecture: std::env::consts::ARCH.into(),
            ssh,
            agent,
        })
    }
    pub async fn set_ssh_executable(
        &self,
        request: SetSshExecutableRequest,
    ) -> Result<SetSshExecutableResponse, AppError> {
        let _permit = self
            .diagnostic_slot
            .try_acquire()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
        self.require_live_mode()?;
        let mut preferences = self.preferences()?.preferences;
        if preferences.revision != request.expected_revision {
            return Err(AppError::new(ErrorCode::StorageConflict));
        }
        let ssh = crate::diagnostics::inspect_ssh(
            request
                .path
                .as_deref()
                .unwrap_or(crate::diagnostics::DEFAULT_SSH),
        )
        .await;
        if ssh.status != SshStatus::Ready {
            return Ok(SetSshExecutableResponse {
                preferences: None,
                ssh,
            });
        }
        preferences.ssh_executable_override = request.path;
        let saved = self
            .settings
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .as_mut()
            .map_err(|e| e.clone())?
            .replace(preferences, request.expected_revision)?;
        Ok(SetSshExecutableResponse {
            preferences: Some(saved),
            ssh,
        })
    }

    fn require_live_mode(&self) -> Result<(), AppError> {
        if self
            .workspace
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .snapshot
            .mode
            == WorkspaceMode::Demo
        {
            return Err(AppError::new(ErrorCode::PermissionDenied));
        }
        Ok(())
    }
    pub fn workspace_mode(&self) -> Result<WorkspaceModeSnapshot, AppError> {
        Ok(self
            .workspace
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .snapshot
            .clone())
    }
    pub fn switch_workspace(
        &self,
        request: SwitchWorkspaceRequest,
    ) -> Result<WorkspaceModeSnapshot, AppError> {
        // Mode transitions cannot race an already-running local SSH version probe.
        let _permit = self
            .diagnostic_slot
            .try_acquire()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
        let _reads = self
            .read_slots
            .try_acquire_many(4)
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
        let mut workspace = self
            .workspace
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?;
        let mut policy = self
            .policy
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?;
        let generation = workspace
            .generation
            .checked_add(1)
            .ok_or_else(|| AppError::new(ErrorCode::ResourceLimit))?;
        let (snapshot, active): (
            WorkspaceModeSnapshot,
            Arc<dyn crate::transport::ReadTransport>,
        ) = match request {
            SwitchWorkspaceRequest::Live => (
                WorkspaceModeSnapshot {
                    mode: WorkspaceMode::Live,
                    scenario: None,
                    scope: None,
                    host: None,
                },
                workspace.live.clone(),
            ),
            SwitchWorkspaceRequest::Demo { scenario } => {
                let host = demo_host();
                let mut bytes = [0u8; 16];
                getrandom::fill(&mut bytes).map_err(|_| AppError::new(ErrorCode::Internal))?;
                let scope = SessionScope {
                    selection: HostSelection {
                        host_id: host.id.clone(),
                        selection_generation: generation,
                    },
                    session_id: SessionId(format!(
                        "s_{}",
                        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
                    )),
                    session_generation: generation,
                    daemon_id: "demo-fixture-daemon".into(),
                };
                scope.validate()?;
                (
                    WorkspaceModeSnapshot {
                        mode: WorkspaceMode::Demo,
                        scenario: Some(scenario.clone()),
                        scope: Some(scope),
                        host: Some(host),
                    },
                    Arc::new(FixtureTransport::new(scenario)),
                )
            }
        };
        // The diagnostic gate excludes connection stages and their child cleanup.
        self.sessions.reset_idle()?;
        *policy = PolicyEngine::default();
        if let Some(scope) = &snapshot.scope {
            policy.register(scope.clone())?;
        }
        workspace.generation = generation;
        workspace.active = active;
        workspace.snapshot = snapshot.clone();
        Ok(snapshot)
    }

    pub fn list_hosts(&self) -> Result<ListHostsResponse, AppError> {
        if let Some(host) = self.workspace_mode()?.host {
            return Ok(ListHostsResponse { hosts: vec![host] });
        }
        Ok(ListHostsResponse {
            hosts: self
                .preferences()?
                .preferences
                .hosts
                .into_iter()
                .map(|host| HostSummary {
                    id: host.id,
                    alias: host.alias,
                    display_name: host.display_name,
                    group: host.group,
                    read_only: host.read_only,
                    connection_state: ConnectionState::Disconnected,
                })
                .collect(),
        })
    }

    pub fn connect_host(
        &self,
        request: ConnectHostRequest,
    ) -> Result<ConnectHostResponse, AppError> {
        request.selection.validate()?;
        self.require_live_mode()?;
        if !self
            .preferences()?
            .preferences
            .hosts
            .iter()
            .any(|host| host.id == request.selection.host_id)
        {
            return Err(AppError::new(ErrorCode::HostNotFound));
        }
        Err(AppError::new(ErrorCode::FeatureUnavailable))
    }

    pub fn require_session(&self, scope: &SessionScope) -> Result<(), AppError> {
        self.policy
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .require_session(scope)
    }

    pub async fn list_containers(
        &self,
        request: ListContainersRequest,
    ) -> Result<ListContainersResponse, AppError> {
        let _permit = self
            .read_slots
            .try_acquire()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
        let authorized = self
            .policy
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .authorize_read(&request.scope, &ReadOperation::ListContainers)?;
        let transport = {
            let workspace = self
                .workspace
                .lock()
                .map_err(|_| AppError::new(ErrorCode::Internal))?;
            if workspace.snapshot.mode == WorkspaceMode::Demo
                && workspace.snapshot.scope.as_ref() != Some(&request.scope)
            {
                return Err(AppError::new(ErrorCode::StaleSession).in_scope(&request.scope));
            }
            workspace.active.clone()
        };
        let result = transport
            .list_containers(request.scope.clone(), authorized.into_plan())
            .await;
        self.require_session(&request.scope)?;
        let response = result?;
        if response.scope != request.scope
            || response
                .containers
                .iter()
                .any(|row| row.scope != request.scope)
        {
            return Err(AppError::new(ErrorCode::InvalidResponse).in_scope(&request.scope));
        }
        Ok(response)
    }

    pub fn inspect_container(
        &self,
        request: InspectContainerRequest,
    ) -> Result<ContainerDetail, AppError> {
        self.policy
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .authorize_read(
                &request.scope,
                &ReadOperation::InspectContainer {
                    container_id: request.container_id,
                },
            )?;
        Err(AppError::new(ErrorCode::FeatureUnavailable).in_scope(&request.scope))
    }
    pub fn container_logs(&self, request: ContainerLogsRequest) -> Result<LogSnapshot, AppError> {
        self.policy
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .authorize_read(
                &request.scope,
                &ReadOperation::ContainerLogs {
                    container_id: request.container_id,
                    tail: request.tail,
                    timeout_seconds: request.timeout_seconds,
                },
            )?;
        Err(AppError::new(ErrorCode::FeatureUnavailable).in_scope(&request.scope))
    }
    pub fn prepare_confirmation(
        &self,
        request: PrepareConfirmationRequest,
    ) -> Result<ConfirmationIntent, AppError> {
        self.policy
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .prepare(request)
    }
    pub fn mutate_container(&self, request: MutationRequest) -> Result<MutationResponse, AppError> {
        let authorized = self
            .policy
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .consume(
                &request.scope,
                &request.intent_id,
                &ConfirmationOperation::Mutation(request.spec),
            )?;
        // A future dispatcher must accept the owned authorization and never replay it.
        // No remote transport exists yet; do not report a synthetic mutation success.
        Err(AppError::new(ErrorCode::FeatureUnavailable).in_scope(authorized.scope()))
    }
    pub fn open_container_terminal(
        &self,
        request: TerminalRequest,
    ) -> Result<TerminalResponse, AppError> {
        let authorized = self
            .policy
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .consume(
                &request.scope,
                &request.intent_id,
                &ConfirmationOperation::Terminal(request.spec),
            )?;
        Err(AppError::new(ErrorCode::FeatureUnavailable).in_scope(authorized.scope()))
    }
    #[cfg(test)]
    pub(crate) fn register_test_session(&self, scope: SessionScope) {
        self.policy.lock().unwrap().register(scope).unwrap();
    }

    pub fn cancel_subscription(
        &self,
        request: CancelSubscriptionRequest,
    ) -> Result<CancelSubscriptionResponse, AppError> {
        request.subscription_id.validate()?;
        self.require_session(&request.scope)?;
        Err(AppError::new(ErrorCode::SubscriptionNotFound).in_scope(&request.scope))
    }
}

#[cfg(test)]
impl Default for Backend {
    fn default() -> Self {
        Self {
            process_runner: crate::ssh::runner::Runner::default(),
            sessions: crate::ssh::sessions::Sessions::default(),
            config_home: std::path::PathBuf::from("/tmp/containerdesk-unused-home"),
            policy: Mutex::new(PolicyEngine::default()),
            settings: Mutex::new(Ok(crate::storage::tests::memory_store())),
            diagnostic_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            workspace: Mutex::new(WorkspaceTransport::default()),
            read_slots: tokio::sync::Semaphore::new(4),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract_tests::scope;

    #[tokio::test]
    async fn discovery_defaults_and_manual_selection_require_no_connection_and_demo_denies_reads() {
        let backend = Backend::default();
        assert!(
            backend
                .config_path(None)
                .unwrap()
                .path
                .ends_with("/.ssh/config")
        );
        let selected = backend
            .select_alias(SelectSshAliasRequest {
                config_path: Some("/tmp/missing-user-config".into()),
                alias: "manual-wildcard-1".into(),
            })
            .unwrap();
        assert_eq!(selected.alias, "manual-wildcard-1");
        assert!(!selected.use_default_config);
        assert!(backend.list_hosts().unwrap().hosts.is_empty());
        assert_eq!(
            backend
                .select_alias(SelectSshAliasRequest {
                    config_path: None,
                    alias: "-option".into()
                })
                .unwrap_err()
                .code,
            ErrorCode::InvalidAlias
        );
        assert_eq!(
            backend
                .select_alias(SelectSshAliasRequest {
                    config_path: Some("relative".into()),
                    alias: "safe".into()
                })
                .unwrap_err()
                .code,
            ErrorCode::InvalidConfigPath
        );
        let report = backend
            .discover_hosts(DiscoverHostsRequest {
                config_path: Some("/dev/null".into()),
            })
            .await
            .unwrap();
        assert!(report.candidates.is_empty());
        assert_eq!(
            report.warnings[0].code,
            DiscoveryWarningCode::UnreadableFile
        );
        let permit = backend.diagnostic_slot.try_acquire().unwrap();
        assert_eq!(
            backend
                .discover_hosts(DiscoverHostsRequest { config_path: None })
                .await
                .unwrap_err()
                .code,
            ErrorCode::ResourceLimit
        );
        drop(permit);
        backend
            .switch_workspace(SwitchWorkspaceRequest::Demo {
                scenario: DemoScenario::Standard,
            })
            .unwrap();
        assert_eq!(
            backend
                .discover_hosts(DiscoverHostsRequest { config_path: None })
                .await
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        );
        assert_eq!(
            backend
                .select_alias(SelectSshAliasRequest {
                    config_path: None,
                    alias: "safe".into()
                })
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        );
    }

    #[tokio::test]
    async fn resolution_rejects_invalid_default_references_and_demo_before_spawn() {
        let backend = Backend::default();
        let mut selected = backend
            .select_alias(SelectSshAliasRequest {
                config_path: None,
                alias: "fixture".into(),
            })
            .unwrap();
        assert!(selected.use_default_config);
        selected.config_path = "/tmp/forged-default-config".into();
        assert_eq!(
            backend
                .resolve_ssh(ResolveSshRequest {
                    selection: selected.clone()
                })
                .await
                .unwrap_err()
                .code,
            ErrorCode::InvalidConfigPath
        );
        let permit = backend.diagnostic_slot.try_acquire().unwrap();
        assert_eq!(
            backend
                .resolve_ssh(ResolveSshRequest {
                    selection: selected.clone()
                })
                .await
                .unwrap_err()
                .code,
            ErrorCode::ResourceLimit
        );
        drop(permit);
        backend
            .switch_workspace(SwitchWorkspaceRequest::Demo {
                scenario: DemoScenario::Standard,
            })
            .unwrap();
        assert_eq!(
            backend
                .resolve_ssh(ResolveSshRequest {
                    selection: selected
                })
                .await
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        );
    }

    #[tokio::test]
    async fn access_check_rejects_invalid_default_references_and_demo_before_spawn() {
        let backend = Backend::default();
        let mut selected = backend
            .select_alias(SelectSshAliasRequest {
                config_path: None,
                alias: "fixture".into(),
            })
            .unwrap();
        assert!(selected.use_default_config);
        selected.config_path = "/tmp/forged-default-config".into();
        assert_eq!(
            backend
                .check_ssh_access(ResolveSshRequest {
                    selection: selected.clone()
                })
                .await
                .unwrap_err()
                .code,
            ErrorCode::InvalidConfigPath
        );
        let permit = backend.diagnostic_slot.try_acquire().unwrap();
        assert_eq!(
            backend
                .check_ssh_access(ResolveSshRequest {
                    selection: selected.clone()
                })
                .await
                .unwrap_err()
                .code,
            ErrorCode::ResourceLimit
        );
        drop(permit);
        backend
            .switch_workspace(SwitchWorkspaceRequest::Demo {
                scenario: DemoScenario::Standard,
            })
            .unwrap();
        assert_eq!(
            backend
                .check_ssh_access(ResolveSshRequest {
                    selection: selected
                })
                .await
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        );
    }

    struct SpyLive {
        calls: Arc<std::sync::atomic::AtomicUsize>,
    }
    impl crate::transport::ReadTransport for SpyLive {
        fn list_containers(
            &self,
            scope: SessionScope,
            _command: crate::policy::registry::CommandPlan,
        ) -> crate::transport::SnapshotFuture<'_> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Box::pin(
                async move { Err(AppError::new(ErrorCode::PermissionDenied).in_scope(&scope)) },
            )
        }
    }
    #[tokio::test]
    async fn explicit_demo_never_uses_live_transport_and_live_failure_never_uses_fixtures() {
        let backend = Backend::default();
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let spy: Arc<dyn crate::transport::ReadTransport> = Arc::new(SpyLive {
            calls: calls.clone(),
        });
        {
            let mut workspace = backend.workspace.lock().unwrap();
            workspace.live = spy.clone();
            workspace.active = spy;
        }
        backend.register_test_session(scope());
        assert_eq!(
            backend
                .list_containers(ListContainersRequest { scope: scope() })
                .await
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        );
        assert_eq!(backend.workspace_mode().unwrap().mode, WorkspaceMode::Live);
        let demo = backend
            .switch_workspace(SwitchWorkspaceRequest::Demo {
                scenario: DemoScenario::Standard,
            })
            .unwrap();
        let demo_scope = demo.scope.unwrap();
        let snapshot = backend
            .list_containers(ListContainersRequest {
                scope: demo_scope.clone(),
            })
            .await
            .unwrap();
        assert_eq!(snapshot.containers.len(), 4);
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "demo cannot call the live provider"
        );
        assert_eq!(
            backend.diagnostics().await.unwrap_err().code,
            ErrorCode::PermissionDenied
        );
        assert_eq!(
            backend
                .set_ssh_executable(SetSshExecutableRequest {
                    expected_revision: 0,
                    path: Some("/usr/bin/ssh".into())
                })
                .await
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        );
        backend
            .switch_workspace(SwitchWorkspaceRequest::Live)
            .unwrap();
        assert_eq!(
            backend
                .list_containers(ListContainersRequest { scope: demo_scope })
                .await
                .unwrap_err()
                .code,
            ErrorCode::SessionNotFound
        );
        backend.register_test_session(scope());
        assert_eq!(
            backend
                .list_containers(ListContainersRequest { scope: scope() })
                .await
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        );
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
    }
    #[tokio::test]
    async fn demo_blocks_native_executable_before_any_spawn() {
        use std::os::unix::fs::PermissionsExt;
        let directory = std::env::temp_dir().join(format!(
            "containerdesk-demo-spawn-{}-{}",
            std::process::id(),
            crate::test_directory_suffix()
        ));
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        let executable = directory.join("ssh-fixture");
        std::fs::write(
            &executable,
            b"#!/bin/sh\n: > \"${0%/*}/spawned\"\nprintf 'OpenSSH_fixture\\n' >&2\n",
        )
        .unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = executable.to_str().unwrap().to_string();
        assert_eq!(
            crate::diagnostics::inspect_ssh(&path).await.status,
            SshStatus::Ready,
            "positive control must create marker"
        );
        let marker = directory.join("spawned");
        assert!(marker.exists());
        std::fs::remove_file(&marker).unwrap();
        let backend = Backend::default();
        let preferences = Preferences {
            ssh_executable_override: Some(path.clone()),
            ..Preferences::default()
        };
        backend
            .settings
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .replace(preferences, 0)
            .unwrap();
        backend
            .switch_workspace(SwitchWorkspaceRequest::Demo {
                scenario: DemoScenario::Standard,
            })
            .unwrap();
        assert_eq!(
            backend.diagnostics().await.unwrap_err().code,
            ErrorCode::PermissionDenied
        );
        assert_eq!(
            backend
                .set_ssh_executable(SetSshExecutableRequest {
                    expected_revision: 1,
                    path: Some(path)
                })
                .await
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        );
        assert!(!marker.exists(), "no version probe may run in demo mode");
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn demo_switch_waits_for_no_native_probes_or_reads_and_invalidates_old_scenarios() {
        let backend = Backend::default();
        let probe = backend.diagnostic_slot.try_acquire().unwrap();
        assert_eq!(
            backend
                .switch_workspace(SwitchWorkspaceRequest::Demo {
                    scenario: DemoScenario::Standard
                })
                .unwrap_err()
                .code,
            ErrorCode::ResourceLimit
        );
        drop(probe);
        let reads = backend.read_slots.try_acquire_many(4).unwrap();
        assert_eq!(
            backend
                .switch_workspace(SwitchWorkspaceRequest::Demo {
                    scenario: DemoScenario::Standard
                })
                .unwrap_err()
                .code,
            ErrorCode::ResourceLimit
        );
        assert_eq!(
            backend
                .list_containers(ListContainersRequest { scope: scope() })
                .await
                .unwrap_err()
                .code,
            ErrorCode::ResourceLimit
        );
        drop(reads);
        let old = backend
            .switch_workspace(SwitchWorkspaceRequest::Demo {
                scenario: DemoScenario::Standard,
            })
            .unwrap()
            .scope
            .unwrap();
        let new = backend
            .switch_workspace(SwitchWorkspaceRequest::Demo {
                scenario: DemoScenario::Empty,
            })
            .unwrap()
            .scope
            .unwrap();
        assert_ne!(old.session_id, new.session_id);
        assert_eq!(
            backend
                .list_containers(ListContainersRequest { scope: old })
                .await
                .unwrap_err()
                .code,
            ErrorCode::SessionNotFound
        );
        assert!(
            backend
                .list_containers(ListContainersRequest { scope: new })
                .await
                .unwrap()
                .containers
                .is_empty()
        );
        assert_eq!(backend.workspace_mode().unwrap().mode, WorkspaceMode::Demo);
    }

    #[tokio::test]
    async fn diagnostics_are_single_flight_and_invalid_override_is_not_persisted() {
        let backend = Backend::default();
        let permit = backend.diagnostic_slot.try_acquire().unwrap();
        assert_eq!(
            backend.diagnostics().await.unwrap_err().code,
            ErrorCode::ResourceLimit
        );
        assert_eq!(
            backend
                .set_ssh_executable(SetSshExecutableRequest {
                    expected_revision: 0,
                    path: None
                })
                .await
                .unwrap_err()
                .code,
            ErrorCode::ResourceLimit
        );
        drop(permit);
        let result = backend
            .set_ssh_executable(SetSshExecutableRequest {
                expected_revision: 0,
                path: Some("relative/ssh".into()),
            })
            .await
            .unwrap();
        assert_eq!(result.ssh.status, SshStatus::Untrusted);
        assert!(result.preferences.is_none());
        assert_eq!(backend.preferences().unwrap().preferences.revision, 0);
        let saved = backend
            .set_ssh_executable(SetSshExecutableRequest {
                expected_revision: 0,
                path: Some("/usr/bin/ssh".into()),
            })
            .await
            .unwrap();
        assert_eq!(
            saved
                .preferences
                .unwrap()
                .preferences
                .ssh_executable_override
                .as_deref(),
            Some("/usr/bin/ssh")
        );
    }

    #[test]
    fn rejects_invalid_ids_before_session_lookup() {
        let backend = Backend::default();
        for id in [
            "",
            "--help",
            "host; touch /tmp/invalid",
            "h_123",
            "h_zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
        ] {
            let result = backend.connect_host(ConnectHostRequest {
                selection: HostSelection {
                    host_id: HostId(id.into()),
                    selection_generation: 1,
                },
            });
            assert_eq!(result.unwrap_err().code, ErrorCode::InvalidId);
        }
        let mut request = scope();
        request.session_id = SessionId("../../socket".into());
        assert_eq!(
            backend.require_session(&request).unwrap_err().code,
            ErrorCode::InvalidId
        );
        request = scope();
        request.session_generation = 0;
        assert_eq!(
            backend.require_session(&request).unwrap_err().code,
            ErrorCode::InvalidGeneration
        );
        for id in ["--all", "abc123", &"f".repeat(65), &"F".repeat(64)] {
            assert_eq!(
                ContainerId(id.into()).validate().unwrap_err().code,
                ErrorCode::InvalidId
            );
        }
        assert!(ContainerId("a".repeat(64)).validate().is_ok());
    }

    #[tokio::test]
    async fn missing_and_stale_sessions_never_return_inventory_or_cancel_other_work() {
        let backend = Backend::default();
        assert_eq!(
            backend
                .list_containers(ListContainersRequest { scope: scope() })
                .await
                .unwrap_err()
                .code,
            ErrorCode::SessionNotFound
        );
        backend.register_test_session(scope());
        let mut candidates = vec![scope(); 4];
        candidates[0].selection.host_id = HostId(format!("h_{}", "2".repeat(32)));
        candidates[1].selection.selection_generation += 1;
        candidates[2].session_generation += 1;
        candidates[3].daemon_id = "other-daemon".into();
        for stale in candidates {
            assert_eq!(
                backend
                    .list_containers(ListContainersRequest {
                        scope: stale.clone()
                    })
                    .await
                    .unwrap_err()
                    .code,
                ErrorCode::StaleSession
            );
            assert_eq!(
                backend
                    .cancel_subscription(CancelSubscriptionRequest {
                        scope: stale,
                        subscription_id: SubscriptionId(format!("sub_{}", "a".repeat(32)))
                    })
                    .unwrap_err()
                    .code,
                ErrorCode::StaleSession
            );
        }
        assert_eq!(
            backend
                .list_containers(ListContainersRequest { scope: scope() })
                .await
                .unwrap_err()
                .code,
            ErrorCode::FeatureUnavailable
        );
    }
}
