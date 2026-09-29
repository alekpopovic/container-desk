import { NetworkInventory } from "./features/networks/NetworkInventory";
import { VolumeInventory } from "./features/volumes/VolumeInventory";
import { ImageInventory } from "./features/images/ImageInventory";
import { ComposeInventory } from "./features/compose/ComposeInventory";
import { isTauri } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";
import { ContainerInventory } from "./features/containers/ContainerInventory";
import { useContainerInventory } from "./features/containers/useContainerInventory";
import { HostInventory } from "./features/hosts/HostInventory";
import type { WorkspaceState } from "./components/WorkspaceShell";
import type { HostInventory as Inventory } from "./lib/ipc/generated";
import { SshDiscovery } from "./features/hosts/SshDiscovery";
import { useWorkspaceMode } from "./features/workspace/useWorkspaceMode";
import { DependencyDiagnostics } from "./components/DependencyDiagnostics";
import { WorkspaceShell } from "./components/WorkspaceShell";
import { getPreferences, setTheme, IpcError } from "./lib/ipc/client";
import type {
  PreferencesSnapshot,
  StorageNotice,
  Theme,
} from "./lib/ipc/generated";

const notices: Record<StorageNotice, string> = {
  migrated:
    "Your saved settings were upgraded. The previous version was retained.",
  recovered_previous:
    "Damaged settings were recovered from the previous version. The damaged file was retained.",
  reset_after_corruption:
    "Saved settings could not be read. Defaults are in use and the damaged file was retained.",
  unsupported_schema:
    "These settings belong to a newer version of ContainerDesk. Saving is disabled to preserve them.",
};
export default function App() {
  const native = isTauri();
  const workspace = useWorkspaceMode(native);
  const [snapshot, setSnapshot] = useState<PreferencesSnapshot | null>(null);
  const [inventory, setInventory] = useState<Inventory | null>(null);
  const inventoryChanged = useCallback((value: Inventory) => {
    setInventory(value);
    if (value.mode === "live") setSnapshot(value.saved);
  }, []);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const mounted = useRef(false);
  const saving = useRef(false);
  useEffect(() => {
    mounted.current = true;
    let active = true;
    if (native) {
      getPreferences()
        .then((result) => {
          if (active) setSnapshot(result);
        })
        .catch(() => {
          if (active)
            setError(
              "Local settings are unavailable. Saving is disabled; existing files have been retained.",
            );
        });
    }
    return () => {
      active = false;
      mounted.current = false;
    };
  }, [native]);
  const changeTheme = async (theme: Theme) => {
    if (!snapshot?.writable || saving.current) return;
    saving.current = true;
    setBusy(true);
    setError(null);
    try {
      const result = await setTheme({
        expectedRevision: snapshot.preferences.revision,
        theme,
      });
      if (mounted.current) setSnapshot(result);
    } catch (failure) {
      if (mounted.current) {
        setError(
          failure instanceof IpcError
            ? failure.message
            : "Settings could not be saved.",
        );
        // The backend may have committed before a response was lost. Reload on the next launch.
        setSnapshot((current) => current && { ...current, writable: false });
      }
    } finally {
      saving.current = false;
      if (mounted.current) setBusy(false);
    }
  };
  const message = error ?? (snapshot?.notice ? notices[snapshot.notice] : null);
  let displayState: WorkspaceState = workspace.state;
  const currentInventory =
    inventory?.mode === workspace.mode.mode ? inventory : null;
  const connection = currentInventory?.connection;
  const selectedHost =
    connection &&
    currentInventory?.saved.preferences.hosts.find(
      (h) => h.id === connection.hostId,
    );
  if (workspace.mode.mode === "live" && selectedHost && connection) {
    const host = {
      name: selectedHost.displayName,
      alias: selectedHost.alias,
      endpoint: connection.docker?.endpoint ?? "Not verified",
    };
    displayState =
      connection.state === "ready"
        ? { kind: "connected", host }
        : connection.state === "disconnected"
          ? { kind: "offline", host }
          : ["error", "degraded"].includes(connection.state)
            ? {
                kind: "error",
                host,
                message: "Review this host's connection diagnostics in Hosts.",
              }
            : { kind: "loading", host };
  }
  const containers = useContainerInventory(
    native,
    workspace.mode,
    connection ?? null,
  );
  if (containers.view.scope && displayState.kind !== "empty") {
    displayState =
      containers.view.error && !containers.view.rows.length
        ? {
            kind: "error",
            host: displayState.host,
            message: containers.view.error,
          }
        : containers.view.updatedAt !== null && !containers.view.stale
          ? {
              kind: "ready",
              host: displayState.host,
              containers: containers.view.rows,
            }
          : { kind: "connected", host: displayState.host };
  }
  return (
    <WorkspaceShell
      state={displayState}
      containersExtra={
        <ContainerInventory
          inspectEnabled={native && workspace.mode.mode === "live"}
          eventStatus={containers.eventStatus}
          view={containers.view}
          host={displayState.kind === "empty" ? null : displayState.host}
          refresh={() => {
            void containers.refresh();
          }}
          select={containers.select}
        />
      }
      composeExtra={
        <ComposeInventory
          host={
            selectedHost
              ? `${selectedHost.displayName} · ${selectedHost.alias}`
              : undefined
          }
          view={containers.view}
          native={native && workspace.mode.mode === "live"}
          refreshContainers={() => {
            void containers.refresh();
          }}
          openContainer={(scope, id) => {
            if (scope !== containers.view.scope) return;
            containers.select(id);
            window.location.hash = "/containers";
          }}
        />
      }
      imagesExtra={
        <ImageInventory
          view={containers.view}
          native={native && workspace.mode.mode === "live"}
          refreshContainers={() => {
            void containers.refresh();
          }}
          openContainer={(scope, id) => {
            if (scope !== containers.view.scope) return;
            containers.select(id);
            window.location.hash = "/containers";
          }}
        />
      }
      volumesExtra={
        <VolumeInventory
          view={containers.view}
          native={native && workspace.mode.mode === "live"}
          refreshContainers={() => {
            void containers.refresh();
          }}
          openContainer={(scope, id) => {
            if (scope !== containers.view.scope) return;
            containers.select(id);
            window.location.hash = "/containers";
          }}
        />
      }
      networksExtra={
        <NetworkInventory
          view={containers.view}
          native={native && workspace.mode.mode === "live"}
          refreshContainers={() => {
            void containers.refresh();
          }}
          openContainer={(scope, id) => {
            if (scope !== containers.view.scope) return;
            containers.select(id);
            window.location.hash = "/containers";
          }}
        />
      }
      savedHosts={
        currentInventory?.saved.preferences.hosts ??
        (workspace.mode.mode === "live"
          ? (snapshot?.preferences.hosts ?? [])
          : [])
      }
      hostsExtra={
        <HostInventory
          key={workspace.mode.mode}
          mode={workspace.mode.mode}
          onChange={inventoryChanged}
        />
      }
      demo={{
        active: workspace.mode.mode === "demo",
        busy: workspace.busy,
        scenario: workspace.mode.scenario ?? "standard",
        onEnter: workspace.enterDemo,
        onExit: workspace.exitDemo,
        onScenario: workspace.scenario,
        message: workspace.message,
      }}
      settingsExtra={
        <>
          <DependencyDiagnostics
            preferences={snapshot}
            onSaved={setSnapshot}
            demo={workspace.mode.mode === "demo"}
          />
          <SshDiscovery demo={workspace.mode.mode === "demo"} />
        </>
      }
      preferences={
        native
          ? {
              theme: snapshot?.preferences.theme ?? "system",
              disabled: busy || !snapshot?.writable,
              onThemeChange: changeTheme,
              message:
                message ??
                (snapshot === null ? "Loading saved settings…" : null),
              error: error !== null,
            }
          : undefined
      }
    />
  );
}
