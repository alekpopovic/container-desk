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
identifier!(ImageId);
identifier!(SubscriptionId);
identifier!(IntentId);

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
fn full_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
impl ContainerId {
    pub fn validate(&self) -> Result<(), AppError> {
        if full_sha256(&self.0) {
            Ok(())
        } else {
            Err(AppError::new(ErrorCode::InvalidId))
        }
    }
}
impl ImageId {
    pub fn validate(&self) -> Result<(), AppError> {
        if full_sha256(self.0.strip_prefix("sha256:").unwrap_or(&self.0)) {
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
    ContainerNotFound,
    LogDriverUnsupported,
    ExportFailed,
    SessionNotFound,
    StaleSession,
    SubscriptionNotFound,
    FeatureUnavailable,
    PermissionDenied,
    ResourceLimit,
    TransportUnavailable,
    InvalidResponse,
    Internal,
    StorageUnavailable,
    StorageConflict,
    InvalidPreferences,
    InvalidLimits,
    InvalidIntent,
    IntentExpired,
    Disconnected,
    OperationTimedOut,
    OperationCancelled,
    InvalidAlias,
    InvalidConfigPath,
    InvalidRemoteArgument,
    SshUnavailable,
    SshResolutionFailed,
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
            ErrorCode::ExportFailed => {
                "The selected log file could not be saved. Choose a writable regular file location."
            }
            ErrorCode::LogDriverUnsupported => {
                "This container logging driver does not support reading logs."
            }
            ErrorCode::ContainerNotFound => {
                "The container no longer exists. Refresh the inventory."
            }
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
            ErrorCode::StorageUnavailable => {
                "Local data could not be read or saved. Check application storage before retrying."
            }
            ErrorCode::StorageConflict => "Settings changed. Reload before saving again.",
            ErrorCode::InvalidPreferences => "Settings contain invalid or unsupported values.",
            ErrorCode::InvalidLimits => "An operation limit is outside the supported range.",
            ErrorCode::InvalidIntent => {
                "This confirmation does not match the operation or was already used."
            }
            ErrorCode::IntentExpired => "This confirmation expired. Review the operation again.",
            ErrorCode::Disconnected => "The connection closed. Reconnect before refreshing.",
            ErrorCode::OperationCancelled => "The operation was cancelled.",
            ErrorCode::OperationTimedOut => "The command exceeded its deadline.",
            ErrorCode::SshUnavailable => {
                "The selected OpenSSH executable is unavailable or untrusted. Check native dependency settings."
            }
            ErrorCode::SshResolutionFailed => {
                "OpenSSH could not resolve this alias. Check the trusted configuration in your terminal."
            }
            ErrorCode::InvalidRemoteArgument => {
                "A remote operation argument is invalid or unsupported."
            }
            ErrorCode::InvalidAlias => {
                "Use a concrete SSH alias: letters, digits, dots, underscores or dashes, starting with a letter or digit (maximum 256 bytes)."
            }
            ErrorCode::InvalidConfigPath => {
                "Choose an absolute local SSH config path or use the default."
            }
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
    Resolving,
    Connecting,
    Probing,
    Ready,
    Degraded,
    Error,
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
pub struct ContainerListDisplay {
    /// CLI name aliases and display strings; never precise inspect/authorization data.
    pub names: Vec<String>,
    pub ports: Option<String>,
    pub created_at: Option<String>,
    pub running_for: Option<String>,
    /// Missing field is unknown. Label values never cross default list IPC.
    pub labels_present: Option<bool>,
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
    pub ports: Vec<ContainerPort>,
    pub compose: Option<ComposeLabels>,
    #[serde(default)]
    pub cli: Option<ContainerListDisplay>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ContainerDetail {
    pub healthcheck_configured: Option<bool>,
    pub oom_killed: Option<bool>,
    pub exposed_ports: Vec<ExposedPort>,
    pub summary: ContainerSummary,
    /// No environment values or unrestricted inspect JSON cross default IPC.
    pub environment_names: Vec<String>,
    pub environment_values_masked: bool,
    pub environment: Vec<DetailValue>,
    pub labels: Vec<DetailValue>,
    pub created_at: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub exit_code: Option<i32>,
    pub restart_count: Option<u32>,
    pub restart_policy: Option<String>,
    pub restart_maximum_retry_count: Option<u32>,
    pub image_id: Option<ImageId>,
    pub mounts: Vec<DetailMount>,
    pub networks: Vec<DetailNetwork>,
    pub resources: ResourceConfiguration,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ExposedPort {
    pub private_port: u16,
    pub protocol: String,
}
/// Debug never includes even explicitly revealed values.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct DetailValue {
    pub name: String,
    pub value: Option<String>,
    pub masked: bool,
}
impl std::fmt::Debug for DetailValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DetailValue")
            .field("masked", &self.masked)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct DetailMount {
    pub kind: Option<String>,
    pub name: Option<String>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub read_write: Option<bool>,
    pub propagation: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct DetailNetwork {
    pub name: String,
    pub network_id: Option<String>,
    pub ipv4: Option<String>,
    pub ipv6: Option<String>,
    pub gateway: Option<String>,
    pub mac_address: Option<String>,
    pub aliases: Vec<String>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ResourceConfiguration {
    /// Exact decimal integers as strings; avoid JS precision loss. Null means unavailable.
    pub memory_bytes: Option<String>,
    pub memory_swap_bytes: Option<String>,
    pub nano_cpus: Option<String>,
    pub cpu_shares: Option<String>,
    pub cpu_period: Option<String>,
    pub cpu_quota: Option<String>,
    pub cpuset_cpus: Option<String>,
    pub pids_limit: Option<String>,
    pub privileged: Option<bool>,
    pub read_only_rootfs: Option<bool>,
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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SavedHost {
    pub id: HostId,
    pub alias: String,
    pub display_name: String,
    pub group: String,
    pub labels: Vec<String>,
    pub read_only: bool,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub ssh: Option<SshSelection>,
    #[serde(default)]
    pub docker: DockerOptions,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Preferences {
    pub schema_version: u32,
    pub revision: u32,
    pub theme: Theme,
    pub selected_host_id: Option<HostId>,
    pub hosts: Vec<SavedHost>,
    /// References only. The app never copies SSH config/key contents.
    pub trusted_config_path: Option<String>,
    pub ssh_executable_override: Option<String>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: 3,
            revision: 0,
            theme: Theme::System,
            selected_host_id: None,
            hosts: vec![],
            trusted_config_path: None,
            ssh_executable_override: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum StorageNotice {
    Migrated,
    RecoveredPrevious,
    ResetAfterCorruption,
    UnsupportedSchema,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct PreferencesSnapshot {
    pub preferences: Preferences,
    pub notice: Option<StorageNotice>,
    pub writable: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SetThemeRequest {
    pub expected_revision: u32,
    pub theme: Theme,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum SshStatus {
    Ready,
    Missing,
    NotExecutable,
    Untrusted,
    Failed,
    TimedOut,
    OutputLimit,
    InvalidVersion,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SshDiagnostic {
    pub path: String,
    pub status: SshStatus,
    pub version: Option<String>,
    pub message: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum AgentStatus {
    Unset,
    Missing,
    NotSocket,
    Inaccessible,
    Reachable,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct AgentDiagnostic {
    pub status: AgentStatus,
    pub message: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct DependencyDiagnostics {
    pub app_version: String,
    pub platform: String,
    pub architecture: String,
    pub ssh: SshDiagnostic,
    pub agent: AgentDiagnostic,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SetSshExecutableRequest {
    pub expected_revision: u32,
    pub path: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SetSshExecutableResponse {
    pub preferences: Option<PreferencesSnapshot>,
    pub ssh: SshDiagnostic,
}

impl IntentId {
    pub fn validate(&self) -> Result<(), AppError> {
        if opaque_id(&self.0, "i_") {
            Ok(())
        } else {
            Err(AppError::new(ErrorCode::InvalidId))
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum HostAccess {
    #[default]
    ReadOnly,
    Manage,
    ManageAndTerminal,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SetManagementRequest {
    pub scope: SessionScope,
    pub enabled: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ManagementState {
    pub scope: SessionScope,
    pub enabled: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum MutationOperation {
    Start,
    Stop,
    Restart,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct MutationSpec {
    pub operation: MutationOperation,
    pub container_ids: Vec<ContainerId>,
    pub timeout_seconds: i32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum TerminalShell {
    Sh,
    Bash,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct TerminalSpec {
    pub container_id: ContainerId,
    pub shell: TerminalShell,
    pub columns: i32,
    pub rows: i32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "category",
    content = "spec",
    rename_all = "snake_case",
    deny_unknown_fields
)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum ConfirmationOperation {
    Mutation(MutationSpec),
    Terminal(TerminalSpec),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct PrepareConfirmationRequest {
    pub scope: SessionScope,
    pub operation: ConfirmationOperation,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ConfirmationIntent {
    pub id: IntentId,
    pub scope: SessionScope,
    pub operation: ConfirmationOperation,
    pub expires_in_ms: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct MutationRequest {
    pub scope: SessionScope,
    pub intent_id: IntentId,
    pub spec: MutationSpec,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum MutationOutcome {
    Succeeded,
    Failed,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct MutationResponse {
    pub scope: SessionScope,
    pub spec: MutationSpec,
    pub outcome: MutationOutcome,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct TerminalRequest {
    pub scope: SessionScope,
    pub intent_id: IntentId,
    pub spec: TerminalSpec,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct TerminalResponse {
    pub scope: SessionScope,
    pub terminal_id: SubscriptionId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct InspectContainerRequest {
    pub scope: SessionScope,
    pub container_id: ContainerId,
    /// Explicit one-request reveal, bound to the current session; never persisted.
    #[serde(default)]
    pub reveal_sensitive: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ContainerLogsRequest {
    /// UTC Unix seconds with optional fractional nanoseconds, validated before dispatch.
    pub since: Option<String>,
    pub until: Option<String>,
    pub scope: SessionScope,
    pub container_id: ContainerId,
    pub tail: i32,
    pub timeout_seconds: i32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct LogSnapshot {
    pub scope: SessionScope,
    pub container_id: ContainerId,
    /// Timestamp order when available; ties retain channel order, not guaranteed transport order.
    pub records: Vec<LogRecord>,
    pub truncated: bool,
    pub dropped_records: u32,
    pub stderr_ambiguous: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum LogChannel {
    Stdout,
    StderrAmbiguous,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct LogRecord {
    pub text: String,
    pub timestamp: Option<String>,
    pub channel: LogChannel,
    pub truncated: bool,
    pub invalid_utf8: bool,
}
impl std::fmt::Debug for LogRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogRecord")
            .field("bytes", &self.text.len())
            .field("channel", &self.channel)
            .field("truncated", &self.truncated)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ContainerPort {
    pub host_ip: Option<String>,
    pub public_port: Option<u16>,
    pub private_port: u16,
    pub protocol: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ComposeLabels {
    pub project: String,
    pub service: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum WorkspaceMode {
    Live,
    Demo,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum DemoScenario {
    Standard,
    Empty,
    PermissionFailure,
    InvalidJson,
    HugeRecord,
    Disconnect,
    Timeout,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum SwitchWorkspaceRequest {
    Live,
    Demo { scenario: DemoScenario },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct WorkspaceModeSnapshot {
    pub mode: WorkspaceMode,
    pub scenario: Option<DemoScenario>,
    pub scope: Option<SessionScope>,
    pub host: Option<HostSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct DiscoverHostsRequest {
    pub config_path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SshConfigPath {
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SshCandidate {
    pub alias: String,
    pub source: String,
    pub line: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct DiscoveryWarning {
    pub code: DiscoveryWarningCode,
    pub source: String,
    pub line: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct HostDiscovery {
    pub config_path: String,
    pub candidates: Vec<SshCandidate>,
    pub warnings: Vec<DiscoveryWarning>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SelectSshAliasRequest {
    pub config_path: Option<String>,
    pub alias: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SshSelection {
    pub use_default_config: bool,
    pub config_path: String,
    pub alias: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum DiscoveryWarningCode {
    MissingFile,
    UnreadableFile,
    UnsupportedSyntax,
    PatternsSkipped,
    ConditionalInclude,
    MatchSkipped,
    IncludeCycle,
    LimitReached,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ResolveSshRequest {
    pub selection: SshSelection,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct EffectiveSshConfig {
    pub selection: SshSelection,
    pub executable_path: String,
    pub hostname: String,
    pub user: String,
    pub port: u16,
    pub proxy_jump: Option<String>,
    pub has_proxy_command: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum SshAccessStatus {
    Verified,
    UnknownHostKey,
    ChangedHostKey,
    HostKeyRejected,
    AuthenticationFailed,
    TimedOut,
    OutputLimit,
    ConnectionFailed,
    RemoteCommandFailed,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SshAccessReport {
    pub selection: SshSelection,
    pub status: SshAccessStatus,
    pub ssh_error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum ConnectionStage {
    Resolve,
    Authenticate,
    Probe,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum ConnectionDiagnosticCode {
    ResolutionFailed,
    UnknownHostKey,
    ChangedHostKey,
    HostKeyRejected,
    AuthenticationFailed,
    ConnectionFailed,
    TimedOut,
    OutputLimit,
    ProbeUnavailable,
    DockerUnavailable,
    RemoteCommandFailed,
    ConnectionLost,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ConnectionToken {
    pub session_id: SessionId,
    pub session_generation: u32,
}
impl ConnectionToken {
    pub fn validate(&self) -> Result<(), AppError> {
        if !opaque_id(&self.session_id.0, "s_") {
            return Err(AppError::new(ErrorCode::InvalidId));
        }
        if self.session_generation == 0 {
            return Err(AppError::new(ErrorCode::InvalidGeneration));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ConnectionRequest {
    pub token: ConnectionToken,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct StageDuration {
    pub stage: ConnectionStage,
    pub duration_ms: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ConnectionDiagnostic {
    pub stage: ConnectionStage,
    pub code: ConnectionDiagnosticCode,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ConnectionSnapshot {
    pub host_id: Option<HostId>,
    pub effective: Option<EffectiveSshConfig>,
    pub token: ConnectionToken,
    pub selection: SshSelection,
    pub state: ConnectionState,
    pub durations: Vec<StageDuration>,
    pub diagnostic: Option<ConnectionDiagnostic>,
    pub has_jump: bool,
    pub transport_mode: SshTransportMode,
    pub docker_options: DockerOptions,
    pub docker: Option<DockerProbeReport>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum SshTransportMode {
    Unconnected,
    Multiplexed,
    DirectFallback,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct DockerOptions {
    pub executable: Option<String>,
    pub context: Option<String>,
    pub sudo: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct BeginSshRequest {
    pub selection: SshSelection,
    #[serde(default)]
    pub docker: DockerOptions,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum DockerProbeStatus {
    Ready,
    DockerMissing,
    DaemonUnavailable,
    PermissionDenied,
    SudoAuthenticationRequired,
    SudoDenied,
    InvalidContext,
    UnsupportedEndpoint,
    UnsupportedOs,
    InvalidResponse,
    IdentityChanged,
    TimedOut,
    OutputLimit,
    ConnectionFailed,
    CommandFailed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum DockerEndpointKind {
    Unix,
    Tcp,
    Ssh,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum ComposeAvailability {
    Available,
    Absent,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct DockerProbeReport {
    pub status: DockerProbeStatus,
    pub context: Option<String>,
    pub endpoint: Option<String>,
    pub endpoint_kind: Option<DockerEndpointKind>,
    pub client_version: Option<String>,
    pub server_version: Option<String>,
    pub daemon_id: Option<String>,
    pub os: Option<String>,
    pub rootless: Option<bool>,
    pub compose: ComposeAvailability,
    pub compose_version: Option<String>,
    pub sudo: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct HostDraft {
    pub ssh: SshSelection,
    pub docker: DockerOptions,
    pub display_name: String,
    pub group: String,
    pub labels: Vec<String>,
    pub favorite: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct InventoryModeRequest {
    pub mode: WorkspaceMode,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SaveHostRequest {
    pub mode: WorkspaceMode,
    pub expected_revision: u32,
    pub id: Option<HostId>,
    pub draft: HostDraft,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct RemoveHostRequest {
    pub mode: WorkspaceMode,
    pub expected_revision: u32,
    pub host_id: HostId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct InventoryConnectRequest {
    pub mode: WorkspaceMode,
    pub host_id: HostId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct InventoryDisconnectRequest {
    pub mode: WorkspaceMode,
    pub host_id: HostId,
    pub token: ConnectionToken,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct HostInventory {
    pub mode: WorkspaceMode,
    pub saved: PreferencesSnapshot,
    pub connection: Option<ConnectionSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct FollowLogsRequest {
    pub scope: SessionScope,
    pub container_id: ContainerId,
    pub tail: i32,
    pub since: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct AckLogsRequest {
    pub scope: SessionScope,
    pub subscription_id: SubscriptionId,
    pub sequence: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct LogBatch {
    pub scope: SessionScope,
    pub container_id: ContainerId,
    pub subscription_id: SubscriptionId,
    pub sequence: u32,
    pub records: Vec<LogRecord>,
    pub dropped_records: u32,
    pub gap: bool,
    pub ended: bool,
    pub error: Option<ErrorCode>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ExportLogsRequest {
    pub scope: SessionScope,
    pub container_id: ContainerId,
    pub lines: Vec<String>,
    pub secrets_acknowledged: bool,
}
impl std::fmt::Debug for ExportLogsRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExportLogsRequest")
            .field("container_id", &self.container_id)
            .field("line_count", &self.lines.len())
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ExportLogsResponse {
    pub saved: bool,
    pub line_count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ContainerStatsRequest {
    pub scope: SessionScope,
    pub container_id: ContainerId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum StatsAvailability {
    Available,
    Stopped,
    Missing,
    Unavailable,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct StatsValues {
    pub cpu_percent: Option<f64>,
    pub memory_usage_bytes: Option<f64>,
    pub memory_limit_bytes: Option<f64>,
    pub memory_percent: Option<f64>,
    pub network_rx_bytes: Option<f64>,
    pub network_tx_bytes: Option<f64>,
    pub block_read_bytes: Option<f64>,
    pub block_write_bytes: Option<f64>,
    pub pids: Option<u32>,
}
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct StatsRaw {
    pub cpu: Option<String>,
    pub memory: Option<String>,
    pub memory_percent: Option<String>,
    pub network: Option<String>,
    pub block: Option<String>,
    pub pids: Option<String>,
}
impl std::fmt::Debug for StatsRaw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StatsRaw { transient metric strings omitted }")
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct StatsSample {
    pub scope: SessionScope,
    pub container_id: ContainerId,
    pub captured_at_ms: f64,
    pub availability: StatsAvailability,
    pub values: StatsValues,
    pub raw: StatsRaw,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct FollowEventsRequest {
    pub scope: SessionScope,
    pub since: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum ContainerEventAction {
    Create,
    Start,
    Stop,
    Die,
    Destroy,
    Restart,
    Pause,
    Unpause,
    Rename,
    HealthStatus,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ContainerEvent {
    pub actor_id: ContainerId,
    pub action: ContainerEventAction,
    pub timestamp_unix_nanos: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct EventBatch {
    pub scope: SessionScope,
    pub subscription_id: SubscriptionId,
    pub sequence: u32,
    pub events: Vec<ContainerEvent>,
    pub dropped_records: u32,
    pub gap: bool,
    pub ended: bool,
    pub error: Option<ErrorCode>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ListComposeRequest {
    pub scope: SessionScope,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum ComposeConfigurationStatus {
    Unverified,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ComposeInstance {
    pub container_id: ContainerId,
    pub name: String,
    pub service: Option<String>,
    pub state: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ComposeProject {
    pub name: String,
    pub status: Option<String>,
    pub from_plugin: bool,
    pub from_labels: bool,
    pub config_files_reported: Vec<String>,
    pub working_directories_reported: Vec<String>,
    pub configuration: ComposeConfigurationStatus,
    pub instances: Vec<ComposeInstance>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ListComposeResponse {
    pub scope: SessionScope,
    pub plugin: ComposeAvailability,
    pub listing_error: Option<ErrorCode>,
    pub projects: Vec<ComposeProject>,
}
