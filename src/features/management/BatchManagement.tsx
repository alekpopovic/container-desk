import {
  ConfirmationDialog,
  useConfirmationFocus,
} from "../../components/ConfirmationDialog";
import { useEffect, useRef, useState } from "react";
import * as bridge from "../../lib/ipc/client";
import type {
  ConfirmationIntent,
  ContainerSummary,
  MutationOperation,
  MutationTargetResult,
  SessionScope,
} from "../../lib/ipc/generated";
const messageFor = (error: unknown) =>
  error instanceof bridge.IpcError
    ? error.message
    : "The operation could not be completed.";
export function BatchManagement({
  scope,
  rows,
  host,
  stale,
  refresh,
  clear,
  onBusy,
}: {
  scope: SessionScope;
  rows: ContainerSummary[];
  host: string;
  stale: boolean;
  refresh: () => void;
  clear: () => void;
  onBusy: (value: boolean) => void;
}) {
  const binding = useRef(scope).current;
  const alive = useRef(true);
  const lock = useRef(false);
  const activeIntent = useRef<string | null>(null);
  const confirmationFocus = useConfirmationFocus();
  const [enabled, setEnabled] = useState(false);
  const [initializing, setInitializing] = useState(true);
  const [busy, setBusy] = useState(false);
  const [intent, setIntent] = useState<ConfirmationIntent | null>(null);
  const [expired, setExpired] = useState(false);
  const [timeout, setTimeoutValue] = useState(10);
  const [message, setMessage] = useState("");
  const [results, setResults] = useState<MutationTargetResult[]>([]);
  const [names, setNames] = useState<Record<string, string>>({});
  const [needsRefresh, setNeedsRefresh] = useState(false);
  const [removalAcknowledged, acknowledgeRemoval] = useState(false);
  const selection = JSON.stringify(rows.map((row) => row.id));
  const current = () => (alive.current ? binding : null);
  function working(value: boolean) {
    lock.current = value;
    setBusy(value);
    onBusy(value);
  }
  useEffect(() => {
    alive.current = true;
    void bridge
      .getManagement(binding, () => (alive.current ? binding : null))
      .then((state) => {
        if (alive.current) setEnabled(state.enabled);
      })
      .catch((error) => {
        if (alive.current) setMessage(messageFor(error));
      })
      .finally(() => {
        if (alive.current) setInitializing(false);
      });
    return () => {
      alive.current = false;
      if (activeIntent.current)
        void bridge
          .cancelMutation(binding, activeIntent.current)
          .catch(() => {});
      onBusy(false);
    };
  }, [binding, onBusy]);
  useEffect(() => {
    if (selection) {
      setIntent(null);
      acknowledgeRemoval(false);
    }
  }, [selection]);
  useEffect(() => {
    if (!intent) return;
    setExpired(false);
    const timer = window.setTimeout(() => setExpired(true), intent.expiresInMs);
    return () => window.clearTimeout(timer);
  }, [intent]);
  async function toggle() {
    if (lock.current) return;
    working(true);
    setIntent(null);
    try {
      const state = await bridge.setManagement(binding, !enabled, current);
      if (alive.current) setEnabled(state.enabled);
    } catch (error) {
      if (alive.current) setMessage(messageFor(error));
    } finally {
      if (alive.current) working(false);
    }
  }
  async function prepare(operation: MutationOperation) {
    if (
      lock.current ||
      !enabled ||
      stale ||
      needsRefresh ||
      rows.length < 1 ||
      rows.length > 20
    )
      return;
    working(true);
    setMessage("");
    acknowledgeRemoval(false);
    try {
      const prepared = await bridge.prepareMutation(
        binding,
        {
          operation,
          containerIds: rows.map((row) => row.id),
          timeoutSeconds: timeout,
        },
        current,
      );
      if (alive.current) {
        setIntent(prepared);
        setNames(Object.fromEntries(rows.map((row) => [row.id, row.name])));
      }
    } catch (error) {
      if (alive.current) setMessage(messageFor(error));
    } finally {
      if (alive.current) working(false);
    }
  }
  async function loadProgress(id: string) {
    const record = (await bridge.getActivity()).find(
      (record) =>
        record.id === id && record.hostId === binding.selection.hostId,
    );
    if (alive.current && activeIntent.current === id && record)
      setResults(record.results);
  }
  async function submit() {
    if (
      lock.current ||
      !intent ||
      expired ||
      intent.operation.category !== "mutation" ||
      !enabled ||
      (intent.operation.spec.operation === "remove" && !removalAcknowledged)
    )
      return;
    const selected = intent;
    const spec = intent.operation.spec;
    working(true);
    setIntent(null);
    setResults([]);
    setNeedsRefresh(true);
    activeIntent.current = selected.id;
    setMessage(
      "Executing one container at a time. Cancel pending work to leave already dispatched actions running to their result.",
    );
    let polling = false;
    const poll = () => {
      if (polling) return;
      polling = true;
      void loadProgress(selected.id)
        .catch(() => {})
        .finally(() => {
          polling = false;
        });
    };
    poll();
    const timer = window.setInterval(poll, 1000);
    try {
      const response = await bridge.mutateContainer(
        binding,
        spec,
        selected.id,
        current,
      );
      if (!alive.current) return;
      activeIntent.current = null;
      setResults(response.results);
      setNeedsRefresh(response.results.some((r) => r.outcome === "unknown"));
      setMessage(
        response.outcome === "unknown"
          ? "At least one outcome is unknown. Refresh the current state before another batch."
          : "Batch finished. Each result below is retained; command completion does not guarantee application readiness.",
      );
      refresh();
    } catch (error) {
      if (alive.current) {
        setMessage(
          `Batch response unavailable. Review the recorded results and refresh state. ${messageFor(error)}`,
        );
        await loadProgress(selected.id).catch(() => {});
      }
    } finally {
      window.clearInterval(timer);
      activeIntent.current = null;
      if (alive.current) working(false);
    }
  }
  async function cancel() {
    const id = activeIntent.current;
    if (!id) return;
    try {
      const response = await bridge.cancelMutation(binding, id);
      if (alive.current)
        setMessage(
          response.pendingCancellationRequested
            ? "Pending cancellation requested. An already dispatched target is allowed to finish; its result remains below."
            : "This batch has already finished. Review its results below.",
        );
    } catch (error) {
      if (alive.current) setMessage(messageFor(error));
    }
  }
  async function readState() {
    if (lock.current) return;
    working(true);
    try {
      await bridge.listContainers(binding, current);
      if (alive.current) {
        setNeedsRefresh(false);
        refresh();
        setMessage(
          "Current inventory was read. Historical unknown outcomes remain in local activity.",
        );
      }
    } catch (error) {
      if (alive.current) setMessage(messageFor(error));
    } finally {
      if (alive.current) working(false);
    }
  }
  const disabled =
    !enabled ||
    busy ||
    stale ||
    needsRefresh ||
    !!intent ||
    !Number.isInteger(timeout) ||
    timeout < 1 ||
    timeout > 120;
  return (
    <section
      className="batch-management container-management"
      aria-label="Selected container actions"
    >
      <h3>
        {rows.length} explicitly selected containers · {host}
      </h3>
      <p>
        One host, up to 20 full IDs. Work runs sequentially. Leaving this
        selection cancels pending actions.
      </p>
      <button
        className="button"
        type="button"
        disabled={busy || initializing}
        onClick={() => void toggle()}
      >
        {enabled ? "Disable batch management" : "Enable batch management"}
      </button>
      <button className="button" type="button" disabled={busy} onClick={clear}>
        Clear batch selection
      </button>
      <label>
        Batch stop timeout (seconds)
        <input
          type="number"
          min={1}
          max={120}
          value={timeout}
          disabled={busy || !!intent}
          onChange={(event) => setTimeoutValue(Number(event.target.value))}
        />
      </label>
      <div className="management-actions">
        {(["start", "stop", "restart"] as const).map((action) => (
          <button
            className="button"
            type="button"
            key={action}
            disabled={disabled}
            onClick={() => void prepare(action)}
          >
            {action[0]?.toUpperCase()}
            {action.slice(1)} selected
          </button>
        ))}
        <button
          className="button"
          type="button"
          disabled={
            disabled ||
            rows.some((row) => !["created", "exited"].includes(row.state))
          }
          onClick={() => void prepare("remove")}
        >
          Remove selected stopped containers
        </button>
      </div>
      {intent && intent.operation.category === "mutation" && (
        <ConfirmationDialog
          returnFocus={confirmationFocus}
          label={
            intent.operation.spec.operation === "remove"
              ? "Confirm stopped-container removal"
              : "Confirm container batch"
          }
          busy={busy}
          onCancel={() => setIntent(null)}
          className="mutation-confirmation"
        >
          <h4>
            Confirm {intent.operation.spec.operation} · {host}
          </h4>
          <p>
            Daemon: <code>{binding.daemonId}</code>
          </p>
          <ul>
            {intent.operation.spec.containerIds.map((id) => (
              <li key={id}>
                {names[id]} <code>{id}</code>
              </li>
            ))}
          </ul>
          {intent.operation.spec.operation === "remove" ? (
            <>
              <p>
                Permanently remove only these stopped containers. No force or
                volume-removal option is used. Running containers are refused.
              </p>
              <label className="removal-ack">
                <input
                  type="checkbox"
                  checked={removalAcknowledged}
                  onChange={(event) => acknowledgeRemoval(event.target.checked)}
                />
                I confirm removal of the listed stopped containers.
              </label>
            </>
          ) : (
            <p>
              Stop/restart timeout: {intent.operation.spec.timeoutSeconds}{" "}
              seconds.
            </p>
          )}
          <p>
            {expired
              ? "Confirmation expired. Cancel and review again."
              : "This exact selection can be submitted once within 30 seconds."}
          </p>
          <button
            className="button"
            type="button"
            disabled={
              busy ||
              expired ||
              (intent.operation.spec.operation === "remove" &&
                !removalAcknowledged)
            }
            onClick={() => void submit()}
          >
            Confirm selected action
          </button>
          <button
            data-cancel
            className="button"
            type="button"
            disabled={busy}
            onClick={() => setIntent(null)}
          >
            Cancel batch confirmation
          </button>
        </ConfirmationDialog>
      )}
      <p role="status">{message}</p>
      {busy && activeIntent.current && (
        <button className="button" type="button" onClick={() => void cancel()}>
          Cancel pending actions
        </button>
      )}
      <button
        className="button"
        type="button"
        disabled={busy}
        onClick={() => void readState()}
      >
        Refresh batch state
      </button>
      {results.length > 0 && (
        <section
          className="batch-results"
          aria-label="Individual action results"
        >
          <h4>Individual results</h4>
          <ul>
            {results.map((result) => (
              <li key={result.containerId} data-outcome={result.outcome}>
                <strong>
                  {names[result.containerId] ?? "Selected container"}
                </strong>
                <code>{result.containerId}</code>
                <span>
                  {busy && result.outcome === "unknown"
                    ? "Dispatched; waiting for result"
                    : result.outcome.replaceAll("_", " ")}{" "}
                  ·{" "}
                  {result.dispatched
                    ? "command dispatched"
                    : "command not dispatched"}
                </span>
                {result.error && (
                  <span>{new bridge.IpcError(result.error).message}</span>
                )}
              </li>
            ))}
          </ul>
          <p>
            Partial results remain in local activity history. Completed commands
            are not replayed.
          </p>
        </section>
      )}
    </section>
  );
}
