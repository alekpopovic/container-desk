import { useContainerEvents } from "../events/useContainerEvents";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  useSyncExternalStore,
} from "react";
import * as nativeBridge from "../../lib/ipc/client";
import * as demoBridge from "../../demo/browser";
import type {
  ConnectionSnapshot,
  HostSelection,
  SessionScope,
  WorkspaceModeSnapshot,
} from "../../lib/ipc/generated";
import { ContainerCache } from "./cache";
export function useContainerInventory(
  native: boolean,
  mode: WorkspaceModeSnapshot,
  connection: ConnectionSnapshot | null,
) {
  const [cache] = useState(() => new ContainerCache());
  const view = useSyncExternalStore(cache.subscribe, cache.getSnapshot);
  const selection = useRef<HostSelection | null>(null);
  const generation = useRef(0);
  const bridge = native ? nativeBridge : demoBridge;
  const refresh = useCallback(async () => {
    const ticket = cache.begin();
    if (!ticket) return false;
    try {
      const response = await bridge.listContainers(
        ticket.scope,
        () => cache.getSnapshot().scope,
      );
      cache.complete(ticket, response);
    } catch (error) {
      cache.fail(
        ticket,
        error instanceof nativeBridge.IpcError
          ? error.message
          : "Container data could not be loaded.",
      );
    }
    return true;
  }, [bridge, cache]);
  const token = connection
    ? JSON.stringify({
        hostId: connection.hostId,
        token: connection.token,
        state: connection.state,
        daemon: connection.docker?.daemonId,
      })
    : "";
  const demoScope = mode.mode === "demo" ? JSON.stringify(mode.scope) : "";
  useEffect(() => {
    let active = true;
    cache.setNamespace(mode.mode);
    cache.unavailable();
    selection.current = null;
    const current: Pick<ConnectionSnapshot, "hostId" | "state"> | null = token
      ? JSON.parse(token)
      : null;
    if (mode.mode === "demo" && demoScope && demoScope !== "null") {
      cache.activate(JSON.parse(demoScope) as SessionScope);
      void refresh();
    } else if (native && current?.state === "ready" && current.hostId) {
      const selected = {
        hostId: current.hostId,
        selectionGeneration: ++generation.current,
      };
      selection.current = selected;
      nativeBridge
        .connectHost({ selection: selected }, () => selection.current)
        .then((response) => {
          if (!active) return;
          cache.activate(response.scope);
          void refresh();
        })
        .catch((error) => {
          if (active)
            cache.unavailable(
              error instanceof nativeBridge.IpcError
                ? error.message
                : "Container session is unavailable.",
            );
        });
    }
    return () => {
      active = false;
      selection.current = null;
      cache.unavailable();
    };
    // Snapshots poll every second; only actual identity/state transitions start a new binding/read.
  }, [cache, native, token, demoScope, mode.mode, refresh]);
  // Fence the render itself, before effect cleanup, so host/mode switches cannot flash old rows.
  const matches =
    view.scope === null ||
    (mode.mode === "demo"
      ? mode.scope !== null && nativeBridge.sameScope(view.scope, mode.scope)
      : connection?.state === "ready" &&
        view.scope.selection.hostId === connection.hostId &&
        view.scope.sessionId === connection.token.sessionId &&
        view.scope.sessionGeneration === connection.token.sessionGeneration &&
        view.scope.daemonId === connection.docker?.daemonId);
  const visible = matches
    ? view
    : {
        scope: null,
        rows: [],
        selectedId: null,
        updatedAt: null,
        loading: false,
        stale: false,
        error: null,
      };
  const eventStatus = useContainerEvents(
    native && mode.mode === "live" ? visible.scope : null,
    refresh,
  );
  return {
    view: visible,
    refresh,
    eventStatus,
    select: (id: string) => cache.select(id),
  };
}
