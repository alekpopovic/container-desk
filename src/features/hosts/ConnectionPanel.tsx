import { useEffect, useRef, useState } from "react";
import {
  beginSshSession,
  getSshSession,
  disconnectSshSession,
  connectionLabels,
  connectionDiagnostics,
  dockerProbeLabels,
  IpcError,
} from "../../lib/ipc/client";
import type { ConnectionSnapshot, SshSelection } from "../../lib/ipc/generated";
const pending = (snapshot: ConnectionSnapshot | null) =>
  snapshot !== null &&
  ["resolving", "connecting", "probing"].includes(snapshot.state);
export function ConnectionPanel({ selection }: { selection: SshSelection }) {
  const [snapshot, setSnapshot] = useState<ConnectionSnapshot | null>(null);
  const [busy, setBusy] = useState(false);
  const [dockerPath, setDockerPath] = useState("");
  const [context, setContext] = useState("");
  const [sudo, setSudo] = useState(false);
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
      const next = await beginSshSession(selection, {
        executable: dockerPath || null,
        context: context || null,
        sudo,
      });
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
      <fieldset
        disabled={
          busy ||
          pending(snapshot) ||
          (!!snapshot &&
            snapshot.state !== "disconnected" &&
            snapshot.state !== "error")
        }
      >
        <legend>Remote Docker access</legend>
        <label>
          Docker executable path (optional)
          <input
            value={dockerPath}
            maxLength={4096}
            placeholder="Remote PATH: docker"
            onChange={(event) => setDockerPath(event.target.value)}
          />
        </label>
        <label>
          Remote Docker context (optional)
          <input
            value={context}
            maxLength={256}
            placeholder="Remote user's current context"
            onChange={(event) => setContext(event.target.value)}
          />
        </label>
        <label>
          <input
            type="checkbox"
            checked={sudo}
            onChange={(event) => setSudo(event.target.checked)}
          />{" "}
          Use existing noninteractive sudo access (sudo -n)
        </label>
        <p className="muted">
          Docker checks run as the SSH user unless sudo is selected. With sudo,
          the administrator's Docker configuration applies. No passwords are
          requested or stored. Disconnect before changing this target.
        </p>
      </fieldset>
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
          {snapshot.docker && (
            <section aria-label="Docker capabilities">
              <p role="status">{dockerProbeLabels[snapshot.docker.status]}</p>
              <dl>
                <dt>Execution</dt>
                <dd>{snapshot.docker.sudo ? "sudo -n" : "SSH user"}</dd>
                <dt>Context</dt>
                <dd>{snapshot.docker.context ?? "Unavailable"}</dd>
                <dt>Endpoint</dt>
                <dd>{snapshot.docker.endpoint ?? "Unavailable"}</dd>
                <dt>Endpoint location</dt>
                <dd>
                  {snapshot.docker.endpointKind === "unix"
                    ? "Unix socket on the SSH host"
                    : snapshot.docker.endpointKind
                      ? "Another endpoint selected by the remote Docker context"
                      : "Unavailable"}
                </dd>
                <dt>Daemon identity</dt>
                <dd>{snapshot.docker.daemonId ?? "Unavailable"}</dd>
                <dt>Client / server versions</dt>
                <dd>
                  {snapshot.docker.clientVersion ?? "Unknown"} /{" "}
                  {snapshot.docker.serverVersion ?? "Unknown"}
                </dd>
                <dt>Daemon OS</dt>
                <dd>{snapshot.docker.os ?? "Unknown"}</dd>
                <dt>Rootless daemon</dt>
                <dd>
                  {snapshot.docker.rootless === null
                    ? "Unknown"
                    : snapshot.docker.rootless
                      ? "Yes"
                      : "No"}
                </dd>
                <dt>Compose plugin</dt>
                <dd>
                  {snapshot.docker.compose}
                  {snapshot.docker.composeVersion &&
                    ` (${snapshot.docker.composeVersion})`}
                </dd>
              </dl>
            </section>
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
