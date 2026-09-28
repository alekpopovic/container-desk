import { useEffect, useState } from "react";
import { VersionInfo } from "./VersionInfo";

// Presentation-only inputs. Backend host/session models are introduced in 004.
export interface DisplayHost {
  name: string;
  alias: string;
  endpoint: string;
}
export type WorkspaceState =
  | { kind: "empty" }
  | { kind: "loading" | "offline"; host: DisplayHost }
  | { kind: "error"; host: DisplayHost; message: string };

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
type Route = Resource | "settings";
const routes: Route[] = [...(Object.keys(resources) as Resource[]), "settings"];
function readRoute(): Route {
  const route = window.location.hash.replace(/^#\/?/, "");
  return routes.find((item) => item === route) ?? "containers";
}

function HostIdentity({ state }: { state: WorkspaceState }) {
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
};

export function WorkspaceShell({ state }: { state: WorkspaceState }) {
  const [route, setRoute] = useState<Route>(readRoute);
  const [group, setGroup] = useState("All hosts");
  const [theme, setTheme] = useState<Theme>("system");
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
  const title = route === "settings" ? "Settings" : resources[route].title;
  return (
    <div className="workspace-shell">
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
            {["All hosts", "Ungrouped"].map((name) => (
              <button
                type="button"
                key={name}
                aria-pressed={group === name}
                onClick={() => setGroup(name)}
              >
                <span>{name}</span>
                <span className="count">0</span>
              </button>
            ))}
          </nav>
          <div className="sidebar-empty">
            <span className="eyebrow">{group}</span>
            <p>No hosts added</p>
            <p className="muted">Your saved SSH hosts will appear here.</p>
          </div>
          <button
            className="button"
            type="button"
            disabled
            aria-describedby="host-setup-note"
          >
            Add host
          </button>
          <p id="host-setup-note" className="supporting-text">
            Host setup is not available in this build.
          </p>
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
            <span className="mode-label">Management unavailable</span>
          </div>
        </header>
        <nav className="resource-nav" aria-label="Resources">
          {routes.map((item) => (
            <a
              key={item}
              href={`#/${item}`}
              aria-current={route === item ? "page" : undefined}
            >
              {item === "settings" ? "Settings" : resources[item].title}
            </a>
          ))}
        </nav>
        <main id="main-content" tabIndex={-1} aria-labelledby="route-title">
          <div className="page-heading">
            <div>
              <p className="eyebrow">
                {route === "settings" ? "Preferences" : "Host resources"}
              </p>
              <h2 id="route-title">{title}</h2>
              <p className="muted">
                {route === "settings"
                  ? "Make this workspace comfortable for you."
                  : resources[route].description}
              </p>
            </div>
            <span className="context-label">
              {state.kind === "empty"
                ? "Choose a host to begin"
                : state.host.alias}
            </span>
          </div>
          {route === "settings" ? (
            <section
              className="settings-panel"
              aria-label="Appearance settings"
            >
              <div>
                <h3>Appearance</h3>
                <p className="muted">Choose a theme for this window.</p>
              </div>
              <fieldset>
                <legend>Theme</legend>
                <div className="theme-options">
                  {(["system", "light", "dark"] as const).map((choice) => (
                    <label key={choice}>
                      <input
                        type="radio"
                        name="theme"
                        value={choice}
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
              <div className="settings-note">
                <h3>Connections & permissions</h3>
                <p className="muted">
                  Host setup, management permissions and terminal access are not
                  available yet. No server is connected.
                </p>
              </div>
            </section>
          ) : (
            <ResourceWorkspace route={route} state={state} />
          )}
        </main>
        <footer className="workspace-footer">
          <span>
            <span className="status-dot" aria-hidden="true" />
            {connectionLabels[state.kind]}
          </span>
          <span>No remote operations available</span>
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
      body: `${resource.title} will appear here after a host is connected. Host connections are not available in this build.`,
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
  };
  const message = messages[state.kind];
  return (
    <div className="resource-split">
      <section
        className="resource-panel"
        aria-label={`${resource.title} inventory`}
        aria-busy={state.kind === "loading"}
      >
        <div className="panel-heading">
          <h3>{resource.title}</h3>
          <span className="muted">No live data</span>
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
