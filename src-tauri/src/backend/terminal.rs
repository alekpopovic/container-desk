use super::*;
impl Backend {
    pub fn terminal_permission(&self, scope: SessionScope) -> Result<ManagementState, AppError> {
        self.require_live_mode()?;
        self.require_session(&scope)?;
        let enabled = self
            .policy
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .access(&scope)?
            == HostAccess::ManageAndTerminal;
        Ok(ManagementState { scope, enabled })
    }
    pub fn set_terminal_permission(
        &self,
        request: SetManagementRequest,
    ) -> Result<ManagementState, AppError> {
        self.require_live_mode()?;
        self.require_session(&request.scope)?;
        if self.workspace_mode()?.scope.as_ref() != Some(&request.scope) {
            return Err(AppError::new(ErrorCode::StaleSession));
        }
        let mut policy = self
            .policy
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?;
        let previous = policy.access(&request.scope)?;
        if request.enabled && previous == HostAccess::ReadOnly {
            return Err(AppError::new(ErrorCode::PermissionDenied));
        }
        let access = if request.enabled {
            HostAccess::ManageAndTerminal
        } else if previous == HostAccess::ReadOnly {
            HostAccess::ReadOnly
        } else {
            HostAccess::Manage
        };
        policy.set_access(&request.scope, access)?;
        Ok(ManagementState {
            scope: request.scope,
            enabled: request.enabled,
        })
    }
    pub async fn open_container_terminal(
        &self,
        request: TerminalRequest,
    ) -> Result<TerminalResponse, AppError> {
        self.require_live_mode()?;
        // Consume the exact one-use intent before any native work or terminal admission.
        self.policy
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .consume(
                &request.scope,
                &request.intent_id,
                &ConfirmationOperation::Terminal(request.spec.clone()),
            )?;
        self.require_session(&request.scope)?;
        let scope = request.scope.clone();
        let spec = request.spec.clone();
        let policy = self.policy.clone();
        let sessions = self.sessions.clone();
        let current = Arc::new(move || {
            policy
                .lock()
                .map_err(|_| AppError::new(ErrorCode::Internal))?
                .authorize_confirmation(&PrepareConfirmationRequest {
                    scope: scope.clone(),
                    operation: ConfirmationOperation::Terminal(spec.clone()),
                })?;
            sessions.require_scope(&scope)
        });
        let reservation = self.terminals.reserve(&request, current)?;
        let _host = self.read_hosts.acquire(&request.scope.selection.host_id)?;
        let _global = self
            .read_slots
            .try_acquire()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
        let launch = self.sessions.terminal_launch(&request).await?;
        self.require_session(&request.scope)?;
        reservation.launch(launch)
    }
    pub fn terminal_output(
        &self,
        request: TerminalHandleRequest,
    ) -> Result<TerminalOutput, AppError> {
        self.require_live_mode()?;
        self.require_session(&request.scope)?;
        self.terminals.read(request)
    }
    pub fn terminal_input(&self, request: TerminalInputRequest) -> Result<(), AppError> {
        self.require_live_mode()?;
        self.require_session(&request.scope)?;
        self.terminals.input(request)
    }
    pub fn resize_terminal(&self, request: TerminalResizeRequest) -> Result<(), AppError> {
        self.require_live_mode()?;
        self.require_session(&request.scope)?;
        self.terminals.resize(request)
    }
    pub async fn close_terminal(&self, request: TerminalHandleRequest) -> Result<(), AppError> {
        // Exact old owner may close its own terminal after navigation/disconnect, never someone else's.
        self.terminals.close(request).await
    }
}
