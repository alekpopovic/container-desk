import { HostRecovery } from "./recovery";
import { watchReadRecovery } from "../../lib/reads/recovery";
import { isTauri } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import * as native from "../../lib/ipc/client";
import * as demoBridge from "../../demo/host-inventory";
import type {
  HostInventory as Inventory,
  SavedHost,
  WorkspaceMode,
  HostDraft,
  HostDiscovery,
} from "../../lib/ipc/generated";
const blank = (): HostDraft => ({
  ssh: { alias: "", configPath: "", useDefaultConfig: false },
  docker: { executable: null, context: null, sudo: false },
  displayName: "",
  group: "dev",
  labels: [],
  favorite: false,
});
export function HostInventory({
  mode,
  onChange,
  onRecovery,
}: {
  mode: WorkspaceMode;
  onChange: (value: Inventory) => void;
  onRecovery?: (message: string | null) => void;
}) {
  const [recovery] = useState(() => new HostRecovery());
  const available = isTauri() || mode === "demo";
  const bridge = isTauri() ? native : demoBridge;
  const [inventory, setInventory] = useState<Inventory | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [draft, setDraft] = useState<HostDraft>(blank);
  const [discovery, setDiscovery] = useState<HostDiscovery | null>(null);
  const [group, setGroup] = useState("All");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const mounted = useRef(false);
  const working = useRef(false);
  const sequence = useRef(0);
  const latest = useRef<Inventory | null>(null);
  function apply(value: Inventory) {
    latest.current = value;
    setInventory(value);
    onChange(value);
  }
  useEffect(() => {
    mounted.current = true;
    let active = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let reading = false;
    const refresh = async () => {
      if (!active || reading) return;
      clearTimeout(timer);
      reading = true;
      if (!working.current && available) {
        const serial = sequence.current;
        try {
          const value = await bridge.getHostInventory(mode);
          if (active && mounted.current && sequence.current === serial) {
            latest.current = value;
            setInventory(value);
            onChange(value);
            const next = recovery.observe(value, Date.now(), !document.hidden);
            onRecovery?.(next.message);
            if (next.connect) {
              working.current = true;
              setBusy(true);
              const attempt = ++sequence.current;
              try {
                const result = await bridge.connectInventoryHost({
                  mode,
                  hostId: next.connect,
                });
                recovery.acceptedAttempt();
                if (active && mounted.current && sequence.current === attempt) {
                  latest.current = result;
                  setInventory(result);
                  onChange(result);
                } else if (result.connection) {
                  // A late connection owns only its returned token, never a later user selection.
                  await bridge.disconnectInventoryHost({
                    mode,
                    hostId: next.connect,
                    token: result.connection.token,
                  });
                }
              } catch (cause) {
                if (
                  cause instanceof native.IpcError &&
                  [
                    "resource_limit",
                    "transport_unavailable",
                    "disconnected",
                    "operation_timed_out",
                  ].includes(cause.code)
                )
                  recovery.failedAttempt();
                else recovery.reset();
                if (active)
                  setError(
                    cause instanceof native.IpcError
                      ? cause.message
                      : "Connection recovery failed.",
                  );
              } finally {
                working.current = false;
                if (active) setBusy(false);
              }
            }
          }
        } catch (cause) {
          if (active && mounted.current && sequence.current === serial)
            setError(
              cause instanceof native.IpcError
                ? cause.message
                : "Host inventory is unavailable.",
            );
        }
      }
      reading = false;
      if (active)
        timer = setTimeout(() => {
          void refresh();
        }, 1000);
    };
    const stopRecovery = watchReadRecovery(() => {
      void refresh();
    });
    void refresh();
    return () => {
      active = false;
      mounted.current = false;
      sequence.current += 1;
      clearTimeout(timer);
      stopRecovery();
      recovery.reset();
      onRecovery?.(null);
    };
  }, [available, bridge, mode, onChange, onRecovery, recovery]);
  async function action(run: () => Promise<Inventory>, chooseSaved = false) {
    if (working.current || !available) return;
    recovery.reset();
    onRecovery?.(null);
    working.current = true;
    setBusy(true);
    setError(null);
    sequence.current += 1;
    try {
      const value = await run();
      if (mounted.current) {
        apply(value);
        if (chooseSaved) {
          const host = value.saved.preferences.hosts.find(
            (h) => h.id === value.saved.preferences.selectedHostId,
          );
          if (host) choose(host);
        } else if (
          selected &&
          !value.saved.preferences.hosts.some((h) => h.id === selected)
        ) {
          setSelected(null);
          setDraft(blank());
        }
      }
    } catch (cause) {
      if (mounted.current)
        setError(
          cause instanceof native.IpcError
            ? cause.message
            : "Host action failed.",
        );
    } finally {
      working.current = false;
      if (mounted.current) setBusy(false);
    }
  }
  function choose(host: SavedHost) {
    setSelected(host.id);
    setDraft({
      ssh: host.ssh ?? {
        alias: host.alias,
        configPath: "",
        useDefaultConfig: false,
      },
      docker: host.docker,
      displayName: host.displayName,
      group: host.group,
      labels: [...host.labels],
      favorite: host.favorite,
    });
    setDiscovery(null);
    setError(null);
  }
  async function browse() {
    if (working.current) return;
    working.current = true;
    setBusy(true);
    setError(null);
    sequence.current += 1;
    try {
      const result =
        mode === "demo"
          ? {
              configPath: "/demo/config",
              candidates: ["demo-direct", "demo-jump"].map((alias) => ({
                alias,
                source: "/demo/config",
                line: 1,
              })),
              warnings: [],
            }
          : await native.discoverSshHosts(draft.ssh.configPath || null);
      if (mounted.current) setDiscovery(result);
    } catch (cause) {
      if (mounted.current)
        setError(
          cause instanceof native.IpcError
            ? cause.message
            : "Host candidates are unavailable.",
        );
    } finally {
      working.current = false;
      if (mounted.current) setBusy(false);
    }
  }
  const hosts = inventory?.saved.preferences.hosts ?? [];
  const chosen = hosts.find((host) => host.id === selected);
  const connection =
    inventory?.connection?.hostId === selected ? inventory.connection : null;
  const filtered = hosts
    .filter(
      (h) =>
        group === "All" ||
        (group === "Favorites"
          ? h.favorite
          : (h.group || "Ungrouped") === group),
    )
    .sort(
      (a, b) =>
        Number(b.favorite) - Number(a.favorite) ||
        a.displayName.localeCompare(b.displayName) ||
        a.id.localeCompare(b.id),
    );
  return (
    <section
      className="host-inventory"
      aria-label="Saved host inventory"
      aria-busy={busy}
    >
      {!available && (
        <p>
          Open the desktop app to manage real SSH hosts, or open demo to try a
          synthetic inventory.
        </p>
      )}
      {mode === "demo" && (
        <p className="demo-badge">
          DEMO inventory · synthetic connections · changes stay in memory
        </p>
      )}
      <p>
        Save references to trusted SSH aliases. Browsing and saving do not
        connect. Remove from app deletes only local metadata; SSH files and
        remote resources are retained.
      </p>
      <label>
        Filter hosts
        <select
          value={group}
          onChange={(event) => setGroup(event.target.value)}
        >
          {[
            ...new Set([
              "All",
              "Favorites",
              "prod",
              "staging",
              "dev",
              ...hosts.map((h) => h.group || "Ungrouped"),
            ]),
          ].map((name) => (
            <option key={name}>{name}</option>
          ))}
        </select>
      </label>
      <ul className="host-cards" aria-label="Saved hosts">
        {filtered.map((host) => (
          <li key={host.id}>
            <button
              type="button"
              className="button"
              aria-pressed={selected === host.id}
              onClick={() => choose(host)}
              disabled={busy}
            >
              {host.favorite ? "★ " : ""}
              {host.displayName} · {host.alias}
            </button>
            <p>
              {host.group || "Ungrouped"} ·{" "}
              {host.readOnly
                ? "Read-only by default"
                : "Management preference (session remains read-only)"}{" "}
              ·{" "}
              {inventory?.connection?.hostId === host.id
                ? native.connectionLabels[inventory.connection.state]
                : "Disconnected"}
            </p>
            <p>{host.labels.join(", ")}</p>
          </li>
        ))}
      </ul>
      {filtered.length === 0 && <p>No saved hosts in this group.</p>}
      <button
        type="button"
        className="button"
        disabled={busy || !available}
        onClick={() => {
          setSelected(null);
          setDraft(blank());
          setDiscovery(null);
          setError(null);
        }}
      >
        New host
      </button>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          void action(async () => {
            const ssh =
              mode === "demo"
                ? {
                    ...draft.ssh,
                    configPath: "/demo/config",
                    useDefaultConfig: false,
                  }
                : draft.ssh.configPath
                  ? draft.ssh
                  : await native.selectSshAlias(null, draft.ssh.alias);
            if (!latest.current)
              throw new native.IpcError("storage_unavailable");
            return bridge.saveHost({
              mode,
              id: selected,
              expectedRevision: latest.current.saved.preferences.revision,
              draft: {
                ...draft,
                displayName: draft.displayName || ssh.alias,
                labels: draft.labels
                  .map((label) => label.trim())
                  .filter(Boolean),
                ssh,
              },
            });
          }, true);
        }}
      >
        <fieldset disabled={busy || !available || !inventory?.saved.writable}>
          <legend>{selected ? "Edit saved host" : "Add a host"}</legend>
          <label>
            Host SSH config path
            <input
              value={draft.ssh.configPath}
              maxLength={4096}
              placeholder="Default SSH config"
              onChange={(e) =>
                setDraft({
                  ...draft,
                  ssh: {
                    ...draft.ssh,
                    configPath: e.target.value,
                    useDefaultConfig: false,
                  },
                })
              }
            />
          </label>
          <button
            type="button"
            className="button"
            onClick={() => {
              void browse();
            }}
          >
            Browse aliases
          </button>
          {discovery && (
            <>
              <ul aria-label="Alias candidates">
                {discovery.candidates.map((candidate) => (
                  <li key={candidate.alias}>
                    <button
                      type="button"
                      className="button"
                      onClick={() =>
                        setDraft({
                          ...draft,
                          ssh: { ...draft.ssh, alias: candidate.alias },
                        })
                      }
                    >
                      Use {candidate.alias}
                    </button>
                  </li>
                ))}
              </ul>
              {discovery.warnings.length > 0 && (
                <p>
                  Discovery returned {discovery.warnings.length} notes; dynamic
                  or unreadable entries may be omitted. Use a concrete alias
                  from your trusted config.
                </p>
              )}
            </>
          )}
          <label>
            Host SSH alias
            <input
              required
              value={draft.ssh.alias}
              maxLength={256}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  ssh: { ...draft.ssh, alias: e.target.value },
                })
              }
            />
          </label>
          <label>
            Display name
            <input
              value={draft.displayName}
              maxLength={256}
              onChange={(e) =>
                setDraft({ ...draft, displayName: e.target.value })
              }
            />
          </label>
          <label>
            Host group
            <input
              list="inventory-groups"
              value={draft.group}
              maxLength={128}
              onChange={(e) => setDraft({ ...draft, group: e.target.value })}
            />
          </label>
          <datalist id="inventory-groups">
            <option value="prod" />
            <option value="staging" />
            <option value="dev" />
          </datalist>
          <label>
            Labels (comma separated)
            <input
              value={draft.labels.join(",")}
              maxLength={4127}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  labels: e.target.value.split(","),
                })
              }
            />
          </label>
          <label className="check-label">
            <input
              type="checkbox"
              checked={draft.favorite}
              onChange={(e) =>
                setDraft({ ...draft, favorite: e.target.checked })
              }
            />
            Favorite host
          </label>
          <label>
            Saved Docker executable
            <input
              value={draft.docker.executable ?? ""}
              maxLength={4096}
              placeholder="Remote PATH: docker"
              onChange={(e) =>
                setDraft({
                  ...draft,
                  docker: {
                    ...draft.docker,
                    executable: e.target.value || null,
                  },
                })
              }
            />
          </label>
          <label>
            Saved Docker context
            <input
              value={draft.docker.context ?? ""}
              maxLength={256}
              placeholder="Remote user's current context"
              onChange={(e) =>
                setDraft({
                  ...draft,
                  docker: { ...draft.docker, context: e.target.value || null },
                })
              }
            />
          </label>
          <label className="check-label">
            <input
              type="checkbox"
              checked={draft.docker.sudo}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  docker: { ...draft.docker, sudo: e.target.checked },
                })
              }
            />
            Use existing sudo -n Docker access
          </label>
          <p className="muted">
            Connecting evaluates trusted SSH configuration and may run its Match
            exec or ProxyCommand directives. New hosts start read-only. Sudo
            requires existing noninteractive policy; no passwords are collected.
          </p>
          <button
            type="submit"
            className="button"
            disabled={!native.isConcreteAlias(draft.ssh.alias)}
          >
            {selected ? "Save host changes" : "Save host"}
          </button>
        </fieldset>
      </form>
      {chosen && (
        <section
          aria-label="Selected saved host"
          className="saved-host-details"
        >
          <h3>
            {chosen.displayName} · {chosen.alias}
          </h3>
          <p>
            Stable host ID: <code>{chosen.id}</code>
          </p>
          <p role="status">
            {connection
              ? native.connectionLabels[connection.state]
              : "Disconnected"}{" "}
            · SSH session
          </p>
          <div className="host-actions">
            <button
              type="button"
              className="button"
              disabled={
                busy ||
                connection?.state === "ready" ||
                (connection !== null &&
                  ["resolving", "connecting", "probing"].includes(
                    connection.state,
                  ))
              }
              onClick={() => {
                void action(() =>
                  bridge.connectInventoryHost({ mode, hostId: chosen.id }),
                );
              }}
            >
              {connection && ["error", "degraded"].includes(connection.state)
                ? "Retry connection"
                : "Connect saved host"}
            </button>
            <button
              type="button"
              className="button"
              disabled={
                busy || !connection || connection.state === "disconnected"
              }
              onClick={() => {
                if (connection)
                  void action(() =>
                    bridge.disconnectInventoryHost({
                      mode,
                      hostId: chosen.id,
                      token: connection.token,
                    }),
                  );
              }}
            >
              Disconnect saved host
            </button>
            <button
              type="button"
              className="button"
              disabled={busy || !inventory?.saved.writable}
              onClick={() => {
                if (inventory)
                  void action(() =>
                    bridge.removeHost({
                      mode,
                      expectedRevision: inventory.saved.preferences.revision,
                      hostId: chosen.id,
                    }),
                  );
              }}
            >
              Remove from app
            </button>
          </div>
          <dl>
            <dt>Effective destination</dt>
            <dd>
              {connection?.effective
                ? `${connection.effective.user}@${connection.effective.hostname}:${connection.effective.port}`
                : "Not resolved in this session"}
            </dd>
            <dt>Jump route</dt>
            <dd>
              {connection?.effective
                ? (connection.effective.proxyJump ??
                  (connection.effective.hasProxyCommand
                    ? "Configured proxy (details hidden)"
                    : "Direct"))
                : "Not resolved"}
            </dd>
            <dt>Docker endpoint</dt>
            <dd>
              {connection?.docker?.endpoint ?? "Not verified in this session"}
            </dd>
            <dt>Docker Engine version</dt>
            <dd>{connection?.docker?.serverVersion ?? "Not verified"}</dd>
            <dt>Daemon identity</dt>
            <dd>{connection?.docker?.daemonId ?? "Not verified"}</dd>
          </dl>
          {connection?.diagnostic && (
            <p role="status">
              {native.connectionDiagnostics[connection.diagnostic.code]}
            </p>
          )}
          {connection?.docker && (
            <p role="status">
              {native.dockerProbeLabels[connection.docker.status]}
            </p>
          )}
          {connection && ["error", "degraded"].includes(connection.state) && (
            <p>
              Review this alias and its jump hosts in Settings → SSH host
              candidates. Check verified host keys and loaded identities in your
              terminal, then retry explicitly.
            </p>
          )}
        </section>
      )}
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
