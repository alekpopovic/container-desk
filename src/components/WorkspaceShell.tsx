import { useEffect, useState, type ReactNode } from "react";
import type {
  ContainerSummary,
  DemoScenario,
  ContainerPort,
  SavedHost,
} from "../lib/ipc/generated";
import { useWorkspaceShortcuts } from "./useWorkspaceShortcuts";
import { VersionInfo } from "./VersionInfo";

// Presentation-only inputs; backend identity/authorization is enforced separately.
export interface DisplayHost {
  name: string;
  alias: string;
  endpoint: string;
}
export type WorkspaceState =
  | { kind: "empty" }
  | { kind: "loading" | "offline" | "connected"; host: DisplayHost }
  | { kind: "error"; host: DisplayHost; message: string }
  | { kind: "ready"; host: DisplayHost; containers: ContainerSummary[] };

type Theme = "system" | "light" | "dark";
const resources = {
  containers: {
    title: "Containers",
    singular: "container",
    columns: ["Name", "Image", "State", "Ports"],
    description: "Inspect workloads, logs and resource usage.",
  },
  compose: {
    title: "Compose",
    singular: "project",
    columns: ["Project", "Services", "State", "Location"],
    description: "Explore existing Compose projects and services.",
  },
  images: {
    title: "Images",
    singular: "image",
    columns: ["Repository", "Tag", "Image ID", "Size"],
    description: "Browse images available on your Docker host.",
  },
  volumes: {
    title: "Volumes",
    singular: "volume",
    columns: ["Name", "Driver", "Scope", "Mounts"],
    description: "Inspect storage and container mount relationships.",
  },
  networks: {
    title: "Networks",
    singular: "network",
    columns: ["Name", "Driver", "Scope", "Containers"],
    description: "Explore networks and their attached containers.",
  },
} as const;
type Resource = keyof typeof resources;
type Route = Resource | "settings" | "hosts";
const routes: Route[] = [
  "hosts",
  ...(Object.keys(resources) as Resource[]),
  "settings",
];
function readRoute(): Route {
  const route = window.location.hash.replace(/^#\/?/, "");
  return routes.find((item) => item === route) ?? "containers";
}

export function HostIdentity({ state }: { state: WorkspaceState }) {
  return (
    <div className="host-identity">
      <span className="eyebrow">Host identity</span>
      <strong>
        {state.kind === "empty" ? "No host selected" : state.host.name}
      </strong>
      <dl>
        <div>
          <dt>SSH alias</dt>
          <dd>{state.kind === "empty" ? "Not selected" : state.host.alias}</dd>
        </div>
        <div>
          <dt>Docker endpoint</dt>
          <dd>
            {state.kind === "empty" ? "Not connected" : state.host.endpoint}
          </dd>
        </div>
      </dl>
    </div>
  );
}

const connectionLabels = {
  empty: "Not connected",
  loading: "Connecting",
  offline: "Offline",
  error: "Connection error",
  ready: "Data available",
  connected: "Connected",
};

export interface WorkspacePreferences {
  theme: Theme;
  disabled: boolean;
  onThemeChange: (theme: Theme) => void;
  message: string | null;
  error: boolean;
}
export interface DemoControls {
  active: boolean;
  busy: boolean;
  scenario: DemoScenario;
  message: string | null;
  onEnter: () => void;
  onExit: () => void;
  onScenario: (scenario: DemoScenario) => void;
}
const scenarioNames: Record<DemoScenario, string> = {
  standard: "Container states",
  empty: "Empty inventory",
  permission_failure: "Permission failure",
  invalid_json: "Invalid JSON",
  huge_record: "Oversized record",
  disconnect: "Disconnected",
  timeout: "Command timeout",
};

export function WorkspaceShell({
  state,
  connectionNotice,
  preferences,
  settingsExtra,
  hostsExtra,
  containersExtra,
  composeExtra,
  imagesExtra,
  volumesExtra,
  networksExtra,
  savedHosts = [],
  demo,
}: {
  state: WorkspaceState;
  connectionNotice?: string | null;
  preferences?: WorkspacePreferences | undefined;
  settingsExtra?: ReactNode;
  hostsExtra?: ReactNode;
  containersExtra?: ReactNode;
  composeExtra?: ReactNode;
  imagesExtra?: ReactNode;
  volumesExtra?: ReactNode;
  networksExtra?: ReactNode;
  savedHosts?: SavedHost[];
  demo?: DemoControls;
}) {
  const [route, setRoute] = useState<Route>(readRoute);
  useWorkspaceShortcuts();
  const [chosenGroup, setGroup] = useState("All hosts");
  const groups = [
    ...new Set([
      "All hosts",
      "Ungrouped",
      ...savedHosts.map((host) => host.group || "Ungrouped"),
    ]),
  ];
  const group = groups.includes(chosenGroup) ? chosenGroup : "All hosts";
  const [previewTheme, setPreviewTheme] = useState<Theme>("system");
  const theme = preferences?.theme ?? previewTheme;
  const setTheme = preferences?.onThemeChange ?? setPreviewTheme;
  useEffect(() => {
    const navigate = () => setRoute(readRoute());
    window.addEventListener("hashchange", navigate);
    return () => window.removeEventListener("hashchange", navigate);
  }, []);
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    return () => {
      delete document.documentElement.dataset.theme;
    };
  }, [theme]);
  const title =
    route === "settings"
      ? "Settings"
      : route === "hosts"
        ? "Hosts"
        : resources[route].title;
  return (
    <div className="workspace-shell">
      {demo?.active && (
        <div className="demo-badge" role="status" aria-label="Demo mode">
          DEMO · Synthetic data · No SSH
        </div>
      )}
      <button
        type="button"
        className="skip-link"
        onClick={() => document.getElementById("main-content")?.focus()}
      >
        Skip to content
      </button>
      <aside className="sidebar" aria-label="Hosts sidebar">
        <a
          className="brand"
          href="#/containers"
          aria-label="ContainerDesk home"
        >
          <span className="brand-mark" aria-hidden="true">
            ▥
          </span>
          <span>
            Container<span className="brand-suffix">Desk</span>
          </span>
        </a>
        <div className="sidebar-section">
          <p className="eyebrow">Workspace</p>
          <h2>Hosts</h2>
          <nav className="host-groups" aria-label="Host groups">
            {groups.map((name) => (
              <button
                type="button"
                key={name}
                aria-pressed={group === name}
                onClick={() => setGroup(name)}
              >
                <span>{name}</span>
                <span className="count">
                  {
                    savedHosts.filter(
                      (host) =>
                        name === "All hosts" ||
                        (host.group || "Ungrouped") === name,
                    ).length
                  }
                </span>
              </button>
            ))}
          </nav>
          <div className="sidebar-empty">
            <span className="eyebrow">{group}</span>
            <p>
              {savedHosts.length
                ? `${savedHosts.length} saved hosts`
                : demo?.active
                  ? "Demo Linux host"
                  : "No hosts added"}
            </p>
            {savedHosts
              .filter(
                (host) =>
                  group === "All hosts" ||
                  (host.group || "Ungrouped") === group,
              )
              .slice(0, 20)
              .map((host) => (
                <p key={host.id}>
                  {host.favorite ? "★ " : ""}
                  {host.displayName} · {host.alias}
                </p>
              ))}
            <p className="muted">
              {demo?.active
                ? "Synthetic host · demo-local"
                : "Your saved SSH hosts will appear here."}
            </p>
          </div>
          <button
            className="button"
            type="button"
            onClick={() => {
              window.location.hash = "/hosts";
            }}
            aria-describedby="host-setup-note"
          >
            Add host
          </button>
          <p id="host-setup-note" className="supporting-text">
            Manage saved aliases, groups and explicit connections.
          </p>
          {demo && (
            <div className="demo-controls">
              {demo.active ? (
                <>
                  <label htmlFor="demo-scenario">Demo scenario</label>
                  <select
                    id="demo-scenario"
                    value={demo.scenario}
                    disabled={demo.busy}
                    onChange={(event) =>
                      demo.onScenario(event.target.value as DemoScenario)
                    }
                  >
                    {Object.entries(scenarioNames).map(([value, label]) => (
                      <option key={value} value={value}>
                        {label}
                      </option>
                    ))}
                  </select>
                  <button
                    className="button"
                    type="button"
                    disabled={demo.busy}
                    onClick={demo.onExit}
                  >
                    Exit demo
                  </button>
                </>
              ) : (
                <button
                  className="button"
                  type="button"
                  disabled={demo.busy}
                  onClick={demo.onEnter}
                >
                  Open demo
                </button>
              )}
              {demo.message && <p role="alert">{demo.message}</p>}
            </div>
          )}
        </div>
        <footer className="sidebar-footer">
          <span className="eyebrow">Local workspace</span>
          <VersionInfo />
        </footer>
      </aside>
      <div className="workspace-body">
        <header className="host-header">
          <div className="host-heading">
            <p className="eyebrow">
              {group} <span aria-hidden="true">/</span> Host workspace
            </p>
            <h1>
              {state.kind === "empty" ? "No host selected" : state.host.name}
            </h1>
          </div>
          <div className="connection-summary">
            <span className="status-badge" data-state={state.kind}>
              <span className="status-dot" aria-hidden="true" />
              {connectionLabels[state.kind]}
            </span>
          </div>
        </header>
        <nav className="resource-nav" aria-label="Resources">
          {routes.map((item) => (
            <a
              key={item}
              href={`#/${item}`}
              aria-current={route === item ? "page" : undefined}
            >
              {item === "settings"
                ? "Settings"
                : item === "hosts"
                  ? "Hosts"
                  : resources[item].title}
            </a>
          ))}
        </nav>
        <main id="main-content" tabIndex={-1} aria-labelledby="route-title">
          {preferences?.message && (
            <p
              className="storage-notice"
              role={preferences.error ? "alert" : "status"}
            >
              {preferences.message}
            </p>
          )}
          <div className="page-heading">
            <div>
              <p className="eyebrow">
                {route === "settings" ? "Preferences" : "Host resources"}
              </p>
              <h2 id="route-title">{title}</h2>
              <p className="muted">
                {route === "settings"
                  ? "Make this workspace comfortable for you."
                  : route === "hosts"
                    ? "Organize trusted SSH aliases and review their connection identity."
                    : resources[route].description}
              </p>
            </div>
            <span className="context-label">
              {state.kind === "empty"
                ? "Choose a host to begin"
                : state.host.alias}
            </span>
          </div>
          {connectionNotice && (
            <p className="connection-recovery" role="status">
              {connectionNotice}
            </p>
          )}
          <div hidden={route !== "hosts"}>{hostsExtra}</div>
          {route === "hosts" ? null : route === "settings" ? (
            <section
              className="settings-panel"
              aria-label="Appearance settings"
            >
              <div>
                <h3>Appearance</h3>
                <p className="muted">
                  {preferences
                    ? "Your preference is saved on this device."
                    : "Choose a theme for this window."}
                </p>
              </div>
              <p className="keyboard-help">
                Keyboard: Ctrl (Linux) / Cmd (macOS) + F searches the current
                resource view; + R refreshes the current resource view; + Shift
                + H focuses saved hosts; + Shift + L focuses the Logs tab.
                Shortcuts pause inside terminal controls and dialogs. Tab moves
                through controls; Escape cancels a confirmation before dispatch.
              </p>
              <fieldset>
                <legend>Theme</legend>
                <div className="theme-options">
                  {(["system", "light", "dark"] as const).map((choice) => (
                    <label key={choice}>
                      <input
                        type="radio"
                        name="theme"
                        value={choice}
                        disabled={preferences?.disabled}
                        checked={theme === choice}
                        onChange={() => setTheme(choice)}
                      />
                      <span>
                        {choice === "system"
                          ? "Use system"
                          : choice === "light"
                            ? "Light"
                            : "Dark"}
                      </span>
                    </label>
                  ))}
                </div>
              </fieldset>
              {settingsExtra}
              <div className="settings-note">
                <h3>Connections & permissions</h3>
                <p className="muted">
                  Manage aliases and connection status in Hosts. Sessions remain
                  read-only; management and terminal actions are not available
                  yet.
                </p>
              </div>
            </section>
          ) : route === "containers" && containersExtra ? (
            containersExtra
          ) : route === "compose" && composeExtra ? (
            composeExtra
          ) : route === "images" && imagesExtra ? (
            imagesExtra
          ) : route === "networks" && networksExtra ? (
            networksExtra
          ) : route === "volumes" && volumesExtra ? (
            volumesExtra
          ) : (
            <ResourceWorkspace route={route} state={state} />
          )}
        </main>
        <footer className="workspace-footer">
          <span>
            <span className="status-dot" aria-hidden="true" />
            {connectionLabels[state.kind]}
          </span>
          <span>
            {demo?.active
              ? "DEMO — no SSH connections"
              : "Explicit SSH connections · read-only"}
          </span>
        </footer>
      </div>
    </div>
  );
}

function ResourceWorkspace({
  route,
  state,
}: {
  route: Resource;
  state: WorkspaceState;
}) {
  const resource = resources[route];
  const messages = {
    empty: {
      title: "Select a host to get started",
      body: `Choose an SSH alias in Hosts. No ${resource.title.toLowerCase()} snapshot is loaded.`,
    },
    connected: {
      title: "Host connection verified",
      body: "No resource snapshot has been loaded for this session.",
    },
    loading: {
      title: "Connecting to host",
      body: "Waiting for a connection. Resource data is not available yet.",
    },
    offline: {
      title: "Host is offline",
      body: "The host is disconnected. No current resource data is available.",
    },
    error: {
      title: "Could not load host resources",
      body: state.kind === "error" ? state.message : "",
    },
    ready: {
      title:
        route === "containers"
          ? "No containers in this snapshot"
          : `${resource.title} are not available yet`,
      body:
        route === "containers"
          ? "The selected inventory is empty."
          : "This view has no implemented data adapter yet.",
    },
  };
  const message = messages[state.kind];
  const rows =
    state.kind === "ready" && route === "containers" ? state.containers : [];
  return (
    <div className="resource-split">
      <section
        className="resource-panel"
        aria-label={`${resource.title} inventory`}
        aria-busy={state.kind === "loading"}
      >
        <div className="panel-heading">
          <h3>{resource.title}</h3>
          <span className="muted">
            {state.kind === "ready" && route === "containers"
              ? `${rows.length} containers`
              : "No live data"}
          </span>
        </div>
        <table>
          <caption className="sr-only">
            {resource.title} on{" "}
            {state.kind === "empty" ? "the selected host" : state.host.name}
          </caption>
          <thead>
            <tr>
              {resource.columns.map((column) => (
                <th key={column} scope="col">
                  {column}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.length > 0 ? (
              rows.map((row) => (
                <tr key={row.id}>
                  <td>
                    <strong>{row.name}</strong>
                    {row.compose && (
                      <small className="container-compose">
                        {row.compose.project}
                        {row.compose.service ? ` / ${row.compose.service}` : ""}
                      </small>
                    )}
                  </td>
                  <td>{row.image}</td>
                  <td>
                    {row.state}
                    {row.health ? ` · ${row.health}` : ""}
                  </td>
                  <td>
                    {row.ports.length
                      ? row.ports.map(formatPort).join(", ")
                      : "No published ports"}
                  </td>
                </tr>
              ))
            ) : (
              <tr>
                <td colSpan={4}>
                  <div
                    className="resource-notice"
                    data-state={state.kind}
                    role={state.kind === "error" ? "alert" : "status"}
                  >
                    <span className="notice-symbol" aria-hidden="true">
                      {state.kind === "error"
                        ? "!"
                        : state.kind === "loading"
                          ? "…"
                          : "▥"}
                    </span>
                    <h3>{message.title}</h3>
                    <p>{message.body}</p>
                  </div>
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </section>
      <aside className="detail-panel" aria-label="Resource details">
        <div className="panel-heading">
          <h3>Details</h3>
          <span className="muted">No selection</span>
        </div>
        <HostIdentity state={state} />
        <div className="detail-empty">
          <h3>No {resource.singular} selected</h3>
          <p className="muted">
            Select a {resource.singular} to inspect its details.
          </p>
        </div>
        <p className="detail-note">
          {route === "images" || route === "volumes" || route === "networks"
            ? "This resource view will be read-only."
            : "Actions require a connected host and explicit management access."}
        </p>
      </aside>
    </div>
  );
}

function formatPort(port: ContainerPort): string {
  const target = `${port.privatePort}/${port.protocol}`;
  if (port.publicPort === null) return target;
  const address = port.hostIp?.includes(":")
    ? `[${port.hostIp}]`
    : (port.hostIp ?? "*");
  return `${address}:${port.publicPort} → ${target}`;
}
