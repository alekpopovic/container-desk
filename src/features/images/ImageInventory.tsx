import { useCallback, useEffect, useRef, useState } from "react";
import {
  IpcError,
  inspectImage,
  listImages,
  sameScope,
} from "../../lib/ipc/client";
import type {
  ImageDetail,
  ListImagesResponse,
  SessionScope,
} from "../../lib/ipc/generated";
import type { InventoryView } from "../containers/cache";
const message = (error: unknown) =>
  error instanceof IpcError ? error.message : "Image data could not be read.";
export function ImageInventory({
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
    data: ListImagesResponse;
    time: number;
  } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [dangling, setDangling] = useState(false);
  const [query, setQuery] = useState("");
  const [page, setPage] = useState(0);
  const [selected, select] = useState<string | null>(null);
  const alive = useRef(false),
    serial = useRef(0),
    current = useRef(view.scope);
  current.current = view.scope;
  const scope = view.scope;
  const refresh = useCallback(async () => {
    if (!native || !scope) return;
    const ticket = ++serial.current;
    setBusy(true);
    setError(null);
    try {
      const data = await listImages({ scope, danglingOnly: dangling }, () =>
        alive.current && ticket === serial.current ? current.current : null,
      );
      if (alive.current && ticket === serial.current)
        setSnapshot({ data, time: Date.now() });
    } catch (error) {
      if (alive.current && ticket === serial.current) setError(message(error));
    } finally {
      if (alive.current && ticket === serial.current) setBusy(false);
    }
  }, [native, scope, dangling]);
  useEffect(() => {
    alive.current = true;
    void refresh();
    return () => {
      alive.current = false;
      serial.current++;
    };
  }, [refresh]);
  const visible =
    scope &&
    snapshot &&
    sameScope(scope, snapshot.data.scope) &&
    snapshot.data.danglingOnly === dangling
      ? snapshot
      : null;
  const rows = visible?.data.images ?? [];
  const filtered = rows.filter((row) =>
    [row.id, ...row.tags, ...row.digests].some((value) =>
      value.toLocaleLowerCase().includes(query.toLocaleLowerCase()),
    ),
  );
  const pages = Math.max(1, Math.ceil(filtered.length / 50)),
    index = Math.min(page, pages - 1);
  const chosen = filtered.find((row) => row.id === selected) ?? filtered[0];
  if (!scope || !native)
    return (
      <section className="image-view">
        <h3>No live image data</h3>
        <p>
          Connect a host to read its images. Demo mode does not provide a native
          image inventory.
        </p>
      </section>
    );
  return (
    <section className="image-view" aria-label="Image inventory">
      <p>
        Read-only images for the selected daemon. Tags may share one image ID.
      </p>
      <div className="image-tools">
        <label>
          Search images
          <input
            type="search"
            maxLength={256}
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setPage(0);
            }}
          />
        </label>
        <label>
          <input
            type="checkbox"
            checked={dangling}
            onChange={(e) => {
              setDangling(e.target.checked);
              setPage(0);
              select(null);
            }}
          />
          Dangling images only
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
          Refresh images
        </button>
      </div>
      <p role={error ? "alert" : "status"}>
        {busy ? "Reading images… " : ""}
        {error ? `Stale image snapshot. ${error} ` : ""}
        {visible ? (
          <>
            Last successful read:{" "}
            <time dateTime={new Date(visible.time).toISOString()}>
              {new Date(visible.time).toLocaleTimeString()}
            </time>
            .
          </>
        ) : (
          "No successful image snapshot yet."
        )}
      </p>
      {!busy && !error && visible && !rows.length && (
        <p>
          {dangling
            ? "No dangling images on this daemon."
            : "No images on this daemon."}
        </p>
      )}
      <p>{filtered.length} matching image identities</p>
      <div className="image-layout">
        <section className="image-list" aria-label="Available images">
          {filtered.slice(index * 50, index * 50 + 50).map((row) => (
            <button
              className="image-choice"
              type="button"
              key={row.id}
              aria-pressed={row.id === chosen?.id}
              onClick={() => select(row.id)}
            >
              <strong>
                {row.tags.length ? row.tags.join(", ") : "No tags"}
              </strong>
              <code>{row.id}</code>
              <span>
                {row.sizeReported ?? "Size unknown"} ·{" "}
                {row.createdAtReported ?? "Creation time unknown"}
              </span>
              {row.digests.map((digest) => (
                <small key={digest}>{digest}</small>
              ))}
            </button>
          ))}
          {pages > 1 && (
            <div className="image-pages">
              <button
                type="button"
                disabled={index === 0}
                onClick={() => setPage(index - 1)}
              >
                Previous images
              </button>
              <span>
                {index + 1} / {pages}
              </span>
              <button
                type="button"
                disabled={index === pages - 1}
                onClick={() => setPage(index + 1)}
              >
                Next images
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
  const [detail, setDetail] = useState<ImageDetail | null>(null),
    [error, setError] = useState<string | null>(null),
    [page, setPage] = useState(0);
  const current = useRef(view.scope);
  current.current = view.scope;
  useEffect(() => {
    let active = true;
    void inspectImage({ scope, imageId: id }, () =>
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
  const pages = Math.max(1, Math.ceil((detail?.containers.length ?? 0) / 50)),
    index = Math.min(page, pages - 1);
  return (
    <article
      className="image-detail"
      aria-label="Image details"
      aria-busy={!detail && !error}
    >
      <h3>Image details</h3>
      {error ? (
        <p role="alert">{error}</p>
      ) : !detail ? (
        <p>Reading image metadata…</p>
      ) : (
        <>
          <dl>
            <dt>Image ID</dt>
            <dd>
              <code>{detail.id}</code>
            </dd>
            <dt>Tags</dt>
            <dd>{detail.tags.length ? detail.tags.join(", ") : "No tags"}</dd>
            <dt>Digests</dt>
            <dd>
              {detail.digests.length
                ? detail.digests.join(", ")
                : "No repository digests reported"}
            </dd>
            <dt>Size</dt>
            <dd>
              {detail.sizeBytes === null
                ? "Unknown"
                : `${detail.sizeBytes.toLocaleString()} bytes`}
            </dd>
            <dt>Created</dt>
            <dd>{detail.createdAt ?? "Unknown"}</dd>
            <dt>Platform</dt>
            <dd>
              {[detail.os, detail.architecture, detail.variant]
                .filter(Boolean)
                .join(" / ") || "Unknown"}
            </dd>
          </dl>
          <h4>Labels · values masked</h4>
          {detail.labels.length ? (
            <ul>
              {detail.labels.map((label) => (
                <li key={label.name}>
                  {label.name}: <span>Masked</span>
                </li>
              ))}
            </ul>
          ) : (
            <p>No labels reported.</p>
          )}
          <h4>Containers using this image</h4>
          <p>
            References match this exact image ID. Refresh to see later changes.
          </p>
          {!detail.containers.length ? (
            <p>No container references in this snapshot.</p>
          ) : (
            <ul>
              {detail.containers
                .slice(index * 50, index * 50 + 50)
                .map((row) => {
                  const available =
                    !stale &&
                    !view.stale &&
                    sameScope(scope, view.scope) &&
                    view.rows.some(
                      (container) => container.id === row.containerId,
                    );
                  return (
                    <li key={row.containerId}>
                      <button
                        className="button"
                        type="button"
                        disabled={!available}
                        onClick={() => openContainer(scope, row.containerId)}
                      >
                        {row.name}
                      </button>
                      <code>{row.containerId}</code>
                      <span>{row.state}</span>
                      {!available && (
                        <small>
                          Refresh containers before opening this reference.
                        </small>
                      )}
                    </li>
                  );
                })}
            </ul>
          )}
          {pages > 1 && (
            <div className="image-pages">
              <button
                type="button"
                disabled={index === 0}
                onClick={() => setPage(index - 1)}
              >
                Previous references
              </button>
              <span>
                {index + 1} / {pages}
              </span>
              <button
                type="button"
                disabled={index === pages - 1}
                onClick={() => setPage(index + 1)}
              >
                Next references
              </button>
            </div>
          )}
        </>
      )}
    </article>
  );
}
