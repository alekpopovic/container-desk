import { isTauri } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
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
  return (
    <WorkspaceShell
      state={workspace.state}
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
