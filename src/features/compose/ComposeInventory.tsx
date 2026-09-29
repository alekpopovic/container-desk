import { ComposeManagement } from "./ComposeManagement";
import { useCallback, useEffect, useRef, useState } from "react";
import type { InventoryView } from "../containers/cache";
import type {
  ComposeProject,
  ListComposeResponse,
  SessionScope,
} from "../../lib/ipc/generated";
import { IpcError, listCompose, sameScope } from "../../lib/ipc/client";
function fromRows(view: InventoryView): ComposeProject[] {
  const projects = new Map<string, ComposeProject>();
  for (const row of view.rows) {
    if (!row.compose) continue;
    let p = projects.get(row.compose.project);
    if (!p) {
      p = {
        name: row.compose.project,
        status: null,
        fromPlugin: false,
        fromLabels: true,
        configuration: "unverified",
        configFilesReported: [],
        workingDirectoriesReported: [],
        instances: [],
      };
      projects.set(p.name, p);
    }
    p.instances.push({
      containerId: row.id,
      name: row.name,
      state: row.state,
      service: row.compose.service,
    });
  }
  return [...projects.values()].sort((a, b) => a.name.localeCompare(b.name));
}
export function ComposeInventory({
  view,
  native,
  host,
  refreshContainers,
  openContainer,
}: {
  view: InventoryView;
  native: boolean;
  host?: string | undefined;
  refreshContainers: () => void;
  openContainer: (scope: SessionScope, id: string) => void;
}) {
  const [result, setResult] = useState<{
    data: ListComposeResponse;
    time: number;
  } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(0);
  const [selected, select] = useState<string | null>(null);
  const current = useRef(view.scope);
  current.current = view.scope;
  const serial = useRef(0);
  const alive = useRef(false);
  const scope = view.scope;
  const refresh = useCallback(async () => {
    if (!scope || !native) return;
    const ticket = ++serial.current;
    setBusy(true);
    setError(null);
    try {
      const data = await listCompose(scope, () =>
        alive.current && serial.current === ticket ? current.current : null,
      );
      if (alive.current && ticket === serial.current)
        setResult({ data, time: Date.now() });
    } catch (e) {
      if (alive.current && ticket === serial.current)
        setError(
          e instanceof IpcError
            ? e.message
            : "Compose projects could not be read.",
        );
    } finally {
      if (alive.current && ticket === serial.current) setBusy(false);
    }
  }, [native, scope]);
  useEffect(() => {
    alive.current = true;
    if (view.updatedAt !== null) void refresh();
    return () => {
      alive.current = false;
      serial.current++;
    };
  }, [refresh, view.updatedAt]);
  const visible =
    scope && result && sameScope(scope, result.data.scope) ? result : null;
  const projects = native ? (visible?.data.projects ?? []) : fromRows(view);
  const filtered = projects.filter((p) =>
    p.name.toLocaleLowerCase().includes(search.toLocaleLowerCase()),
  );
  const pages = Math.max(1, Math.ceil(filtered.length / 50));
  const pageIndex = Math.min(page, pages - 1);
  const chosen = projects.find((p) => p.name === selected) ?? filtered[0];
  if (!scope)
    return (
      <section className="compose-empty">
        <h3>No Compose data</h3>
        <p>
          Connect a host to discover its projects. A disconnected host is not an
          empty daemon.
        </p>
      </section>
    );
  return (
    <section className="compose-view" aria-label="Compose project inventory">
      <div className="compose-tools">
        <label>
          Search projects
          <input
            type="search"
            maxLength={256}
            value={search}
            onChange={(e) => {
              setSearch(e.target.value);
              setPage(0);
            }}
          />
        </label>
        <button
          type="button"
          className="button"
          disabled={busy || view.loading}
          onClick={() => {
            refreshContainers();
            void refresh();
          }}
        >
          Refresh projects
        </button>
      </div>
      <p role={error ? "alert" : "status"}>
        {busy ? "Reading projects… " : ""}
        {error ? `Stale project snapshot. ${error} ` : ""}
        {visible ? (
          <>
            Last successful read:{" "}
            <time dateTime={new Date(visible.time).toISOString()}>
              {new Date(visible.time).toLocaleTimeString()}
            </time>
            .
          </>
        ) : native ? (
          "No successful project snapshot yet."
        ) : (
          "DEMO · projects grouped from synthetic container labels."
        )}
      </p>
      {view.stale && (
        <p role="status">
          Container data is stale. Refresh projects before opening an instance.
        </p>
      )}
      {native && visible && (
        <p>
          {visible.data.plugin === "available"
            ? "Remote Compose plugin available."
            : visible.data.plugin === "absent"
              ? "Remote Compose plugin absent; grouping uses container labels."
              : "Remote Compose plugin availability is unknown; grouping uses container labels."}
          {visible.data.listingError
            ? " Compose project listing failed; inspected container labels supply the available groups."
            : ""}
        </p>
      )}
      <p className="muted">
        Configuration paths are reported remote metadata. They have not been
        checked for existence, readability or completeness. Project actions
        require separate confirmation and verification of remote configuration.
      </p>
      {!busy && !error && !projects.length && (
        <p>No Compose projects discovered on this daemon.</p>
      )}
      <div className="compose-layout">
        <aside aria-label="Compose projects">
          <p>{filtered.length} matching projects</p>
          {filtered.slice(pageIndex * 50, pageIndex * 50 + 50).map((p) => (
            <button
              type="button"
              className="compose-project-choice"
              key={p.name}
              aria-pressed={chosen?.name === p.name}
              onClick={() => select(p.name)}
            >
              {p.name}
              <small>{p.instances.length} container instances</small>
            </button>
          ))}
          {pages > 1 && (
            <div className="compose-pages">
              <button
                type="button"
                disabled={pageIndex === 0}
                onClick={() => setPage(pageIndex - 1)}
              >
                Previous projects
              </button>
              <span>
                {pageIndex + 1} / {pages}
              </span>
              <button
                type="button"
                disabled={pageIndex === pages - 1}
                onClick={() => setPage(pageIndex + 1)}
              >
                Next projects
              </button>
            </div>
          )}
        </aside>
        {chosen && (
          <Project
            key={JSON.stringify([scope, chosen.name])}
            project={chosen}
            actions={
              native && visible?.data.plugin === "available" ? (
                <ComposeManagement
                  scope={scope}
                  project={chosen.name}
                  host={host ?? scope.selection.hostId}
                  stale={view.stale || !!error}
                  refresh={refreshContainers}
                />
              ) : undefined
            }
            view={view}
            openContainer={openContainer}
          />
        )}
      </div>
    </section>
  );
}
function Project({
  project: p,
  actions,
  view,
  openContainer,
}: {
  project: ComposeProject;
  actions?: import("react").ReactNode;
  view: InventoryView;
  openContainer: (scope: SessionScope, id: string) => void;
}) {
  const [page, setPage] = useState(0);
  const pages = Math.max(1, Math.ceil(p.instances.length / 50));
  const index = Math.min(page, pages - 1);
  return (
    <article className="compose-project" aria-label={`Project ${p.name}`}>
      <h3>{p.name}</h3>
      <p>
        {p.fromPlugin ? "Compose listing" : ""}
        {p.fromPlugin && p.fromLabels ? " + " : ""}
        {p.fromLabels ? "Inspected container labels" : ""}
      </p>
      {p.status && <p>Reported project state: {p.status}</p>}
      <h4>Reported remote configuration · unverified</h4>
      {p.configFilesReported.length ? (
        <ul>
          {p.configFilesReported.map((path) => (
            <li key={path}>
              <code>{path}</code>
            </li>
          ))}
        </ul>
      ) : (
        <p>No configuration files reported.</p>
      )}
      {p.workingDirectoriesReported.length > 0 && (
        <>
          <h4>Reported remote working directories · unverified</h4>
          <ul>
            {p.workingDirectoriesReported.map((path) => (
              <li key={path}>
                <code>{path}</code>
              </li>
            ))}
          </ul>
        </>
      )}
      {actions}
      <h4>Services and container instances</h4>
      {p.instances.length === 0 ? (
        <p>No inspected instances in this snapshot.</p>
      ) : (
        <table className="compose-instances">
          <thead>
            <tr>
              <th>Service</th>
              <th>Container</th>
              <th>State</th>
            </tr>
          </thead>
          <tbody>
            {p.instances.slice(index * 50, index * 50 + 50).map((item) => {
              const available =
                !!view.scope &&
                view.rows.some((row) => row.id === item.containerId);
              return (
                <tr key={item.containerId}>
                  <td>{item.service ?? "Unspecified"}</td>
                  <td>
                    <button
                      type="button"
                      className="button"
                      disabled={!available || view.stale}
                      onClick={() => {
                        if (view.scope)
                          openContainer(view.scope, item.containerId);
                      }}
                    >
                      {item.name}
                    </button>
                    <code>{item.containerId}</code>
                    {!available && (
                      <small>Refresh containers to open this instance.</small>
                    )}
                  </td>
                  <td>{item.state}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
      {pages > 1 && (
        <div className="compose-pages">
          <button
            type="button"
            disabled={index === 0}
            onClick={() => setPage(index - 1)}
          >
            Previous instances
          </button>
          <span>
            {index + 1} / {pages}
          </span>
          <button
            type="button"
            disabled={index === pages - 1}
            onClick={() => setPage(index + 1)}
          >
            Next instances
          </button>
        </div>
      )}
    </article>
  );
}
