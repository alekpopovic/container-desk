import { useCallback, useEffect, useId, useRef, useState } from "react";
import type {
  ContainerDetail,
  ContainerId,
  SessionScope,
} from "../../lib/ipc/generated";
import { inspectContainer, IpcError } from "../../lib/ipc/client";

/** Mounted only for a fresh selection; no detail/reveal cache or persistent state. */
export function ContainerDetails({
  scope,
  id,
}: {
  scope: SessionScope;
  id: ContainerId;
}) {
  const [tab, setTab] = useState<DetailTab>("Overview");
  const panelId = useId();
  const [detail, setDetail] = useState<ContainerDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const alive = useRef(false);
  const serial = useRef(0);
  const load = useCallback(
    async (revealSensitive: boolean) => {
      const ticket = ++serial.current;
      setDetail(null);
      setError(null);
      setBusy(true);
      try {
        const result = await inspectContainer(
          { scope, containerId: id, revealSensitive },
          () => (alive.current && ticket === serial.current ? scope : null),
        );
        if (alive.current && ticket === serial.current) setDetail(result);
      } catch (failure) {
        if (alive.current && ticket === serial.current)
          setError(
            failure instanceof IpcError
              ? failure.message
              : "Container details are unavailable.",
          );
      } finally {
        if (alive.current && ticket === serial.current) setBusy(false);
      }
    },
    [scope, id],
  );
  useEffect(() => {
    alive.current = true;
    void load(false);
    return () => {
      alive.current = false;
      serial.current += 1;
    };
    // Parent keys the component by the complete scope, full ID and snapshot timestamp.
  }, [load]);
  function hide() {
    serial.current += 1;
    setBusy(false);
    setDetail(
      (current) =>
        current && {
          ...current,
          environmentValuesMasked: true,
          environment: current.environment.map((e) => ({
            ...e,
            value: null,
            masked: true,
          })),
          labels: current.labels.map((e) => ({
            ...e,
            value: null,
            masked: true,
          })),
        },
    );
  }
  return (
    <section
      className="inspect-detail"
      aria-label="Inspected container details"
      aria-busy={busy}
    >
      <h3>Inspect details</h3>
      <div
        className="detail-tabs"
        role="tablist"
        aria-label="Container detail sections"
      >
        {tabs.map((name, index) => (
          <button
            key={name}
            type="button"
            role="tab"
            id={`${panelId}-${name}`}
            aria-controls={`${panelId}-panel`}
            aria-selected={tab === name}
            tabIndex={tab === name ? 0 : -1}
            onClick={() => setTab(name)}
            onKeyDown={(event) => {
              const next =
                event.key === "Home"
                  ? 0
                  : event.key === "End"
                    ? tabs.length - 1
                    : event.key === "ArrowRight"
                      ? (index + 1) % tabs.length
                      : event.key === "ArrowLeft"
                        ? (index + tabs.length - 1) % tabs.length
                        : null;
              if (next === null) return;
              event.preventDefault();
              const selected = tabs[next];
              if (selected) setTab(selected);
              event.currentTarget.parentElement
                ?.querySelectorAll<HTMLButtonElement>('[role="tab"]')
                [next]?.focus();
            }}
          >
            {name}
          </button>
        ))}
      </div>
      <div
        role="tabpanel"
        id={`${panelId}-panel`}
        aria-labelledby={`${panelId}-${tab}`}
      >
        {busy && <p role="status">Loading container details…</p>}
        {error && <p role="alert">{error}</p>}
        {!busy && (
          <button
            type="button"
            className="button"
            onClick={() => void load(false)}
          >
            Refresh details
          </button>
        )}
        {detail && (
          <>
            <p className="muted">
              Inspect snapshot ·{" "}
              {detail.environmentValuesMasked
                ? "Sensitive values masked"
                : "Sensitive values revealed for this selection"}
            </p>
            {tab === "Overview" && (
              <>
                <Field label="Container ID" value={detail.summary.id} copy />
                <Field label="Image ID" value={detail.imageId} copy />
                <Field label="Running state" value={detail.summary.state} />
                <Field
                  label="Healthcheck"
                  value={
                    detail.healthcheckConfigured === false
                      ? "Not configured"
                      : detail.healthcheckConfigured === true
                        ? "Configured"
                        : "Unknown configuration"
                  }
                />
                <Field
                  label="Health state"
                  value={detail.summary.health ?? "No status reported"}
                />
                <Field label="Created" value={detail.createdAt} />
                <Field label="Started" value={detail.startedAt} />
                <Field label="Finished" value={detail.finishedAt} />
                <Field label="Exit code" value={detail.exitCode} />
                <Field label="OOM killed" value={detail.oomKilled} />
                {detail.oomKilled === true && (
                  <p className="inspect-warning">
                    Docker reports an out-of-memory kill.
                  </p>
                )}
                <Field label="Restart count" value={detail.restartCount} />
                <Field label="Restart policy" value={detail.restartPolicy} />
                <Field
                  label="Maximum restart retries"
                  value={detail.restartMaximumRetryCount}
                />
                <h4>Resource configuration</h4>
                <p className="muted">
                  Exact configured values. Zero commonly means no explicit
                  limit; swap/PID −1 means unlimited.
                </p>
                {(
                  [
                    ["Memory bytes", detail.resources.memoryBytes],
                    ["Memory + swap bytes", detail.resources.memorySwapBytes],
                    ["Nano CPUs", detail.resources.nanoCpus],
                    ["CPU shares", detail.resources.cpuShares],
                    ["CPU period", detail.resources.cpuPeriod],
                    ["CPU quota", detail.resources.cpuQuota],
                    ["CPU set", detail.resources.cpusetCpus],
                    ["PID limit", detail.resources.pidsLimit],
                    ["Privileged", detail.resources.privileged],
                    [
                      "Read-only root filesystem",
                      detail.resources.readOnlyRootfs,
                    ],
                  ] as const
                ).map(([label, value]) => (
                  <Field key={label} label={label} value={value} />
                ))}
              </>
            )}
            {tab === "Ports" && (
              <>
                <h4>Exposed container ports</h4>
                <p>
                  Declared inside the container. Exposure alone does not publish
                  a host port.
                </p>
                {detail.exposedPorts.length ? (
                  <ul>
                    {detail.exposedPorts.map((p) => (
                      <li key={`${p.privatePort}/${p.protocol}`}>
                        {p.privatePort}/{p.protocol}
                      </li>
                    ))}
                  </ul>
                ) : (
                  <p>No exposed ports reported.</p>
                )}
                <h4>Active host bindings</h4>
                {detail.summary.ports.some((p) => p.publicPort !== null) ? (
                  <ul>
                    {detail.summary.ports
                      .filter((p) => p.publicPort !== null)
                      .map((p) => (
                        <li key={JSON.stringify(p)}>
                          {p.hostIp?.includes(":")
                            ? `[${p.hostIp}]`
                            : (p.hostIp ?? "*")}
                          :{p.publicPort} → {p.privatePort}/{p.protocol}
                        </li>
                      ))}
                  </ul>
                ) : (
                  <p>No active host bindings reported.</p>
                )}
              </>
            )}
            {tab === "Mounts" &&
              (detail.mounts.length ? (
                detail.mounts.map((m) => (
                  <div className="inspect-item" key={JSON.stringify(m)}>
                    <Field
                      label="Mount destination"
                      value={m.destination}
                      copy
                    />
                    <Field label="Mount source" value={m.source} copy />
                    <Field label="Mount type" value={m.kind} />
                    <Field label="Mount name" value={m.name} />
                    <Field
                      label="Access"
                      value={
                        m.readWrite === null
                          ? null
                          : m.readWrite
                            ? "Read/write"
                            : "Read-only"
                      }
                    />
                    <Field label="Propagation" value={m.propagation} />
                  </div>
                ))
              ) : (
                <p>No mounts reported.</p>
              ))}
            {tab === "Networks" &&
              (detail.networks.length ? (
                detail.networks.map((n) => (
                  <div className="inspect-item" key={n.name}>
                    <Field label="Network name" value={n.name} />
                    <Field label="Network ID" value={n.networkId} copy />
                    <Field label="IPv4" value={n.ipv4} copy />
                    <Field label="IPv6" value={n.ipv6} copy />
                    <Field label="Gateway" value={n.gateway} />
                    <Field label="MAC address" value={n.macAddress} />
                    <Field
                      label="Aliases"
                      value={n.aliases.join(", ") || "None reported"}
                    />
                  </div>
                ))
              ) : (
                <p>No network addresses reported.</p>
              ))}
            {(tab === "Environment" || tab === "Labels") && (
              <>
                <p>
                  Values are masked by default. Revealed values stay in this
                  selection and clear on refresh, container, route or session
                  change.
                </p>
                <button
                  className="button"
                  type="button"
                  onClick={() =>
                    detail.environmentValuesMasked ? void load(true) : hide()
                  }
                >
                  {detail.environmentValuesMasked
                    ? "Reveal sensitive values"
                    : "Hide sensitive values"}
                </button>
                <SecretEntries
                  values={
                    tab === "Environment" ? detail.environment : detail.labels
                  }
                  empty={
                    tab === "Environment"
                      ? "No environment entries reported."
                      : "No labels reported."
                  }
                />
              </>
            )}
          </>
        )}
      </div>
    </section>
  );
}
const tabs = [
  "Overview",
  "Ports",
  "Mounts",
  "Networks",
  "Labels",
  "Environment",
] as const;
type DetailTab = (typeof tabs)[number];
function Field({
  label,
  value,
  copy = false,
}: {
  label: string;
  value: string | number | boolean | null;
  copy?: boolean;
}) {
  return (
    <dl>
      <dt>{label}</dt>
      <dd>
        {value === null ? "Unknown" : String(value)}
        {copy && typeof value === "string" && value !== "" && (
          <CopyField label={label} value={value} />
        )}
      </dd>
    </dl>
  );
}
function CopyField({ label, value }: { label: string; value: string }) {
  const [status, setStatus] = useState("");
  const [busy, setBusy] = useState(false);
  return (
    <span className="copy-field">
      <button
        className="button"
        type="button"
        disabled={busy}
        aria-label={`Copy ${label.toLowerCase()}`}
        onClick={async () => {
          setBusy(true);
          setStatus("");
          try {
            await navigator.clipboard.writeText(value);
            setStatus(`Copied ${label.toLowerCase()}.`);
          } catch {
            setStatus("Copy unavailable. Select this text and copy manually.");
          } finally {
            setBusy(false);
          }
        }}
      >
        Copy
      </button>
      <span role="status">{status}</span>
    </span>
  );
}
function SecretEntries({
  values,
  empty,
}: {
  values: ContainerDetail["environment"];
  empty: string;
}) {
  return values.length ? (
    <dl>
      {values.map((v, i) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: Immutable snapshot permits repeated environment names; items have no local state.
        <div key={`${v.name}-${i}`}>
          <dt>{v.name}</dt>
          <dd>{v.masked ? "••••" : v.value === "" ? "(empty)" : v.value}</dd>
        </div>
      ))}
    </dl>
  ) : (
    <p>{empty}</p>
  );
}
