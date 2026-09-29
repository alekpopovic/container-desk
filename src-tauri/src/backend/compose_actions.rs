use super::*;
use std::time::{Duration, Instant};
const VERIFICATION_TTL: Duration = Duration::from_secs(300);
#[derive(Clone)]
pub(super) struct Verified {
    view: ComposeVerification,
    native: crate::docker::compose_actions::VerifiedRead,
    created: Instant,
}
impl Backend {
    fn cached_compose(
        &self,
        scope: &SessionScope,
        spec: &ComposeActionSpec,
    ) -> Result<Verified, AppError> {
        self.require_live_mode()?;
        self.require_session(scope)?;
        let entries = self
            .compose_verified
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?;
        let record = entries
            .get(&scope.selection.host_id)
            .ok_or_else(|| AppError::new(ErrorCode::ComposeVerificationExpired))?;
        if record.created.elapsed() >= VERIFICATION_TTL {
            return Err(AppError::new(ErrorCode::ComposeVerificationExpired));
        }
        if record.view.scope != *scope
            || record.view.id != spec.verification_id
            || record.view.configuration != spec.configuration
            || record.view.services != spec.services
            || record.view.container_ids != spec.container_ids
        {
            return Err(AppError::new(ErrorCode::ComposeProjectMismatch));
        }
        Ok(record.clone())
    }
    async fn compose_read_admitted(
        &self,
        request: &VerifyComposeRequest,
    ) -> Result<crate::docker::compose_actions::VerifiedRead, AppError> {
        self.require_live_mode()?;
        self.require_session(&request.scope)?;
        self.policy
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .authorize_read(
                &request.scope,
                &ReadOperation::ComposeValidate {
                    configuration: request.configuration.clone(),
                },
            )?;
        if !request.acknowledged {
            return Err(AppError::new(ErrorCode::PermissionDenied));
        }
        let _host = self.read_hosts.acquire(&request.scope.selection.host_id)?;
        let _global = self
            .read_slots
            .try_acquire()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
        let result = self.sessions.verify_compose(request).await;
        self.require_session(&request.scope)?;
        result
    }
    pub async fn verify_compose_project(
        &self,
        request: VerifyComposeRequest,
    ) -> Result<ComposeVerification, AppError> {
        let native = self.compose_read_admitted(&request).await?;
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| AppError::new(ErrorCode::Internal))?;
        let id = ComposeVerificationId(format!(
            "v_{}",
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ));
        let view = ComposeVerification {
            scope: request.scope,
            configuration: request.configuration,
            id,
            services: native.services.clone(),
            container_ids: native.container_ids.clone(),
            expires_in_ms: 300_000,
        };
        let mut entries = self
            .compose_verified
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?;
        entries.retain(|_, record| {
            record.created.elapsed() < VERIFICATION_TTL
                && self.sessions.require_scope(&record.view.scope).is_ok()
        });
        if entries.len() >= 3 && !entries.contains_key(&view.scope.selection.host_id) {
            return Err(AppError::new(ErrorCode::ResourceLimit));
        }
        entries.insert(
            view.scope.selection.host_id.clone(),
            Verified {
                view: view.clone(),
                native,
                created: Instant::now(),
            },
        );
        Ok(view)
    }
    pub(super) async fn recheck_compose(
        &self,
        scope: &SessionScope,
        spec: &ComposeActionSpec,
    ) -> Result<(), AppError> {
        let expected = self.cached_compose(scope, spec)?;
        let current = self
            .compose_read_admitted(&VerifyComposeRequest {
                scope: scope.clone(),
                configuration: spec.configuration.clone(),
                acknowledged: true,
            })
            .await?;
        if current != expected.native {
            return Err(AppError::new(ErrorCode::ComposeProjectMismatch));
        }
        self.cached_compose(scope, spec)?;
        Ok(())
    }
    pub async fn mutate_compose_project(
        &self,
        request: ComposeMutationRequest,
    ) -> Result<ComposeMutationResponse, AppError> {
        let expected = self.cached_compose(&request.scope, &request.spec)?;
        let owner = self
            .activities
            .as_ref()
            .map_err(Clone::clone)?
            .begin_compose(
                &mut *self
                    .policy
                    .lock()
                    .map_err(|_| AppError::new(ErrorCode::Internal))?,
                request.clone(),
            )?;
        let permit = tokio::time::timeout(
            Duration::from_secs(3),
            self.read_slots.clone().acquire_owned(),
        )
        .await
        .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?
        .map_err(|_| AppError::new(ErrorCode::Disconnected))?;
        let sessions = self.sessions.clone();
        let policy = self.policy.clone();
        let scope = request.scope.clone();
        let operation = ConfirmationOperation::Compose(request.spec.clone());
        let current_sessions = sessions.clone();
        let current = Arc::new(move || {
            policy
                .lock()
                .map_err(|_| AppError::new(ErrorCode::Internal))?
                .authorize_confirmation(&PrepareConfirmationRequest {
                    scope: scope.clone(),
                    operation: operation.clone(),
                })?;
            current_sessions.require_scope(&scope)
        });
        tauri::async_runtime::spawn(async move {
            let _permit = permit;
            sessions
                .mutate_compose(request, expected.native, owner, current)
                .await
        })
        .await
        .map_err(|_| AppError::new(ErrorCode::Internal))?
    }
}
