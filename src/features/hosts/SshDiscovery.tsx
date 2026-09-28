import { isTauri } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import {
  discoverSshHosts,
  getSshConfigPath,
  selectSshAlias,
  isConcreteAlias,
  IpcError,
} from "../../lib/ipc/client";
import type {
  DiscoveryWarningCode,
  HostDiscovery,
  SshSelection,
} from "../../lib/ipc/generated";
const warnings: Record<DiscoveryWarningCode, string> = {
  missing_file: "A config file or include was not found.",
  unreadable_file: "A config file could not be read as a regular file.",
  unsupported_syntax:
    "A discovery expression is unsupported. Enter the alias manually.",
  patterns_skipped:
    "Wildcard, negated or unsupported Host tokens were omitted. Enter a concrete alias manually.",
  conditional_include:
    "A conditional Include was skipped. Enter the alias manually.",
  match_skipped:
    "Match conditions were not evaluated; executable directives were not run.",
  include_cycle: "An Include cycle was stopped.",
  limit_reached:
    "Discovery reached an application limit; results may be incomplete.",
};
export function SshDiscovery({ demo }: { demo: boolean }) {
  const native = isTauri() && !demo;
  const [path, setPath] = useState("");
  const [defaultPath, setDefaultPath] = useState<string | null>(null);
  const [alias, setAlias] = useState("");
  const [report, setReport] = useState<HostDiscovery | null>(null);
  const [selected, setSelected] = useState<SshSelection | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const sequence = useRef(0);
  const working = useRef(false);
  useEffect(() => {
    const request = ++sequence.current;
    setReport(null);
    setSelected(null);
    setMessage(null);
    if (native)
      getSshConfigPath()
        .then((value) => {
          if (request === sequence.current) setDefaultPath(value.path);
        })
        .catch(() => {
          if (request === sequence.current)
            setMessage(
              "Default SSH config path is unavailable. Choose an absolute path.",
            );
        });
    return () => {
      sequence.current += 1;
    };
  }, [native]);
  async function execute<T>(
    action: () => Promise<T>,
    apply: (value: T) => void,
  ) {
    if (working.current || !native) return;
    working.current = true;
    setBusy(true);
    setMessage(null);
    const request = ++sequence.current;
    try {
      const result = await action();
      if (request === sequence.current) apply(result);
    } catch (error) {
      if (request === sequence.current)
        setMessage(
          error instanceof IpcError
            ? error.message
            : "SSH configuration could not be read.",
        );
    } finally {
      working.current = false;
      setBusy(false);
    }
  }
  const discover = () => {
    setReport(null);
    setSelected(null);
    void execute(() => discoverSshHosts(path || null), setReport);
  };
  const choose = (value: string) => {
    setSelected(null);
    void execute(() => selectSshAlias(path || null, value), setSelected);
  };
  return (
    <section
      className="diagnostics-panel"
      aria-labelledby="ssh-discovery-title"
      aria-busy={busy}
    >
      <h3 id="ssh-discovery-title">SSH host candidates</h3>
      <p className="muted">
        Browse your trusted OpenSSH configuration, then select a concrete alias.
        Browsing reads files only. Configuration can contain executable
        directives; resolving a selected alias in a later step may run them.
      </p>
      {!native && (
        <p>
          {demo
            ? "SSH discovery is disabled while DEMO is active."
            : "Open the desktop app to browse local SSH configuration."}
        </p>
      )}
      {defaultPath && (
        <p className="config-path">
          Default config: <code>{defaultPath}</code>
        </p>
      )}
      <form
        onSubmit={(event) => {
          event.preventDefault();
          discover();
        }}
      >
        <label htmlFor="ssh-config-path">SSH config path</label>
        <input
          id="ssh-config-path"
          value={path}
          maxLength={4096}
          placeholder={defaultPath ?? "Default: ~/.ssh/config"}
          disabled={!native || busy}
          autoComplete="off"
          spellCheck={false}
          onChange={(event) => {
            setPath(event.target.value);
            setReport(null);
            setSelected(null);
          }}
        />
        <p className="muted">
          Leave empty for the default path shown above, or enter an absolute
          path to your trusted user config. Relative Include paths are based on
          ~/.ssh. Files are never rewritten.
        </p>
        <button type="submit" className="button" disabled={!native || busy}>
          Browse host candidates
        </button>
      </form>
      {report && (
        <div>
          <p className="config-path">
            Read config: <code>{report.configPath}</code>
          </p>
          {report.candidates.length === 0 ? (
            <p>
              No literal host candidates found. Enter an alias manually below.
            </p>
          ) : (
            <ul className="ssh-candidates" aria-label="SSH host candidates">
              {report.candidates.map((candidate) => (
                <li key={candidate.alias}>
                  <button
                    className="button"
                    type="button"
                    disabled={busy || !native}
                    onClick={() => choose(candidate.alias)}
                  >
                    Select {candidate.alias}
                  </button>
                  <small>
                    {candidate.source}:{candidate.line}
                  </small>
                </li>
              ))}
            </ul>
          )}
          {report.warnings.length > 0 && (
            <ul className="discovery-warnings" aria-label="Discovery notes">
              {report.warnings.map((warning) => (
                <li key={`${warning.source}:${warning.line}:${warning.code}`}>
                  {warnings[warning.code]}{" "}
                  <small>
                    {warning.source}
                    {warning.line ? `:${warning.line}` : ""}
                  </small>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
      <form
        onSubmit={(event) => {
          event.preventDefault();
          choose(alias);
        }}
      >
        <label htmlFor="manual-ssh-alias">Manual SSH alias</label>
        <input
          id="manual-ssh-alias"
          value={alias}
          maxLength={256}
          disabled={!native || busy}
          autoComplete="off"
          spellCheck={false}
          onChange={(event) => {
            setAlias(event.target.value);
            setSelected(null);
          }}
          aria-describedby="manual-alias-help"
        />
        <p id="manual-alias-help" className="muted">
          Use 1–256 ASCII letters, digits, dots, underscores or dashes, starting
          with a letter or digit. For wildcard or dynamic configuration, enter a
          matching concrete alias. Create a simple SSH alias for unsupported
          names.
        </p>
        <button
          type="submit"
          className="button"
          disabled={!native || busy || !isConcreteAlias(alias)}
        >
          Select manual alias
        </button>
      </form>
      {selected && (
        <p role="status" className="config-path">
          Selected {selected.alias} from {selected.configPath}. No connection
          was made. Effective configuration resolution is not available yet.
        </p>
      )}
      {message && <p role="alert">{message}</p>}
    </section>
  );
}
