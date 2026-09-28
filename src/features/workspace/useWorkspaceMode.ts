import { useCallback, useEffect, useRef, useState } from "react";
import * as nativeBridge from "../../lib/ipc/client";
import * as browserBridge from "../../demo/browser";
import type { WorkspaceState } from "../../components/WorkspaceShell";
import type {
  DemoScenario,
  SwitchWorkspaceRequest,
  WorkspaceModeSnapshot,
} from "../../lib/ipc/generated";

export function useWorkspaceMode(native: boolean) {
  const bridge = native ? nativeBridge : browserBridge;
  const [mode, setMode] = useState<WorkspaceModeSnapshot>({
    mode: "live",
    scenario: null,
    scope: null,
    host: null,
  });
  const [state, setState] = useState<WorkspaceState>({ kind: "empty" });
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const mounted = useRef(false);
  const working = useRef(false);
  const serial = useRef(0);

  const apply = useCallback(
    async (snapshot: WorkspaceModeSnapshot, request: number) => {
      if (!mounted.current || serial.current !== request) return;
      setMode(snapshot);
      if (!snapshot.scope || !snapshot.host) {
        setState({ kind: "empty" });
        return;
      }
      setState({
        kind: "connected",
        host: {
          name: snapshot.host.displayName,
          alias: snapshot.host.alias,
          endpoint:
            snapshot.mode === "demo"
              ? "Synthetic Docker data"
              : "Connected Docker host",
        },
      });
    },
    [],
  );
  // Only bootstrap an already-selected mode; no failure ever enables demo.
  useEffect(() => {
    mounted.current = true;
    const request = ++serial.current;
    bridge
      .getWorkspaceMode()
      .then((snapshot) => apply(snapshot, request))
      .catch(() => {
        if (mounted.current && serial.current === request)
          setMessage(
            "Workspace mode could not be loaded. No demo data was substituted.",
          );
      });
    return () => {
      mounted.current = false;
      serial.current += 1;
    };
  }, [bridge, apply]);

  const change = async (request: SwitchWorkspaceRequest) => {
    if (working.current) return;
    working.current = true;
    setBusy(true);
    setMessage(null);
    const sequence = ++serial.current;
    try {
      await apply(await bridge.switchWorkspace(request), sequence);
    } catch (error) {
      if (mounted.current)
        setMessage(
          error instanceof nativeBridge.IpcError
            ? error.message
            : "Workspace mode could not be changed.",
        );
    } finally {
      working.current = false;
      if (mounted.current) setBusy(false);
    }
  };
  return {
    mode,
    state,
    busy,
    message,
    enterDemo: () => change({ mode: "demo", scenario: "standard" }),
    exitDemo: () => change({ mode: "live" }),
    scenario: (scenario: DemoScenario) => change({ mode: "demo", scenario }),
  };
}
