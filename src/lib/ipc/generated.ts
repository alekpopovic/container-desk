// Generated from src-tauri/src/domain.rs with ts-rs. Do not edit.
export type HostId = string;
export type SessionId = string;
export type ContainerId = string;
export type SubscriptionId = string;
export type HostSelection = { hostId: HostId,
/**
 * Renderer selection epoch. Nonzero u32 is exact in JavaScript.
 */
selectionGeneration: number, };
export type SessionScope = { selection: HostSelection, sessionId: SessionId,
/**
 * Backend-issued epoch; changes after reconnect or daemon/context changes.
 */
sessionGeneration: number, daemonId: string, };
export type ErrorCode = "invalid_id" | "invalid_generation" | "host_not_found" | "session_not_found" | "stale_session" | "subscription_not_found" | "feature_unavailable" | "permission_denied" | "resource_limit" | "transport_unavailable" | "invalid_response" | "internal" | "storage_unavailable" | "storage_conflict" | "invalid_preferences" | "invalid_limits" | "invalid_intent" | "intent_expired";
export type AppError = { code: ErrorCode, message: string, scope: SessionScope | null, };
export type ConnectionState = "disconnected" | "connecting" | "connected" | "reconnecting" | "failed";
export type HostCapabilities = { docker: boolean, compose: boolean, management: boolean, terminal: boolean, };
export type HostSummary = { id: HostId, alias: string, displayName: string, group: string, readOnly: boolean, connectionState: ConnectionState, };
export type ContainerSummary = { scope: SessionScope, id: ContainerId, name: string, image: string,
/**
 * Preserve unknown Docker states as untrusted text.
 */
state: string, status: string, health: string | null, };
export type ContainerDetail = { summary: ContainerSummary,
/**
 * No environment values or unrestricted inspect JSON cross default IPC.
 */
environmentNames: Array<string>, environmentValuesMasked: boolean, };
export type ListHostsResponse = { hosts: Array<HostSummary>, };
export type ConnectHostRequest = { selection: HostSelection, };
export type ConnectHostResponse = { scope: SessionScope, capabilities: HostCapabilities, };
export type ListContainersRequest = { scope: SessionScope, };
export type ListContainersResponse = { scope: SessionScope, containers: Array<ContainerSummary>, };
export type CancelSubscriptionRequest = { scope: SessionScope, subscriptionId: SubscriptionId, };
export type CancelSubscriptionResponse = { scope: SessionScope, subscriptionId: SubscriptionId, };
export type AppVersion = { version: string, };
export type Theme = "system" | "light" | "dark";
export type SavedHost = { id: HostId, alias: string, displayName: string, group: string, labels: Array<string>, readOnly: boolean, };
export type Preferences = { schemaVersion: number, revision: number, theme: Theme, selectedHostId: HostId | null, hosts: Array<SavedHost>,
/**
 * References only. The app never copies SSH config/key contents.
 */
trustedConfigPath: string | null, sshExecutableOverride: string | null, };
export type StorageNotice = "migrated" | "recovered_previous" | "reset_after_corruption" | "unsupported_schema";
export type PreferencesSnapshot = { preferences: Preferences, notice: StorageNotice | null, writable: boolean, };
export type SetThemeRequest = { expectedRevision: number, theme: Theme, };
export type SshStatus = "ready" | "missing" | "not_executable" | "untrusted" | "failed" | "timed_out" | "output_limit" | "invalid_version";
export type SshDiagnostic = { path: string, status: SshStatus, version: string | null, message: string, };
export type AgentStatus = "unset" | "missing" | "not_socket" | "inaccessible" | "reachable";
export type AgentDiagnostic = { status: AgentStatus, message: string, };
export type DependencyDiagnostics = { appVersion: string, platform: string, architecture: string, ssh: SshDiagnostic, agent: AgentDiagnostic, };
export type SetSshExecutableRequest = { expectedRevision: number, path: string | null, };
export type SetSshExecutableResponse = { preferences: PreferencesSnapshot | null, ssh: SshDiagnostic, };
export type IntentId = string;
export type HostAccess = "read_only" | "manage" | "manage_and_terminal";
export type MutationOperation = "start" | "stop" | "restart";
export type MutationSpec = { operation: MutationOperation, containerIds: Array<ContainerId>, timeoutSeconds: number, };
export type TerminalShell = "sh" | "bash";
export type TerminalSpec = { containerId: ContainerId, shell: TerminalShell, columns: number, rows: number, };
export type ConfirmationOperation = { "category": "mutation", "spec": MutationSpec } | { "category": "terminal", "spec": TerminalSpec };
export type PrepareConfirmationRequest = { scope: SessionScope, operation: ConfirmationOperation, };
export type ConfirmationIntent = { id: IntentId, scope: SessionScope, operation: ConfirmationOperation, expiresInMs: number, };
export type MutationRequest = { scope: SessionScope, intentId: IntentId, spec: MutationSpec, };
export type MutationOutcome = "succeeded" | "failed" | "unknown";
export type MutationResponse = { scope: SessionScope, spec: MutationSpec, outcome: MutationOutcome, };
export type TerminalRequest = { scope: SessionScope, intentId: IntentId, spec: TerminalSpec, };
export type TerminalResponse = { scope: SessionScope, terminalId: SubscriptionId, };
export type InspectContainerRequest = { scope: SessionScope, containerId: ContainerId, };
export type ContainerLogsRequest = { scope: SessionScope, containerId: ContainerId, tail: number, timeoutSeconds: number, };
export type LogSnapshot = { scope: SessionScope, containerId: ContainerId, text: string, truncated: boolean, };
