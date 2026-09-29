import { useRef, useState } from "react";
import { mockIPC } from "@tauri-apps/api/mocks";
import { ContainerManagement } from "../../src/features/management/ContainerManagement";
import type {
  ActivityRecord,
  ContainerDetail,
  MutationRequest,
  PrepareConfirmationRequest,
} from "../../src/lib/ipc/generated";
import fixtures from "../fixtures/ipc.json";
export function ManagementFixture() {
  const [epoch, setEpoch] = useState(1);
  const [calls, setCalls] = useState(0);
  const [refreshes, setRefreshes] = useState(0);
  const [unknown, setUnknown] = useState(false);
  const [hold, setHold] = useState(false);
  const [short, setShort] = useState(false);
  const completion = useRef<(() => void) | null>(null);
  const state = useRef("running");
  const activity = useRef<ActivityRecord[]>([]);
  const detail = structuredClone(fixtures.detail) as ContainerDetail;
  detail.summary.scope = { ...detail.summary.scope, sessionGeneration: epoch };
  detail.summary.name = "<img src=x onerror=alert(1)> fixture";
  const scope = detail.summary.scope;
  mockIPC((command, args) => {
    const request = (args as { request: unknown })?.request;
    if (command === "get_activity") return activity.current;
    if (command === "get_management") return { scope, enabled: false };
    if (command === "set_management") return request;
    if (command === "prepare_confirmation")
      return {
        ...(request as PrepareConfirmationRequest),
        id: `i_${"c".repeat(32)}`,
        expiresInMs: short ? 50 : 30000,
      };
    if (command === "mutate_container") {
      const received = request as MutationRequest;
      setCalls((n) => n + 1);
      state.current = received.spec.operation === "stop" ? "exited" : "running";
      const result = {
        scope: received.scope,
        spec: received.spec,
        outcome: unknown ? "unknown" : "succeeded",
      };
      activity.current.push({
        id: received.intentId,
        hostId: received.scope.selection.hostId,
        action: received.spec.operation,
        targets: received.spec.containerIds,
        startedAtMs: 100,
        updatedAtMs: 101,
        outcome: unknown ? "unknown" : "succeeded",
      });
      if (hold)
        return new Promise((resolve) => {
          completion.current = () => resolve(result);
        });
      return result;
    }
    if (command === "inspect_container") {
      detail.summary.state = state.current;
      detail.summary.health = state.current === "running" ? "starting" : null;
      return detail;
    }
    throw Error(`Unexpected fixture command ${command}`);
  });
  return (
    <main style={{ padding: 24, maxWidth: 800 }}>
      <button type="button" onClick={() => setUnknown(true)}>
        Fixture unknown outcome
      </button>
      <button type="button" onClick={() => setHold(true)}>
        Fixture hold mutation
      </button>
      <button type="button" onClick={() => completion.current?.()}>
        Fixture release mutation
      </button>
      <button type="button" onClick={() => setShort(true)}>
        Fixture short expiry
      </button>
      <button type="button" onClick={() => setEpoch((n) => n + 1)}>
        Fixture reconnect
      </button>
      <fieldset aria-label="Mutation count">{calls}</fieldset>
      <fieldset aria-label="Refresh count">{refreshes}</fieldset>
      <ContainerManagement
        key={epoch}
        scope={scope}
        row={detail.summary}
        host="Disposable fixture host"
        stale={false}
        refresh={() => setRefreshes((n) => n + 1)}
      />
    </main>
  );
}
