import { useState } from "react";
import { mockIPC } from "@tauri-apps/api/mocks";
import type { Channel } from "@tauri-apps/api/core";
import type {
  ExportLogsRequest,
  FollowLogsRequest,
  LogRecord,
} from "../../src/lib/ipc/generated";
import { LiveLogs } from "../../src/features/logs/LiveLogs";
import fixture from "../fixtures/ipc.json";
let onBatch: Channel<unknown> | null = null;
let scope = fixture.logs.scope;
let sequence = 0;
let cancelled = 0;
let exported: ExportLogsRequest | null = null;
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
  if (command === "export_container_logs") {
    exported = (payload as { request: ExportLogsRequest }).request;
    return { saved: true, lineCount: exported.lines.length };
  }
  return null;
});
function send(records: LogRecord[], droppedRecords = 0) {
  sequence++;
  onBatch?.onmessage({
    ...fixture.logs,
    subscriptionId: `sub_${"a".repeat(32)}`,
    sequence,
    droppedRecords,
    gap: droppedRecords > 0,
    ended: false,
    error: null,
    records,
  });
}
function row(text: string): LogRecord {
  return {
    text,
    timestamp: "2026-09-29T10:00:00.000000001Z",
    channel: "stdout",
    truncated: false,
    invalidUtf8: false,
  };
}
export function LogsFixture() {
  const [mounted, mount] = useState(true);
  const [stops, setStops] = useState(0);
  const [sent, setSent] = useState(0);
  const [exportText, setExportText] = useState("");
  return (
    <main>
      <button
        type="button"
        onClick={() =>
          send(
            [
              {
                ...row("<img src=x onerror=alert(1)> synthetic Unicode čćž"),
                channel: "stderr_ambiguous",
                truncated: true,
              },
            ],
            43210,
          )
        }
      >
        Deliver fixture overflow
      </button>
      <button
        type="button"
        onClick={async () => {
          for (let offset = 0; offset < 1024; offset += 128) {
            send(
              Array.from({ length: 128 }, (_, i) =>
                row(`fixture-line-${String(offset + i).padStart(4, "0")} čćž`),
              ),
            );
            await new Promise((resolve) => setTimeout(resolve, 10));
          }
          setSent((value) => value + 1024);
        }}
      >
        Deliver 1024 fixture lines
      </button>
      <button
        type="button"
        onClick={() =>
          send([
            row("first chosen"),
            row("unrelated other line"),
            row(
              "\x1b[31mthird chosen čćž\x1b[0m \x1b]8;;https://secret.invalid\x07link\x1b]8;;\x1b\\",
            ),
          ])
        }
      >
        Deliver export fixture
      </button>
      <button
        type="button"
        onClick={() =>
          setExportText(exported ? JSON.stringify(exported) : "No export")
        }
      >
        Read fixture export
      </button>
      <button type="button" onClick={() => mount(false)}>
        Unmount fixture logs
      </button>
      <button type="button" onClick={() => setStops(cancelled)}>
        Read fixture cancellations
      </button>
      <p>
        {stops} fixture cancellations · {sent} fixture lines sent
      </p>
      <output aria-label="Fixture export">{exportText}</output>
      {mounted && <LiveLogs scope={scope} id={fixture.logs.containerId} />}
    </main>
  );
}
