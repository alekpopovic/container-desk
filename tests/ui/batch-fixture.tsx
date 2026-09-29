import { useRef, useState } from "react";
import { mockIPC } from "@tauri-apps/api/mocks";
import { ContainerInventory } from "../../src/features/containers/ContainerInventory";
import type {
  ActivityRecord,
  ContainerSummary,
  MutationRequest,
  MutationTargetResult,
  PrepareConfirmationRequest,
} from "../../src/lib/ipc/generated";
import fixtures from "../fixtures/ipc.json";
export function BatchFixture() {
  const [epoch, setEpoch] = useState(1);
  const [calls, setCalls] = useState(0);
  const [hold, setHold] = useState(false);
  const [removed, setRemoved] = useState<string[]>([]);
  const completion = useRef<(() => void) | null>(null);
  const cancelled = useRef(false);
  const records = useRef<ActivityRecord[]>([]);
  const scope = {
    ...fixtures.detail.summary.scope,
    selection: {
      ...fixtures.detail.summary.scope.selection,
      hostId: `h_${String(epoch).repeat(32)}`,
      selectionGeneration: epoch,
    },
    sessionGeneration: epoch,
  };
  const rows: ContainerSummary[] = Array.from(
    { length: 21 },
    (_, index) =>
      ({
        ...fixtures.detail.summary,
        scope,
        id: (index + 1).toString(16).padStart(64, "0"),
        name: `batch-${String(index + 1).padStart(2, "0")}`,
        state: index === 20 ? "running" : "exited",
      }) as ContainerSummary,
  ).filter((row) => !removed.includes(row.id));
  mockIPC((command, args) => {
    const request = (args as { request: unknown })?.request;
    if (command === "get_management") return { scope, enabled: false };
    if (command === "set_management") return request;
    if (command === "get_activity") return records.current;
    if (command === "list_containers") return { scope, containers: rows };
    if (command === "prepare_confirmation")
      return {
        ...(request as PrepareConfirmationRequest),
        id: `i_${"c".repeat(32)}`,
        expiresInMs: 30000,
      };
    if (command === "cancel_mutation") {
      cancelled.current = true;
      return { pendingCancellationRequested: true };
    }
    if (command === "mutate_container") {
      const received = request as MutationRequest;
      setCalls((n) => n + 1);
      cancelled.current = false;
      const finish = () => {
        const results: MutationTargetResult[] = received.spec.containerIds.map(
          (containerId, index) => ({
            containerId,
            outcome:
              index === 0
                ? "succeeded"
                : cancelled.current
                  ? "cancelled"
                  : "failed",
            dispatched: index === 0 || (!cancelled.current && index === 2),
            error:
              index === 0
                ? null
                : cancelled.current
                  ? "operation_cancelled"
                  : index === 1
                    ? "container_not_found"
                    : "permission_denied",
          }),
        );
        const outcome =
          results.length === 1 ? ("succeeded" as const) : ("partial" as const);
        records.current = [
          {
            id: received.intentId,
            hostId: scope.selection.hostId,
            action: received.spec.operation,
            targets: received.spec.containerIds,
            startedAtMs: 100,
            updatedAtMs: 101,
            outcome,
            results,
          },
        ];
        if (received.spec.operation === "remove")
          setRemoved(
            results
              .filter((r) => r.outcome === "succeeded")
              .map((r) => r.containerId),
          );
        return { scope: received.scope, spec: received.spec, outcome, results };
      };
      if (hold)
        return new Promise((resolve) => {
          completion.current = () => resolve(finish());
        });
      return finish();
    }
    throw Error(`Unexpected fixture command ${command}`);
  });
  return (
    <main style={{ padding: 24 }}>
      <button type="button" onClick={() => setEpoch((n) => n + 1)}>
        Fixture switch host
      </button>
      <button type="button" onClick={() => setHold(true)}>
        Fixture hold batch
      </button>
      <button type="button" onClick={() => completion.current?.()}>
        Fixture release batch
      </button>
      <fieldset aria-label="Batch calls">{calls}</fieldset>
      <ContainerInventory
        view={{
          scope,
          rows,
          selectedId: null,
          updatedAt: Date.now(),
          loading: false,
          stale: false,
          error: null,
        }}
        host={{
          name: `Batch host ${epoch}`,
          alias: "fixture-only",
          endpoint: "No SSH",
        }}
        refresh={() => {}}
        select={() => {}}
        inspectEnabled
      />
    </main>
  );
}
