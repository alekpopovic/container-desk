//! Wire DTOs are the single source for generated TypeScript declarations.
use serde::{Deserialize, Serialize};

macro_rules! identifier {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[cfg_attr(test, derive(ts_rs::TS))]
        pub struct $name(pub String);
    };
}
identifier!(HostId);
identifier!(SessionId);
identifier!(ContainerId);
identifier!(SubscriptionId);

fn opaque_id(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|v| {
        v.len() == 32
            && v.bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    })
}
impl HostId {
    pub fn validate(&self) -> Result<(), AppError> {
        if opaque_id(&self.0, "h_") {
            Ok(())
        } else {
            Err(AppError::new(ErrorCode::InvalidId))
        }
    }
}
impl ContainerId {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.0.len() == 64
            && self
                .0
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            Ok(())
        } else {
            Err(AppError::new(ErrorCode::InvalidId))
        }
    }
}
impl SubscriptionId {
    pub fn validate(&self) -> Result<(), AppError> {
        if opaque_id(&self.0, "sub_") {
            Ok(())
        } else {
            Err(AppError::new(ErrorCode::InvalidId))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct HostSelection {
    pub host_id: HostId,
    /// Renderer selection epoch. Nonzero u32 is exact in JavaScript.
    pub selection_generation: u32,
}
impl HostSelection {
    pub fn validate(&self) -> Result<(), AppError> {
        self.host_id.validate()?;
        if self.selection_generation == 0 {
            return Err(AppError::new(ErrorCode::InvalidGeneration));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SessionScope {
    pub selection: HostSelection,
    pub session_id: SessionId,
    /// Backend-issued epoch; changes after reconnect or daemon/context changes.
    pub session_generation: u32,
    pub daemon_id: String,
}
impl SessionScope {
    pub fn validate(&self) -> Result<(), AppError> {
        self.selection.validate()?;
        if !opaque_id(&self.session_id.0, "s_")
            || self.daemon_id.is_empty()
            || self.daemon_id.len() > 256
            || self.daemon_id.chars().any(char::is_control)
        {
            return Err(AppError::new(ErrorCode::InvalidId));
        }
        if self.session_generation == 0 {
            return Err(AppError::new(ErrorCode::InvalidGeneration));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum ErrorCode {
    InvalidId,
    InvalidGeneration,
    HostNotFound,
    SessionNotFound,
    StaleSession,
    SubscriptionNotFound,
    FeatureUnavailable,
    PermissionDenied,
    ResourceLimit,
    TransportUnavailable,
    InvalidResponse,
    Internal,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    pub scope: Option<SessionScope>,
}
impl AppError {
    pub fn new(code: ErrorCode) -> Self {
        // Deliberately no raw payload, command, environment or stderr in an error.
        let message = match code {
            ErrorCode::InvalidId => "Invalid resource identifier.",
            ErrorCode::InvalidGeneration => "Invalid session or selection generation.",
            ErrorCode::HostNotFound => "Saved host does not exist.",
            ErrorCode::SessionNotFound => "Connection session does not exist.",
            ErrorCode::StaleSession => "The connection changed. Refresh the selected host.",
            ErrorCode::SubscriptionNotFound => "Subscription does not exist in this session.",
            ErrorCode::FeatureUnavailable => "This operation is not available yet.",
            ErrorCode::PermissionDenied => "This operation is not permitted.",
            ErrorCode::ResourceLimit => "The operation exceeded an application limit.",
            ErrorCode::TransportUnavailable => "The desktop connection is unavailable.",
            ErrorCode::InvalidResponse => "The desktop returned an invalid response.",
            ErrorCode::Internal => "The operation could not be completed.",
        };
        Self {
            code,
            message: message.into(),
            scope: None,
        }
    }
    pub fn in_scope(mut self, scope: &SessionScope) -> Self {
        self.scope = Some(scope.clone());
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct HostCapabilities {
    pub docker: bool,
    pub compose: bool,
    pub management: bool,
    pub terminal: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct HostSummary {
    pub id: HostId,
    pub alias: String,
    pub display_name: String,
    pub group: String,
    pub read_only: bool,
    pub connection_state: ConnectionState,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ContainerSummary {
    pub scope: SessionScope,
    pub id: ContainerId,
    pub name: String,
    pub image: String,
    /// Preserve unknown Docker states as untrusted text.
    pub state: String,
    pub status: String,
    pub health: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ContainerDetail {
    pub summary: ContainerSummary,
    /// No environment values or unrestricted inspect JSON cross default IPC.
    pub environment_names: Vec<String>,
    pub environment_values_masked: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ListHostsResponse {
    pub hosts: Vec<HostSummary>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ConnectHostRequest {
    pub selection: HostSelection,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ConnectHostResponse {
    pub scope: SessionScope,
    pub capabilities: HostCapabilities,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ListContainersRequest {
    pub scope: SessionScope,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ListContainersResponse {
    pub scope: SessionScope,
    pub containers: Vec<ContainerSummary>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct CancelSubscriptionRequest {
    pub scope: SessionScope,
    pub subscription_id: SubscriptionId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct CancelSubscriptionResponse {
    pub scope: SessionScope,
    pub subscription_id: SubscriptionId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct AppVersion {
    pub version: String,
}
