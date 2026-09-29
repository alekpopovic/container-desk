import { useEffect, useRef, useState } from "react";
import * as bridge from "../../lib/ipc/client";
import type {
  ComposeActionOperation,
  ComposeVerification,
  ConfirmationIntent,
  SessionScope,
} from "../../lib/ipc/generated";
const message = (error: unknown) =>
  error instanceof bridge.IpcError
    ? error.message
    : "Compose operation could not be completed.";
export function ComposeManagement({
  scope,
  project,
  host,
  stale,
  refresh,
}: {
  scope: SessionScope;
  project: string;
  host: string;
  stale: boolean;
  refresh: () => void;
}) {
  const [directory, setDirectory] = useState(""),
    [files, setFiles] = useState(""),
    [name, setName] = useState(project),
    [acknowledged, setAcknowledged] = useState(false),
    [enabled, setEnabled] = useState(false),
    [busy, setBusy] = useState(false),
    [initializing, setInitializing] = useState(true),
    [timeout, setTimeoutValue] = useState(10),
    [verification, setVerification] = useState<ComposeVerification | null>(
      null,
    ),
    [intent, setIntent] = useState<ConfirmationIntent | null>(null),
    [expired, setExpired] = useState(false),
    [status, setStatus] = useState(""),
    [observations, setObservations] = useState<string[]>([]);
  const alive = useRef(true),
    lock = useRef(false),
    activeIntent = useRef<string | null>(null),
    binding = useRef(scope).current;
  const current = () => (alive.current ? binding : null);
  useEffect(() => {
    alive.current = true;
    void bridge
      .getManagement(binding, () => (alive.current ? binding : null))
      .then((value) => {
        if (alive.current) setEnabled(value.enabled);
      })
      .catch((error) => {
        if (alive.current) setStatus(message(error));
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
    };
  }, [binding]);
  useEffect(() => {
    if (!verification) return;
    const timer = window.setTimeout(() => {
      setVerification(null);
      setIntent(null);
      setStatus("Verification expired. Verify the remote project again.");
    }, verification.expiresInMs);
    return () => window.clearTimeout(timer);
  }, [verification]);
  useEffect(() => {
    setExpired(false);
    if (!intent) return;
    const timer = window.setTimeout(() => setExpired(true), intent.expiresInMs);
    return () => window.clearTimeout(timer);
  }, [intent]);
  function edit() {
    setVerification(null);
    setIntent(null);
    setAcknowledged(false);
    setObservations([]);
  }
  async function perform(action: () => Promise<void>) {
    if (lock.current) return;
    lock.current = true;
    setBusy(true);
    try {
      await action();
    } catch (error) {
      if (alive.current) setStatus(message(error));
    } finally {
      lock.current = false;
      if (alive.current) setBusy(false);
    }
  }
  async function verify() {
    await perform(async () => {
      setIntent(null);
      setVerification(null);
      setStatus("Verifying remote configuration and existing services…");
      const result = await bridge.verifyComposeProject(
        {
          scope: binding,
          configuration: {
            projectName: name,
            workingDirectory: directory,
            configFiles: files.split(/\r?\n/).filter((file) => file.length > 0),
          },
          acknowledged,
        },
        current,
      );
      if (alive.current) {
        setVerification(result);
        setStatus(
          "Verified remote configuration and existing service identities. Actions require separate confirmation.",
        );
      }
    });
  }
  async function prepare(operation: ComposeActionOperation) {
    if (!verification || !enabled || stale) return;
    await perform(async () => {
      const spec = {
        verificationId: verification.id,
        configuration: verification.configuration,
        services: verification.services,
        containerIds: verification.containerIds,
        operation,
        timeoutSeconds: timeout,
      };
      const prepared = await bridge.prepareComposeAction(
        binding,
        spec,
        current,
      );
      if (alive.current) setIntent(prepared);
    });
  }
  async function observe(ids: string[]) {
    const rows: string[] = [];
    for (const id of ids) {
      if (!alive.current) return;
      try {
        const detail = await bridge.inspectContainer(
          { scope: binding, containerId: id, revealSensitive: false },
          current,
        );
        rows.push(
          `${detail.summary.name} · ${id} · ${detail.summary.state} · health: ${detail.summary.health ?? "not reported"}`,
        );
      } catch (error) {
        rows.push(`${id} · ${message(error)}`);
      }
    }
    if (alive.current) {
      setObservations(rows);
      refresh();
    }
  }
  async function submit() {
    if (
      !intent ||
      intent.operation.category !== "compose" ||
      expired ||
      stale ||
      !enabled
    )
      return;
    const selected = intent,
      spec = intent.operation.spec;
    await perform(async () => {
      setIntent(null);
      setVerification(null);
      activeIntent.current = selected.id;
      setStatus(
        "Compose action submitted once. Waiting for completion; it will not be retried.",
      );
      try {
        const result = await bridge.mutateComposeProject(
          { scope: binding, intentId: selected.id, spec },
          current,
        );
        if (!alive.current) return;
        setStatus(
          result.outcome === "succeeded"
            ? "Compose command completed. Fresh service state is shown below; this is not a readiness guarantee."
            : result.outcome === "unknown"
              ? "Unknown outcome: the project may have changed partially or completely. Read current state and verify again before choosing another action."
              : "Compose action was not dispatched. Verify the configuration and current project state again.",
        );
        await observe(spec.containerIds);
      } catch (error) {
        if (alive.current)
          setStatus(
            `The action outcome may be unknown. Verify current state before another action. ${message(error)}`,
          );
      } finally {
        activeIntent.current = null;
      }
    });
  }
  return (
    <section
      className="container-management compose-management"
      aria-label="Verified Compose actions"
    >
      <h4>Configure remote project actions</h4>
      <p>
        Host: {host}. Configure paths on this remote host. Start, stop and
        restart apply only to the verified existing services.
      </p>
      <label>
        Remote project directory
        <input
          value={directory}
          maxLength={4096}
          disabled={busy || !!intent}
          onChange={(event) => {
            edit();
            setDirectory(event.target.value);
          }}
          placeholder="/srv/my project"
        />
      </label>
      <label>
        Ordered remote config files (one absolute path per line)
        <textarea
          value={files}
          maxLength={32776}
          rows={3}
          disabled={busy || !!intent}
          onChange={(event) => {
            edit();
            setFiles(event.target.value);
          }}
        />
      </label>
      <label>
        Explicit project name
        <input
          value={name}
          maxLength={128}
          disabled={busy || !!intent}
          onChange={(event) => {
            edit();
            setName(event.target.value);
          }}
        />
      </label>
      <label>
        <input
          type="checkbox"
          checked={acknowledged}
          disabled={busy || !!intent}
          onChange={(event) => {
            setAcknowledged(event.target.checked);
            setVerification(null);
          }}
        />
        I confirm these paths and this project name refer to trusted
        configuration on the selected remote host.
      </label>
      <button
        className="button"
        type="button"
        disabled={
          busy || stale || !acknowledged || !directory || !files || !name
        }
        onClick={() => void verify()}
      >
        Verify remote project
      </button>
      {verification && (
        <p>
          Verified services: {verification.services.join(", ")} ·{" "}
          {verification.containerIds.length} existing instance(s). Verification
          expires in five minutes.
        </p>
      )}
      <button
        className="button"
        type="button"
        disabled={busy || initializing || stale}
        onClick={() =>
          void perform(async () => {
            setIntent(null);
            const value = await bridge.setManagement(
              binding,
              !enabled,
              current,
            );
            if (alive.current) setEnabled(value.enabled);
          })
        }
      >
        {enabled ? "Disable Compose management" : "Enable Compose management"}
      </button>
      <label>
        Compose stop timeout (seconds)
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
        {(["start", "stop", "restart"] as const).map((operation) => (
          <button
            className="button"
            type="button"
            key={operation}
            disabled={
              busy ||
              stale ||
              !enabled ||
              !verification ||
              !!intent ||
              !Number.isInteger(timeout) ||
              timeout < 1 ||
              timeout > 120
            }
            onClick={() => void prepare(operation)}
          >
            {operation === "start"
              ? "Start verified services"
              : operation === "stop"
                ? "Stop verified services"
                : "Restart verified services"}
          </button>
        ))}
      </div>
      {intent && intent.operation.category === "compose" && (
        <div
          role="dialog"
          aria-modal="false"
          aria-label="Confirm Compose action"
          className="mutation-confirmation"
        >
          <h4>Confirm Compose {intent.operation.spec.operation}</h4>
          <dl>
            <dt>Host</dt>
            <dd>{host}</dd>
            <dt>Daemon</dt>
            <dd>
              <code>{binding.daemonId}</code>
            </dd>
            <dt>Project</dt>
            <dd>{intent.operation.spec.configuration.projectName}</dd>
            <dt>Remote directory</dt>
            <dd>
              <code>
                {intent.operation.spec.configuration.workingDirectory}
              </code>
            </dd>
            <dt>Ordered files</dt>
            <dd>
              <ol>
                {intent.operation.spec.configuration.configFiles.map((file) => (
                  <li key={file}>
                    <code>{file}</code>
                  </li>
                ))}
              </ol>
            </dd>
            <dt>Services</dt>
            <dd>{intent.operation.spec.services.join(", ")}</dd>
            <dt>Full container IDs</dt>
            <dd>
              {intent.operation.spec.containerIds.map((id) => (
                <p key={id}>
                  <code>{id}</code>
                </p>
              ))}
            </dd>
            <dt>Stop timeout</dt>
            <dd>{intent.operation.spec.timeoutSeconds} seconds</dd>
          </dl>
          <p>
            {expired
              ? "Confirmation expired. Cancel and review again."
              : "Valid for up to 30 seconds; one use only. Stop/restart may kill processes after the timeout."}
          </p>
          <button
            className="button"
            type="button"
            disabled={busy || expired || stale}
            onClick={() => void submit()}
          >
            Confirm Compose action
          </button>
          <button
            className="button"
            type="button"
            disabled={busy}
            onClick={() => setIntent(null)}
          >
            Cancel Compose action
          </button>
        </div>
      )}
      <p role="status">{status}</p>
      {observations.length > 0 && (
        <ul aria-label="Observed Compose service state">
          {observations.map((row) => (
            <li key={row}>{row}</li>
          ))}
        </ul>
      )}
    </section>
  );
}
