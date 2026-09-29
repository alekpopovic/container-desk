import { useEffect, useRef, useState } from "react";
import { followContainerLogs, IpcError } from "../../lib/ipc/client";
import type {
  ContainerId,
  LogRecord,
  SessionScope,
} from "../../lib/ipc/generated";
/** Minimal stream controls; extended viewer and explicit export follow in prompt 025. */
export function LiveLogs({
  scope,
  id,
}: {
  scope: SessionScope;
  id: ContainerId;
}) {
  const [running, setRunning] = useState(false);
  const [status, setStatus] = useState("Stopped");
  const [rows, setRows] = useState<LogRecord[]>([]);
  const [dropped, setDropped] = useState(0);
  const [gap, setGap] = useState(false);
  const alive = useRef<AbortController | null>(null);
  const serial = useRef(0);
  const resume = useRef<string | null>(null);
  useEffect(
    () => () => {
      serial.current += 1;
      alive.current?.abort();
    },
    [],
  );
  async function start() {
    const ticket = ++serial.current;
    alive.current?.abort();
    const controller = new AbortController();
    alive.current = controller;
    setRunning(true);
    setStatus("Starting…");
    setGap(rows.length > 0);
    try {
      await followContainerLogs(
        { scope, containerId: id, tail: 100, since: resume.current },
        () =>
          ticket === serial.current && !controller.signal.aborted
            ? scope
            : null,
        (batch) => {
          if (controller.signal.aborted || ticket !== serial.current) return;
          for (const row of batch.records) {
            if (
              row.timestamp &&
              /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{9}Z$/.test(
                row.timestamp,
              )
            ) {
              const seconds = Math.floor(Date.parse(row.timestamp) / 1000);
              if (Number.isFinite(seconds) && seconds >= 0)
                resume.current = `${seconds}.${row.timestamp.slice(20, 29)}`;
            }
          }
          setRows((old) =>
            [
              ...old,
              ...batch.records.map((row) => ({
                ...row,
                text: row.text.slice(0, 4000),
                truncated: row.truncated || row.text.length > 4000,
              })),
            ].slice(-100),
          );
          setDropped((count) => count + batch.droppedRecords);
          setGap((old) => old || batch.gap);
          setStatus(
            batch.error
              ? new IpcError(batch.error).message
              : batch.ended
                ? "Stream ended"
                : "Following",
          );
          if (batch.ended) setRunning(false);
        },
        (error) => {
          setStatus(error.message);
          setGap(true);
          setRunning(false);
        },
        controller.signal,
      );
    } catch (error) {
      if (controller.signal.aborted || ticket !== serial.current) return;
      setStatus(
        error instanceof IpcError ? error.message : "Log stream unavailable.",
      );
      setRunning(false);
    }
  }
  return (
    <section aria-label="Live container logs" className="live-logs">
      <h3>Live logs</h3>
      <button
        type="button"
        className="button"
        disabled={running}
        onClick={() => void start()}
      >
        Start logs
      </button>
      <button
        type="button"
        className="button"
        disabled={!running}
        onClick={() => {
          serial.current += 1;
          alive.current?.abort();
          setRunning(false);
          setStatus("Stopped");
          setGap(true);
        }}
      >
        Stop logs
      </button>
      <p role="status">{status}</p>
      <p>
        Preview of the latest 100 lines, up to 4,000 characters per line. Server
        retention: 20,000 lines or 8 MiB. Stderr may include transport
        diagnostics.
      </p>
      {(dropped > 0 || gap) && (
        <p role="status">
          Log gap · {dropped} records dropped by stream retention. Restart is
          best-effort; gaps and duplicates are possible.
        </p>
      )}
      <div className="log-preview">
        {rows.map((row, index) => (
          <pre
            // biome-ignore lint/suspicious/noArrayIndexKey: Read-only transient preview with no row state.
            key={index}
          >
            {row.timestamp ?? "No timestamp"}{" "}
            {row.channel === "stderr_ambiguous" ? "[stderr / diagnostic] " : ""}
            {row.text.slice(0, 4000)}
            {row.truncated || row.text.length > 4000 ? " [truncated]" : ""}
            {row.invalidUtf8 ? " [invalid UTF-8 replaced]" : ""}
          </pre>
        ))}
      </div>
    </section>
  );
}
