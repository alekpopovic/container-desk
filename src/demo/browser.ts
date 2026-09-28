// Explicit browser demo provider. Native failures never call this provider.
import fixture from "./workspace.generated.json" with { type: "json" };
import { IpcError, sameScope } from "../lib/ipc/client.ts";
import type {
  DemoScenario,
  ErrorCode,
  ListContainersResponse,
  SessionScope,
  SwitchWorkspaceRequest,
  WorkspaceModeSnapshot,
} from "../lib/ipc/generated.ts";

let current: WorkspaceModeSnapshot = {
  mode: "live",
  scenario: null,
  scope: null,
  host: null,
};
let generation = 0;
const errors: Partial<Record<DemoScenario, ErrorCode>> = {
  permission_failure: "permission_denied",
  invalid_json: "invalid_response",
  huge_record: "resource_limit",
  disconnect: "disconnected",
  timeout: "operation_timed_out",
};
export async function getWorkspaceMode(): Promise<WorkspaceModeSnapshot> {
  return structuredClone(current);
}
export async function switchWorkspace(
  request: SwitchWorkspaceRequest,
): Promise<WorkspaceModeSnapshot> {
  generation += 1;
  if (generation > 0xffffffff) throw new IpcError("resource_limit");
  if (request.mode === "live")
    current = { mode: "live", scenario: null, scope: null, host: null };
  else {
    const host = {
      ...fixture.workspace.host,
      readOnly: true,
      connectionState: "ready" as const,
    };
    const scope: SessionScope = {
      selection: { hostId: host.id, selectionGeneration: generation },
      sessionId: `s_${crypto.randomUUID().replaceAll("-", "")}`,
      sessionGeneration: generation,
      daemonId: "demo-fixture-daemon",
    };
    current = { mode: "demo", scenario: request.scenario, scope, host };
  }
  return getWorkspaceMode();
}
export async function listContainers(
  expected: SessionScope,
  active: () => SessionScope | null,
): Promise<ListContainersResponse> {
  if (current.mode !== "demo") throw new IpcError("feature_unavailable");
  if (!sameScope(expected, current.scope) || !sameScope(expected, active()))
    throw new IpcError("stale_session");
  const code = current.scenario ? errors[current.scenario] : undefined;
  if (code) throw new IpcError(code, expected);
  return {
    scope: expected,
    containers:
      current.scenario === "empty"
        ? []
        : fixture.inventory.containers.map((row) => ({
            ...structuredClone(row),
            scope: expected,
          })),
  };
}
