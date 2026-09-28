import { invoke } from "@tauri-apps/api/core";
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
    "Local settings cannot be saved. The original files were retained.",
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
): Promise<unknown> {
  let result: unknown;
  try {
    result = await invoke<unknown>(
      command,
      request === undefined ? {} : { request },
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
  const result = await call("list_containers", { scope: expected }, () =>
    sameScope(expected, current()),
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
