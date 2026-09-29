import {
  ConfirmationDialog,
  useConfirmationFocus,
} from "../../components/ConfirmationDialog";
import { useCallback, useEffect, useRef, useState } from "react";
import * as bridge from "../../lib/ipc/client";
import type {
  ActivityRecord,
  ConfirmationIntent,
  ContainerSummary,
  MutationOperation,
  MutationSpec,
  SessionScope,
} from "../../lib/ipc/generated";
const errorText = (error: unknown) =>
  error instanceof bridge.IpcError
    ? error.message
    : "The operation could not be completed.";
export function ContainerManagement({
  scope,
  row,
  host,
  stale,
  refresh,
}: {
  scope: SessionScope;
  row: ContainerSummary;
  host: string;
  stale: boolean;
  refresh: () => void;
}) {
  const confirmationFocus = useConfirmationFocus();
  const [enabled, setEnabled] = useState(false);
  const [initializing, setInitializing] = useState(true);
  const binding = useRef(scope).current;
  const [busy, setBusy] = useState(false);
  const [timeout, setTimeoutValue] = useState(10);
  const [intent, setIntent] = useState<ConfirmationIntent | null>(null);
  const [message, setMessage] = useState("");
  const [needsRefresh, setNeedsRefresh] = useState(false);
  const [history, setHistory] = useState<ActivityRecord[]>([]);
  const [expired, setExpired] = useState(false);
  const alive = useRef(true);
  const lock = useRef(false);
  const current = () => (alive.current ? scope : null);
  const loadHistory = useCallback(
    async (checkUnknown = false) => {
      try {
        const records = (await bridge.getActivity()).filter(
          (record) =>
            record.hostId === scope.selection.hostId &&
            record.targets.includes(row.id),
        );
        if (!alive.current) return;
        setHistory(records.slice(-5).reverse());
        if (checkUnknown && records.at(-1)?.outcome === "unknown") {
          setNeedsRefresh(true);
          setMessage(
            "Previous action has an unknown outcome. Refresh state before choosing another action.",
          );
        }
      } catch (error) {
        if (alive.current) setMessage(errorText(error));
      }
    },
    [scope.selection.hostId, row.id],
  );
  useEffect(() => {
    alive.current = true;
    void bridge
      .getManagement(binding, () => (alive.current ? binding : null))
      .then((state) => {
        if (alive.current) setEnabled(state.enabled);
      })
      .catch((error) => {
        if (alive.current) setMessage(errorText(error));
      })
      .finally(() => {
        if (alive.current) setInitializing(false);
      });
    void loadHistory(true);
    return () => {
      alive.current = false;
    };
    // Parent keys this panel by full scope and container ID.
  }, [loadHistory, binding]);
  useEffect(() => {
    if (!intent) return;
    setExpired(false);
    const timer = window.setTimeout(() => setExpired(true), intent.expiresInMs);
    return () => window.clearTimeout(timer);
  }, [intent]);
  async function toggle() {
    if (lock.current) return;
    lock.current = true;
    setBusy(true);
    setIntent(null);
    try {
      const result = await bridge.setManagement(scope, !enabled, current);
      if (alive.current) setEnabled(result.enabled);
    } catch (error) {
      if (alive.current) setMessage(errorText(error));
    } finally {
      lock.current = false;
      if (alive.current) setBusy(false);
    }
  }
  async function prepare(action: MutationOperation) {
    if (lock.current || !enabled || stale || needsRefresh) return;
    lock.current = true;
    setBusy(true);
    setMessage("");
    const spec: MutationSpec = {
      operation: action,
      containerIds: [row.id],
      timeoutSeconds: timeout,
    };
    try {
      const prepared = await bridge.prepareMutation(scope, spec, current);
      if (alive.current) setIntent(prepared);
    } catch (error) {
      if (alive.current) setMessage(errorText(error));
    } finally {
      lock.current = false;
      if (alive.current) setBusy(false);
    }
  }
  async function observe(prefix = "", expected?: string) {
    refresh();
    const detail = await bridge.inspectContainer(
      { scope, containerId: row.id, revealSensitive: false },
      current,
    );
    if (!alive.current) return;
    const summary = detail.summary;
    setNeedsRefresh(!!expected && summary.state !== expected);
    setMessage(
      `${prefix}${expected && summary.state !== expected ? `Expected ${expected} is not yet observed; convergence is pending. ` : ""}Observed state: ${summary.state}; health: ${summary.health ?? "not reported"}. ${summary.health === "starting" ? "Health check is still starting. Refresh to check readiness." : "This is a fresh snapshot, not a readiness guarantee."}`,
    );
  }
  async function refreshState() {
    if (lock.current) return;
    lock.current = true;
    setBusy(true);
    try {
      await observe();
    } catch (error) {
      if (alive.current)
        setMessage(`State could not be verified. ${errorText(error)}`);
    } finally {
      lock.current = false;
      if (alive.current) setBusy(false);
    }
  }
  async function submit() {
    if (
      lock.current ||
      !intent ||
      expired ||
      intent.operation.category !== "mutation" ||
      !enabled ||
      stale ||
      needsRefresh
    )
      return;
    const selected = intent;
    const spec = intent.operation.spec;
    lock.current = true;
    setBusy(true);
    setIntent(null);
    setNeedsRefresh(true);
    setMessage(
      "Action submitted. Waiting for its result; it will not be retried automatically.",
    );
    try {
      const result = await bridge.mutateContainer(
        scope,
        spec,
        selected.id,
        current,
      );
      if (!alive.current) return;
      if (result.outcome === "unknown") {
        setMessage(
          "Unknown outcome. The connection may have closed after the action ran. Refresh state before choosing another action.",
        );
      } else {
        const prefix =
          result.outcome === "succeeded"
            ? "Command completed. "
            : "Command reported failure. ";
        try {
          await observe(
            prefix,
            spec.operation === "stop" ? "exited" : "running",
          );
        } catch (error) {
          if (alive.current)
            setMessage(
              `${prefix}State verification is pending. ${errorText(error)}`,
            );
        }
      }
    } catch (error) {
      if (alive.current)
        setMessage(
          `The action could not be confirmed. Its outcome may be unknown. Refresh state before choosing another action. ${errorText(error)}`,
        );
    } finally {
      lock.current = false;
      if (alive.current) {
        setBusy(false);
        void loadHistory();
      }
    }
  }
  return (
    <section className="container-management" aria-label="Container management">
      <h3>Container management</h3>
      <p>
        {enabled
          ? "Management enabled for this session."
          : "Read-only controls. Enable management to invoke an action."}
      </p>
      <button
        className="button"
        type="button"
        disabled={busy || initializing}
        onClick={() => void toggle()}
      >
        {enabled ? "Disable management" : "Enable management"}
      </button>
      <label>
        Stop timeout (seconds)
        <input
          type="number"
          min={1}
          max={120}
          value={timeout}
          disabled={busy || !!intent}
          onChange={(event) => setTimeoutValue(Number(event.target.value))}
        />
      </label>
      <p className="muted">
        Stop and restart send the container's stop signal, then Docker may kill
        it after the timeout.
      </p>
      <div className="management-actions">
        {(["start", "stop", "restart"] as const).map((action) => (
          <button
            className="button"
            key={action}
            type="button"
            disabled={
              !enabled ||
              busy ||
              stale ||
              needsRefresh ||
              !!intent ||
              !Number.isInteger(timeout) ||
              timeout < 1 ||
              timeout > 120
            }
            onClick={() => void prepare(action)}
          >
            {action === "start"
              ? "Start container"
              : action === "stop"
                ? "Stop container"
                : "Restart container"}
          </button>
        ))}
      </div>
      {intent && intent.operation.category === "mutation" && (
        <ConfirmationDialog
          returnFocus={confirmationFocus}
          label="Confirm container action"
          busy={busy}
          onCancel={() => setIntent(null)}
          className="mutation-confirmation"
        >
          <h4>Confirm {intent.operation.spec.operation}</h4>
          <dl>
            <dt>Host</dt>
            <dd>{host}</dd>
            <dt>Daemon</dt>
            <dd>
              <code>{scope.daemonId}</code>
            </dd>
            <dt>Container</dt>
            <dd>{row.name}</dd>
            <dt>Full ID</dt>
            <dd>
              <code>{row.id}</code>
            </dd>
            <dt>Stop timeout</dt>
            <dd>{intent.operation.spec.timeoutSeconds} seconds</dd>
          </dl>
          <p>
            {expired
              ? "Confirmation expired. Cancel and review the action again."
              : "This confirmation is valid for up to 30 seconds and can be used once."}
          </p>
          <button
            className="button"
            type="button"
            disabled={expired || busy || stale}
            onClick={() => void submit()}
          >
            Confirm action
          </button>
          <button
            data-cancel
            className="button"
            type="button"
            disabled={busy}
            onClick={() => setIntent(null)}
          >
            Cancel action
          </button>
        </ConfirmationDialog>
      )}
      <p role="status">{message}</p>
      <button
        className="button"
        type="button"
        disabled={busy}
        onClick={() => void refreshState()}
      >
        Refresh action state
      </button>
      {history.length > 0 && (
        <details>
          <summary>Recent local activity for this container</summary>
          <p>
            Local history can be edited; it is not a tamper-proof audit log.
          </p>
          <ul>
            {history.map((item) => (
              <li key={item.id}>
                {new Date(item.updatedAtMs).toLocaleString()} · {item.action} ·{" "}
                {item.outcome.replaceAll("_", " ")}
              </li>
            ))}
          </ul>
        </details>
      )}
    </section>
  );
}
