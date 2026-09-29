import { useCallback, useEffect, useRef, useState } from "react";
import {
  IpcError,
  inspectNetwork,
  listNetworks,
  sameScope,
} from "../../lib/ipc/client";
import type {
  ListNetworksResponse,
  SessionScope,
  NetworkDetail,
} from "../../lib/ipc/generated";
import type { InventoryView } from "../containers/cache";
const message = (error: unknown) =>
  error instanceof IpcError
    ? error.message
    : "Network metadata could not be read.";
export function NetworkInventory({
  view,
  native,
  refreshContainers,
  openContainer,
}: {
  view: InventoryView;
  native: boolean;
  refreshContainers: () => void;
  openContainer: (scope: SessionScope, id: string) => void;
}) {
  const [snapshot, setSnapshot] = useState<{
      data: ListNetworksResponse;
      time: number;
    } | null>(null),
    [error, setError] = useState<string | null>(null),
    [busy, setBusy] = useState(false),
    [query, setQuery] = useState(""),
    [page, setPage] = useState(0),
    [selected, select] = useState<string | null>(null);
  const current = useRef(view.scope),
    alive = useRef(false),
    serial = useRef(0);
  current.current = view.scope;
  const scope = view.scope;
  const refresh = useCallback(async () => {
    if (!native || !scope) return;
    const ticket = ++serial.current;
    setBusy(true);
    setError(null);
    try {
      const data = await listNetworks(scope, () =>
        alive.current && ticket === serial.current ? current.current : null,
      );
      if (alive.current && ticket === serial.current)
        setSnapshot({ data, time: Date.now() });
    } catch (error) {
      if (alive.current && ticket === serial.current) setError(message(error));
    } finally {
      if (alive.current && ticket === serial.current) setBusy(false);
    }
  }, [native, scope]);
  useEffect(() => {
    alive.current = true;
    void refresh();
    return () => {
      alive.current = false;
      serial.current++;
    };
  }, [refresh]);
  const visible =
    scope && snapshot && sameScope(scope, snapshot.data.scope)
      ? snapshot
      : null;
  const rows = visible?.data.networks ?? [],
    filtered = rows.filter((row) =>
      [row.name, row.id, row.driver ?? "", row.networkScope ?? ""].some(
        (value) =>
          value.toLocaleLowerCase().includes(query.toLocaleLowerCase()),
      ),
    );
  const pages = Math.max(1, Math.ceil(filtered.length / 50)),
    index = Math.min(page, pages - 1),
    chosen = filtered.find((row) => row.id === selected) ?? filtered[0];
  if (!scope || !native)
    return (
      <section className="image-view">
        <h3>No live network data</h3>
        <p>
          Connect a host to read network metadata and container attachments.
          Demo mode has no native network inventory.
        </p>
      </section>
    );
  return (
    <section className="image-view network-view" aria-label="Network inventory">
      <p>
        Read-only network metadata. Network settings and attachments are
        read-only.
      </p>
      <div className="image-tools">
        <label>
          Search networks
          <input
            type="search"
            data-shortcut="search"
            aria-keyshortcuts="Control+f Meta+f"
            value={query}
            maxLength={256}
            onChange={(e) => {
              setQuery(e.target.value);
              setPage(0);
            }}
          />
        </label>
        <button
          data-shortcut="refresh"
          aria-keyshortcuts="Control+r Meta+r"
          className="button"
          type="button"
          disabled={busy}
          onClick={() => {
            refreshContainers();
            void refresh();
          }}
        >
          Refresh networks
        </button>
      </div>
      <p role={error ? "alert" : "status"}>
        {busy ? "Reading networks… " : ""}
        {error ? `Stale network snapshot. ${error} ` : ""}
        {visible ? (
          <>
            Last successful read:{" "}
            <time dateTime={new Date(visible.time).toISOString()}>
              {new Date(visible.time).toLocaleTimeString()}
            </time>
            .
          </>
        ) : (
          "No successful network snapshot yet."
        )}
      </p>
      {!busy && !error && visible && !rows.length && (
        <p>No networks on this daemon.</p>
      )}
      <p>{filtered.length} matching networks</p>
      <div className="image-layout">
        <section className="image-list" aria-label="Available networks">
          {filtered.slice(index * 50, index * 50 + 50).map((row) => (
            <button
              className="image-choice network-choice"
              type="button"
              key={row.id}
              aria-pressed={chosen?.id === row.id}
              onClick={() => select(row.id)}
            >
              <strong>{row.name}</strong>
              <code>{row.id}</code>
              <span>
                Driver: {row.driver ?? "Unknown"} · Scope:{" "}
                {row.networkScope ?? "Unknown"}
              </span>
            </button>
          ))}
          {pages > 1 && (
            <div className="image-pages">
              <button
                type="button"
                disabled={index === 0}
                onClick={() => setPage(index - 1)}
              >
                Previous networks
              </button>
              <span>
                {index + 1} / {pages}
              </span>
              <button
                type="button"
                disabled={index === pages - 1}
                onClick={() => setPage(index + 1)}
              >
                Next networks
              </button>
            </div>
          )}
        </section>
        {chosen && visible && (
          <Details
            key={JSON.stringify([scope, chosen.id, visible.time])}
            scope={scope}
            id={chosen.id}
            view={view}
            stale={!!error}
            openContainer={openContainer}
          />
        )}
      </div>
    </section>
  );
}
function Details({
  scope,
  id,
  view,
  stale,
  openContainer,
}: {
  scope: SessionScope;
  id: string;
  view: InventoryView;
  stale: boolean;
  openContainer: (scope: SessionScope, id: string) => void;
}) {
  const [detail, setDetail] = useState<NetworkDetail | null>(null),
    [error, setError] = useState<string | null>(null),
    [page, setPage] = useState(0);
  const current = useRef(view.scope);
  current.current = view.scope;
  useEffect(() => {
    let active = true;
    void inspectNetwork({ scope, networkId: id }, () =>
      active ? current.current : null,
    )
      .then((value) => {
        if (active) setDetail(value);
      })
      .catch((error) => {
        if (active) setError(message(error));
      });
    return () => {
      active = false;
    };
  }, [scope, id]);
  const pages = Math.max(1, Math.ceil((detail?.attachments.length ?? 0) / 50)),
    index = Math.min(page, pages - 1);
  const flag = (value: boolean | null) =>
    value === null ? "Unknown" : value ? "Yes" : "No";
  return (
    <article
      className="image-detail network-detail"
      aria-label="Network details"
      aria-busy={!detail && !error}
    >
      <h3>Network details</h3>
      {error ? (
        <p role="alert">{error}</p>
      ) : !detail ? (
        <p>Reading network metadata…</p>
      ) : (
        <>
          {detail.metadataIncomplete && (
            <p role="status">
              Some optional metadata was malformed and is shown as unknown.
              Refresh to reconcile.
            </p>
          )}
          <dl>
            <dt>Name</dt>
            <dd>{detail.summary.name}</dd>
            <dt>ID</dt>
            <dd>
              <code>{detail.summary.id}</code>
            </dd>
            <dt>Driver</dt>
            <dd>{detail.summary.driver ?? "Unknown"}</dd>
            <dt>Scope</dt>
            <dd>{detail.summary.networkScope ?? "Unknown"}</dd>
            <dt>Internal</dt>
            <dd>{flag(detail.summary.internal)}</dd>
            <dt>IPv6 enabled</dt>
            <dd>{flag(detail.summary.ipv6)}</dd>
            <dt>Created</dt>
            <dd>{detail.createdAt ?? "Unknown"}</dd>
            <dt>IPAM driver</dt>
            <dd>{detail.ipamDriver ?? "Not reported"}</dd>
          </dl>
          <h4>IPAM configuration</h4>
          {!detail.ipamConfig.length ? (
            <p>No IPAM configuration reported.</p>
          ) : (
            detail.ipamConfig.map((config, i) => (
              <dl key={JSON.stringify([config, i])}>
                <dt>Subnet</dt>
                <dd>{config.subnet ?? "Not reported"}</dd>
                <dt>IP range</dt>
                <dd>{config.ipRange ?? "Not reported"}</dd>
                <dt>Gateway</dt>
                <dd>{config.gateway ?? "Not reported"}</dd>
                {config.auxiliaryAddresses.map((row) => (
                  <div key={row.name}>
                    <dt>{row.name}</dt>
                    <dd>{row.address ?? "Unknown address"}</dd>
                  </div>
                ))}
              </dl>
            ))
          )}
          {[
            { title: "Labels", values: detail.labels },
            { title: "Network options", values: detail.options },
            { title: "IPAM options", values: detail.ipamOptions },
          ].map((group) => (
            <div key={group.title}>
              <h4>{group.title} · values masked</h4>
              {group.values.length ? (
                <ul>
                  {group.values.map((row) => (
                    <li key={row.name}>{row.name}: Masked</li>
                  ))}
                </ul>
              ) : (
                <p>None reported.</p>
              )}
            </div>
          ))}
          <h4>Container attachments</h4>
          <p>
            {detail.attachmentsReported
              ? `${detail.attachments.length} endpoint(s) reported in this snapshot.`
              : "Attachments were not reported by this driver."}{" "}
            Refresh to reconcile stale or deleted endpoints.
          </p>
          <ul>
            {detail.attachments
              .slice(index * 50, index * 50 + 50)
              .map((row) => {
                const known =
                  !!row.containerId &&
                  view.rows.some(
                    (container) => container.id === row.containerId,
                  );
                return (
                  <li key={row.endpointKey}>
                    <button
                      className="button"
                      type="button"
                      disabled={
                        stale ||
                        view.stale ||
                        !sameScope(scope, view.scope) ||
                        !known
                      }
                      onClick={() => {
                        if (row.containerId)
                          openContainer(scope, row.containerId);
                      }}
                    >
                      {row.name ?? "Unnamed endpoint"}
                    </button>
                    <code>{row.endpointKey}</code>
                    {!known && (
                      <span>
                        Not in the current container snapshot: deleted, stale or
                        non-container endpoint.
                      </span>
                    )}
                    <span>Endpoint ID: {row.endpointId ?? "Not reported"}</span>
                    <span>IPv4: {row.ipv4Address ?? "Not reported"}</span>
                    <span>IPv6: {row.ipv6Address ?? "Not reported"}</span>
                  </li>
                );
              })}
          </ul>
          {pages > 1 && (
            <div className="image-pages">
              <button
                type="button"
                disabled={index === 0}
                onClick={() => setPage(index - 1)}
              >
                Previous endpoints
              </button>
              <span>
                {index + 1} / {pages}
              </span>
              <button
                type="button"
                disabled={index === pages - 1}
                onClick={() => setPage(index + 1)}
              >
                Next endpoints
              </button>
            </div>
          )}
        </>
      )}
    </article>
  );
}
