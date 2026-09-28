import { useCallback, useEffect, useRef, useState } from "react";
import * as nativeBridge from "../../lib/ipc/client";
import * as browserBridge from "../../demo/browser";
import type { WorkspaceState } from "../../components/WorkspaceShell";
import type {
  DemoScenario,
  SessionScope,
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
  const activeScope = useRef<SessionScope | null>(null);
  const mounted = useRef(false);
  const working = useRef(false);
  const serial = useRef(0);

  const apply = useCallback(
    async (snapshot: WorkspaceModeSnapshot, request: number) => {
      if (!mounted.current || serial.current !== request) return;
      setMode(snapshot);
      activeScope.current = snapshot.scope;
      if (!snapshot.scope || !snapshot.host) {
        setState({ kind: "empty" });
        return;
      }
      const scope = snapshot.scope;
      const host = {
        name: snapshot.host.displayName,
        alias: snapshot.host.alias,
        endpoint: "Synthetic Docker data",
      };
      setState({ kind: "loading", host });
      try {
        const response = await bridge.listContainers(
          scope,
          () => activeScope.current,
        );
        if (mounted.current && serial.current === request)
          setState({ kind: "ready", host, containers: response.containers });
      } catch (error) {
        if (!mounted.current || serial.current !== request) return;
        if (
          error instanceof nativeBridge.IpcError &&
          error.code === "disconnected"
        )
          setState({ kind: "offline", host });
        else
          setState({
            kind: "error",
            host,
            message:
              error instanceof nativeBridge.IpcError
                ? error.message
                : "Resource data could not be loaded.",
          });
      }
    },
    [bridge],
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
      activeScope.current = null;
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
