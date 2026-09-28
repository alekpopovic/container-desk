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
    "connecting",
    "connected",
    "reconnecting",
    "failed",
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
    p.schemaVersion === 2 &&
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
    value.host.connectionState === "connected" &&
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
