import { useState } from "react";
import { mockIPC } from "@tauri-apps/api/mocks";
import type { Channel } from "@tauri-apps/api/core";
import type { FollowLogsRequest } from "../../src/lib/ipc/generated";
import { LiveLogs } from "../../src/features/logs/LiveLogs";
import fixture from "../fixtures/ipc.json";
let onBatch: Channel<unknown> | null = null;
let scope = fixture.logs.scope;
let sequence = 0;
let cancelled = 0;
mockIPC((command, payload) => {
  if (command === "follow_container_logs") {
    const args = payload as {
      request: FollowLogsRequest;
      onBatch: Channel<unknown>;
    };
    scope = args.request.scope;
    onBatch = args.onBatch;
    sequence = 0;
    return { scope, subscriptionId: `sub_${"a".repeat(32)}` };
  }
  if (command === "cancel_subscription") {
    cancelled++;
    return { scope, subscriptionId: `sub_${"a".repeat(32)}` };
  }
  return null;
});
export function LogsFixture() {
  const [mounted, mount] = useState(true);
  const [stops, setStops] = useState(0);
  return (
    <main>
      <button
        type="button"
        onClick={() => {
          sequence++;
          onBatch?.onmessage({
            ...fixture.logs,
            subscriptionId: `sub_${"a".repeat(32)}`,
            sequence,
            droppedRecords: 43210,
            gap: true,
            ended: false,
            error: null,
            records: [
              {
                text: "<img src=x onerror=alert(1)> synthetic Unicode čćž",
                timestamp: "2026-09-29T10:00:00.000000001Z",
                channel: "stderr_ambiguous",
                truncated: true,
                invalidUtf8: false,
              },
            ],
          });
        }}
      >
        Deliver fixture overflow
      </button>
      <button type="button" onClick={() => mount(false)}>
        Unmount fixture logs
      </button>
      <button type="button" onClick={() => setStops(cancelled)}>
        Read fixture cancellations
      </button>
      <p>{stops} fixture cancellations</p>
      {mounted && <LiveLogs scope={scope} id={fixture.logs.containerId} />}
    </main>
  );
}
