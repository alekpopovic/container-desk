import { isTauri } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import {
  getDependencyDiagnostics,
  setSshExecutable,
  IpcError,
} from "../lib/ipc/client";
import type {
  DependencyDiagnostics as Report,
  PreferencesSnapshot,
  SshDiagnostic,
} from "../lib/ipc/generated";

export function DependencyDiagnostics({
  preferences,
  onSaved,
  demo = false,
}: {
  preferences: PreferencesSnapshot | null;
  demo?: boolean;
  onSaved: (preferences: PreferencesSnapshot) => void;
}) {
  const native = isTauri() && !demo;
  const [report, setReport] = useState<Report | null>(null);
  const [ssh, setSsh] = useState<SshDiagnostic | null>(null);
  const [path, setPath] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const active = useRef(false);
  const working = useRef(false);
  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
    };
  }, []);
  useEffect(() => {
    setPath(preferences?.preferences.sshExecutableOverride ?? "");
  }, [preferences?.preferences.sshExecutableOverride]);
  const execute = async (action: () => Promise<void>) => {
    if (working.current) return;
    working.current = true;
    setBusy(true);
    setMessage(null);
    try {
      await action();
    } catch (error) {
      if (active.current)
        setMessage(
          error instanceof IpcError
            ? error.message
            : "The dependency check could not finish.",
        );
    } finally {
      working.current = false;
      if (active.current) setBusy(false);
    }
  };
  const inspect = () =>
    execute(async () => {
      const value = await getDependencyDiagnostics();
      if (active.current) {
        setReport(value);
        setSsh(value.ssh);
      }
    });
  const save = () =>
    execute(async () => {
      if (!preferences) return;
      const result = await setSshExecutable({
        expectedRevision: preferences.preferences.revision,
        path: path === "" ? null : path,
      });
      if (active.current) {
        setSsh(result.ssh);
        if (result.preferences) {
          onSaved(result.preferences);
          setMessage("SSH executable preference saved.");
        }
      }
    });
  return (
    <section
      className="diagnostics-panel"
      aria-labelledby="diagnostics-title"
      aria-busy={busy}
    >
      <h3 id="diagnostics-title">Native dependencies</h3>
      <p className="muted">
        ContainerDesk needs OpenSSH and the system desktop libraries. Local
        Docker, jq, Python, Rust and Node are not required. Python is used only
        by the development tracker.
      </p>
      <p className="muted">
        Desktop launchers can use a different agent environment from your
        terminal. An encrypted key must already be available to OpenSSH through
        your OS agent or configured Keychain. ContainerDesk never asks for its
        passphrase or runs shell profile scripts. Check each jump host too.
      </p>
      {!native && (
        <p className="muted">
          {demo
            ? "SSH checks are disabled while DEMO is active."
            : "Open the desktop app to inspect native dependencies."}
        </p>
      )}
      <button
        className="button"
        type="button"
        disabled={!native || busy}
        onClick={inspect}
      >
        {busy ? "Checking…" : "Run diagnostics"}
      </button>
      {report && (
        <dl className="diagnostics-facts">
          <div>
            <dt>Application</dt>
            <dd>{report.appVersion}</dd>
          </div>
          <div>
            <dt>Platform</dt>
            <dd>
              {report.platform} / {report.architecture}
            </dd>
          </div>
          <div>
            <dt>SSH agent</dt>
            <dd>{report.agent.message}</dd>
          </div>
        </dl>
      )}
      {ssh && (
        <div className="diagnostic-result" role="status">
          <strong>
            OpenSSH: {ssh.status === "ready" ? "Available" : "Needs attention"}
          </strong>
          <p>{ssh.path}</p>
          <p>{ssh.message}</p>
          {ssh.version && <p>{ssh.version}</p>}
        </div>
      )}
      <form
        onSubmit={(event) => {
          event.preventDefault();
          void save();
        }}
      >
        <label htmlFor="ssh-executable">OpenSSH executable override</label>
        <input
          id="ssh-executable"
          type="text"
          value={path}
          maxLength={4096}
          placeholder="/usr/bin/ssh (default)"
          autoComplete="off"
          spellCheck={false}
          disabled={!native || busy || !preferences?.writable}
          onChange={(event) => setPath(event.target.value)}
          aria-describedby="ssh-executable-help"
        />
        <p className="muted" id="ssh-executable-help">
          Choose only an OpenSSH executable you trust. Use an absolute path, or
          leave empty for /usr/bin/ssh. Saving runs a bounded version check; it
          does not connect to a host.
        </p>
        <button
          type="submit"
          className="button"
          disabled={!native || busy || !preferences?.writable}
        >
          Validate and save SSH path
        </button>
      </form>
      {message && <p role="status">{message}</p>}
    </section>
  );
}
