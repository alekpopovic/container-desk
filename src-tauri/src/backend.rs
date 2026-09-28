use crate::{
    domain::*,
    storage::{FileStorage, SettingsStore},
};
use std::{collections::HashMap, sync::Mutex};

/// No process launcher exists here. Later transport work must pass these gates.
pub struct Backend {
    sessions: Mutex<HashMap<SessionId, SessionScope>>,
    settings: Mutex<Result<SettingsStore, AppError>>,
}
impl Backend {
    pub fn new(app_data: &std::path::Path) -> Self {
        let settings = FileStorage::open(app_data)
            .map_err(|_| AppError::new(ErrorCode::StorageUnavailable))
            .and_then(|adapter| SettingsStore::load(Box::new(adapter)));
        Self {
            sessions: Mutex::new(HashMap::new()),
            settings: Mutex::new(settings),
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
        scope.validate()?;
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?;
        match sessions.get(&scope.session_id) {
            None => Err(AppError::new(ErrorCode::SessionNotFound).in_scope(scope)),
            Some(current) if current != scope => {
                Err(AppError::new(ErrorCode::StaleSession).in_scope(scope))
            }
            Some(_) => Ok(()),
        }
    }

    pub fn list_containers(
        &self,
        request: ListContainersRequest,
    ) -> Result<ListContainersResponse, AppError> {
        self.require_session(&request.scope)?;
        Err(AppError::new(ErrorCode::FeatureUnavailable).in_scope(&request.scope))
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
            sessions: Mutex::new(HashMap::new()),
            settings: Mutex::new(Ok(crate::storage::tests::memory_store())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract_tests::scope;

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
        backend
            .sessions
            .lock()
            .unwrap()
            .insert(scope().session_id.clone(), scope());
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
