// Generated from src-tauri/src/domain.rs with ts-rs. Do not edit.
export type HostId = string;
export type SessionId = string;
export type ContainerId = string;
export type ImageId = string;
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
export type ErrorCode = "invalid_id" | "invalid_generation" | "host_not_found" | "container_not_found" | "log_driver_unsupported" | "export_failed" | "session_not_found" | "stale_session" | "subscription_not_found" | "feature_unavailable" | "permission_denied" | "resource_limit" | "transport_unavailable" | "invalid_response" | "internal" | "storage_unavailable" | "storage_conflict" | "invalid_preferences" | "invalid_limits" | "invalid_intent" | "intent_expired" | "disconnected" | "operation_timed_out" | "operation_cancelled" | "invalid_alias" | "invalid_config_path" | "invalid_remote_argument" | "ssh_unavailable" | "ssh_resolution_failed";
export type AppError = { code: ErrorCode, message: string, scope: SessionScope | null, };
export type ConnectionState = "disconnected" | "resolving" | "connecting" | "probing" | "ready" | "degraded" | "error";
export type HostCapabilities = { docker: boolean, compose: boolean, management: boolean, terminal: boolean, };
export type HostSummary = { id: HostId, alias: string, displayName: string, group: string, readOnly: boolean, connectionState: ConnectionState, };
export type ContainerSummary = { scope: SessionScope, id: ContainerId, name: string, image: string,
/**
 * Preserve unknown Docker states as untrusted text.
 */
state: string, status: string, health: string | null, ports: Array<ContainerPort>, compose: ComposeLabels | null, cli: ContainerListDisplay | null, };
export type ContainerListDisplay = {
/**
 * CLI name aliases and display strings; never precise inspect/authorization data.
 */
names: Array<string>, ports: string | null, createdAt: string | null, runningFor: string | null,
/**
 * Missing field is unknown. Label values never cross default list IPC.
 */
labelsPresent: boolean | null, };
export type ContainerDetail = { healthcheckConfigured: boolean | null, oomKilled: boolean | null, exposedPorts: Array<ExposedPort>, summary: ContainerSummary,
/**
 * No environment values or unrestricted inspect JSON cross default IPC.
 */
environmentNames: Array<string>, environmentValuesMasked: boolean, environment: Array<DetailValue>, labels: Array<DetailValue>, createdAt: string | null, startedAt: string | null, finishedAt: string | null, exitCode: number | null, restartCount: number | null, restartPolicy: string | null, restartMaximumRetryCount: number | null, imageId: ImageId | null, mounts: Array<DetailMount>, networks: Array<DetailNetwork>, resources: ResourceConfiguration, };
export type ExposedPort = { privatePort: number, protocol: string, };
export type DetailValue = { name: string, value: string | null, masked: boolean, };
export type DetailMount = { kind: string | null, name: string | null, source: string | null, destination: string | null, readWrite: boolean | null, propagation: string | null, };
export type DetailNetwork = { name: string, networkId: string | null, ipv4: string | null, ipv6: string | null, gateway: string | null, macAddress: string | null, aliases: Array<string>, };
export type ResourceConfiguration = {
/**
 * Exact decimal integers as strings; avoid JS precision loss. Null means unavailable.
 */
memoryBytes: string | null, memorySwapBytes: string | null, nanoCpus: string | null, cpuShares: string | null, cpuPeriod: string | null, cpuQuota: string | null, cpusetCpus: string | null, pidsLimit: string | null, privileged: boolean | null, readOnlyRootfs: boolean | null, };
export type ListHostsResponse = { hosts: Array<HostSummary>, };
export type ConnectHostRequest = { selection: HostSelection, };
export type ConnectHostResponse = { scope: SessionScope, capabilities: HostCapabilities, };
export type ListContainersRequest = { scope: SessionScope, };
export type ListContainersResponse = { scope: SessionScope, containers: Array<ContainerSummary>, };
export type CancelSubscriptionRequest = { scope: SessionScope, subscriptionId: SubscriptionId, };
export type CancelSubscriptionResponse = { scope: SessionScope, subscriptionId: SubscriptionId, };
export type AppVersion = { version: string, };
export type Theme = "system" | "light" | "dark";
export type SavedHost = { id: HostId, alias: string, displayName: string, group: string, labels: Array<string>, readOnly: boolean, favorite: boolean, ssh: SshSelection | null, docker: DockerOptions, };
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
export type InspectContainerRequest = { scope: SessionScope, containerId: ContainerId,
/**
 * Explicit one-request reveal, bound to the current session; never persisted.
 */
revealSensitive: boolean, };
export type ContainerLogsRequest = {
/**
 * UTC Unix seconds with optional fractional nanoseconds, validated before dispatch.
 */
since: string | null, until: string | null, scope: SessionScope, containerId: ContainerId, tail: number, timeoutSeconds: number, };
export type LogSnapshot = { scope: SessionScope, containerId: ContainerId,
/**
 * Timestamp order when available; ties retain channel order, not guaranteed transport order.
 */
records: Array<LogRecord>, truncated: boolean, droppedRecords: number, stderrAmbiguous: boolean, };
export type ContainerStatsRequest = { scope: SessionScope, containerId: ContainerId, };
export type StatsAvailability = "available" | "stopped" | "missing" | "unavailable";
export type StatsValues = { cpuPercent: number | null, memoryUsageBytes: number | null, memoryLimitBytes: number | null, memoryPercent: number | null, networkRxBytes: number | null, networkTxBytes: number | null, blockReadBytes: number | null, blockWriteBytes: number | null, pids: number | null, };
export type StatsRaw = { cpu: string | null, memory: string | null, memoryPercent: string | null, network: string | null, block: string | null, pids: string | null, };
export type StatsSample = { scope: SessionScope, containerId: ContainerId, capturedAtMs: number, availability: StatsAvailability, values: StatsValues, raw: StatsRaw, };
export type LogChannel = "stdout" | "stderr_ambiguous";
export type LogRecord = { text: string, timestamp: string | null, channel: LogChannel, truncated: boolean, invalidUtf8: boolean, };
export type FollowLogsRequest = { scope: SessionScope, containerId: ContainerId, tail: number, since: string | null, };
export type AckLogsRequest = { scope: SessionScope, subscriptionId: SubscriptionId, sequence: number, };
export type LogBatch = { scope: SessionScope, containerId: ContainerId, subscriptionId: SubscriptionId, sequence: number, records: Array<LogRecord>, droppedRecords: number, gap: boolean, ended: boolean, error: ErrorCode | null, };
export type FollowEventsRequest = { scope: SessionScope, since: string | null, };
export type ContainerEventAction = "create" | "start" | "stop" | "die" | "destroy" | "restart" | "pause" | "unpause" | "rename" | "health_status";
export type ContainerEvent = { actorId: ContainerId, action: ContainerEventAction, timestampUnixNanos: string, };
export type EventBatch = { scope: SessionScope, subscriptionId: SubscriptionId, sequence: number, events: Array<ContainerEvent>, droppedRecords: number, gap: boolean, ended: boolean, error: ErrorCode | null, };
export type ExportLogsRequest = { scope: SessionScope, containerId: ContainerId, lines: Array<string>, secretsAcknowledged: boolean, };
export type ExportLogsResponse = { saved: boolean, lineCount: number, };
export type ContainerPort = { hostIp: string | null, publicPort: number | null, privatePort: number, protocol: string, };
export type ComposeLabels = { project: string, service: string | null, };
export type WorkspaceMode = "live" | "demo";
export type DemoScenario = "standard" | "empty" | "permission_failure" | "invalid_json" | "huge_record" | "disconnect" | "timeout";
export type SwitchWorkspaceRequest = { "mode": "live" } | { "mode": "demo", scenario: DemoScenario, };
export type WorkspaceModeSnapshot = { mode: WorkspaceMode, scenario: DemoScenario | null, scope: SessionScope | null, host: HostSummary | null, };
export type DiscoverHostsRequest = { configPath: string | null, };
export type SshConfigPath = { path: string, };
export type SshCandidate = { alias: string, source: string, line: number, };
export type DiscoveryWarning = { code: DiscoveryWarningCode, source: string, line: number | null, };
export type DiscoveryWarningCode = "missing_file" | "unreadable_file" | "unsupported_syntax" | "patterns_skipped" | "conditional_include" | "match_skipped" | "include_cycle" | "limit_reached";
export type HostDiscovery = { configPath: string, candidates: Array<SshCandidate>, warnings: Array<DiscoveryWarning>, };
export type SelectSshAliasRequest = { configPath: string | null, alias: string, };
export type SshSelection = { useDefaultConfig: boolean, configPath: string, alias: string, };
export type ResolveSshRequest = { selection: SshSelection, };
export type EffectiveSshConfig = { selection: SshSelection, executablePath: string, hostname: string, user: string, port: number, proxyJump: string | null, hasProxyCommand: boolean, };
export type SshAccessStatus = "verified" | "unknown_host_key" | "changed_host_key" | "host_key_rejected" | "authentication_failed" | "timed_out" | "output_limit" | "connection_failed" | "remote_command_failed";
export type SshAccessReport = { selection: SshSelection, status: SshAccessStatus, sshError: string | null, };
export type ConnectionStage = "resolve" | "authenticate" | "probe";
export type ConnectionDiagnosticCode = "resolution_failed" | "unknown_host_key" | "changed_host_key" | "host_key_rejected" | "authentication_failed" | "connection_failed" | "timed_out" | "output_limit" | "probe_unavailable" | "docker_unavailable" | "remote_command_failed" | "connection_lost";
export type ConnectionToken = { sessionId: SessionId, sessionGeneration: number, };
export type ConnectionRequest = { token: ConnectionToken, };
export type StageDuration = { stage: ConnectionStage, durationMs: number, };
export type ConnectionDiagnostic = { stage: ConnectionStage, code: ConnectionDiagnosticCode, };
export type ConnectionSnapshot = { hostId: HostId | null, effective: EffectiveSshConfig | null, token: ConnectionToken, selection: SshSelection, state: ConnectionState, durations: Array<StageDuration>, diagnostic: ConnectionDiagnostic | null, hasJump: boolean, transportMode: SshTransportMode, dockerOptions: DockerOptions, docker: DockerProbeReport | null, };
export type HostDraft = { ssh: SshSelection, docker: DockerOptions, displayName: string, group: string, labels: Array<string>, favorite: boolean, };
export type InventoryModeRequest = { mode: WorkspaceMode, };
export type SaveHostRequest = { mode: WorkspaceMode, expectedRevision: number, id: HostId | null, draft: HostDraft, };
export type RemoveHostRequest = { mode: WorkspaceMode, expectedRevision: number, hostId: HostId, };
export type InventoryConnectRequest = { mode: WorkspaceMode, hostId: HostId, };
export type InventoryDisconnectRequest = { mode: WorkspaceMode, hostId: HostId, token: ConnectionToken, };
export type HostInventory = { mode: WorkspaceMode, saved: PreferencesSnapshot, connection: ConnectionSnapshot | null, };
export type SshTransportMode = "unconnected" | "multiplexed" | "direct_fallback";
export type DockerOptions = { executable: string | null, context: string | null, sudo: boolean, };
export type BeginSshRequest = { selection: SshSelection, docker: DockerOptions, };
export type DockerProbeStatus = "ready" | "docker_missing" | "daemon_unavailable" | "permission_denied" | "sudo_authentication_required" | "sudo_denied" | "invalid_context" | "unsupported_endpoint" | "unsupported_os" | "invalid_response" | "identity_changed" | "timed_out" | "output_limit" | "connection_failed" | "command_failed";
export type DockerEndpointKind = "unix" | "tcp" | "ssh";
export type ComposeAvailability = "available" | "absent" | "unknown";
export type DockerProbeReport = { status: DockerProbeStatus, context: string | null, endpoint: string | null, endpointKind: DockerEndpointKind | null, clientVersion: string | null, serverVersion: string | null, daemonId: string | null, os: string | null, rootless: boolean | null, compose: ComposeAvailability, composeVersion: string | null, sudo: boolean, };
