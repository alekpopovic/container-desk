import { useMemo, useRef, useState } from "react";
import { mockIPC } from "@tauri-apps/api/mocks";
import { ComposeManagement } from "../../src/features/compose/ComposeManagement";
import type {
  ComposeActionSpec,
  ComposeConfiguration,
  SessionScope,
} from "../../src/lib/ipc/generated";
import fixtures from "../fixtures/ipc.json";
export function ComposeActionsFixture() {
  const [epoch, setEpoch] = useState(1),
    [unknown, setUnknown] = useState(false),
    [calls, setCalls] = useState(0);
  const enabled = useRef(false);
  const scope = useMemo(
    () => ({
      ...fixtures.detail.summary.scope,
      selection: {
        hostId: `h_${String(epoch).repeat(32)}`,
        selectionGeneration: epoch,
      },
      sessionGeneration: epoch,
      daemonId: `compose-${epoch}`,
    }),
    [epoch],
  );
  mockIPC((command, args) => {
    const request = (
      args as {
        request: {
          scope: SessionScope;
          configuration: ComposeConfiguration;
          acknowledged: boolean;
          enabled: boolean;
          spec: ComposeActionSpec;
          operation: { category: string; spec: ComposeActionSpec };
        };
      }
    ).request;
    if (command === "get_management")
      return { scope, enabled: enabled.current };
    if (command === "set_management") {
      enabled.current = request.enabled;
      return { scope, enabled: enabled.current };
    }
    if (command === "verify_compose_project") {
      if (!request.acknowledged) throw { code: "permission_denied" };
      if (request.configuration.projectName !== "owned")
        throw { code: "compose_project_mismatch" };
      return {
        scope,
        id: `v_${"c".repeat(32)}`,
        configuration: request.configuration,
        services: ["web", "worker"],
        containerIds: [fixtures.detail.summary.id, "b".repeat(64)],
        expiresInMs: 300000,
      };
    }
    if (command === "prepare_confirmation") {
      if (!enabled.current) throw { code: "permission_denied" };
      return {
        scope,
        id: `i_${"d".repeat(32)}`,
        operation: request.operation,
        expiresInMs: 30000,
      };
    }
    if (command === "mutate_compose_project") {
      setCalls((n) => n + 1);
      return {
        scope,
        spec: request.spec,
        outcome: unknown ? "unknown" : "succeeded",
        results: request.spec.containerIds.map((containerId) => ({
          containerId,
          outcome: unknown ? "unknown" : "succeeded",
          dispatched: true,
          error: unknown ? "transport_unavailable" : null,
        })),
      };
    }
    if (command === "inspect_container")
      return {
        ...fixtures.detail,
        summary: {
          ...fixtures.detail.summary,
          scope,
          id: (request as unknown as { containerId: string }).containerId,
          state: "running",
        },
      };
    if (command === "cancel_mutation")
      return { pendingCancellationRequested: true };
    throw Error(`Unexpected Compose action fixture ${command}`);
  });
  return (
    <main style={{ padding: 24 }}>
      <button
        type="button"
        onClick={() => {
          enabled.current = false;
          setEpoch((n) => n + 1);
        }}
      >
        Fixture switch Compose host
      </button>
      <button type="button" onClick={() => setUnknown(true)}>
        Fixture unknown Compose outcome
      </button>
      <output aria-label="Compose dispatch count">{calls}</output>
      <ComposeManagement
        key={epoch}
        scope={scope}
        project="owned"
        host={`Fixture host ${epoch}`}
        stale={false}
        refresh={() => {}}
      />
    </main>
  );
}
