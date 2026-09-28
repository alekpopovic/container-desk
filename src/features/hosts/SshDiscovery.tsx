import { isTauri } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import {
  checkSshAccess,
  sshAccessHelp,
  discoverSshHosts,
  getSshConfigPath,
  selectSshAlias,
  isConcreteAlias,
  resolveSshConfig,
  IpcError,
} from "../../lib/ipc/client";
import type {
  DiscoveryWarningCode,
  EffectiveSshConfig,
  HostDiscovery,
  SshSelection,
  SshAccessReport,
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
  const [access, setAccess] = useState<SshAccessReport | null>(null);
  const [resolved, setResolved] = useState<EffectiveSshConfig | null>(null);
  const [selected, setSelected] = useState<SshSelection | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const sequence = useRef(0);
  const working = useRef(false);
  useEffect(() => {
    const request = ++sequence.current;
    setReport(null);
    setSelected(null);
    setResolved(null);
    setAccess(null);
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
    setResolved(null);
    setAccess(null);
    void execute(() => discoverSshHosts(path || null), setReport);
  };
  const choose = (value: string) => {
    setSelected(null);
    setResolved(null);
    setAccess(null);
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
            setResolved(null);
            setAccess(null);
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
            setResolved(null);
            setAccess(null);
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
          was made by selecting the alias.
        </p>
      )}
      {selected && (
        <div>
          <p className="muted">
            Resolve only configuration you trust. OpenSSH -G evaluates Match
            exec and may run local commands. ProxyCommand is trusted executable
            configuration too; its contents are not displayed. Identity contents
            are never read by ContainerDesk.
          </p>
          <button
            className="button"
            type="button"
            disabled={!native || busy}
            onClick={() => {
              setResolved(null);
              setAccess(null);
              void execute(() => resolveSshConfig(selected), setResolved);
            }}
          >
            Resolve selected alias
          </button>
        </div>
      )}
      {selected && (
        <div>
          <p className="muted">
            Check access opens an SSH connection and runs a fixed inert command.
            It uses strict host verification and your configured identities,
            with no password prompts or agent forwarding.
          </p>
          <button
            className="button"
            type="button"
            disabled={!native || busy}
            onClick={() => {
              setAccess(null);
              void execute(() => checkSshAccess(selected), setAccess);
            }}
          >
            Check SSH access
          </button>
          <details>
            <summary>Terminal setup and host trust</summary>
            <p>
              In your terminal, connect to each jump alias first, then the
              selected destination, using the same config and OpenSSH
              executable. Compare host fingerprints with the administrator
              through an independent trusted channel before accepting them.
            </p>
            <p>
              For a changed key, investigate first. Only after a verified
              rotation, repair the affected known_hosts entry yourself.
              ContainerDesk never edits trust files.
            </p>
            <p>
              Load encrypted keys using ssh-add or your normal OS agent. Keep
              the agent available to the desktop app. Passwords and passphrases
              are never requested or stored here. After completing setup, press
              Check SSH access again.
            </p>
          </details>
        </div>
      )}
      {access && (
        <div role="status" aria-label="SSH access result">
          <p>{sshAccessHelp[access.status]}</p>
          {access.sshError && (
            <p>
              SSH diagnostic (sanitized): <code>{access.sshError}</code>
            </p>
          )}
        </div>
      )}
      {resolved && (
        <dl
          className="diagnostics-facts"
          aria-label="Effective SSH configuration"
        >
          <div>
            <dt>Original alias</dt>
            <dd>{resolved.selection.alias}</dd>
          </div>
          <div>
            <dt>Effective host</dt>
            <dd>{resolved.hostname}</dd>
          </div>
          <div>
            <dt>Remote user</dt>
            <dd>{resolved.user}</dd>
          </div>
          <div>
            <dt>Port</dt>
            <dd>{resolved.port}</dd>
          </div>
          <div>
            <dt>Jump hosts</dt>
            <dd>{resolved.proxyJump ?? "None"}</dd>
          </div>
          <div>
            <dt>ProxyCommand</dt>
            <dd>
              {resolved.hasProxyCommand
                ? "Configured (contents hidden)"
                : "None"}
            </dd>
          </div>
          <div>
            <dt>OpenSSH executable</dt>
            <dd>{resolved.executablePath}</dd>
          </div>
          <div>
            <dt>Config policy</dt>
            <dd>
              {resolved.selection.useDefaultConfig
                ? "Native user and system defaults"
                : "Explicit user config (-F); system config excluded"}
            </dd>
          </div>
        </dl>
      )}
      {message && <p role="alert">{message}</p>}
    </section>
  );
}
