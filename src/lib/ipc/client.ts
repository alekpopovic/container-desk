import { ReadScheduler, type ReadCommand } from "../reads/scheduler.ts";
import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  AppError,
  ErrorCode,
  HostSelection,
  SessionScope,
  ListHostsResponse,
  ConnectHostRequest,
  ConnectHostResponse,
  ListContainersResponse,
  CancelSubscriptionRequest,
  CancelSubscriptionResponse,
  ContainerSummary,
} from "./generated.ts";

const messages: Record<ErrorCode, string> = {
  invalid_id: "Invalid resource identifier.",
  invalid_remote_argument:
    "A remote operation argument is invalid or unsupported.",
  ssh_unavailable:
    "The selected OpenSSH executable is unavailable or untrusted. Check native dependency settings.",
  ssh_resolution_failed:
    "OpenSSH could not resolve this alias. Check the trusted configuration in your terminal.",
  invalid_alias:
    "Use a concrete SSH alias: letters, digits, dots, underscores or dashes, starting with a letter or digit (maximum 256 bytes).",
  invalid_config_path:
    "Choose an absolute local SSH config path or use the default.",
  invalid_generation: "Invalid session or selection generation.",
  export_failed:
    "The selected log file could not be saved. Choose a writable regular file location.",
  log_driver_unsupported:
    "This container logging driver does not support reading logs.",
  container_not_found: "The container no longer exists. Refresh the inventory.",
  compose_configuration_unavailable:
    "Compose configuration could not be verified. Check remote paths, required environment files and configuration dependencies in your terminal.",
  compose_project_mismatch:
    "The configured Compose project does not match the existing services, or its configuration changed. Verify the project again.",
  compose_verification_expired:
    "Compose verification expired. Verify the project again before requesting an action.",
  network_not_found:
    "The network no longer exists. Refresh the network inventory.",
  volume_not_found:
    "The volume no longer exists. Refresh the volume inventory.",
  image_not_found: "The image no longer exists. Refresh the image inventory.",
  container_not_running: "Open a terminal only in a running container.",
  terminal_closed: "The terminal is closed. Open a new session explicitly.",
  terminal_shell_unavailable:
    "The selected shell could not be started. Choose an installed shell explicitly.",
  container_not_stopped: "Only stopped containers can be removed.",
  host_not_found: "Saved host does not exist.",
  session_not_found: "Connection session does not exist.",
  stale_session: "The connection changed. Refresh the selected host.",
  subscription_not_found: "Subscription does not exist in this session.",
  feature_unavailable: "This operation is not available yet.",
  permission_denied: "This operation is not permitted.",
  resource_limit: "The operation exceeded an application limit.",
  transport_unavailable: "The desktop connection is unavailable.",
  invalid_response: "The desktop returned an invalid response.",
  internal: "The operation could not be completed.",
  storage_unavailable:
    "Local data could not be read or saved. Check application storage before retrying.",
  storage_conflict: "Settings changed. Reload before saving again.",
  invalid_preferences: "Settings contain invalid or unsupported values.",
  invalid_limits: "An operation limit is outside the supported range.",
  invalid_intent:
    "This confirmation does not match the operation or was already used.",
  intent_expired: "This confirmation expired. Review the operation again.",
  disconnected: "The connection closed. Reconnect before refreshing.",
  operation_cancelled: "The operation was cancelled.",
  operation_timed_out: "The command exceeded its deadline.",
};

export class IpcError extends Error implements AppError {
  readonly code: ErrorCode;
  readonly scope: SessionScope | null;
  constructor(code: ErrorCode, scope: SessionScope | null = null) {
    super(messages[code]);
    this.name = "IpcError";
    this.code = code;
    this.scope = scope;
  }
}

const reads = new ReadScheduler((code) => new IpcError(code));
function readHostKey(scope: SessionScope) {
  return JSON.stringify([scope.selection.hostId, scope.daemonId]);
}
async function mutationCall(
  command: "mutate_container" | "mutate_compose_project",
  request:
    | import("./generated.ts").MutationRequest
    | import("./generated.ts").ComposeMutationRequest,
  current: () => SessionScope | null,
) {
  const selected = structuredClone(request.scope);
  try {
    return await call(command, request, () => sameScope(selected, current()));
  } finally {
    // Even a lost response can follow a completed mutation. Reconciliation must start a new read.
    reads.invalidate(readHostKey(selected));
  }
}
function scheduledRead(
  command: ReadCommand,
  request: { scope: SessionScope },
  current: () => SessionScope | null,
) {
  const captured = structuredClone(request);
  // Exact request (including session, full ID and reveal flag) is the single-flight identity.
  return reads.run(
    readHostKey(captured.scope),
    JSON.stringify([command, captured]),
    command,
    () => sameScope(captured.scope, current()),
    () => call(command, captured),
  );
}

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
function generation(value: unknown): value is number {
  return (
    typeof value === "number" &&
    Number.isInteger(value) &&
    value > 0 &&
    value <= 0xffffffff
  );
}
function text(value: unknown, max = 4096): value is string {
  return typeof value === "string" && value.length <= max;
}
function selection(value: unknown): value is HostSelection {
  return (
    record(value) &&
    text(value.hostId, 34) &&
    /^h_[a-f0-9]{32}$/.test(value.hostId) &&
    generation(value.selectionGeneration)
  );
}
function scope(value: unknown): value is SessionScope {
  return (
    record(value) &&
    selection(value.selection) &&
    text(value.sessionId, 34) &&
    /^s_[a-f0-9]{32}$/.test(value.sessionId) &&
    generation(value.sessionGeneration) &&
    text(value.daemonId, 256) &&
    value.daemonId.length > 0
  );
}
function sameSelection(a: HostSelection, b: HostSelection): boolean {
  return (
    a.hostId === b.hostId && a.selectionGeneration === b.selectionGeneration
  );
}
export function sameScope(a: SessionScope, b: SessionScope | null): boolean {
  return (
    b !== null &&
    sameSelection(a.selection, b.selection) &&
    a.sessionId === b.sessionId &&
    a.sessionGeneration === b.sessionGeneration &&
    a.daemonId === b.daemonId
  );
}

async function call(
  command: string,
  request?: unknown,
  isCurrent = () => true,
  extra: Record<string, unknown> = {},
): Promise<unknown> {
  let result: unknown;
  try {
    result = await invoke<unknown>(
      command,
      request === undefined ? extra : { request, ...extra },
    );
  } catch (error) {
    if (!isCurrent()) throw new IpcError("stale_session");
    // No raw bridge/remote message is propagated to diagnostics or the UI.
    if (
      record(error) &&
      typeof error.code === "string" &&
      Object.hasOwn(messages, error.code)
    ) {
      throw new IpcError(
        error.code as ErrorCode,
        scope(error.scope) ? error.scope : null,
      );
    }
    throw new IpcError("transport_unavailable");
  }
  if (!isCurrent()) throw new IpcError("stale_session");
  return result;
}

export async function listHosts(): Promise<ListHostsResponse> {
  const result = await call("list_hosts");
  const states = [
    "disconnected",
    "resolving",
    "connecting",
    "probing",
    "ready",
    "degraded",
    "error",
  ];
  if (
    !record(result) ||
    !Array.isArray(result.hosts) ||
    result.hosts.length > 1000 ||
    !result.hosts.every(
      (host) =>
        record(host) &&
        text(host.id, 34) &&
        /^h_[a-f0-9]{32}$/.test(host.id) &&
        text(host.alias, 256) &&
        text(host.displayName) &&
        text(host.group) &&
        typeof host.readOnly === "boolean" &&
        typeof host.connectionState === "string" &&
        states.includes(host.connectionState),
    )
  ) {
    throw new IpcError("invalid_response");
  }
  return result as ListHostsResponse;
}

export async function connectHost(
  request: ConnectHostRequest,
  current: () => HostSelection | null,
): Promise<ConnectHostResponse> {
  const result = await call("connect_host", request, () => {
    const selected = current();
    return selected !== null && sameSelection(request.selection, selected);
  });
  if (
    !record(result) ||
    !scope(result.scope) ||
    !record(result.capabilities) ||
    !["docker", "compose", "management", "terminal"].every(
      (key) =>
        typeof (result.capabilities as Record<string, unknown>)[key] ===
        "boolean",
    )
  ) {
    throw new IpcError("invalid_response");
  }
  const selected = current();
  if (
    selected === null ||
    !sameSelection(request.selection, selected) ||
    !sameSelection(result.scope.selection, selected)
  ) {
    throw new IpcError("stale_session");
  }
  return result as ConnectHostResponse;
}

function portNumber(value: unknown): value is number {
  return (
    typeof value === "number" &&
    Number.isInteger(value) &&
    value > 0 &&
    value <= 65535
  );
}

function container(
  value: unknown,
  expected: SessionScope,
): value is ContainerSummary {
  return (
    record(value) &&
    scope(value.scope) &&
    sameScope(value.scope, expected) &&
    text(value.id, 64) &&
    /^[a-f0-9]{64}$/.test(value.id) &&
    text(value.name) &&
    text(value.image) &&
    text(value.state) &&
    text(value.status) &&
    (value.health === null || text(value.health)) &&
    Array.isArray(value.ports) &&
    value.ports.length <= 128 &&
    value.ports.every(
      (port) =>
        record(port) &&
        (port.hostIp === null || text(port.hostIp, 64)) &&
        (port.publicPort === null || portNumber(port.publicPort)) &&
        portNumber(port.privatePort) &&
        ["tcp", "udp", "sctp"].includes(String(port.protocol)),
    ) &&
    (value.cli === null ||
      (record(value.cli) &&
        Array.isArray(value.cli.names) &&
        value.cli.names.length <= 128 &&
        value.cli.names.every((name) => text(name)) &&
        (value.cli.ports === null || text(value.cli.ports, 16384)) &&
        (value.cli.createdAt === null || text(value.cli.createdAt)) &&
        (value.cli.runningFor === null || text(value.cli.runningFor)) &&
        (value.cli.labelsPresent === null ||
          typeof value.cli.labelsPresent === "boolean"))) &&
    (value.compose === null ||
      (record(value.compose) &&
        text(value.compose.project, 256) &&
        (value.compose.service === null || text(value.compose.service, 256))))
  );
}

export async function listContainers(
  expected: SessionScope,
  current: () => SessionScope | null,
): Promise<ListContainersResponse> {
  const result = await scheduledRead(
    "list_containers",
    { scope: expected },
    current,
  );
  if (
    !record(result) ||
    !scope(result.scope) ||
    !Array.isArray(result.containers) ||
    result.containers.length > 50000
  ) {
    throw new IpcError("invalid_response");
  }
  if (!sameScope(result.scope, expected) || !sameScope(expected, current()))
    throw new IpcError("stale_session");
  if (!result.containers.every((value) => container(value, expected)))
    throw new IpcError("invalid_response");
  return result as ListContainersResponse;
}

export async function cancelSubscription(
  request: CancelSubscriptionRequest,
  current: () => SessionScope | null,
): Promise<CancelSubscriptionResponse> {
  const result = await call("cancel_subscription", request, () =>
    sameScope(request.scope, current()),
  );
  if (
    !record(result) ||
    !scope(result.scope) ||
    result.subscriptionId !== request.subscriptionId
  )
    throw new IpcError("invalid_response");
  if (
    !sameScope(result.scope, request.scope) ||
    !sameScope(request.scope, current())
  )
    throw new IpcError("stale_session");
  return result as CancelSubscriptionResponse;
}

function preferencesSnapshot(
  value: unknown,
): value is import("./generated.ts").PreferencesSnapshot {
  if (
    !record(value) ||
    !record(value.preferences) ||
    typeof value.writable !== "boolean" ||
    (value.notice !== null &&
      ![
        "migrated",
        "recovered_previous",
        "reset_after_corruption",
        "unsupported_schema",
      ].includes(String(value.notice)))
  )
    return false;
  const p = value.preferences;
  return (
    p.schemaVersion === 3 &&
    typeof p.revision === "number" &&
    Number.isInteger(p.revision) &&
    p.revision >= 0 &&
    p.revision <= 0xffffffff &&
    ["system", "light", "dark"].includes(String(p.theme)) &&
    (p.selectedHostId === null ||
      (text(p.selectedHostId, 34) &&
        /^h_[a-f0-9]{32}$/.test(p.selectedHostId))) &&
    [p.trustedConfigPath, p.sshExecutableOverride].every(
      (path) => path === null || text(path, 4096),
    ) &&
    Array.isArray(p.hosts) &&
    p.hosts.length <= 1000 &&
    p.hosts.every(
      (host) =>
        record(host) &&
        text(host.id, 34) &&
        /^h_[a-f0-9]{32}$/.test(host.id) &&
        text(host.alias, 256) &&
        text(host.displayName, 256) &&
        text(host.group, 128) &&
        typeof host.readOnly === "boolean" &&
        typeof host.favorite === "boolean" &&
        validDockerOptions(host.docker) &&
        (host.ssh === null ||
          (validSshSelection(host.ssh) && host.ssh.alias === host.alias)) &&
        Array.isArray(host.labels) &&
        host.labels.length <= 32 &&
        host.labels.every((label) => text(label, 128)),
    )
  );
}
export async function getPreferences(): Promise<
  import("./generated.ts").PreferencesSnapshot
> {
  const result = await call("get_preferences");
  if (!preferencesSnapshot(result)) throw new IpcError("invalid_response");
  return result;
}
export async function setTheme(
  request: import("./generated.ts").SetThemeRequest,
): Promise<import("./generated.ts").PreferencesSnapshot> {
  const result = await call("set_theme", request);
  if (
    !preferencesSnapshot(result) ||
    result.preferences.theme !== request.theme ||
    result.preferences.revision !== request.expectedRevision + 1
  )
    throw new IpcError("invalid_response");
  return result;
}

function sshDiagnostic(
  value: unknown,
): value is import("./generated.ts").SshDiagnostic {
  return (
    record(value) &&
    text(value.path, 4096) &&
    text(value.message) &&
    (value.version === null || text(value.version, 512)) &&
    [
      "ready",
      "missing",
      "not_executable",
      "untrusted",
      "failed",
      "timed_out",
      "output_limit",
      "invalid_version",
    ].includes(String(value.status))
  );
}
export async function getDependencyDiagnostics(): Promise<
  import("./generated.ts").DependencyDiagnostics
> {
  const result = await call("dependency_diagnostics");
  if (
    !record(result) ||
    !text(result.appVersion, 128) ||
    !text(result.platform, 64) ||
    !text(result.architecture, 64) ||
    !sshDiagnostic(result.ssh) ||
    !record(result.agent) ||
    !text(result.agent.message) ||
    !["unset", "missing", "not_socket", "inaccessible", "reachable"].includes(
      String(result.agent.status),
    )
  )
    throw new IpcError("invalid_response");
  return result as import("./generated.ts").DependencyDiagnostics;
}
export async function setSshExecutable(
  request: import("./generated.ts").SetSshExecutableRequest,
): Promise<import("./generated.ts").SetSshExecutableResponse> {
  const result = await call("set_ssh_executable", request);
  if (
    !record(result) ||
    !sshDiagnostic(result.ssh) ||
    (result.preferences !== null && !preferencesSnapshot(result.preferences))
  )
    throw new IpcError("invalid_response");
  if (
    result.preferences !== null &&
    (result.ssh.status !== "ready" ||
      result.preferences.preferences.revision !==
        request.expectedRevision + 1 ||
      result.preferences.preferences.sshExecutableOverride !== request.path)
  )
    throw new IpcError("invalid_response");
  return result as import("./generated.ts").SetSshExecutableResponse;
}

function workspaceMode(
  value: unknown,
): value is import("./generated.ts").WorkspaceModeSnapshot {
  if (!record(value)) return false;
  if (value.mode === "live")
    return (
      value.scenario === null && value.scope === null && value.host === null
    );
  return (
    value.mode === "demo" &&
    scope(value.scope) &&
    record(value.host) &&
    value.host.id === value.scope.selection.hostId &&
    text(value.host.alias, 256) &&
    text(value.host.displayName, 256) &&
    text(value.host.group, 128) &&
    value.host.readOnly === true &&
    value.host.connectionState === "ready" &&
    [
      "standard",
      "empty",
      "permission_failure",
      "invalid_json",
      "huge_record",
      "disconnect",
      "timeout",
    ].includes(String(value.scenario))
  );
}
export async function getWorkspaceMode(): Promise<
  import("./generated.ts").WorkspaceModeSnapshot
> {
  const result = await call("get_workspace_mode");
  if (!workspaceMode(result)) throw new IpcError("invalid_response");
  return result;
}
export async function switchWorkspace(
  request: import("./generated.ts").SwitchWorkspaceRequest,
): Promise<import("./generated.ts").WorkspaceModeSnapshot> {
  const result = await call("switch_workspace", request);
  if (
    !workspaceMode(result) ||
    result.mode !== request.mode ||
    (request.mode === "demo" && result.scenario !== request.scenario)
  )
    throw new IpcError("invalid_response");
  return result;
}

import type {
  HostDiscovery,
  SshConfigPath,
  SshSelection,
} from "./generated.ts";
const discoveryCodes = new Set([
  "missing_file",
  "unreadable_file",
  "unsupported_syntax",
  "patterns_skipped",
  "conditional_include",
  "match_skipped",
  "include_cycle",
  "limit_reached",
]);
export function isConcreteAlias(value: string): boolean {
  return value.length <= 256 && /^[a-zA-Z0-9][a-zA-Z0-9._-]*$/.test(value);
}
export async function getSshConfigPath(): Promise<SshConfigPath> {
  const value = await call("get_ssh_config_path");
  if (!record(value) || !text(value.path) || !value.path.startsWith("/"))
    throw new IpcError("invalid_response");
  return value as SshConfigPath;
}
export async function discoverSshHosts(
  configPath: string | null,
): Promise<HostDiscovery> {
  const value = await call("discover_ssh_hosts", { configPath });
  if (
    !record(value) ||
    !text(value.configPath) ||
    !value.configPath.startsWith("/") ||
    !Array.isArray(value.candidates) ||
    value.candidates.length > 1000 ||
    !value.candidates.every(
      (c) =>
        record(c) &&
        text(c.alias, 256) &&
        isConcreteAlias(c.alias) &&
        text(c.source) &&
        generation(c.line),
    ) ||
    !Array.isArray(value.warnings) ||
    value.warnings.length > 128 ||
    !value.warnings.every(
      (w) =>
        record(w) &&
        typeof w.code === "string" &&
        discoveryCodes.has(w.code) &&
        text(w.source) &&
        (w.line === null || generation(w.line)),
    )
  )
    throw new IpcError("invalid_response");
  return value as HostDiscovery;
}
export async function selectSshAlias(
  configPath: string | null,
  alias: string,
): Promise<SshSelection> {
  if (!isConcreteAlias(alias)) throw new IpcError("invalid_alias");
  const value = await call("select_ssh_alias", { configPath, alias });
  if (
    !record(value) ||
    value.alias !== alias ||
    typeof value.useDefaultConfig !== "boolean" ||
    (configPath !== null && value.configPath !== configPath) ||
    !text(value.configPath) ||
    !value.configPath.startsWith("/")
  )
    throw new IpcError("invalid_response");
  return value as SshSelection;
}

import type { EffectiveSshConfig } from "./generated.ts";
export async function resolveSshConfig(
  selection: SshSelection,
): Promise<EffectiveSshConfig> {
  return decodeEffective(
    await call("resolve_ssh_config", { selection }),
    selection,
  );
}
function decodeEffective(
  value: unknown,
  selection: SshSelection,
): EffectiveSshConfig {
  if (
    !record(value) ||
    !record(value.selection) ||
    value.selection.alias !== selection.alias ||
    value.selection.configPath !== selection.configPath ||
    value.selection.useDefaultConfig !== selection.useDefaultConfig ||
    !text(value.executablePath) ||
    !value.executablePath.startsWith("/") ||
    !text(value.hostname, 1024) ||
    value.hostname.length === 0 ||
    !text(value.user, 1024) ||
    value.user.length === 0 ||
    typeof value.port !== "number" ||
    !Number.isInteger(value.port) ||
    value.port < 1 ||
    value.port > 65535 ||
    !(value.proxyJump === null || text(value.proxyJump, 1024)) ||
    typeof value.hasProxyCommand !== "boolean"
  )
    throw new IpcError("invalid_response");
  return value as EffectiveSshConfig;
}

import type { SshAccessReport, SshAccessStatus } from "./generated.ts";
export const sshAccessHelp: Record<SshAccessStatus, string> = {
  remote_command_failed:
    "SSH authenticated, but the fixed remote access command failed or returned an unexpected result.",
  verified: "SSH access verified. Docker readiness has not been checked.",
  unknown_host_key:
    "A destination or jump host is not trusted yet. Verify its fingerprint independently in your terminal, then retry explicitly.",
  changed_host_key:
    "A destination or jump host key has changed. Stop and verify the change with its administrator before repairing known_hosts in your terminal.",
  host_key_rejected:
    "SSH rejected a destination or jump host key. Review trust and revocation with its administrator.",
  authentication_failed:
    "SSH authentication failed at the destination or a jump host. Check each hop and load encrypted keys in your normal OS agent.",
  timed_out:
    "The SSH check reached its 15-second deadline. Check network access, each jump host and your agent, then retry explicitly.",
  output_limit:
    "The SSH check exceeded its output limit. Review remote login output in your terminal.",
  connection_failed:
    "SSH access could not be verified. Check the selected alias and each jump host in your terminal.",
};
export async function checkSshAccess(
  selection: SshSelection,
): Promise<SshAccessReport> {
  const value = await call("check_ssh_access", { selection });
  if (
    !record(value) ||
    !record(value.selection) ||
    value.selection.alias !== selection.alias ||
    value.selection.configPath !== selection.configPath ||
    value.selection.useDefaultConfig !== selection.useDefaultConfig ||
    typeof value.status !== "string" ||
    !Object.hasOwn(sshAccessHelp, value.status) ||
    !(value.sshError === null || text(value.sshError, 256))
  )
    throw new IpcError("invalid_response");
  return value as SshAccessReport;
}

import type {
  ConnectionSnapshot,
  ConnectionToken,
  ConnectionState,
  ConnectionStage,
  ConnectionDiagnosticCode,
  DockerOptions,
  DockerProbeReport,
  DockerProbeStatus,
} from "./generated.ts";
export const connectionLabels: Record<ConnectionState, string> = {
  disconnected: "Disconnected",
  resolving: "Resolving configuration",
  connecting: "Authenticating SSH",
  probing: "Checking remote capabilities",
  ready: "Ready",
  degraded: "Limited connection",
  error: "Connection error",
};
export const connectionDiagnostics: Record<ConnectionDiagnosticCode, string> = {
  docker_unavailable:
    "SSH access works; review the Docker capability result below.",
  connection_lost:
    "The app-owned SSH connection ended or reached its idle limit. Reconnect explicitly; no command was replayed.",
  resolution_failed: "OpenSSH could not resolve the selected configuration.",
  unknown_host_key: sshAccessHelp.unknown_host_key,
  changed_host_key: sshAccessHelp.changed_host_key,
  host_key_rejected: sshAccessHelp.host_key_rejected,
  authentication_failed: sshAccessHelp.authentication_failed,
  connection_failed: sshAccessHelp.connection_failed,
  timed_out:
    "This connection stage exceeded its deadline. Retry explicitly after checking access.",
  output_limit: "SSH output exceeded the application limit.",
  probe_unavailable:
    "SSH access verified. Docker capability checks are not available in this increment.",
  remote_command_failed:
    "SSH connected, but the remote capability command failed.",
};
const connectionStages: ConnectionStage[] = [
  "resolve",
  "authenticate",
  "probe",
];
export function decodeConnection(
  value: unknown,
  expected: SshSelection,
  token?: ConnectionToken,
  dockerOptions?: DockerOptions,
): ConnectionSnapshot {
  if (
    !record(value) ||
    !record(value.token) ||
    !text(value.token.sessionId, 34) ||
    !/^s_[a-f0-9]{32}$/.test(value.token.sessionId) ||
    !generation(value.token.sessionGeneration) ||
    (token &&
      (value.token.sessionId !== token.sessionId ||
        value.token.sessionGeneration !== token.sessionGeneration)) ||
    !validDockerOptions(value.dockerOptions) ||
    (dockerOptions !== undefined &&
      (value.dockerOptions.executable !== dockerOptions.executable ||
        value.dockerOptions.context !== dockerOptions.context ||
        value.dockerOptions.sudo !== dockerOptions.sudo)) ||
    !(
      value.docker === null ||
      (validDockerReport(value.docker) &&
        value.docker.sudo === value.dockerOptions.sudo)
    ) ||
    !record(value.selection) ||
    value.selection.alias !== expected.alias ||
    value.selection.configPath !== expected.configPath ||
    value.selection.useDefaultConfig !== expected.useDefaultConfig ||
    typeof value.state !== "string" ||
    !Object.hasOwn(connectionLabels, value.state) ||
    typeof value.hasJump !== "boolean" ||
    !["unconnected", "multiplexed", "direct_fallback"].includes(
      value.transportMode as string,
    ) ||
    !Array.isArray(value.durations) ||
    value.durations.length > 3 ||
    !value.durations.every(
      (duration) =>
        record(duration) &&
        connectionStages.includes(duration.stage as ConnectionStage) &&
        typeof duration.durationMs === "number" &&
        Number.isInteger(duration.durationMs) &&
        duration.durationMs >= 0 &&
        duration.durationMs <= 0xffffffff,
    ) ||
    new Set(value.durations.map((duration) => duration.stage)).size !==
      value.durations.length ||
    !(
      value.diagnostic === null ||
      (record(value.diagnostic) &&
        connectionStages.includes(value.diagnostic.stage as ConnectionStage) &&
        typeof value.diagnostic.code === "string" &&
        Object.hasOwn(connectionDiagnostics, value.diagnostic.code))
    )
  )
    throw new IpcError("invalid_response");
  if (
    !(
      value.hostId === null ||
      (text(value.hostId, 34) && /^h_[a-f0-9]{32}$/.test(value.hostId))
    )
  )
    throw new IpcError("invalid_response");
  if (value.effective !== null) decodeEffective(value.effective, expected);
  return value as ConnectionSnapshot;
}
export async function beginSshSession(
  selected: SshSelection,
  docker: DockerOptions = { executable: null, context: null, sudo: false },
): Promise<ConnectionSnapshot> {
  return decodeConnection(
    await call("begin_ssh_session", { selection: selected, docker }),
    selected,
    undefined,
    docker,
  );
}
export async function getSshSession(
  current: ConnectionSnapshot,
): Promise<ConnectionSnapshot> {
  return decodeConnection(
    await call("get_ssh_session", { token: current.token }),
    current.selection,
    current.token,
    current.dockerOptions,
  );
}
export async function disconnectSshSession(
  current: ConnectionSnapshot,
): Promise<ConnectionSnapshot> {
  const result = decodeConnection(
    await call("disconnect_ssh_session", { token: current.token }),
    current.selection,
    undefined,
    current.dockerOptions,
  );
  if (
    result.token.sessionId !== current.token.sessionId ||
    result.token.sessionGeneration <= current.token.sessionGeneration ||
    result.state !== "disconnected"
  )
    throw new IpcError("invalid_response");
  return result;
}

export const dockerProbeLabels: Record<DockerProbeStatus, string> = {
  ready: "Linux Docker Engine is accessible.",
  docker_missing:
    "Docker was not found in the remote command path. Check its installation or choose an absolute executable path.",
  daemon_unavailable:
    "The selected Docker daemon is not reachable. Check whether it is running and the context endpoint is correct.",
  permission_denied:
    "The SSH user cannot access this Docker endpoint. Ask the server administrator to review existing access.",
  sudo_authentication_required:
    "sudo requires authentication. ContainerDesk uses sudo -n and cannot request a password.",
  sudo_denied:
    "The existing sudo policy does not allow this noninteractive Docker command.",
  invalid_context:
    "The selected remote Docker context is missing or unsupported.",
  unsupported_endpoint:
    "The Docker endpoint is unsupported or contains credential parameters; it was not exposed or used.",
  unsupported_os: "This daemon is not a supported Linux Docker Engine.",
  invalid_response:
    "Docker returned an invalid or unsupported capability response.",
  identity_changed:
    "The Docker context or daemon identity changed. Disconnect and connect again to review the new target.",
  timed_out: "The Docker capability check exceeded its deadline.",
  output_limit: "The Docker capability response exceeded the size limit.",
  connection_failed: "The SSH transport failed during the Docker check.",
  command_failed:
    "The remote Docker command failed. Check the selected binary, context and noninteractive environment.",
};
function validDockerOptions(value: unknown): value is DockerOptions {
  return (
    record(value) &&
    typeof value.sudo === "boolean" &&
    (value.executable === null || text(value.executable, 4096)) &&
    (value.context === null ||
      (text(value.context, 256) &&
        /^[a-zA-Z0-9][a-zA-Z0-9._-]*$/.test(value.context)))
  );
}
function validDockerReport(value: unknown): value is DockerProbeReport {
  if (
    !record(value) ||
    typeof value.status !== "string" ||
    !Object.hasOwn(dockerProbeLabels, value.status) ||
    typeof value.sudo !== "boolean" ||
    !(value.rootless === null || typeof value.rootless === "boolean") ||
    ![null, "unix", "tcp", "ssh"].includes(
      value.endpointKind as string | null,
    ) ||
    !["available", "absent", "unknown"].includes(value.compose as string)
  )
    return false;
  for (const [field, limit] of [
    ["context", 256],
    ["endpoint", 4096],
    ["clientVersion", 128],
    ["serverVersion", 128],
    ["daemonId", 256],
    ["os", 32],
    ["composeVersion", 128],
  ] as const) {
    if (value[field] !== null && !text(value[field], limit)) return false;
  }
  return (
    value.status !== "ready" ||
    (value.os === "linux" &&
      value.daemonId !== null &&
      value.endpoint !== null &&
      value.context !== null &&
      value.clientVersion !== null &&
      value.serverVersion !== null)
  );
}

function validSshSelection(value: unknown): value is SshSelection {
  return (
    record(value) &&
    typeof value.alias === "string" &&
    isConcreteAlias(value.alias) &&
    text(value.configPath, 4096) &&
    value.configPath.startsWith("/") &&
    typeof value.useDefaultConfig === "boolean"
  );
}
import type {
  HostInventory,
  WorkspaceMode,
  SaveHostRequest,
  RemoveHostRequest,
  InventoryConnectRequest,
  InventoryDisconnectRequest,
} from "./generated.ts";
export function decodeInventory(
  value: unknown,
  mode: WorkspaceMode,
): HostInventory {
  if (
    !record(value) ||
    value.mode !== mode ||
    !preferencesSnapshot(value.saved)
  )
    throw new IpcError("invalid_response");
  const ids = value.saved.preferences.hosts.map((h) => h.id);
  if (new Set(ids).size !== ids.length) throw new IpcError("invalid_response");
  if (value.connection !== null) {
    if (
      !record(value.connection) ||
      !validSshSelection(value.connection.selection)
    )
      throw new IpcError("invalid_response");
    const connection = decodeConnection(
      value.connection,
      value.connection.selection,
    );
    const host = value.saved.preferences.hosts.find(
      (h) => h.id === connection.hostId,
    );
    if (
      !host ||
      host.alias !== connection.selection.alias ||
      host.docker.context !== connection.dockerOptions.context ||
      host.docker.executable !== connection.dockerOptions.executable ||
      host.docker.sudo !== connection.dockerOptions.sudo ||
      (host.ssh &&
        (host.ssh.configPath !== connection.selection.configPath ||
          host.ssh.useDefaultConfig !== connection.selection.useDefaultConfig))
    )
      throw new IpcError("invalid_response");
  }
  return value as HostInventory;
}
export async function getHostInventory(
  mode: WorkspaceMode,
): Promise<HostInventory> {
  return decodeInventory(await call("get_host_inventory", { mode }), mode);
}
export async function saveHost(
  request: SaveHostRequest,
): Promise<HostInventory> {
  const result = decodeInventory(
    await call("save_host", request),
    request.mode,
  );
  if (
    result.saved.preferences.revision !== request.expectedRevision + 1 ||
    (request.id &&
      !result.saved.preferences.hosts.some((h) => h.id === request.id))
  )
    throw new IpcError("invalid_response");
  return result;
}
export async function removeHost(
  request: RemoveHostRequest,
): Promise<HostInventory> {
  const result = decodeInventory(
    await call("remove_host", request),
    request.mode,
  );
  if (
    result.saved.preferences.revision !== request.expectedRevision + 1 ||
    result.saved.preferences.hosts.some((h) => h.id === request.hostId)
  )
    throw new IpcError("invalid_response");
  return result;
}
export async function connectInventoryHost(
  request: InventoryConnectRequest,
): Promise<HostInventory> {
  const result = decodeInventory(
    await call("connect_inventory_host", request),
    request.mode,
  );
  if (result.connection?.hostId !== request.hostId)
    throw new IpcError("invalid_response");
  return result;
}
export async function disconnectInventoryHost(
  request: InventoryDisconnectRequest,
): Promise<HostInventory> {
  const result = decodeInventory(
    await call("disconnect_inventory_host", request),
    request.mode,
  );
  if (
    result.connection?.hostId !== request.hostId ||
    result.connection.state !== "disconnected" ||
    result.connection.token.sessionGeneration <=
      request.token.sessionGeneration ||
    result.connection.token.sessionId !== request.token.sessionId
  )
    throw new IpcError("invalid_response");
  return result;
}

export async function inspectContainer(
  request: import("./generated.ts").InspectContainerRequest,
  current: () => SessionScope | null,
): Promise<import("./generated.ts").ContainerDetail> {
  const result = await scheduledRead("inspect_container", request, current);
  const optional = (v: unknown) => v === null || text(v);
  const integer = (v: unknown) =>
    v === null || (typeof v === "number" && Number.isSafeInteger(v));
  const strings = (v: unknown, max: number) =>
    Array.isArray(v) && v.length <= max && v.every((x) => text(x));
  const secretValues = (v: unknown) =>
    Array.isArray(v) &&
    v.length <= 1024 &&
    v.every(
      (x) =>
        record(x) &&
        text(x.name) &&
        x.masked === !request.revealSensitive &&
        (x.masked ? x.value === null : text(x.value, 65536)),
    );
  if (
    !record(result) ||
    !container(result.summary, request.scope) ||
    result.summary.id !== request.containerId ||
    ![result.healthcheckConfigured, result.oomKilled].every(
      (v) => v === null || typeof v === "boolean",
    ) ||
    !Array.isArray(result.exposedPorts) ||
    result.exposedPorts.length > 128 ||
    !result.exposedPorts.every(
      (p) =>
        record(p) &&
        portNumber(p.privatePort) &&
        ["tcp", "udp", "sctp"].includes(String(p.protocol)),
    ) ||
    result.environmentValuesMasked !== !request.revealSensitive ||
    !strings(result.environmentNames, 1024) ||
    !secretValues(result.environment) ||
    !secretValues(result.labels) ||
    ![
      result.createdAt,
      result.startedAt,
      result.finishedAt,
      result.restartPolicy,
    ].every(optional) ||
    ![
      result.exitCode,
      result.restartCount,
      result.restartMaximumRetryCount,
    ].every(integer) ||
    !(
      result.imageId === null ||
      (text(result.imageId, 71) &&
        /^(sha256:)?[a-f0-9]{64}$/.test(result.imageId))
    ) ||
    !Array.isArray(result.mounts) ||
    result.mounts.length > 128 ||
    !result.mounts.every(
      (m) =>
        record(m) &&
        [m.kind, m.name, m.source, m.destination, m.propagation].every(
          optional,
        ) &&
        (m.readWrite === null || typeof m.readWrite === "boolean"),
    ) ||
    !Array.isArray(result.networks) ||
    result.networks.length > 128 ||
    !result.networks.every(
      (n) =>
        record(n) &&
        text(n.name) &&
        [n.networkId, n.ipv4, n.ipv6, n.gateway, n.macAddress].every(
          optional,
        ) &&
        strings(n.aliases, 128),
    ) ||
    !record(result.resources) ||
    ![
      "memoryBytes",
      "memorySwapBytes",
      "nanoCpus",
      "cpuShares",
      "cpuPeriod",
      "cpuQuota",
      "pidsLimit",
    ].every(
      (k) =>
        result.resources !== null &&
        record(result.resources) &&
        (result.resources[k] === null ||
          (text(result.resources[k], 21) &&
            /^-?[0-9]+$/.test(result.resources[k] as string))),
    ) ||
    !optional(result.resources.cpusetCpus) ||
    ![result.resources.privileged, result.resources.readOnlyRootfs].every(
      (x) => x === null || typeof x === "boolean",
    )
  )
    throw new IpcError("invalid_response");
  return result as import("./generated.ts").ContainerDetail;
}

export async function containerLogs(
  request: import("./generated.ts").ContainerLogsRequest,
  current: () => SessionScope | null,
): Promise<import("./generated.ts").LogSnapshot> {
  const result = await scheduledRead("container_logs", request, current);
  if (
    !record(result) ||
    !scope(result.scope) ||
    !sameScope(result.scope, request.scope) ||
    result.containerId !== request.containerId ||
    typeof result.truncated !== "boolean" ||
    typeof result.stderrAmbiguous !== "boolean" ||
    !Number.isSafeInteger(result.droppedRecords) ||
    Number(result.droppedRecords) < 0 ||
    !Array.isArray(result.records) ||
    result.records.length > 20000
  )
    throw new IpcError("invalid_response");
  validateLogRecords(result.records);
  return result as import("./generated.ts").LogSnapshot;
}
function validateLogRecords(records: unknown[], maxBytes = 8 * 1024 * 1024) {
  let retained = 0;
  const encoder = new TextEncoder();
  for (const row of records) {
    if (
      !record(row) ||
      !text(row.text, 262144) ||
      (row.timestamp !== null && !text(row.timestamp, 30)) ||
      !["stdout", "stderr_ambiguous"].includes(String(row.channel)) ||
      typeof row.truncated !== "boolean" ||
      typeof row.invalidUtf8 !== "boolean"
    )
      throw new IpcError("invalid_response");
    const length = encoder.encode(row.text).length;
    if (length > 262144) throw new IpcError("invalid_response");
    retained +=
      length + (typeof row.timestamp === "string" ? row.timestamp.length : 0);
    if (retained > maxBytes) throw new IpcError("invalid_response");
  }
}

type StreamEnvelope = {
  scope: SessionScope;
  subscriptionId: string;
  sequence: number;
  ended: boolean;
  error: ErrorCode | null;
};
async function followAcknowledged<B extends StreamEnvelope>(
  startCommand: string,
  ackCommand: string,
  request: { scope: SessionScope },
  validatePayload: (value: Record<string, unknown>) => void,
  current: () => SessionScope | null,
  consume: (batch: B) => void | Promise<void>,
  failed: (error: IpcError) => void,
  signal: AbortSignal,
): Promise<{ stop: () => Promise<void> }> {
  if (signal.aborted || !sameScope(request.scope, current()))
    throw new IpcError("stale_session");
  let subscriptionId: string | null = null;
  let sequence = 0;
  let stopped = false;
  let consuming = false;
  const channel = new Channel<unknown>();
  const stop = async () => {
    if (stopped) return;
    stopped = true;
    signal.removeEventListener("abort", abort);
    if (subscriptionId) {
      try {
        await call("cancel_subscription", {
          scope: request.scope,
          subscriptionId,
        });
      } catch (e) {
        if (!(e instanceof IpcError && e.code === "subscription_not_found"))
          throw e;
      }
    }
  };
  const abort = () => {
    void stop().catch(() => {});
  };
  channel.onmessage = (value) => {
    void (async () => {
      if (stopped) return;
      if (signal.aborted || !sameScope(request.scope, current())) {
        await stop().catch(() => {});
        return;
      }
      let holdsConsumption = false;
      try {
        if (
          consuming ||
          !record(value) ||
          !scope(value.scope) ||
          !sameScope(value.scope, request.scope) ||
          value.subscriptionId !== subscriptionId ||
          value.sequence !== sequence + 1 ||
          !Number.isSafeInteger(value.sequence) ||
          !Number.isSafeInteger(value.droppedRecords) ||
          Number(value.droppedRecords) < 0 ||
          Number(value.droppedRecords) > 0xffffffff ||
          typeof value.gap !== "boolean" ||
          typeof value.ended !== "boolean" ||
          (value.error !== null &&
            (typeof value.error !== "string" ||
              !Object.hasOwn(messages, value.error)))
        )
          throw new IpcError("invalid_response");
        validatePayload(value);
        consuming = true;
        holdsConsumption = true;
        sequence += 1;
        const batch = value as unknown as B;
        await consume(batch);
        consuming = false;
        holdsConsumption = false;
        if (batch.ended) {
          stopped = true;
          signal.removeEventListener("abort", abort);
          return;
        }
        if (signal.aborted || !sameScope(request.scope, current())) {
          await stop();
          return;
        }
        if (!stopped)
          await call(ackCommand, {
            scope: request.scope,
            subscriptionId,
            sequence,
          });
      } catch (error) {
        const report =
          error instanceof IpcError
            ? error
            : new IpcError("transport_unavailable");
        await stop().catch(() => {});
        if (!signal.aborted && sameScope(request.scope, current()))
          failed(report);
      } finally {
        if (holdsConsumption) consuming = false;
      }
    })();
  };
  // Do not discard a late successful start before its owned ID can be cancelled.
  const result = await call(startCommand, request, () => true, {
    onBatch: channel,
  });
  if (
    !record(result) ||
    !scope(result.scope) ||
    !sameScope(result.scope, request.scope) ||
    typeof result.subscriptionId !== "string" ||
    !/^sub_[a-f0-9]{32}$/.test(result.subscriptionId)
  )
    throw new IpcError("invalid_response");
  subscriptionId = result.subscriptionId;
  signal.addEventListener("abort", abort, { once: true });
  if (signal.aborted || !sameScope(request.scope, current())) {
    await stop();
    throw new IpcError("stale_session");
  }
  try {
    await call(ackCommand, {
      scope: request.scope,
      subscriptionId,
      sequence: 0,
    });
  } catch (error) {
    await stop().catch(() => {});
    throw error;
  }
  return { stop };
}

export function followContainerLogs(
  request: import("./generated.ts").FollowLogsRequest,
  current: () => SessionScope | null,
  consume: (batch: import("./generated.ts").LogBatch) => void | Promise<void>,
  failed: (error: IpcError) => void,
  signal: AbortSignal,
): Promise<{ stop: () => Promise<void> }> {
  return followAcknowledged(
    "follow_container_logs",
    "ack_container_logs",
    request,
    (value) => {
      if (
        value.containerId !== request.containerId ||
        !Array.isArray(value.records) ||
        value.records.length > 128
      )
        throw new IpcError("invalid_response");
      validateLogRecords(value.records, 262174);
    },
    current,
    consume,
    failed,
    signal,
  );
}
export function followDockerEvents(
  request: import("./generated.ts").FollowEventsRequest,
  current: () => SessionScope | null,
  consume: (batch: import("./generated.ts").EventBatch) => void | Promise<void>,
  failed: (error: IpcError) => void,
  signal: AbortSignal,
): Promise<{ stop: () => Promise<void> }> {
  return followAcknowledged(
    "follow_docker_events",
    "ack_docker_events",
    request,
    (value) => {
      if (!Array.isArray(value.events) || value.events.length > 64)
        throw new IpcError("invalid_response");
      for (const event of value.events) {
        if (
          !record(event) ||
          typeof event.actorId !== "string" ||
          !/^[a-f0-9]{64}$/.test(event.actorId) ||
          typeof event.timestampUnixNanos !== "string" ||
          !/^(0|[1-9]\d{0,19})$/.test(event.timestampUnixNanos) ||
          BigInt(event.timestampUnixNanos) > 18446744073709551615n ||
          ![
            "create",
            "start",
            "stop",
            "die",
            "destroy",
            "restart",
            "pause",
            "unpause",
            "rename",
            "health_status",
          ].includes(String(event.action))
        )
          throw new IpcError("invalid_response");
      }
    },
    current,
    consume,
    failed,
    signal,
  );
}

export async function exportContainerLogs(
  request: import("./generated.ts").ExportLogsRequest,
  current: () => SessionScope | null,
): Promise<import("./generated.ts").ExportLogsResponse> {
  const result = await call("export_container_logs", request, () =>
    sameScope(request.scope, current()),
  );
  if (
    !record(result) ||
    typeof result.saved !== "boolean" ||
    result.lineCount !== (result.saved ? request.lines.length : 0)
  )
    throw new IpcError("invalid_response");
  return result as import("./generated.ts").ExportLogsResponse;
}

export async function containerStats(
  request: import("./generated.ts").ContainerStatsRequest,
  current: () => SessionScope | null,
): Promise<import("./generated.ts").StatsSample> {
  const result = await scheduledRead("container_stats", request, current);
  if (
    !record(result) ||
    !scope(result.scope) ||
    !sameScope(request.scope, result.scope) ||
    result.containerId !== request.containerId ||
    typeof result.capturedAtMs !== "number" ||
    !Number.isSafeInteger(result.capturedAtMs) ||
    result.capturedAtMs <= 0 ||
    !["available", "stopped", "missing", "unavailable"].includes(
      String(result.availability),
    ) ||
    !record(result.values) ||
    !record(result.raw)
  )
    throw new IpcError("invalid_response");
  for (const field of [
    "cpuPercent",
    "memoryUsageBytes",
    "memoryLimitBytes",
    "memoryPercent",
    "networkRxBytes",
    "networkTxBytes",
    "blockReadBytes",
    "blockWriteBytes",
    "pids",
  ]) {
    const value = result.values[field];
    const max =
      field === "pids"
        ? 0xffffffff
        : field.endsWith("Percent")
          ? 1_000_000
          : Number.MAX_SAFE_INTEGER;
    if (
      value !== null &&
      (typeof value !== "number" ||
        !Number.isFinite(value) ||
        value < 0 ||
        value > max ||
        (field === "pids" && !Number.isInteger(value)))
    )
      throw new IpcError("invalid_response");
    if (result.availability !== "available" && value !== null)
      throw new IpcError("invalid_response");
  }
  for (const field of [
    "cpu",
    "memory",
    "memoryPercent",
    "network",
    "block",
    "pids",
  ]) {
    const raw = result.raw[field];
    if (
      raw !== null &&
      (!text(raw, 128) ||
        [...raw].some((char) => {
          const code = char.codePointAt(0) ?? 0;
          return code < 32 || (code >= 127 && code <= 159);
        }))
    )
      throw new IpcError("invalid_response");
  }
  return result as import("./generated.ts").StatsSample;
}

export async function listCompose(
  expected: SessionScope,
  current: () => SessionScope | null,
): Promise<import("./generated.ts").ListComposeResponse> {
  const value = await scheduledRead(
    "list_compose",
    { scope: expected },
    current,
  );
  if (
    !record(value) ||
    !scope(value.scope) ||
    !sameScope(value.scope, expected) ||
    !["available", "absent", "unknown"].includes(String(value.plugin)) ||
    !(
      value.listingError === null ||
      (typeof value.listingError === "string" &&
        Object.hasOwn(messages, value.listingError))
    ) ||
    !Array.isArray(value.projects) ||
    value.projects.length > 1000
  )
    throw new IpcError("invalid_response");
  const names = new Set<string>();
  const ids = new Set<string>();
  const reports = (v: unknown) =>
    Array.isArray(v) && v.length <= 128 && v.every((x) => text(x));
  for (const p of value.projects) {
    if (
      !record(p) ||
      !text(p.name) ||
      !p.name ||
      names.has(p.name) ||
      !(p.status === null || text(p.status)) ||
      typeof p.fromPlugin !== "boolean" ||
      typeof p.fromLabels !== "boolean" ||
      (!p.fromPlugin && !p.fromLabels) ||
      p.configuration !== "unverified" ||
      !reports(p.configFilesReported) ||
      !reports(p.workingDirectoriesReported) ||
      !Array.isArray(p.instances) ||
      p.instances.length > 5000
    )
      throw new IpcError("invalid_response");
    names.add(p.name);
    for (const item of p.instances) {
      if (
        !record(item) ||
        typeof item.containerId !== "string" ||
        !/^[a-f0-9]{64}$/.test(item.containerId) ||
        ids.has(item.containerId) ||
        !text(item.name) ||
        !text(item.state) ||
        !(item.service === null || text(item.service))
      )
        throw new IpcError("invalid_response");
      ids.add(item.containerId);
      if (ids.size > 5000) throw new IpcError("invalid_response");
    }
  }
  return value as import("./generated.ts").ListComposeResponse;
}

import type {
  ActivityRecord,
  ConfirmationIntent,
  ManagementState,
  MutationSpec,
  MutationResponse,
  MutationTargetResult,
} from "./generated.ts";
function sameMutation(value: unknown, expected: MutationSpec): boolean {
  return (
    record(value) &&
    value.operation === expected.operation &&
    value.timeoutSeconds === expected.timeoutSeconds &&
    Array.isArray(value.containerIds) &&
    value.containerIds.length === expected.containerIds.length &&
    value.containerIds.every((id, i) => id === expected.containerIds[i])
  );
}
export async function setManagement(
  selected: SessionScope,
  enabled: boolean,
  current: () => SessionScope | null,
): Promise<ManagementState> {
  const value = await call("set_management", { scope: selected, enabled }, () =>
    sameScope(selected, current()),
  );
  if (
    !record(value) ||
    !scope(value.scope) ||
    !sameScope(value.scope, selected) ||
    value.enabled !== enabled
  )
    throw new IpcError("invalid_response");
  return value as unknown as ManagementState;
}
export async function prepareMutation(
  selected: SessionScope,
  spec: MutationSpec,
  current: () => SessionScope | null,
): Promise<ConfirmationIntent> {
  const value = await call(
    "prepare_confirmation",
    { scope: selected, operation: { category: "mutation", spec } },
    () => sameScope(selected, current()),
  );
  if (
    !record(value) ||
    !scope(value.scope) ||
    !sameScope(value.scope, selected) ||
    typeof value.id !== "string" ||
    !/^i_[a-f0-9]{32}$/.test(value.id) ||
    !record(value.operation) ||
    value.operation.category !== "mutation" ||
    !sameMutation(value.operation.spec, spec) ||
    !Number.isInteger(value.expiresInMs) ||
    (value.expiresInMs as number) < 1 ||
    (value.expiresInMs as number) > 30000
  )
    throw new IpcError("invalid_response");
  return value as unknown as ConfirmationIntent;
}
function targetResults(
  value: unknown,
  ids: string[],
): value is MutationTargetResult[] {
  return (
    Array.isArray(value) &&
    value.length === ids.length &&
    value.length <= 20 &&
    value.every(
      (r, i) =>
        record(r) &&
        r.containerId === ids[i] &&
        typeof r.dispatched === "boolean" &&
        [
          "not_dispatched",
          "succeeded",
          "failed",
          "unknown",
          "cancelled",
        ].includes(r.outcome as string) &&
        (r.error === null ||
          (typeof r.error === "string" && Object.hasOwn(messages, r.error))) &&
        (["not_dispatched", "cancelled"].includes(r.outcome as string)
          ? !r.dispatched
          : ["succeeded", "unknown"].includes(r.outcome as string)
            ? r.dispatched
            : true),
    )
  );
}
function aggregateTargets(results: MutationTargetResult[]): string {
  if (results.some((r) => r.outcome === "unknown")) return "unknown";
  for (const outcome of ["succeeded", "failed", "cancelled", "not_dispatched"])
    if (results.every((r) => r.outcome === outcome)) return outcome;
  return "partial";
}
export async function mutateContainer(
  selected: SessionScope,
  spec: MutationSpec,
  intentId: string,
  current: () => SessionScope | null,
): Promise<MutationResponse> {
  // Mutations never enter scheduledRead: exactly one invocation, including on transport failure.
  const value = await mutationCall(
    "mutate_container",
    { scope: selected, spec, intentId },
    current,
  );
  if (
    !record(value) ||
    !scope(value.scope) ||
    !sameScope(value.scope, selected) ||
    !sameMutation(value.spec, spec) ||
    !["succeeded", "failed", "unknown", "partial", "cancelled"].includes(
      value.outcome as string,
    ) ||
    !targetResults(value.results, spec.containerIds) ||
    (aggregateTargets(value.results) === "not_dispatched"
      ? "cancelled"
      : aggregateTargets(value.results)) !== value.outcome
  )
    throw new IpcError("invalid_response");
  return value as unknown as MutationResponse;
}
export async function getActivity(): Promise<ActivityRecord[]> {
  const value = await call("get_activity");
  if (
    !Array.isArray(value) ||
    value.length > 200 ||
    !value.every(
      (row) =>
        record(row) &&
        typeof row.id === "string" &&
        /^i_[a-f0-9]{32}$/.test(row.id) &&
        typeof row.hostId === "string" &&
        /^h_[a-f0-9]{32}$/.test(row.hostId) &&
        ["start", "stop", "restart", "remove"].includes(row.action as string) &&
        Array.isArray(row.targets) &&
        row.targets.length > 0 &&
        row.targets.length <= 20 &&
        row.targets.every(
          (id: unknown) => typeof id === "string" && /^[a-f0-9]{64}$/.test(id),
        ) &&
        new Set(row.targets).size === row.targets.length &&
        Number.isSafeInteger(row.startedAtMs) &&
        (row.startedAtMs as number) >= 0 &&
        Number.isSafeInteger(row.updatedAtMs) &&
        (row.updatedAtMs as number) >= (row.startedAtMs as number) &&
        (row.updatedAtMs as number) <= 253402300799999 &&
        [
          "not_dispatched",
          "unknown",
          "succeeded",
          "failed",
          "partial",
          "cancelled",
        ].includes(row.outcome as string) &&
        targetResults(row.results, row.targets as string[]) &&
        aggregateTargets(row.results) === row.outcome,
    )
  )
    throw new IpcError("invalid_response");
  return value as ActivityRecord[];
}

export async function getManagement(
  selected: SessionScope,
  current: () => SessionScope | null,
): Promise<ManagementState> {
  const value = await call("get_management", { scope: selected }, () =>
    sameScope(selected, current()),
  );
  if (
    !record(value) ||
    !scope(value.scope) ||
    !sameScope(value.scope, selected) ||
    typeof value.enabled !== "boolean"
  )
    throw new IpcError("invalid_response");
  return value as unknown as ManagementState;
}

export async function cancelMutation(
  selected: SessionScope,
  intentId: string,
): Promise<import("./generated.ts").CancelMutationResponse> {
  const value = await call("cancel_mutation", { scope: selected, intentId });
  if (!record(value) || typeof value.pendingCancellationRequested !== "boolean")
    throw new IpcError("invalid_response");
  return value as unknown as import("./generated.ts").CancelMutationResponse;
}

const fullImageId = (v: unknown): v is string =>
  typeof v === "string" && /^sha256:[a-f0-9]{64}$/.test(v);
const imageStrings = (v: unknown): v is string[] =>
  Array.isArray(v) &&
  v.length <= 128 &&
  v.every((x) => text(x)) &&
  new Set(v).size === v.length;
export async function listImages(
  request: import("./generated.ts").ListImagesRequest,
  current: () => SessionScope | null,
): Promise<import("./generated.ts").ListImagesResponse> {
  const value = await scheduledRead("list_images", request, current);
  if (
    !record(value) ||
    !scope(value.scope) ||
    !sameScope(value.scope, request.scope) ||
    value.danglingOnly !== request.danglingOnly ||
    !Array.isArray(value.images) ||
    value.images.length > 5000
  )
    throw new IpcError("invalid_response");
  const ids = new Set<string>();
  for (const row of value.images) {
    if (
      !record(row) ||
      !scope(row.scope) ||
      !sameScope(row.scope, request.scope) ||
      !fullImageId(row.id) ||
      ids.has(row.id) ||
      !imageStrings(row.tags) ||
      !imageStrings(row.digests) ||
      !(row.sizeReported === null || text(row.sizeReported)) ||
      !(row.createdAtReported === null || text(row.createdAtReported))
    )
      throw new IpcError("invalid_response");
    ids.add(row.id);
  }
  return value as unknown as import("./generated.ts").ListImagesResponse;
}
export async function inspectImage(
  request: import("./generated.ts").InspectImageRequest,
  current: () => SessionScope | null,
): Promise<import("./generated.ts").ImageDetail> {
  const value = await scheduledRead("inspect_image", request, current);
  if (
    !record(value) ||
    !scope(value.scope) ||
    !sameScope(value.scope, request.scope) ||
    !fullImageId(value.id) ||
    value.id !== request.imageId ||
    !imageStrings(value.tags) ||
    !imageStrings(value.digests) ||
    !(
      value.sizeBytes === null ||
      (Number.isSafeInteger(value.sizeBytes) && Number(value.sizeBytes) >= 0)
    ) ||
    ![value.createdAt, value.os, value.architecture, value.variant].every(
      (v) => v === null || text(v),
    ) ||
    !Array.isArray(value.labels) ||
    value.labels.length > 256 ||
    !value.labels.every(
      (v) => record(v) && text(v.name) && v.value === null && v.masked === true,
    ) ||
    !Array.isArray(value.containers) ||
    value.containers.length > 5000
  )
    throw new IpcError("invalid_response");
  const ids = new Set<string>();
  for (const row of value.containers) {
    if (
      !record(row) ||
      typeof row.containerId !== "string" ||
      !/^[a-f0-9]{64}$/.test(row.containerId) ||
      ids.has(row.containerId) ||
      !text(row.name) ||
      !text(row.state)
    )
      throw new IpcError("invalid_response");
    ids.add(row.containerId);
  }
  return value as unknown as import("./generated.ts").ImageDetail;
}

const volumeName = (value: unknown): value is string =>
  typeof value === "string" && /^[a-zA-Z0-9][a-zA-Z0-9_.-]{0,254}$/.test(value);
function volumeSummary(
  value: unknown,
  expected: SessionScope,
): value is import("./generated.ts").VolumeSummary {
  return (
    record(value) &&
    scope(value.scope) &&
    sameScope(value.scope, expected) &&
    volumeName(value.name) &&
    (value.driver === null || text(value.driver)) &&
    (value.volumeScope === null || text(value.volumeScope))
  );
}
export async function listVolumes(
  expected: SessionScope,
  current: () => SessionScope | null,
): Promise<import("./generated.ts").ListVolumesResponse> {
  const value = await scheduledRead(
    "list_volumes",
    { scope: expected },
    current,
  );
  if (
    !record(value) ||
    !scope(value.scope) ||
    !sameScope(value.scope, expected) ||
    !Array.isArray(value.volumes) ||
    value.volumes.length > 5000 ||
    !value.volumes.every((row) => volumeSummary(row, expected)) ||
    new Set(value.volumes.map((row) => row.name)).size !== value.volumes.length
  )
    throw new IpcError("invalid_response");
  return value as unknown as import("./generated.ts").ListVolumesResponse;
}
export async function inspectVolume(
  request: import("./generated.ts").InspectVolumeRequest,
  current: () => SessionScope | null,
): Promise<import("./generated.ts").VolumeDetail> {
  const value = await scheduledRead("inspect_volume", request, current);
  const masked = (rows: unknown) =>
    Array.isArray(rows) &&
    rows.length <= 256 &&
    rows.every(
      (row) =>
        record(row) &&
        text(row.name) &&
        row.value === null &&
        row.masked === true,
    ) &&
    new Set(rows.map((row) => row.name)).size === rows.length;
  if (
    !record(value) ||
    !volumeSummary(value.summary, request.scope) ||
    value.summary.name !== request.name ||
    ![value.createdAt, value.mountpointReported].every(
      (v) => v === null || text(v),
    ) ||
    !masked(value.labels) ||
    !masked(value.options) ||
    !Array.isArray(value.references) ||
    value.references.length > 5000 ||
    !["referenced", "unreferenced", "incomplete"].includes(
      String(value.referenceObservation),
    ) ||
    !Array.isArray(value.unresolvedContainerIds) ||
    value.unresolvedContainerIds.length > 5000 ||
    !value.unresolvedContainerIds.every(
      (id) => typeof id === "string" && /^[a-f0-9]{64}$/.test(id),
    ) ||
    new Set(value.unresolvedContainerIds).size !==
      value.unresolvedContainerIds.length
  )
    throw new IpcError("invalid_response");
  const keys = new Set<string>();
  for (const row of value.references) {
    if (
      !record(row) ||
      typeof row.containerId !== "string" ||
      !/^[a-f0-9]{64}$/.test(row.containerId) ||
      !text(row.name) ||
      !text(row.state) ||
      !(row.destination === null || text(row.destination)) ||
      !(row.readOnly === null || typeof row.readOnly === "boolean") ||
      value.unresolvedContainerIds.includes(row.containerId)
    )
      throw new IpcError("invalid_response");
    const key = JSON.stringify([row.containerId, row.destination]);
    if (keys.has(key)) throw new IpcError("invalid_response");
    keys.add(key);
  }
  if (
    (value.referenceObservation === "referenced" &&
      (!value.references.length || value.unresolvedContainerIds.length)) ||
    (value.referenceObservation === "unreferenced" &&
      (value.references.length || value.unresolvedContainerIds.length))
  )
    throw new IpcError("invalid_response");
  return value as unknown as import("./generated.ts").VolumeDetail;
}

const networkId = (value: unknown): value is string =>
  typeof value === "string" && /^(?:[a-f0-9]{64}|[a-z0-9]{25})$/.test(value);
const optionalNetworkText = (value: unknown) => value === null || text(value);
const optionalNetworkFlag = (value: unknown) =>
  value === null || typeof value === "boolean";
function networkSummary(
  value: unknown,
  expected: SessionScope,
): value is import("./generated.ts").NetworkSummary {
  return (
    record(value) &&
    scope(value.scope) &&
    sameScope(value.scope, expected) &&
    networkId(value.id) &&
    text(value.name) &&
    optionalNetworkText(value.driver) &&
    optionalNetworkText(value.networkScope) &&
    optionalNetworkFlag(value.internal) &&
    optionalNetworkFlag(value.ipv6)
  );
}
export async function listNetworks(
  expected: SessionScope,
  current: () => SessionScope | null,
): Promise<import("./generated.ts").ListNetworksResponse> {
  const value = await scheduledRead(
    "list_networks",
    { scope: expected },
    current,
  );
  if (
    !record(value) ||
    !scope(value.scope) ||
    !sameScope(value.scope, expected) ||
    !Array.isArray(value.networks) ||
    value.networks.length > 5000 ||
    !value.networks.every((row) => networkSummary(row, expected)) ||
    new Set(value.networks.map((row) => row.id)).size !== value.networks.length
  )
    throw new IpcError("invalid_response");
  return value as unknown as import("./generated.ts").ListNetworksResponse;
}
export async function inspectNetwork(
  request: import("./generated.ts").InspectNetworkRequest,
  current: () => SessionScope | null,
): Promise<import("./generated.ts").NetworkDetail> {
  const value = await scheduledRead("inspect_network", request, current);
  const masked = (rows: unknown) =>
    Array.isArray(rows) &&
    rows.length <= 256 &&
    rows.every(
      (row) =>
        record(row) &&
        text(row.name) &&
        row.value === null &&
        row.masked === true,
    ) &&
    new Set(rows.map((row) => row.name)).size === rows.length;
  if (
    !record(value) ||
    !networkSummary(value.summary, request.scope) ||
    value.summary.id !== request.networkId ||
    !optionalNetworkText(value.createdAt) ||
    !optionalNetworkText(value.ipamDriver) ||
    !masked(value.labels) ||
    !masked(value.options) ||
    !masked(value.ipamOptions) ||
    typeof value.metadataIncomplete !== "boolean" ||
    typeof value.attachmentsReported !== "boolean" ||
    !Array.isArray(value.ipamConfig) ||
    value.ipamConfig.length > 128 ||
    !Array.isArray(value.attachments) ||
    value.attachments.length > 5000 ||
    (!value.attachmentsReported && value.attachments.length > 0)
  )
    throw new IpcError("invalid_response");
  for (const entry of value.ipamConfig) {
    if (
      !record(entry) ||
      ![entry.subnet, entry.ipRange, entry.gateway].every(
        optionalNetworkText,
      ) ||
      !Array.isArray(entry.auxiliaryAddresses) ||
      entry.auxiliaryAddresses.length > 256 ||
      !entry.auxiliaryAddresses.every(
        (row) =>
          record(row) && text(row.name) && optionalNetworkText(row.address),
      ) ||
      new Set(entry.auxiliaryAddresses.map((row) => row.name)).size !==
        entry.auxiliaryAddresses.length
    )
      throw new IpcError("invalid_response");
  }
  const keys = new Set<string>();
  for (const row of value.attachments) {
    if (
      !record(row) ||
      !text(row.endpointKey) ||
      keys.has(row.endpointKey) ||
      !(
        row.containerId === null ||
        (typeof row.containerId === "string" &&
          /^[a-f0-9]{64}$/.test(row.containerId) &&
          row.containerId === row.endpointKey)
      ) ||
      ![row.name, row.endpointId, row.ipv4Address, row.ipv6Address].every(
        optionalNetworkText,
      )
    )
      throw new IpcError("invalid_response");
    keys.add(row.endpointKey);
  }
  return value as unknown as import("./generated.ts").NetworkDetail;
}

function sameComposeConfiguration(
  value: unknown,
  expected: import("./generated.ts").ComposeConfiguration,
): boolean {
  return (
    record(value) &&
    value.projectName === expected.projectName &&
    value.workingDirectory === expected.workingDirectory &&
    Array.isArray(value.configFiles) &&
    value.configFiles.length === expected.configFiles.length &&
    value.configFiles.every((file, i) => file === expected.configFiles[i])
  );
}
function sameComposeSpec(
  value: unknown,
  expected: import("./generated.ts").ComposeActionSpec,
): boolean {
  return (
    record(value) &&
    value.verificationId === expected.verificationId &&
    sameComposeConfiguration(value.configuration, expected.configuration) &&
    value.operation === expected.operation &&
    value.timeoutSeconds === expected.timeoutSeconds &&
    Array.isArray(value.services) &&
    value.services.length === expected.services.length &&
    value.services.every((service, i) => service === expected.services[i]) &&
    Array.isArray(value.containerIds) &&
    value.containerIds.length === expected.containerIds.length &&
    value.containerIds.every((id, i) => id === expected.containerIds[i])
  );
}
export async function verifyComposeProject(
  request: import("./generated.ts").VerifyComposeRequest,
  current: () => SessionScope | null,
): Promise<import("./generated.ts").ComposeVerification> {
  // Explicit verification acknowledges trusted remote configuration; no automatic retries.
  const value = await call("verify_compose_project", request, () =>
    sameScope(request.scope, current()),
  );
  if (
    !record(value) ||
    !scope(value.scope) ||
    !sameScope(value.scope, request.scope) ||
    typeof value.id !== "string" ||
    !/^v_[a-f0-9]{32}$/.test(value.id) ||
    !sameComposeConfiguration(value.configuration, request.configuration) ||
    !Array.isArray(value.services) ||
    !value.services.length ||
    value.services.length > 20 ||
    !value.services.every(
      (v) =>
        typeof v === "string" && /^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$/.test(v),
    ) ||
    new Set(value.services).size !== value.services.length ||
    !Array.isArray(value.containerIds) ||
    !value.containerIds.length ||
    value.containerIds.length > 20 ||
    !value.containerIds.every(
      (id) => typeof id === "string" && /^[a-f0-9]{64}$/.test(id),
    ) ||
    new Set(value.containerIds).size !== value.containerIds.length ||
    !Number.isInteger(value.expiresInMs) ||
    Number(value.expiresInMs) < 1 ||
    Number(value.expiresInMs) > 300000
  )
    throw new IpcError("invalid_response");
  return value as unknown as import("./generated.ts").ComposeVerification;
}
export async function prepareComposeAction(
  selected: SessionScope,
  spec: import("./generated.ts").ComposeActionSpec,
  current: () => SessionScope | null,
): Promise<ConfirmationIntent> {
  const value = await call(
    "prepare_confirmation",
    { scope: selected, operation: { category: "compose", spec } },
    () => sameScope(selected, current()),
  );
  if (
    !record(value) ||
    !scope(value.scope) ||
    !sameScope(value.scope, selected) ||
    typeof value.id !== "string" ||
    !/^i_[a-f0-9]{32}$/.test(value.id) ||
    !record(value.operation) ||
    value.operation.category !== "compose" ||
    !sameComposeSpec(value.operation.spec, spec) ||
    !Number.isInteger(value.expiresInMs) ||
    Number(value.expiresInMs) < 1 ||
    Number(value.expiresInMs) > 30000
  )
    throw new IpcError("invalid_response");
  return value as unknown as ConfirmationIntent;
}
export async function mutateComposeProject(
  request: import("./generated.ts").ComposeMutationRequest,
  current: () => SessionScope | null,
): Promise<import("./generated.ts").ComposeMutationResponse> {
  // A dispatched project command is never sent through the read retry scheduler.
  const value = await mutationCall("mutate_compose_project", request, current);
  if (
    !record(value) ||
    !scope(value.scope) ||
    !sameScope(value.scope, request.scope) ||
    !sameComposeSpec(value.spec, request.spec) ||
    !targetResults(value.results, request.spec.containerIds) ||
    (aggregateTargets(value.results) === "not_dispatched"
      ? "cancelled"
      : aggregateTargets(value.results)) !== value.outcome
  )
    throw new IpcError("invalid_response");
  return value as unknown as import("./generated.ts").ComposeMutationResponse;
}

import type {
  TerminalSpec,
  TerminalRequest,
  TerminalResponse,
  TerminalHandleRequest,
  TerminalInputRequest,
  TerminalResizeRequest,
  TerminalOutput,
} from "./generated.ts";
function sameTerminalSpec(value: unknown, expected: TerminalSpec) {
  return (
    record(value) &&
    value.containerId === expected.containerId &&
    value.shell === expected.shell &&
    value.columns === expected.columns &&
    value.rows === expected.rows
  );
}
export async function getTerminalPermission(
  selected: SessionScope,
  current: () => SessionScope | null,
): Promise<ManagementState> {
  const value = await call("get_terminal_permission", { scope: selected }, () =>
    sameScope(selected, current()),
  );
  if (
    !record(value) ||
    !scope(value.scope) ||
    !sameScope(value.scope, selected) ||
    typeof value.enabled !== "boolean"
  )
    throw new IpcError("invalid_response");
  return value as unknown as ManagementState;
}
export async function setTerminalPermission(
  selected: SessionScope,
  enabled: boolean,
  current: () => SessionScope | null,
): Promise<ManagementState> {
  const value = await call(
    "set_terminal_permission",
    { scope: selected, enabled },
    () => sameScope(selected, current()),
  );
  if (
    !record(value) ||
    !scope(value.scope) ||
    !sameScope(value.scope, selected) ||
    value.enabled !== enabled
  )
    throw new IpcError("invalid_response");
  return value as unknown as ManagementState;
}
export async function prepareTerminal(
  selected: SessionScope,
  spec: TerminalSpec,
  current: () => SessionScope | null,
): Promise<ConfirmationIntent> {
  const value = await call(
    "prepare_confirmation",
    { scope: selected, operation: { category: "terminal", spec } },
    () => sameScope(selected, current()),
  );
  if (
    !record(value) ||
    !scope(value.scope) ||
    !sameScope(value.scope, selected) ||
    typeof value.id !== "string" ||
    !/^i_[a-f0-9]{32}$/.test(value.id) ||
    !record(value.operation) ||
    value.operation.category !== "terminal" ||
    !sameTerminalSpec(value.operation.spec, spec) ||
    !Number.isInteger(value.expiresInMs) ||
    Number(value.expiresInMs) < 1 ||
    Number(value.expiresInMs) > 30000
  )
    throw new IpcError("invalid_response");
  return value as unknown as ConfirmationIntent;
}
export async function closeTerminal(
  request: TerminalHandleRequest,
): Promise<void> {
  const value = await call("close_terminal", request);
  if (value !== null) throw new IpcError("invalid_response");
}
export async function openTerminal(
  request: TerminalRequest,
  current: () => SessionScope | null,
): Promise<TerminalResponse> {
  if (!sameScope(request.scope, current())) throw new IpcError("stale_session");
  // Preserve a late handle long enough to close it after navigation. Never repeat the opening.
  const value = await call("open_container_terminal", request);
  if (
    !record(value) ||
    !scope(value.scope) ||
    !sameScope(value.scope, request.scope) ||
    typeof value.terminalId !== "string" ||
    !/^sub_[a-f0-9]{32}$/.test(value.terminalId)
  )
    throw new IpcError("invalid_response");
  const result = value as unknown as TerminalResponse;
  if (!sameScope(request.scope, current())) {
    await closeTerminal(result);
    throw new IpcError("stale_session");
  }
  return result;
}
export async function readTerminal(
  request: TerminalHandleRequest,
  current: () => SessionScope | null,
): Promise<TerminalOutput> {
  const value = await call("read_terminal", request, () =>
    sameScope(request.scope, current()),
  );
  if (
    !record(value) ||
    !scope(value.scope) ||
    !sameScope(value.scope, request.scope) ||
    value.terminalId !== request.terminalId ||
    !generation(value.sequence) ||
    !Array.isArray(value.bytes) ||
    value.bytes.length > 32768 ||
    !value.bytes.every((b) => Number.isInteger(b) && b >= 0 && b <= 255) ||
    !["starting", "running", "exited"].includes(String(value.state)) ||
    !(
      value.exitCode === null ||
      (Number.isInteger(value.exitCode) &&
        Number(value.exitCode) >= 0 &&
        Number(value.exitCode) <= 0xffffffff)
    ) ||
    !(
      value.error === null ||
      (typeof value.error === "string" && Object.hasOwn(messages, value.error))
    )
  )
    throw new IpcError("invalid_response");
  return value as unknown as TerminalOutput;
}
export async function writeTerminal(
  request: TerminalInputRequest,
  current: () => SessionScope | null,
): Promise<void> {
  if (
    !generation(request.sequence) ||
    !request.bytes.length ||
    request.bytes.length > 16384 ||
    !request.bytes.every((b) => Number.isInteger(b) && b >= 0 && b <= 255)
  )
    throw new IpcError("invalid_limits");
  const value = await call("write_terminal", request, () =>
    sameScope(request.scope, current()),
  );
  if (value !== null) throw new IpcError("invalid_response");
}
export async function resizeTerminal(
  request: TerminalResizeRequest,
  current: () => SessionScope | null,
): Promise<void> {
  if (
    !Number.isInteger(request.columns) ||
    request.columns < 20 ||
    request.columns > 500 ||
    !Number.isInteger(request.rows) ||
    request.rows < 5 ||
    request.rows > 300
  )
    throw new IpcError("invalid_limits");
  const value = await call("resize_terminal", request, () =>
    sameScope(request.scope, current()),
  );
  if (value !== null) throw new IpcError("invalid_response");
}
