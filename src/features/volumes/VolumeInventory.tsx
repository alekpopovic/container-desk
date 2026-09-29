import { useCallback, useEffect, useRef, useState } from "react";
import {
  IpcError,
  inspectVolume,
  listVolumes,
  sameScope,
} from "../../lib/ipc/client";
import type {
  ListVolumesResponse,
  SessionScope,
  VolumeDetail,
} from "../../lib/ipc/generated";
import type { InventoryView } from "../containers/cache";
const message = (error: unknown) =>
  error instanceof IpcError
    ? error.message
    : "Volume metadata could not be read.";
export function VolumeInventory({
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
      data: ListVolumesResponse;
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
      const data = await listVolumes(scope, () =>
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
  const rows = visible?.data.volumes ?? [],
    filtered = rows.filter((row) =>
      [row.name, row.driver ?? "", row.volumeScope ?? ""].some((value) =>
        value.toLocaleLowerCase().includes(query.toLocaleLowerCase()),
      ),
    );
  const pages = Math.max(1, Math.ceil(filtered.length / 50)),
    index = Math.min(page, pages - 1),
    chosen = filtered.find((row) => row.name === selected) ?? filtered[0];
  if (!scope || !native)
    return (
      <section className="image-view">
        <h3>No live volume data</h3>
        <p>
          Connect a host to read volume metadata and mount relationships. Demo
          mode has no native volume inventory.
        </p>
      </section>
    );
  return (
    <section className="image-view volume-view" aria-label="Volume inventory">
      <p>
        Read-only volume metadata. Remote directories and volume contents are
        not opened.
      </p>
      <div className="image-tools">
        <label>
          Search volumes
          <input
            type="search"
            value={query}
            maxLength={256}
            onChange={(e) => {
              setQuery(e.target.value);
              setPage(0);
            }}
          />
        </label>
        <button
          className="button"
          type="button"
          disabled={busy}
          onClick={() => {
            refreshContainers();
            void refresh();
          }}
        >
          Refresh volumes
        </button>
      </div>
      <p role={error ? "alert" : "status"}>
        {busy ? "Reading volumes… " : ""}
        {error ? `Stale volume snapshot. ${error} ` : ""}
        {visible ? (
          <>
            Last successful read:{" "}
            <time dateTime={new Date(visible.time).toISOString()}>
              {new Date(visible.time).toLocaleTimeString()}
            </time>
            .
          </>
        ) : (
          "No successful volume snapshot yet."
        )}
      </p>
      {!busy && !error && visible && !rows.length && (
        <p>No volumes on this daemon.</p>
      )}
      <p>{filtered.length} matching volumes</p>
      <div className="image-layout">
        <section className="image-list" aria-label="Available volumes">
          {filtered.slice(index * 50, index * 50 + 50).map((row) => (
            <button
              className="image-choice volume-choice"
              type="button"
              key={row.name}
              aria-pressed={chosen?.name === row.name}
              onClick={() => select(row.name)}
            >
              <strong>{row.name}</strong>
              <span>
                Driver: {row.driver ?? "Unknown"} · Scope:{" "}
                {row.volumeScope ?? "Unknown"}
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
                Previous volumes
              </button>
              <span>
                {index + 1} / {pages}
              </span>
              <button
                type="button"
                disabled={index === pages - 1}
                onClick={() => setPage(index + 1)}
              >
                Next volumes
              </button>
            </div>
          )}
        </section>
        {chosen && visible && (
          <Details
            key={JSON.stringify([scope, chosen.name, visible.time])}
            scope={scope}
            name={chosen.name}
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
  name,
  view,
  stale,
  openContainer,
}: {
  scope: SessionScope;
  name: string;
  view: InventoryView;
  stale: boolean;
  openContainer: (scope: SessionScope, id: string) => void;
}) {
  const [detail, setDetail] = useState<VolumeDetail | null>(null),
    [error, setError] = useState<string | null>(null),
    [page, setPage] = useState(0);
  const current = useRef(view.scope);
  current.current = view.scope;
  useEffect(() => {
    let active = true;
    void inspectVolume({ scope, name }, () => (active ? current.current : null))
      .then((detail) => {
        if (active) setDetail(detail);
      })
      .catch((error) => {
        if (active) setError(message(error));
      });
    return () => {
      active = false;
    };
  }, [scope, name]);
  const pages = Math.max(1, Math.ceil((detail?.references.length ?? 0) / 50)),
    index = Math.min(page, pages - 1);
  return (
    <article
      className="image-detail volume-detail"
      aria-label="Volume details"
      aria-busy={!detail && !error}
    >
      <h3>Volume details</h3>
      {error ? (
        <p role="alert">{error}</p>
      ) : !detail ? (
        <p>Reading volume metadata…</p>
      ) : (
        <>
          <dl>
            <dt>Name</dt>
            <dd>{detail.summary.name}</dd>
            <dt>Driver</dt>
            <dd>{detail.summary.driver ?? "Unknown"}</dd>
            <dt>Scope</dt>
            <dd>{detail.summary.volumeScope ?? "Unknown"}</dd>
            <dt>Created</dt>
            <dd>{detail.createdAt ?? "Unknown"}</dd>
            <dt>Reported mountpoint · metadata only</dt>
            <dd>
              <code>{detail.mountpointReported ?? "Not reported"}</code>
            </dd>
          </dl>
          {[
            { title: "Labels", values: detail.labels },
            { title: "Driver options", values: detail.options },
          ].map((group) => (
            <div key={group.title}>
              <h4>{group.title} · values masked</h4>
              {group.values.length ? (
                <ul>
                  {group.values.map((value) => (
                    <li key={value.name}>{value.name}: Masked</li>
                  ))}
                </ul>
              ) : (
                <p>None reported.</p>
              )}
            </div>
          ))}
          <h4>Container mount relationships</h4>
          <p role="status">
            {detail.referenceObservation === "incomplete"
              ? `Incomplete reference snapshot: ${detail.unresolvedContainerIds.length} container(s) were not observed during inspection. Refresh to reconcile.`
              : detail.referenceObservation === "unreferenced"
                ? "No container references were observed in this snapshot."
                : `${detail.references.length} mount reference(s) observed in this snapshot.`}
          </p>
          <p>
            A snapshot without references is not proof that deletion is safe.
            Volume contents and deletion are unavailable.
          </p>
          <ul>
            {detail.references.slice(index * 50, index * 50 + 50).map((row) => (
              <li key={JSON.stringify([row.containerId, row.destination])}>
                <button
                  className="button"
                  type="button"
                  disabled={
                    stale ||
                    view.stale ||
                    !sameScope(scope, view.scope) ||
                    !view.rows.some(
                      (container) => container.id === row.containerId,
                    )
                  }
                  onClick={() => openContainer(scope, row.containerId)}
                >
                  {row.name}
                </button>
                <code>{row.containerId}</code>
                <span>
                  {row.state} · {row.destination ?? "Destination not reported"}{" "}
                  ·{" "}
                  {row.readOnly === null
                    ? "Access mode unknown"
                    : row.readOnly
                      ? "Read-only mount"
                      : "Read/write mount"}
                </span>
              </li>
            ))}
          </ul>
          {pages > 1 && (
            <div className="image-pages">
              <button
                type="button"
                disabled={index === 0}
                onClick={() => setPage(index - 1)}
              >
                Previous mounts
              </button>
              <span>
                {index + 1} / {pages}
              </span>
              <button
                type="button"
                disabled={index === pages - 1}
                onClick={() => setPage(index + 1)}
              >
                Next mounts
              </button>
            </div>
          )}
        </>
      )}
    </article>
  );
}
