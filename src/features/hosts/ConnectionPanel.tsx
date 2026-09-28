import { useEffect, useRef, useState } from "react";
import {
  beginSshSession,
  getSshSession,
  disconnectSshSession,
  connectionLabels,
  connectionDiagnostics,
  IpcError,
} from "../../lib/ipc/client";
import type { ConnectionSnapshot, SshSelection } from "../../lib/ipc/generated";
const pending = (snapshot: ConnectionSnapshot | null) =>
  snapshot !== null &&
  ["resolving", "connecting", "probing"].includes(snapshot.state);
export function ConnectionPanel({ selection }: { selection: SshSelection }) {
  const [snapshot, setSnapshot] = useState<ConnectionSnapshot | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const current = useRef<ConnectionSnapshot | null>(null);
  const sequence = useRef(0);
  const alive = useRef(true);
  const working = useRef(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      sequence.current += 1;
      if (timer.current) clearTimeout(timer.current);
      if (current.current && current.current.state !== "disconnected")
        void disconnectSshSession(current.current).catch(() => {});
    };
  }, []);
  function apply(next: ConnectionSnapshot) {
    current.current = next;
    setSnapshot(next);
  }
  function valid(request: number) {
    return alive.current && sequence.current === request;
  }
  function fail(cause: unknown) {
    setError(
      cause instanceof IpcError
        ? cause.message
        : "Connection state is unavailable.",
    );
  }
  function observed(next: ConnectionSnapshot) {
    return pending(next) || next.transportMode !== "unconnected";
  }
  async function poll(next: ConnectionSnapshot, request: number) {
    if (!valid(request) || !observed(next)) return;
    try {
      const result = await getSshSession(next);
      if (!valid(request)) return;
      apply(result);
      if (observed(result))
        timer.current = setTimeout(
          () => {
            void poll(result, request);
          },
          pending(result) ? 200 : 1000,
        );
    } catch (cause) {
      if (valid(request)) fail(cause);
    }
  }
  async function start() {
    if (working.current || pending(current.current)) return;
    working.current = true;
    setBusy(true);
    setError(null);
    const request = ++sequence.current;
    try {
      const next = await beginSshSession(selection);
      if (!valid(request)) {
        // A late start still owns a backend attempt: cancel its exact token, never a newer one.
        void disconnectSshSession(next).catch(() => {});
        return;
      }
      apply(next);
      void poll(next, request);
    } catch (cause) {
      if (valid(request)) fail(cause);
    } finally {
      working.current = false;
      if (valid(request)) setBusy(false);
    }
  }
  async function disconnect() {
    if (working.current || !current.current) return;
    working.current = true;
    const request = ++sequence.current;
    if (timer.current) clearTimeout(timer.current);
    setBusy(true);
    setError(null);
    try {
      const next = await disconnectSshSession(current.current);
      if (valid(request)) apply(next);
    } catch (cause) {
      if (valid(request)) fail(cause);
    } finally {
      working.current = false;
      if (valid(request)) setBusy(false);
    }
  }
  return (
    <section aria-label="SSH connection session">
      <h4>Connection progress</h4>
      <p role="status">
        {snapshot ? connectionLabels[snapshot.state] : "Disconnected"}
      </p>
      <button
        className="button"
        type="button"
        disabled={busy || pending(snapshot) || snapshot?.state === "ready"}
        onClick={() => {
          void start();
        }}
      >
        Connect selected host
      </button>{" "}
      <button
        className="button"
        type="button"
        disabled={busy || !snapshot || snapshot.state === "disconnected"}
        onClick={() => {
          void disconnect();
        }}
      >
        {pending(snapshot) ? "Cancel connection" : "Disconnect"}
      </button>
      {snapshot && (
        <>
          <p className="muted">
            {snapshot.transportMode === "multiplexed"
              ? "SSH transport: app-owned shared connection."
              : snapshot.transportMode === "direct_fallback"
                ? "SSH transport: direct fallback. Connection reuse is unavailable; each command opens its own strict SSH connection."
                : "No active SSH transport."}
          </p>
          <p className="muted">
            Session generation {snapshot.token.sessionGeneration}.{" "}
            {snapshot.hasJump &&
              "This route uses a jump host or proxy; check every hop if authentication fails."}
          </p>
          {snapshot.durations.length > 0 && (
            <ul aria-label="Connection stage durations">
              {snapshot.durations.map((duration) => (
                <li key={duration.stage}>
                  {duration.stage}: {duration.durationMs} ms
                </li>
              ))}
            </ul>
          )}
          {snapshot.diagnostic && (
            <p role="status">
              {connectionDiagnostics[snapshot.diagnostic.code]}
            </p>
          )}
        </>
      )}
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
