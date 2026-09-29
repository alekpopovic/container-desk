pub mod registry;
use crate::domain::*;
use registry::{CommandPlan, OperationCategory, ReadOperation};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

const MAX_SESSIONS: usize = 3;
const MAX_INTENTS_PER_SESSION: usize = 32;
const INTENT_TTL: Duration = Duration::from_secs(30);
struct IntentRecord {
    operation: ConfirmationOperation,
    expires: Instant,
}
struct SessionPolicy {
    scope: SessionScope,
    access: HostAccess,
    intents: HashMap<IntentId, IntentRecord>,
}
#[derive(Default)]
pub struct PolicyEngine {
    sessions: HashMap<SessionId, SessionPolicy>,
}
/// A one-use authorization result, not serializable and not Clone.
pub struct AuthorizedCommand {
    scope: SessionScope,
    plan: CommandPlan,
}
impl AuthorizedCommand {
    pub fn scope(&self) -> &SessionScope {
        &self.scope
    }
    pub fn into_plan(self) -> CommandPlan {
        self.plan
    }
}
impl PolicyEngine {
    /// Called only by a backend-owned connection manager. Every new registration resets grants.
    pub fn register(&mut self, scope: SessionScope) -> Result<(), AppError> {
        scope.validate()?;
        if self
            .sessions
            .get(&scope.session_id)
            .is_some_and(|s| s.scope.selection.host_id != scope.selection.host_id)
        {
            return Err(AppError::new(ErrorCode::InvalidId));
        }
        let existing_host = self
            .sessions
            .values()
            .any(|s| s.scope.selection.host_id == scope.selection.host_id);
        if !existing_host && self.sessions.len() >= MAX_SESSIONS {
            return Err(AppError::new(ErrorCode::ResourceLimit));
        }
        self.sessions
            .retain(|_, s| s.scope.selection.host_id != scope.selection.host_id);
        self.sessions.insert(
            scope.session_id.clone(),
            SessionPolicy {
                scope,
                access: HostAccess::ReadOnly,
                intents: HashMap::new(),
            },
        );
        Ok(())
    }
    pub fn remove(&mut self, scope: &SessionScope) -> Result<(), AppError> {
        self.require_session(scope)?;
        self.sessions.remove(&scope.session_id);
        Ok(())
    }
    pub fn require_session(&self, scope: &SessionScope) -> Result<(), AppError> {
        scope.validate()?;
        match self.sessions.get(&scope.session_id) {
            None => Err(AppError::new(ErrorCode::SessionNotFound).in_scope(scope)),
            Some(current) if current.scope != *scope => {
                Err(AppError::new(ErrorCode::StaleSession).in_scope(scope))
            }
            Some(_) => Ok(()),
        }
    }
    /// A future explicit management/terminal opt-in handler may call this after selecting a host.
    /// Persisted readOnly=false must never grant runtime access automatically.
    pub fn access(&self, scope: &SessionScope) -> Result<HostAccess, AppError> {
        self.require_session(scope)?;
        Ok(self.sessions[&scope.session_id].access.clone())
    }
    pub fn set_access(&mut self, scope: &SessionScope, access: HostAccess) -> Result<(), AppError> {
        self.require_session(scope)?;
        let session = self
            .sessions
            .get_mut(&scope.session_id)
            .expect("validated session");
        session.access = access;
        session.intents.clear();
        Ok(())
    }
    pub fn authorize_read(
        &self,
        scope: &SessionScope,
        operation: &ReadOperation,
    ) -> Result<AuthorizedCommand, AppError> {
        let plan = registry::read(operation)?;
        self.require_session(scope)?;
        Ok(AuthorizedCommand {
            scope: scope.clone(),
            plan,
        })
    }
    fn require_access(
        &self,
        scope: &SessionScope,
        category: &OperationCategory,
    ) -> Result<(), AppError> {
        self.require_session(scope)?;
        let access = &self.sessions[&scope.session_id].access;
        let allowed = match category {
            OperationCategory::Read => true,
            OperationCategory::Mutation => *access != HostAccess::ReadOnly,
            OperationCategory::Terminal => *access == HostAccess::ManageAndTerminal,
        };
        if allowed {
            Ok(())
        } else {
            Err(AppError::new(ErrorCode::PermissionDenied).in_scope(scope))
        }
    }
    pub fn authorize_confirmation(
        &self,
        request: &PrepareConfirmationRequest,
    ) -> Result<(), AppError> {
        let plan = registry::confirmation(&request.operation)?;
        self.require_access(&request.scope, plan.category())
    }
    pub fn prepare(
        &mut self,
        request: PrepareConfirmationRequest,
    ) -> Result<ConfirmationIntent, AppError> {
        self.prepare_at(request, Instant::now())
    }
    fn prepare_at(
        &mut self,
        request: PrepareConfirmationRequest,
        now: Instant,
    ) -> Result<ConfirmationIntent, AppError> {
        let plan = registry::confirmation(&request.operation)?;
        self.require_access(&request.scope, plan.category())?;
        let session = self
            .sessions
            .get_mut(&request.scope.session_id)
            .expect("validated session");
        session.intents.retain(|_, intent| now < intent.expires);
        if session.intents.len() >= MAX_INTENTS_PER_SESSION {
            return Err(AppError::new(ErrorCode::ResourceLimit));
        }
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| AppError::new(ErrorCode::Internal))?;
        let id = IntentId(format!(
            "i_{}",
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ));
        if session.intents.contains_key(&id) {
            return Err(AppError::new(ErrorCode::Internal));
        }
        session.intents.insert(
            id.clone(),
            IntentRecord {
                operation: request.operation.clone(),
                expires: now + INTENT_TTL,
            },
        );
        Ok(ConfirmationIntent {
            id,
            scope: request.scope,
            operation: request.operation,
            expires_in_ms: INTENT_TTL.as_millis() as u32,
        })
    }
    pub fn consume(
        &mut self,
        scope: &SessionScope,
        id: &IntentId,
        operation: &ConfirmationOperation,
    ) -> Result<AuthorizedCommand, AppError> {
        self.consume_at(scope, id, operation, Instant::now())
    }
    fn consume_at(
        &mut self,
        scope: &SessionScope,
        id: &IntentId,
        operation: &ConfirmationOperation,
        now: Instant,
    ) -> Result<AuthorizedCommand, AppError> {
        id.validate()?;
        let plan = registry::confirmation(operation)?;
        self.require_access(scope, plan.category())?;
        let intent = self
            .sessions
            .get_mut(&scope.session_id)
            .expect("validated session")
            .intents
            .remove(id)
            .ok_or_else(|| AppError::new(ErrorCode::InvalidIntent).in_scope(scope))?;
        if now >= intent.expires {
            return Err(AppError::new(ErrorCode::IntentExpired).in_scope(scope));
        }
        if intent.operation != *operation {
            return Err(AppError::new(ErrorCode::InvalidIntent).in_scope(scope));
        }
        Ok(AuthorizedCommand {
            scope: scope.clone(),
            plan,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract_tests::scope;
    fn operation() -> ConfirmationOperation {
        ConfirmationOperation::Mutation(MutationSpec {
            operation: MutationOperation::Restart,
            container_ids: vec![ContainerId("a".repeat(64))],
            timeout_seconds: 10,
        })
    }
    fn request() -> PrepareConfirmationRequest {
        PrepareConfirmationRequest {
            scope: scope(),
            operation: operation(),
        }
    }
    fn managed() -> PolicyEngine {
        let mut p = PolicyEngine::default();
        p.register(scope()).unwrap();
        p.set_access(&scope(), HostAccess::Manage).unwrap();
        p
    }
    #[test]
    fn default_read_only_allows_reads_and_denies_mutations_and_terminals() {
        let mut p = PolicyEngine::default();
        p.register(scope()).unwrap();
        assert!(
            p.authorize_read(&scope(), &ReadOperation::ListContainers)
                .is_ok()
        );
        assert_eq!(
            p.prepare(request()).unwrap_err().code,
            ErrorCode::PermissionDenied
        );
        let terminal = ConfirmationOperation::Terminal(TerminalSpec {
            container_id: ContainerId("a".repeat(64)),
            shell: TerminalShell::Sh,
            columns: 80,
            rows: 24,
        });
        p.set_access(&scope(), HostAccess::Manage).unwrap();
        assert_eq!(
            p.prepare(PrepareConfirmationRequest {
                scope: scope(),
                operation: terminal.clone()
            })
            .unwrap_err()
            .code,
            ErrorCode::PermissionDenied
        );
        p.set_access(&scope(), HostAccess::ManageAndTerminal)
            .unwrap();
        let intent = p
            .prepare(PrepareConfirmationRequest {
                scope: scope(),
                operation: terminal.clone(),
            })
            .unwrap();
        assert_eq!(
            p.consume(&scope(), &intent.id, &terminal)
                .unwrap()
                .into_plan()
                .category(),
            &OperationCategory::Terminal
        );
    }
    #[test]
    fn intents_are_exact_one_use_and_expire_without_replay() {
        let mut p = managed();
        let now = Instant::now();
        let intent = p.prepare_at(request(), now).unwrap();
        assert!(
            p.consume_at(&scope(), &intent.id, &operation(), now)
                .is_ok()
        );
        assert_eq!(
            p.consume_at(&scope(), &intent.id, &operation(), now)
                .err()
                .unwrap()
                .code,
            ErrorCode::InvalidIntent
        );
        let intent = p.prepare_at(request(), now).unwrap();
        assert_eq!(
            p.consume_at(&scope(), &intent.id, &operation(), now + INTENT_TTL)
                .err()
                .unwrap()
                .code,
            ErrorCode::IntentExpired
        );
        let intent = p.prepare(request()).unwrap();
        let wrong = ConfirmationOperation::Mutation(MutationSpec {
            operation: MutationOperation::Stop,
            container_ids: vec![ContainerId("a".repeat(64))],
            timeout_seconds: 10,
        });
        assert_eq!(
            p.consume(&scope(), &intent.id, &wrong).err().unwrap().code,
            ErrorCode::InvalidIntent
        );
        assert!(p.consume(&scope(), &intent.id, &operation()).is_err());
    }
    #[test]
    fn target_timeout_and_selection_cannot_be_swapped_after_confirmation() {
        let mut p = managed();
        for changed in [
            MutationSpec {
                operation: MutationOperation::Restart,
                container_ids: vec![ContainerId("b".repeat(64))],
                timeout_seconds: 10,
            },
            MutationSpec {
                operation: MutationOperation::Restart,
                container_ids: vec![ContainerId("a".repeat(64))],
                timeout_seconds: 11,
            },
        ] {
            let intent = p.prepare(request()).unwrap();
            assert_eq!(
                p.consume(
                    &scope(),
                    &intent.id,
                    &ConfirmationOperation::Mutation(changed)
                )
                .err()
                .unwrap()
                .code,
                ErrorCode::InvalidIntent
            );
        }
        let intent = p.prepare(request()).unwrap();
        let mut changed = scope();
        changed.selection.selection_generation += 1;
        assert_eq!(
            p.consume(&changed, &intent.id, &operation())
                .err()
                .unwrap()
                .code,
            ErrorCode::StaleSession
        );
        assert!(p.consume(&scope(), &intent.id, &operation()).is_ok());
    }
    #[test]
    fn session_capacity_is_bounded_and_disconnect_releases_the_slot() {
        let mut p = PolicyEngine::default();
        for n in 1..=3 {
            let mut s = scope();
            s.selection.host_id = HostId(format!("h_{n:032x}"));
            s.session_id = SessionId(format!("s_{n:032x}"));
            p.register(s).unwrap();
        }
        let mut fourth = scope();
        fourth.selection.host_id = HostId(format!("h_{:032x}", 4));
        fourth.session_id = SessionId(format!("s_{:032x}", 4));
        assert_eq!(
            p.register(fourth.clone()).unwrap_err().code,
            ErrorCode::ResourceLimit
        );
        let mut first = scope();
        first.selection.host_id = HostId(format!("h_{:032x}", 1));
        first.session_id = SessionId(format!("s_{:032x}", 1));
        p.remove(&first).unwrap();
        assert!(p.register(fourth).is_ok());
    }

    #[test]
    fn reconnect_daemon_change_and_revocation_invalidate_grants_and_intents() {
        let mut p = managed();
        let intent = p.prepare(request()).unwrap();
        p.set_access(&scope(), HostAccess::ReadOnly).unwrap();
        assert_eq!(
            p.consume(&scope(), &intent.id, &operation())
                .err()
                .unwrap()
                .code,
            ErrorCode::PermissionDenied
        );
        p.set_access(&scope(), HostAccess::Manage).unwrap();
        assert_eq!(
            p.consume(&scope(), &intent.id, &operation())
                .err()
                .unwrap()
                .code,
            ErrorCode::InvalidIntent
        );
        let intent = p.prepare(request()).unwrap();
        let mut new = scope();
        new.session_generation += 1;
        new.daemon_id = "changed".into();
        p.register(new.clone()).unwrap();
        assert_eq!(
            p.remove(&scope()).unwrap_err().code,
            ErrorCode::StaleSession
        );
        assert_eq!(
            p.consume(&scope(), &intent.id, &operation())
                .err()
                .unwrap()
                .code,
            ErrorCode::StaleSession
        );
        assert_eq!(
            p.prepare(PrepareConfirmationRequest {
                scope: new,
                operation: operation()
            })
            .unwrap_err()
            .code,
            ErrorCode::PermissionDenied
        );
    }
    #[test]
    fn host_policies_are_independent_and_intent_storage_is_bounded() {
        let mut p = managed();
        let mut other = scope();
        other.selection.host_id = HostId(format!("h_{}", "3".repeat(32)));
        other.session_id = SessionId(format!("s_{}", "4".repeat(32)));
        p.register(other.clone()).unwrap();
        assert_eq!(
            p.prepare(PrepareConfirmationRequest {
                scope: other.clone(),
                operation: operation()
            })
            .unwrap_err()
            .code,
            ErrorCode::PermissionDenied
        );
        p.set_access(&other, HostAccess::Manage).unwrap();
        let intent = p.prepare(request()).unwrap();
        assert_eq!(
            p.consume(&other, &intent.id, &operation())
                .err()
                .unwrap()
                .code,
            ErrorCode::InvalidIntent
        );
        assert!(p.consume(&scope(), &intent.id, &operation()).is_ok());
        for _ in 0..MAX_INTENTS_PER_SESSION {
            p.prepare(request()).unwrap();
        }
        assert_eq!(
            p.prepare(request()).unwrap_err().code,
            ErrorCode::ResourceLimit
        );
    }
}
