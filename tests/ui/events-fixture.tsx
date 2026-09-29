import { useCallback, useEffect, useState, useSyncExternalStore } from "react";
import { mockIPC } from "@tauri-apps/api/mocks";
import type { Channel } from "@tauri-apps/api/core";
import { useContainerEvents } from "../../src/features/events/useContainerEvents";
import { ContainerCache } from "../../src/features/containers/cache";
import type {
  ContainerSummary,
  EventBatch,
  FollowEventsRequest,
} from "../../src/lib/ipc/generated";
import fixture from "../fixtures/ipc.json";
const scope = fixture.logs.scope;
const row: ContainerSummary = {
  scope,
  id: "a".repeat(64),
  name: "event-fixture",
  image: "fixture",
  state: "running",
  status: "Up",
  health: null,
  ports: [],
  compose: null,
  cli: {
    names: [],
    ports: "",
    createdAt: "",
    runningFor: "",
    labelsPresent: false,
  },
};
export function EventsFixture() {
  const [store] = useState(() => ({
    cache: new ContainerCache(),
    rows: [row],
    channel: null as Channel<unknown> | null,
    sequence: 0,
    starts: 0,
    cancels: 0,
    reads: 0,
    since: null as string | null,
  }));
  const [connected, connect] = useState(true);
  const [reads, setReads] = useState(0);
  const [starts, setStarts] = useState(0);
  const view = useSyncExternalStore(
    store.cache.subscribe,
    store.cache.getSnapshot,
  );
  function emit(ended = false, duplicate = false) {
    const events = Array.from({ length: 64 }, (_, n) => ({
      actorId: row.id,
      action: "destroy" as const,
      timestampUnixNanos: String(
        1760000000000000000n + BigInt(duplicate ? 0 : store.sequence * 64 + n),
      ),
    }));
    store.channel?.onmessage({
      scope,
      subscriptionId: `sub_${"b".repeat(32)}`,
      sequence: ++store.sequence,
      events,
      droppedRecords: 0,
      gap: false,
      ended,
      error: ended ? "transport_unavailable" : null,
    } satisfies EventBatch);
  }
  mockIPC((command, args) => {
    if (command === "follow_docker_events") {
      const a = args as {
        request: FollowEventsRequest;
        onBatch: Channel<unknown>;
      };
      store.channel = a.onBatch;
      store.sequence = 0;
      store.since = a.request.since;
      store.starts++;
      setStarts(store.starts);
      return { scope, subscriptionId: `sub_${"b".repeat(32)}` };
    }
    if (command === "cancel_subscription") store.cancels++;
    if (command === "ack_docker_events" || command === "cancel_subscription")
      return null;
    throw Error(`Unexpected event fixture command ${command}`);
  });
  const refresh = useCallback(async () => {
    const ticket = store.cache.begin();
    if (!ticket) return false;
    store.reads++;
    setReads(store.reads);
    store.cache.complete(ticket, { scope, containers: store.rows });
    return true;
  }, [store]);
  useEffect(() => {
    store.cache.activate(scope);
    void refresh();
    store.cache.select(row.id);
    return () => store.cache.unavailable();
  }, [store, refresh]);
  const status = useContainerEvents(connected ? view.scope : null, refresh);
  return (
    <main>
      <p role="status">{status}</p>
      <output aria-label="Reads">{reads}</output>
      <output aria-label="Starts">{starts}</output>
      <output aria-label="Rows">{view.rows.length}</output>
      <output aria-label="Selected">{view.selectedId ?? "none"}</output>
      <output aria-label="Recovery since">{store.since ?? "none"}</output>
      <button type="button" onClick={() => emit()}>
        Fixture event burst
      </button>
      <button type="button" onClick={() => emit(false, true)}>
        Fixture duplicate
      </button>
      <button
        type="button"
        onClick={() => {
          store.rows = [];
        }}
      >
        Fixture delete silently
      </button>
      <button type="button" onClick={() => emit(true)}>
        Fixture event gap
      </button>
      <button type="button" onClick={() => connect(false)}>
        Fixture disconnect
      </button>
    </main>
  );
}
