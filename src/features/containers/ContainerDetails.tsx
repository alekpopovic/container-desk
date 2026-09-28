import { useCallback, useEffect, useRef, useState } from "react";
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
          <dl>
            <dt>Image ID</dt>
            <dd>{detail.imageId ?? "Unknown"}</dd>
            <dt>Created</dt>
            <dd>{detail.createdAt ?? "Unknown"}</dd>
            <dt>Started</dt>
            <dd>{detail.startedAt ?? "Not available"}</dd>
            <dt>Finished</dt>
            <dd>{detail.finishedAt ?? "Not available"}</dd>
            <dt>Exit code</dt>
            <dd>{detail.exitCode ?? "Unknown"}</dd>
            <dt>Restart count</dt>
            <dd>{detail.restartCount ?? "Unknown"}</dd>
            <dt>Restart policy</dt>
            <dd>
              {detail.restartPolicy ?? "Unknown"} · maximum retries{" "}
              {detail.restartMaximumRetryCount ?? "unknown"}
            </dd>
            <dt>Health</dt>
            <dd>{detail.summary.health ?? "No health status available"}</dd>
            <dt>Inspected state</dt>
            <dd>{detail.summary.state}</dd>
          </dl>
          <h4>Port bindings</h4>
          {detail.summary.ports.length ? (
            <ul>
              {detail.summary.ports.map((p) => (
                <li key={JSON.stringify(p)}>
                  {p.privatePort}/{p.protocol} →{" "}
                  {p.publicPort === null
                    ? "Not published"
                    : `${p.hostIp?.includes(":") ? `[${p.hostIp}]` : (p.hostIp ?? "*")}:${p.publicPort}`}
                </li>
              ))}
            </ul>
          ) : (
            <p>No bindings reported.</p>
          )}
          <h4>Resource configuration</h4>
          <p className="muted">
            Exact configured values. Zero commonly means no explicit limit;
            swap/PID −1 means unlimited. Unknown fields remain unavailable.
          </p>
          <dl>
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
                ["Read-only root filesystem", detail.resources.readOnlyRootfs],
              ] as const
            ).map(([name, value]) => (
              <div key={name}>
                <dt>{name}</dt>
                <dd>{value === null ? "Unknown" : String(value)}</dd>
              </div>
            ))}
          </dl>
          <h4>Mounts</h4>
          {detail.mounts.length ? (
            detail.mounts.map((m) => (
              <dl key={JSON.stringify(m)}>
                <dt>{m.destination ?? "Unknown destination"}</dt>
                <dd>
                  {m.kind ?? "Unknown type"} · {m.source ?? "Unknown source"} ·{" "}
                  {m.readWrite === null
                    ? "Unknown access"
                    : m.readWrite
                      ? "Read/write"
                      : "Read-only"}
                  {m.name ? ` · ${m.name}` : ""}
                  {m.propagation ? ` · ${m.propagation}` : ""}
                </dd>
              </dl>
            ))
          ) : (
            <p>No mounts reported.</p>
          )}
          <h4>Networks</h4>
          {detail.networks.length ? (
            detail.networks.map((n) => (
              <dl key={n.name}>
                <dt>{n.name}</dt>
                <dd>{n.networkId ?? "Unknown network ID"}</dd>
                <dt>IPv4 / IPv6</dt>
                <dd>
                  {n.ipv4 ?? "Not assigned"} / {n.ipv6 ?? "Not assigned"}
                </dd>
                <dt>Gateway / MAC</dt>
                <dd>
                  {n.gateway ?? "Unknown"} / {n.macAddress ?? "Unknown"}
                </dd>
                <dt>Aliases</dt>
                <dd>{n.aliases.join(", ") || "None reported"}</dd>
              </dl>
            ))
          ) : (
            <p>No network addresses reported.</p>
          )}
          <h4>Environment and labels</h4>
          <p>
            Values are masked by default. Reveal displays them only in this
            selection; changing container, connection, route or refreshing
            clears them.
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
          <h4>Environment</h4>
          <dl>
            {detail.environment.map((v, i) => (
              // biome-ignore lint/suspicious/noArrayIndexKey: Immutable snapshot permits repeated environment names; items have no local state.
              <div key={`${v.name}-${i}`}>
                <dt>{v.name}</dt>
                <dd>
                  {v.masked ? "••••" : v.value === "" ? "(empty)" : v.value}
                </dd>
              </div>
            ))}
          </dl>
          {!detail.environment.length && (
            <p>No environment entries reported.</p>
          )}
          <h4>Labels</h4>
          <dl>
            {detail.labels.map((v) => (
              <div key={v.name}>
                <dt>{v.name}</dt>
                <dd>
                  {v.masked ? "••••" : v.value === "" ? "(empty)" : v.value}
                </dd>
              </div>
            ))}
          </dl>
          {!detail.labels.length && <p>No labels reported.</p>}
        </>
      )}
    </section>
  );
}
