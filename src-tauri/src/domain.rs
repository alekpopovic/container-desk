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
    StorageUnavailable,
    StorageConflict,
    InvalidPreferences,
    InvalidLimits,
    InvalidIntent,
    IntentExpired,
    Disconnected,
    OperationTimedOut,
    InvalidAlias,
    InvalidConfigPath,
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
                "Local settings cannot be saved. The original files were retained."
            }
            ErrorCode::StorageConflict => "Settings changed. Reload before saving again.",
            ErrorCode::InvalidPreferences => "Settings contain invalid or unsupported values.",
            ErrorCode::InvalidLimits => "An operation limit is outside the supported range.",
            ErrorCode::InvalidIntent => {
                "This confirmation does not match the operation or was already used."
            }
            ErrorCode::IntentExpired => "This confirmation expired. Review the operation again.",
            ErrorCode::Disconnected => "The connection closed. Reconnect before refreshing.",
            ErrorCode::OperationTimedOut => "The command exceeded its deadline.",
            ErrorCode::SshUnavailable => {
                "The selected OpenSSH executable is unavailable or untrusted. Check native dependency settings."
            }
            ErrorCode::SshResolutionFailed => {
                "OpenSSH could not resolve this alias. Check the trusted configuration in your terminal."
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
    pub ports: Vec<ContainerPort>,
    pub compose: Option<ComposeLabels>,
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
            schema_version: 2,
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
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ContainerLogsRequest {
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
    pub text: String,
    pub truncated: bool,
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
