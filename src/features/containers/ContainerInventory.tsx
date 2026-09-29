import { useRemSize } from "../../components/useRemSize";
import { useEffect, useMemo, useRef, useState } from "react";
import { BatchManagement } from "../management/BatchManagement";
import { sameScope } from "../../lib/ipc/client";
import { ContainerManagement } from "../management/ContainerManagement";
import { ContainerStats } from "../stats/ContainerStats";
import { StatsHistory } from "../stats/sampling";
import { ContainerConsole } from "../terminal/ContainerConsole";
import { ContainerDetails } from "./ContainerDetails";
import type { ContainerSummary } from "../../lib/ipc/generated";
import {
  HostIdentity,
  type DisplayHost,
} from "../../components/WorkspaceShell";
import type { InventoryView } from "./cache";
const columns = ["Name", "State", "Health", "Image", "Ports", "Age"] as const;
type Column = (typeof columns)[number];
const collator = new Intl.Collator(undefined, {
  numeric: true,
  sensitivity: "base",
});
function ports(row: ContainerSummary): string {
  if (row.cli)
    return row.cli.ports === ""
      ? "No ports listed"
      : (row.cli.ports ?? "Unknown");
  return (
    row.ports
      .map((port) => {
        const target = `${port.privatePort}/${port.protocol}`;
        if (port.publicPort === null) return target;
        const ip = port.hostIp?.includes(":")
          ? `[${port.hostIp}]`
          : (port.hostIp ?? "*");
        return `${ip}:${port.publicPort} → ${target}`;
      })
      .join(", ") || "No published ports"
  );
}
function value(row: ContainerSummary, column: Column): string {
  switch (column) {
    case "Name":
      return row.name;
    case "State":
      return row.state;
    case "Health":
      return row.health ?? "Unknown";
    case "Image":
      return row.image;
    case "Ports":
      return ports(row);
    case "Age":
      return row.cli?.createdAt ?? "";
  }
}
function createdAt(row: ContainerSummary): number | null {
  const parts = row.cli?.createdAt?.match(
    /^(\d{4}-\d{2}-\d{2}) (\d{2}:\d{2}:\d{2})(?:\.\d+)? ([+-]\d{2})(\d{2}) \S+$/,
  );
  if (!parts) return null;
  const epoch = Date.parse(`${parts[1]}T${parts[2]}${parts[3]}:${parts[4]}`);
  return Number.isFinite(epoch) ? epoch : null;
}
export function ContainerInventory({
  view,
  host,
  refresh,
  select,
  inspectEnabled = false,
  eventStatus,
}: {
  view: InventoryView;
  host: DisplayHost | null;
  refresh: () => void;
  select: (id: string) => void;
  inspectEnabled?: boolean;
  eventStatus?: string;
}) {
  const [checked, setChecked] = useState<Map<string, ContainerSummary>>(
    () => new Map(),
  );
  const [batchBusy, setBatchBusy] = useState(false);
  const scopeKey = JSON.stringify(view.scope);
  useEffect(() => {
    if (scopeKey) {
      setChecked(new Map());
      setBatchBusy(false);
    }
  }, [scopeKey]);
  const selected = Array.from(checked.values())
    .filter((row) => sameScope(row.scope, view.scope))
    .map((row) => view.rows.find((current) => current.id === row.id) ?? row);
  const [statsHistory] = useState(() => new StatsHistory());
  const [search, setSearch] = useState("");
  const [state, setState] = useState("all");
  const [sort, setSort] = useState<{ column: Column; ascending: boolean }>({
    column: "Name",
    ascending: true,
  });
  const rem = useRemSize();
  const rowHeight = 4 * rem;
  const [paged, setPaged] = useState(false);
  const [page, setPage] = useState(0);
  const [scroll, setScroll] = useState(0);
  const viewport = useRef<HTMLElement>(null);
  const rows = useMemo(() => {
    const query = search.toLocaleLowerCase();
    return view.rows
      .filter(
        (row) =>
          (state === "all" || row.state === state) &&
          [row.name, row.image, row.id, ...(row.cli?.names ?? [])].some(
            (text) => text.toLocaleLowerCase().includes(query),
          ),
      )
      .sort((a, b) => {
        let compared: number;
        if (sort.column === "Age") {
          const left = createdAt(a),
            right = createdAt(b);
          if (left === null || right === null)
            return left === right
              ? a.id.localeCompare(b.id)
              : left === null
                ? 1
                : -1;
          compared = right - left;
        } else
          compared = collator.compare(
            value(a, sort.column),
            value(b, sort.column),
          );
        return (sort.ascending ? 1 : -1) * compared || a.id.localeCompare(b.id);
      });
  }, [view.rows, search, state, sort]);
  const virtual = rows.length > 200;
  const start =
    virtual && paged
      ? Math.min(page * 24, Math.floor((rows.length - 1) / 24) * 24)
      : virtual
        ? Math.max(
            0,
            Math.min(
              rows.length - 1,
              Math.floor(Math.max(0, scroll - (44 / 14) * rem) / rowHeight) - 6,
            ),
          )
        : 0;
  const end = virtual ? Math.min(rows.length, start + 24) : rows.length;
  const visible = rows.slice(start, end);
  const chosen = view.rows.find((row) => row.id === view.selectedId);
  function resetScroll() {
    setPage(0);
    setScroll(0);
    if (viewport.current) viewport.current.scrollTop = 0;
  }
  return (
    <div className="resource-split container-split">
      <section
        className="resource-panel"
        aria-label="Containers inventory"
        aria-busy={view.loading}
      >
        <div className="panel-heading">
          <h3>Containers</h3>
          <span className="muted">
            {view.scope ? `${view.rows.length} containers` : "No live data"}
          </span>
        </div>
        {eventStatus && (
          <p className="event-status" role="status">
            {eventStatus}
          </p>
        )}
        <div className="container-tools">
          <label>
            Search containers
            <input
              data-shortcut="search"
              aria-keyshortcuts="Control+f Meta+f"
              type="search"
              value={search}
              maxLength={256}
              onChange={(event) => {
                setSearch(event.target.value);
                resetScroll();
              }}
              placeholder="Name, image or full ID"
            />
          </label>
          <label>
            Container state
            <select
              value={state}
              onChange={(event) => {
                setState(event.target.value);
                resetScroll();
              }}
            >
              <option value="all">All states</option>
              {[
                ...new Set([
                  ...(state === "all" ? [] : [state]),
                  ...view.rows.map((row) => row.state),
                ]),
              ]
                .sort()
                .map((name) => (
                  <option key={name}>{name}</option>
                ))}
            </select>
          </label>
          <button
            type="button"
            className="button"
            disabled={!view.scope || view.loading}
            data-shortcut="refresh"
            aria-keyshortcuts="Control+r Meta+r"
            onClick={refresh}
          >
            Refresh containers
          </button>
        </div>
        <p className="snapshot-state" role={view.error ? "alert" : "status"}>
          {view.loading ? "Refreshing… " : ""}
          {view.error ? `${view.error} ` : ""}
          {view.stale ? "Stale snapshot. " : ""}
          {view.updatedAt !== null ? (
            <>
              Last successful read:{" "}
              <time dateTime={new Date(view.updatedAt).toISOString()}>
                {new Date(view.updatedAt).toLocaleTimeString()}
              </time>
              .
            </>
          ) : (
            "No successful snapshot yet."
          )}
        </p>
        {virtual && (
          <div className="table-pages">
            <label>
              <input
                type="checkbox"
                checked={paged}
                onChange={(event) => {
                  setPaged(event.target.checked);
                  resetScroll();
                }}
              />
              Use paged table (24 rows per page)
            </label>
            {paged && (
              <>
                <button
                  type="button"
                  className="button"
                  disabled={start === 0}
                  onClick={() => {
                    setPage(Math.max(0, Math.floor(start / 24) - 1));
                    if (viewport.current) viewport.current.scrollTop = 0;
                  }}
                >
                  Previous containers
                </button>
                <button
                  type="button"
                  className="button"
                  disabled={end === rows.length}
                  onClick={() => {
                    setPage(Math.floor(start / 24) + 1);
                    if (viewport.current) viewport.current.scrollTop = 0;
                  }}
                >
                  Next containers
                </button>
                <span role="status">
                  Rows {start + 1}–{end} of {rows.length}
                </span>
              </>
            )}
          </div>
        )}
        <section
          // biome-ignore lint/a11y/noNoninteractiveTabindex: A scroll region must support keyboard scrolling.
          tabIndex={0}
          aria-label="Scrollable container table"
          className="container-scroll"
          ref={viewport}
          onScroll={(event) => setScroll(event.currentTarget.scrollTop)}
        >
          <table className="container-table" aria-rowcount={rows.length + 1}>
            <caption className="sr-only">
              Containers on the selected host
            </caption>
            <thead>
              <tr>
                {columns.map((column) => (
                  <th
                    key={column}
                    scope="col"
                    aria-sort={
                      sort.column === column
                        ? sort.ascending
                          ? "ascending"
                          : "descending"
                        : "none"
                    }
                  >
                    <button
                      type="button"
                      onClick={() => {
                        setSort({
                          column,
                          ascending: sort.column !== column || !sort.ascending,
                        });
                        resetScroll();
                      }}
                    >
                      {column}
                      {sort.column === column
                        ? sort.ascending
                          ? " ↑"
                          : " ↓"
                        : ""}
                    </button>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {!paged && start > 0 && (
                <tr aria-hidden="true" tabIndex={-1} className="table-spacer">
                  <td colSpan={6} style={{ height: start * rowHeight }} />
                </tr>
              )}
              {visible.map((row, offset) => (
                <tr
                  key={row.id}
                  data-container-id={row.id}
                  data-selected={row.id === view.selectedId}
                  aria-rowindex={start + offset + 2}
                >
                  <td>
                    <div className="container-name-controls">
                      {inspectEnabled && (
                        <input
                          type="checkbox"
                          aria-label={`Select ${row.name} (${row.id.slice(0, 12)}) for batch`}
                          checked={selected.some((item) => item.id === row.id)}
                          disabled={
                            batchBusy ||
                            (!checked.has(row.id) && selected.length >= 20)
                          }
                          onChange={(event) => {
                            const include = event.target.checked;
                            setChecked((previous) => {
                              const next = new Map(previous);
                              if (include && next.size < 20)
                                next.set(row.id, row);
                              else if (!include) next.delete(row.id);
                              return next;
                            });
                          }}
                        />
                      )}
                      <button
                        className="container-select"
                        type="button"
                        aria-pressed={row.id === view.selectedId}
                        title={row.name}
                        onClick={() => select(row.id)}
                      >
                        {row.name}
                      </button>
                    </div>
                    {row.compose && (
                      <small className="container-compose">
                        {row.compose.project}
                        {row.compose.service ? ` / ${row.compose.service}` : ""}
                      </small>
                    )}
                  </td>
                  <td title={row.status}>{row.state}</td>
                  <td>{row.health ?? "Unknown"}</td>
                  <td title={row.image}>{row.image}</td>
                  <td title={ports(row)}>{ports(row)}</td>
                  <td title={row.cli?.createdAt ?? undefined}>
                    {row.cli?.runningFor ?? "Unknown"}
                  </td>
                </tr>
              ))}
              {!paged && end < rows.length && (
                <tr aria-hidden="true" tabIndex={-1} className="table-spacer">
                  <td
                    colSpan={6}
                    style={{ height: (rows.length - end) * rowHeight }}
                  />
                </tr>
              )}
              {rows.length === 0 && (
                <tr>
                  <td colSpan={6}>
                    <div
                      className="resource-notice"
                      data-state={
                        view.error
                          ? "error"
                          : view.loading
                            ? "loading"
                            : "empty"
                      }
                    >
                      <h3>
                        {!view.scope
                          ? "Select a host to get started"
                          : view.loading
                            ? "Loading containers"
                            : view.error
                              ? "Container data unavailable"
                              : view.stale
                                ? "Cached inventory is empty"
                                : view.rows.length
                                  ? "No matching containers"
                                  : "No containers in this snapshot"}
                      </h3>
                      <p>
                        {!view.scope
                          ? "Connect a saved host in Hosts to load its containers."
                          : view.rows.length
                            ? "Change the search or state filter."
                            : "Only a successful read confirms an empty inventory."}
                      </p>
                    </div>
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </section>
        <p className="muted table-count">
          {rows.length} matching · {view.rows.length} total
          {virtual ? ` · rows ${start + 1}–${end} visible` : ""}
        </p>
      </section>
      <aside className="detail-panel" aria-label="Resource details">
        <div className="panel-heading">
          <h3>Details</h3>
          <span className="muted">
            {chosen ? "Selected container" : "No selection"}
          </span>
        </div>
        <HostIdentity
          state={host ? { kind: "connected", host } : { kind: "empty" }}
        />
        {chosen ? (
          <>
            <h3>{chosen.name}</h3>
            <dl className="selected-container">
              <dt>Full container ID</dt>
              <dd>
                <code>{chosen.id}</code>
              </dd>
              <dt>Image</dt>
              <dd>{chosen.image}</dd>
              <dt>State</dt>
              <dd>{chosen.state}</dd>
              <dt>Status</dt>
              <dd>{chosen.status || "Unknown"}</dd>
              <dt>Ports</dt>
              <dd>{ports(chosen)}</dd>
              <dt>Health</dt>
              <dd>{chosen.health ?? "Unknown"}</dd>
            </dl>
            {inspectEnabled && view.scope && selected.length === 0 && (
              <ContainerManagement
                key={JSON.stringify([view.scope, chosen.id])}
                scope={view.scope}
                row={chosen}
                host={host?.name ?? "Selected host"}
                stale={view.stale && !view.loading}
                refresh={refresh}
              />
            )}
            {inspectEnabled && (!view.stale || view.loading) && view.scope && (
              <ContainerDetails
                key={JSON.stringify([view.scope, chosen.id, view.updatedAt])}
                scope={view.scope}
                id={chosen.id}
              />
            )}
            {view.stale && (
              <p>These details are from the last successful snapshot.</p>
            )}
          </>
        ) : (
          <div className="detail-empty">
            <h3>No container selected</h3>
            <p>Select a container to view its summary.</p>
          </div>
        )}
      </aside>
      {inspectEnabled && view.scope && selected.length > 0 && (
        <BatchManagement
          key={scopeKey}
          scope={view.scope}
          rows={selected}
          host={host?.name ?? "Selected host"}
          stale={view.stale && !view.loading}
          refresh={refresh}
          clear={() => setChecked(new Map())}
          onBusy={setBatchBusy}
        />
      )}
      {chosen &&
        inspectEnabled &&
        (!view.stale || view.loading) &&
        view.scope && (
          <ContainerStats
            key={JSON.stringify(["stats", view.scope, chosen.id])}
            scope={view.scope}
            id={chosen.id}
            history={statsHistory}
          />
        )}
      {chosen &&
        inspectEnabled &&
        (!view.stale || view.loading) &&
        view.scope && (
          <ContainerConsole
            key={JSON.stringify(["console", view.scope, chosen.id])}
            scope={view.scope}
            id={chosen.id}
            name={chosen.name}
            host={host ? `${host.name} · ${host.alias}` : "Selected host"}
          />
        )}
    </div>
  );
}
