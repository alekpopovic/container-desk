// Explicit browser-only demo inventory. No disk, SSH or native fallback.
import { getWorkspaceMode } from "./browser.ts";
import {
  IpcError,
  decodeInventory,
  isConcreteAlias,
} from "../lib/ipc/client.ts";
import type {
  HostInventory,
  HostDraft,
  WorkspaceMode,
  SaveHostRequest,
  RemoveHostRequest,
  InventoryConnectRequest,
  InventoryDisconnectRequest,
} from "../lib/ipc/generated.ts";
const state: HostInventory = {
  mode: "demo",
  saved: {
    writable: true,
    notice: null,
    preferences: {
      schemaVersion: 3,
      revision: 0,
      theme: "system",
      hosts: [],
      selectedHostId: null,
      trustedConfigPath: null,
      sshExecutableOverride: null,
    },
  },
  connection: null,
};
let generation = 0;
async function requireDemo(mode: WorkspaceMode) {
  if (mode !== "demo" || (await getWorkspaceMode()).mode !== "demo")
    throw new IpcError("permission_denied");
}
export function hasActiveInventoryConnection() {
  return state.connection !== null && state.connection.state !== "disconnected";
}
export async function getHostInventory(mode: WorkspaceMode) {
  await requireDemo(mode);
  return decodeInventory(structuredClone(state), "demo");
}
function validate(draft: HostDraft) {
  if (
    !isConcreteAlias(draft.ssh.alias) ||
    !draft.displayName ||
    draft.displayName.length > 256 ||
    draft.group.length > 128 ||
    draft.labels.length > 32 ||
    draft.labels.some((v) => v.length > 128)
  )
    throw new IpcError("invalid_preferences");
}
export async function saveHost(request: SaveHostRequest) {
  await requireDemo(request.mode);
  if (request.expectedRevision !== state.saved.preferences.revision)
    throw new IpcError("storage_conflict");
  validate(request.draft);
  const hosts = state.saved.preferences.hosts;
  const old = hosts.find((h) => h.id === request.id);
  if (request.id && !old) throw new IpcError("host_not_found");
  if (
    hosts.some(
      (h) =>
        h.id !== request.id &&
        h.alias === request.draft.ssh.alias &&
        h.ssh?.configPath === request.draft.ssh.configPath,
    )
  )
    throw new IpcError("invalid_preferences");
  if (!old && hosts.length >= 1000) throw new IpcError("resource_limit");
  const id = request.id ?? `h_${crypto.randomUUID().replaceAll("-", "")}`;
  const host = {
    ...structuredClone(request.draft),
    id,
    alias: request.draft.ssh.alias,
    readOnly: true,
  };
  if (old) hosts[hosts.indexOf(old)] = host;
  else hosts.push(host);
  if (
    state.connection?.hostId === id &&
    old &&
    (JSON.stringify(old.ssh) !== JSON.stringify(host.ssh) ||
      JSON.stringify(old.docker) !== JSON.stringify(host.docker))
  )
    state.connection = null;
  state.saved.preferences.selectedHostId = id;
  state.saved.preferences.revision += 1;
  return getHostInventory(request.mode);
}
export async function removeHost(request: RemoveHostRequest) {
  await requireDemo(request.mode);
  if (request.expectedRevision !== state.saved.preferences.revision)
    throw new IpcError("storage_conflict");
  const hosts = state.saved.preferences.hosts;
  const index = hosts.findIndex((h) => h.id === request.hostId);
  if (index < 0) throw new IpcError("host_not_found");
  hosts.splice(index, 1);
  if (state.connection?.hostId === request.hostId) state.connection = null;
  if (state.saved.preferences.selectedHostId === request.hostId)
    state.saved.preferences.selectedHostId = null;
  state.saved.preferences.revision += 1;
  return getHostInventory(request.mode);
}
export async function connectInventoryHost(request: InventoryConnectRequest) {
  await requireDemo(request.mode);
  const host = state.saved.preferences.hosts.find(
    (h) => h.id === request.hostId,
  );
  if (!host?.ssh) throw new IpcError("host_not_found");
  generation += 1;
  if (generation > 0xffffffff) throw new IpcError("resource_limit");
  state.connection = {
    hostId: host.id,
    selection: host.ssh,
    dockerOptions: host.docker,
    token: {
      sessionId: `s_${crypto.randomUUID().replaceAll("-", "")}`,
      sessionGeneration: generation,
    },
    state: "ready",
    hasJump: host.alias.includes("jump"),
    transportMode: "unconnected",
    durations: [],
    diagnostic: null,
    effective: {
      selection: host.ssh,
      executablePath: "/demo/ssh",
      hostname: "192.0.2.10",
      user: "demo",
      port: 22,
      proxyJump: host.alias.includes("jump") ? "demo-bastion" : null,
      hasProxyCommand: false,
    },
    docker: {
      status: "ready",
      context: host.docker.context ?? "demo",
      endpoint: "unix:///demo/docker.sock",
      endpointKind: "unix",
      clientVersion: "demo",
      serverVersion: "demo",
      daemonId: "demo-inventory-daemon",
      os: "linux",
      rootless: false,
      compose: "available",
      composeVersion: "demo",
      sudo: host.docker.sudo,
    },
  };
  return getHostInventory(request.mode);
}
export async function disconnectInventoryHost(
  request: InventoryDisconnectRequest,
) {
  await requireDemo(request.mode);
  if (
    state.connection?.hostId !== request.hostId ||
    state.connection.token.sessionId !== request.token.sessionId ||
    state.connection.token.sessionGeneration !== request.token.sessionGeneration
  )
    throw new IpcError("stale_session");
  state.connection.state = "disconnected";
  state.connection.docker = null;
  state.connection.token.sessionGeneration = ++generation;
  return getHostInventory(request.mode);
}
