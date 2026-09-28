use crate::policy::{PolicyEngine, registry::ReadOperation};
use crate::{
    domain::*,
    storage::{FileStorage, SettingsStore},
};
use std::sync::Mutex;

/// Resource operations remain behind identity gates. Local version probes are separately bounded.
pub struct Backend {
    policy: Mutex<PolicyEngine>,
    settings: Mutex<Result<SettingsStore, AppError>>,
    diagnostic_slot: tokio::sync::Semaphore,
}
impl Backend {
    pub fn new(app_data: &std::path::Path) -> Self {
        let settings = FileStorage::open(app_data)
            .map_err(|_| AppError::new(ErrorCode::StorageUnavailable))
            .and_then(|adapter| SettingsStore::load(Box::new(adapter)));
        Self {
            policy: Mutex::new(PolicyEngine::default()),
            settings: Mutex::new(settings),
            diagnostic_slot: tokio::sync::Semaphore::new(1),
        }
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

    pub fn list_hosts(&self) -> Result<ListHostsResponse, AppError> {
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

    pub fn list_containers(
        &self,
        request: ListContainersRequest,
    ) -> Result<ListContainersResponse, AppError> {
        self.policy
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .authorize_read(&request.scope, &ReadOperation::ListContainers)?;
        Err(AppError::new(ErrorCode::FeatureUnavailable).in_scope(&request.scope))
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
            policy: Mutex::new(PolicyEngine::default()),
            settings: Mutex::new(Ok(crate::storage::tests::memory_store())),
            diagnostic_slot: tokio::sync::Semaphore::new(1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract_tests::scope;

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

    #[test]
    fn missing_and_stale_sessions_never_return_inventory_or_cancel_other_work() {
        let backend = Backend::default();
        assert_eq!(
            backend
                .list_containers(ListContainersRequest { scope: scope() })
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
                .unwrap_err()
                .code,
            ErrorCode::FeatureUnavailable
        );
    }
}
