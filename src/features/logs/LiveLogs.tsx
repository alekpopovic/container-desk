import { trapDialogTab } from "../../components/ConfirmationDialog";
import { useRemSize } from "../../components/useRemSize";
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import {
  exportContainerLogs,
  followContainerLogs,
  IpcError,
} from "../../lib/ipc/client";
import type { ContainerId, SessionScope } from "../../lib/ipc/generated";
import {
  type BufferedLog,
  filterLogs,
  LogBuffer,
  MAX_SEARCH,
  selectedText,
  visibleLog,
} from "./buffer";
export function LiveLogs({
  scope,
  id,
  source = "Selected container",
}: {
  scope: SessionScope;
  id: ContainerId;
  source?: string;
}) {
  const rowHeight = useRemSize() * (24 / 14);
  const buffer = useRef(new LogBuffer());
  const [rows, setRows] = useState<BufferedLog[]>([]);
  const [running, setRunning] = useState(false);
  const [status, setStatus] = useState("Stopped");
  const [paused, setPaused] = useState(false);
  const pauseRef = useRef(false);
  const [selecting, setSelecting] = useState(false);
  const selectionRef = useRef(false);
  const [follow, setFollow] = useState(true);
  const [search, setSearch] = useState("");
  const [timestamps, setTimestamps] = useState(true);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [dropped, setDropped] = useState(0);
  const [retained, setRetained] = useState({ lines: 0, bytes: 0, dropped: 0 });
  const [gap, setGap] = useState(true);
  const [scroll, setScroll] = useState(0);
  const [copyStatus, setCopyStatus] = useState("");
  const [exportLines, setExportLines] = useState<string[] | null>(null);
  const [exportBusy, setExportBusy] = useState(false);
  const [exportStatus, setExportStatus] = useState("");
  const alive = useRef<AbortController | null>(null);
  const serial = useRef(0);
  const mounted = useRef(true);
  const resume = useRef<string | null>(null);
  const viewport = useRef<HTMLElement>(null);
  const dialog = useRef<HTMLDialogElement>(null);
  const filtered = useMemo(() => filterLogs(rows, search), [rows, search]);
  const startRow = Math.max(
    0,
    Math.min(
      Math.max(0, filtered.length - 1),
      Math.floor(scroll / rowHeight) - 6,
    ),
  );
  const endRow = Math.min(filtered.length, startRow + 32);
  const visible = filtered.slice(startRow, endRow);
  useEffect(() => {
    mounted.current = true;
    const changed = () => {
      const range = document.getSelection();
      const held =
        !!range &&
        !range.isCollapsed &&
        !!viewport.current?.contains(range.anchorNode);
      selectionRef.current = held;
      setSelecting(held);
      if (!held && !pauseRef.current) setRows(buffer.current.snapshot());
    };
    document.addEventListener("selectionchange", changed);
    return () => {
      mounted.current = false;
      serial.current++;
      alive.current?.abort();
      document.removeEventListener("selectionchange", changed);
    };
  }, []);
  // biome-ignore lint/correctness/useExhaustiveDependencies: A rem resize changes scrollHeight without changing buffered rows.
  useLayoutEffect(() => {
    if (
      filtered.length > 0 &&
      follow &&
      !paused &&
      !selecting &&
      viewport.current
    )
      viewport.current.scrollTop = viewport.current.scrollHeight;
  }, [follow, paused, selecting, filtered, rowHeight]);
  useEffect(() => {
    if (exportLines) {
      dialog.current?.showModal();
      dialog.current
        ?.querySelector<HTMLButtonElement>("[data-cancel]")
        ?.focus();
    } else dialog.current?.close();
  }, [exportLines]);
  function showBuffer() {
    const current = buffer.current.snapshot();
    setRetained({
      lines: current.length,
      bytes: buffer.current.bytes,
      dropped: buffer.current.dropped,
    });
    if (!pauseRef.current && !selectionRef.current) setRows(current);
    const ids = new Set(current.map((row) => row.key));
    setSelected((old) =>
      [...old].some((key) => !ids.has(key))
        ? new Set([...old].filter((key) => ids.has(key)))
        : old,
    );
  }
  async function start() {
    const ticket = ++serial.current;
    alive.current?.abort();
    const controller = new AbortController();
    alive.current = controller;
    setRunning(true);
    setStatus("Starting…");
    setGap(true); // Every explicit start is a new observation; intervening history can be missing.
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
          buffer.current.append(batch.records);
          showBuffer();
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
  function clear() {
    document.getSelection()?.removeAllRanges();
    buffer.current.clear();
    setRows([]);
    setSelected(new Set());
    setRetained({ lines: 0, bytes: 0, dropped: 0 });
    setDropped(0);
    setGap(false);
    setCopyStatus("");
    setExportLines(null);
    setScroll(0);
    if (viewport.current) viewport.current.scrollTop = 0;
  }
  function chosenText() {
    return selectedText(buffer.current.snapshot(), selected, timestamps);
  }
  async function save() {
    if (!exportLines || exportBusy) return;
    setExportBusy(true);
    setExportStatus("");
    try {
      const result = await exportContainerLogs(
        {
          scope,
          containerId: id,
          lines: exportLines,
          secretsAcknowledged: true,
        },
        () => (mounted.current ? scope : null),
      );
      if (mounted.current) {
        setExportStatus(
          result.saved
            ? `Saved ${result.lineCount} selected lines.`
            : "Export cancelled.",
        );
        setExportLines(null);
      }
    } catch (error) {
      if (mounted.current)
        setExportStatus(
          error instanceof IpcError ? error.message : "Export failed.",
        );
    } finally {
      if (mounted.current) setExportBusy(false);
    }
  }
  return (
    <section aria-label="Live container logs" className="live-logs">
      <h3>Live logs · {source}</h3>
      <p className="muted log-source">
        Container <code>{id}</code>
      </p>
      <div className="log-tools">
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
            serial.current++;
            alive.current?.abort();
            setRunning(false);
            setStatus("Stopped");
            setGap(true);
          }}
        >
          Stop logs
        </button>
        <button
          type="button"
          className="button"
          aria-pressed={paused}
          onClick={() => {
            const next = !paused;
            pauseRef.current = next;
            setPaused(next);
            if (!next && !selectionRef.current)
              setRows(buffer.current.snapshot());
          }}
        >
          {paused ? "Resume display" : "Pause display"}
        </button>
        <button type="button" className="button" onClick={clear}>
          Clear view
        </button>
        <label>
          <input
            type="checkbox"
            checked={follow}
            onChange={(event) => setFollow(event.target.checked)}
          />
          Follow new lines
        </label>
        <label>
          <input
            type="checkbox"
            checked={timestamps}
            onChange={(event) => setTimestamps(event.target.checked)}
          />
          Show timestamps
        </label>
        <label>
          Search logs
          <input
            type="search"
            maxLength={MAX_SEARCH}
            value={search}
            onChange={(event) => {
              setSearch(event.target.value.slice(0, MAX_SEARCH));
              setScroll(0);
              if (viewport.current) viewport.current.scrollTop = 0;
            }}
          />
        </label>
      </div>
      <p role="status">
        {status}
        {paused
          ? " · Display paused; remote stream and bounded buffering continue."
          : selecting
            ? " · Text selection holds the display; buffering continues."
            : ""}
      </p>
      <p>
        Retained: {retained.lines} lines · {retained.bytes} bytes. Limit: 20,000
        lines or 8 MiB; oldest lines are discarded. Pause holds this display;
        Stop ends the remote stream. Stderr may include transport diagnostics.
      </p>
      {(dropped > 0 || gap || retained.dropped > 0) && (
        <p role="status">
          Log gap · {dropped} records dropped by stream retention ·{" "}
          {retained.dropped} by view retention. Restart is best-effort; gaps and
          duplicates are possible.
        </p>
      )}
      <section
        // biome-ignore lint/a11y/noNoninteractiveTabindex: A scroll region must support keyboard scrolling.
        tabIndex={0}
        aria-label="Scrollable buffered logs"
        className="log-preview log-viewport"
        ref={viewport}
        onScroll={(event) => setScroll(event.currentTarget.scrollTop)}
      >
        <table className="log-table" aria-rowcount={filtered.length}>
          <caption className="sr-only">Buffered container logs</caption>
          <tbody>
            {startRow > 0 && (
              <tr aria-hidden="true" tabIndex={-1}>
                <td colSpan={2} style={{ height: startRow * rowHeight }} />
              </tr>
            )}
            {visible.map((row, offset) => (
              <tr
                key={row.key}
                aria-rowindex={startRow + offset + 1}
                data-log-key={row.key}
              >
                <td>
                  <input
                    type="checkbox"
                    aria-label={`Select log line ${row.key}`}
                    checked={selected.has(row.key)}
                    onChange={(event) =>
                      setSelected((old) => {
                        const next = new Set(old);
                        if (event.target.checked) next.add(row.key);
                        else next.delete(row.key);
                        return next;
                      })
                    }
                  />
                </td>
                <td>
                  <pre>{visibleLog(row, timestamps)}</pre>
                </td>
              </tr>
            ))}
            {endRow < filtered.length && (
              <tr aria-hidden="true" tabIndex={-1}>
                <td
                  colSpan={2}
                  style={{ height: (filtered.length - endRow) * rowHeight }}
                />
              </tr>
            )}
          </tbody>
        </table>
        {filtered.length === 0 && <p>No matching buffered lines.</p>}
      </section>
      <div className="log-tools">
        <span>
          {filtered.length} matching · {selected.size} selected
        </span>
        <button
          type="button"
          className="button"
          disabled={!filtered.length}
          onClick={() => setSelected(new Set(filtered.map((row) => row.key)))}
        >
          Select matching lines
        </button>
        <button
          type="button"
          className="button"
          disabled={!selected.size}
          onClick={() => setSelected(new Set())}
        >
          Clear line selection
        </button>
        <button
          type="button"
          className="button"
          disabled={!selected.size}
          onClick={async () => {
            try {
              await navigator.clipboard.writeText(chosenText().join("\n"));
              setCopyStatus("Copied selected visible text.");
            } catch {
              setCopyStatus(
                "Copy unavailable. Select visible text and copy manually.",
              );
            }
          }}
        >
          Copy selected lines
        </button>
        <button
          type="button"
          className="button"
          disabled={!selected.size || exportBusy}
          onClick={() => {
            setExportStatus("");
            setExportLines(chosenText());
          }}
        >
          Export selected lines
        </button>
      </div>
      <p role="status">
        {copyStatus} {exportStatus}
      </p>
      <dialog
        onKeyDown={trapDialogTab}
        ref={dialog}
        aria-labelledby={`export-${id}`}
        className="log-export"
        onCancel={(event) => {
          event.preventDefault();
          if (!exportBusy) setExportLines(null);
        }}
      >
        <h3 id={`export-${id}`}>Export selected logs</h3>
        <p>
          Logs may contain application secrets. Review the selected lines before
          choosing a file.
        </p>
        <p>
          {source} · container {id.slice(0, 12)} · {exportLines?.length ?? 0}{" "}
          selected lines, in buffered order.
        </p>
        <p>
          Preview of the first three selected lines (up to 1,000 characters
          each):
        </p>
        <pre>
          {exportLines
            ?.slice(0, 3)
            .map(
              (line) =>
                line.slice(0, 1000) +
                (line.length > 1000 ? " [preview clipped]" : ""),
            )
            .join("\n")}
        </pre>
        <button
          type="button"
          className="button"
          disabled={exportBusy}
          onClick={() => void save()}
        >
          Save selected logs…
        </button>
        <button
          data-cancel
          type="button"
          className="button"
          disabled={exportBusy}
          onClick={() => setExportLines(null)}
        >
          Cancel export
        </button>
        {exportBusy && (
          <p role="status">Choose a file in the native Save dialog.</p>
        )}
      </dialog>
    </section>
  );
}
