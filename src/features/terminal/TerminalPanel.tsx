import { useEffect, useRef, useState } from "react";
import { Terminal as Xterm } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import * as ipc from "../../lib/ipc/client";
import type {
  ConfirmationIntent,
  SessionScope,
  TerminalShell,
} from "../../lib/ipc/generated";
import { pastedText, TerminalSession, type TerminalNotice } from "./session";
const errorText = (error: unknown) =>
  error instanceof ipc.IpcError ? error.message : "Terminal operation failed.";
export function TerminalPanel({
  scope,
  id,
  name,
  host,
}: {
  scope: SessionScope;
  id: string;
  name: string;
  host: string;
}) {
  const surface = useRef<HTMLElement>(null);
  const terminal = useRef<Xterm | null>(null);
  const session = useRef<TerminalSession | null>(null);
  const alive = useRef(true);
  const lock = useRef(false);
  const pasteBlocked = useRef(false);
  const closeButton = useRef<HTMLButtonElement>(null);
  const [managed, setManaged] = useState(false);
  const [enabled, setEnabled] = useState(false);
  const [initializing, setInitializing] = useState(true);
  const [busy, setBusy] = useState(false);
  const [shell, setShell] = useState<TerminalShell>("sh");
  const [intent, setIntent] = useState<ConfirmationIntent | null>(null);
  const [expired, setExpired] = useState(false);
  const [paste, setPaste] = useState<string | null>(null);
  const [selected, setSelected] = useState(false);
  const [dimensions, setDimensions] = useState({ columns: 80, rows: 24 });
  const [notice, setNotice] = useState<TerminalNotice>({
    phase: "closed",
    message: "Disconnected · open a terminal explicitly",
  });
  const [error, setError] = useState("");
  const current = () => (alive.current ? scope : null);
  useEffect(() => {
    alive.current = true;
    pasteBlocked.current = false;
    const hostElement = surface.current;
    if (!hostElement) return;
    const term = new Xterm({
      allowProposedApi: true,
      scrollback: 1000,
      cols: 80,
      rows: 24,
      fontSize: 13,
      fontFamily: "ui-monospace, monospace",
      cursorBlink: false,
      disableStdin: true,
      screenReaderMode: true,
      altClickMovesCursor: false,
      windowOptions: {},
      linkHandler: { activate: () => {} },
      logLevel: "off",
      theme: {
        background: "#101820",
        foreground: "#e3edf2",
        cursor: "#5ed4c2",
      },
    });
    terminal.current = term;
    const fit = new FitAddon();
    term.loadAddon(fit);
    // No clipboard, URL, title, cwd, notification or shell-integration escape side effects.
    const hooks = [0, 1, 2, 7, 8, 9, 52, 133, 633, 777, 1337].map((code) =>
      term.parser.registerOscHandler(code, () => true),
    );
    term.open(hostElement);
    const measure = () => {
      if (!alive.current || !hostElement.isConnected) return;
      const proposed = fit.proposeDimensions();
      if (!proposed) return;
      const columns = Math.min(500, Math.max(20, proposed.cols));
      const rows = Math.min(300, Math.max(5, proposed.rows));
      if (term.cols !== columns || term.rows !== rows) {
        term.resize(columns, rows);
        setDimensions({ columns, rows });
      }
      session.current?.resize(columns, rows);
    };
    const frame = requestAnimationFrame(measure);
    const observer = new ResizeObserver(measure);
    observer.observe(hostElement);
    const input = term.onData((value) => {
      if (
        alive.current &&
        document.visibilityState === "visible" &&
        !pasteBlocked.current
      )
        session.current?.input(new TextEncoder().encode(value));
    });
    const binary = term.onBinary((value) => {
      if (
        alive.current &&
        document.visibilityState === "visible" &&
        !pasteBlocked.current
      )
        session.current?.input(Uint8Array.from(value, (c) => c.charCodeAt(0)));
    });
    const selection = term.onSelectionChange(() => {
      if (alive.current) setSelected(term.hasSelection());
    });
    term.attachCustomKeyEventHandler((event) => {
      if (
        event.type === "keydown" &&
        event.ctrlKey &&
        event.shiftKey &&
        event.key === "Escape"
      ) {
        event.preventDefault();
        closeButton.current?.focus();
        return false;
      }
      return (
        !!session.current?.active &&
        !pasteBlocked.current &&
        hostElement.contains(document.activeElement) &&
        document.visibilityState === "visible"
      );
    });
    const onPaste = (event: ClipboardEvent) => {
      event.preventDefault();
      event.stopImmediatePropagation();
      if (
        !session.current?.active ||
        !hostElement.contains(document.activeElement)
      )
        return;
      try {
        const value = pastedText(
          event.clipboardData?.getData("text/plain") ?? "",
        );
        if (!value.text) return;
        if (value.multiline) {
          pasteBlocked.current = true;
          setPaste(value.text);
          term.options.disableStdin = true;
        } else term.paste(value.text);
      } catch {
        setError(
          "Paste rejected: at most 16 KiB of plain text; control characters are not allowed.",
        );
      }
    };
    hostElement.addEventListener("paste", onPaste, true);
    const hidden = () => {
      if (document.visibilityState === "hidden")
        void session.current?.stop(
          "Terminal closed when the application became hidden.",
        );
    };
    document.addEventListener("visibilitychange", hidden);
    void Promise.all([
      ipc.getManagement(scope, () => (alive.current ? scope : null)),
      ipc.getTerminalPermission(scope, () => (alive.current ? scope : null)),
    ])
      .then(
        ([management, permission]) => {
          if (alive.current) {
            setManaged(management.enabled);
            setEnabled(permission.enabled);
          }
        },
        (e) => {
          if (alive.current) setError(errorText(e));
        },
      )
      .finally(() => {
        if (alive.current) setInitializing(false);
      });
    return () => {
      alive.current = false;
      pasteBlocked.current = true;
      cancelAnimationFrame(frame);
      observer.disconnect();
      hostElement.removeEventListener("paste", onPaste, true);
      document.removeEventListener("visibilitychange", hidden);
      void session.current?.stop("Terminal tab closed.");
      session.current = null;
      input.dispose();
      binary.dispose();
      selection.dispose();
      for (const hook of hooks) hook.dispose();
      term.dispose();
      terminal.current = null;
    };
  }, [scope]);
  useEffect(() => {
    if (!intent) return;
    setExpired(false);
    const timer = setTimeout(() => setExpired(true), intent.expiresInMs);
    return () => clearTimeout(timer);
  }, [intent]);
  async function perform(action: () => Promise<void>) {
    if (lock.current) return;
    lock.current = true;
    setBusy(true);
    setError("");
    try {
      await action();
    } catch (e) {
      if (alive.current) setError(errorText(e));
    } finally {
      lock.current = false;
      if (alive.current) setBusy(false);
    }
  }
  function update(value: TerminalNotice) {
    if (!alive.current) return;
    setNotice(value);
    const term = terminal.current;
    if (term)
      term.options.disableStdin =
        value.phase !== "connected" || pasteBlocked.current;
    if (value.phase === "closed") {
      pasteBlocked.current = false;
      setPaste(null);
    }
  }
  async function connect() {
    if (!intent || expired || intent.operation.category !== "terminal") return;
    const chosen = intent;
    const spec = intent.operation.spec;
    setIntent(null);
    terminal.current?.reset();
    const owner = await ipc.openTerminal(
      { scope, intentId: chosen.id, spec },
      current,
    );
    if (!alive.current) {
      await ipc.closeTerminal(owner);
      return;
    }
    const transport = new TerminalSession(
      owner,
      current,
      (bytes) =>
        new Promise<void>((resolve, reject) => {
          if (!alive.current || !terminal.current) {
            resolve();
            return;
          }
          const deadline = setTimeout(
            () => reject(new ipc.IpcError("resource_limit")),
            2000,
          );
          terminal.current.write(bytes, () => {
            clearTimeout(deadline);
            resolve();
          });
        }),
      update,
    );
    session.current = transport;
    transport.start();
    if (terminal.current) {
      transport.resize(terminal.current.cols, terminal.current.rows);
      terminal.current.focus();
    }
  }
  function finishPaste(confirmed: boolean) {
    const text = paste;
    pasteBlocked.current = false;
    setPaste(null);
    const term = terminal.current;
    if (!term) return;
    term.options.disableStdin = !session.current?.active;
    if (confirmed && text !== null && session.current?.active) term.paste(text);
    term.focus();
  }
  const connected = notice.phase !== "closed";
  return (
    <section className="terminal-panel" aria-label="Container terminal">
      <header className="terminal-identity">
        <strong>
          {host} · {name}
        </strong>
        <code>{id}</code>
        <span>Daemon: {scope.daemonId} · user 1000:1000</span>
      </header>
      <p className="terminal-status" role="status">
        {notice.message}
      </p>
      <p className="muted">
        Interactive input can change this container. Closing this tab closes its
        terminal. No automatic reconnect. Ctrl+Shift+Escape leaves terminal
        focus.
      </p>
      <div className="terminal-controls">
        {!managed && (
          <button
            className="button"
            type="button"
            disabled={busy || initializing}
            onClick={() =>
              void perform(async () => {
                const value = await ipc.setManagement(scope, true, current);
                if (alive.current) setManaged(value.enabled);
              })
            }
          >
            Enable management for terminal
          </button>
        )}
        <button
          className="button"
          type="button"
          disabled={busy || initializing || !managed}
          onClick={() =>
            void perform(async () => {
              setIntent(null);
              const value = await ipc.setTerminalPermission(
                scope,
                !enabled,
                current,
              );
              if (alive.current) setEnabled(value.enabled);
              if (!value.enabled)
                await session.current?.stop("Terminal access revoked.");
            })
          }
        >
          {enabled ? "Disable terminal access" : "Enable terminal access"}
        </button>
        <label>
          Terminal shell
          <select
            value={shell}
            disabled={busy || connected}
            onChange={(e) => {
              setShell(e.target.value as TerminalShell);
              setIntent(null);
            }}
          >
            <option value="sh">/bin/sh</option>
            <option value="bash">/bin/bash</option>
          </select>
        </label>
        <button
          className="button"
          type="button"
          disabled={busy || initializing || !enabled || connected}
          onClick={() =>
            void perform(async () => {
              const term = terminal.current;
              const value = await ipc.prepareTerminal(
                scope,
                {
                  containerId: id,
                  shell,
                  columns: term?.cols ?? 80,
                  rows: term?.rows ?? 24,
                },
                current,
              );
              if (alive.current) setIntent(value);
            })
          }
        >
          Open terminal
        </button>
        <button
          ref={closeButton}
          className="button"
          type="button"
          disabled={!connected || notice.phase === "closing"}
          onClick={() => void session.current?.stop()}
        >
          Disconnect terminal
        </button>
        <button
          className="button"
          type="button"
          disabled={!selected}
          onClick={() =>
            void perform(async () => {
              const text = terminal.current?.getSelection() ?? "";
              if (new TextEncoder().encode(text).length > 65536)
                throw new ipc.IpcError("invalid_limits");
              if (text) await navigator.clipboard.writeText(text);
            })
          }
        >
          Copy terminal selection
        </button>
      </div>
      {intent?.operation.category === "terminal" && (
        <div
          className="mutation-confirmation"
          role="dialog"
          aria-label="Confirm terminal access"
        >
          <h4>Open interactive shell</h4>
          <p>
            {host} · {name}
          </p>
          <code>{id}</code>
          <p>
            Daemon: {scope.daemonId} · /bin/{intent.operation.spec.shell} · user
            1000:1000 · {intent.operation.spec.columns}×
            {intent.operation.spec.rows}
          </p>
          <p>
            {expired
              ? "Confirmation expired. Review the target again."
              : "Input may perform arbitrary changes within this user's container permissions."}
          </p>
          <button
            className="button"
            type="button"
            disabled={busy || expired}
            onClick={() => void perform(connect)}
          >
            Confirm terminal
          </button>
          <button
            className="button"
            type="button"
            onClick={() => setIntent(null)}
          >
            Cancel terminal
          </button>
        </div>
      )}
      {paste !== null && (
        <div
          className="terminal-paste"
          role="dialog"
          aria-label="Confirm multiline paste"
        >
          <h4>Paste multiple lines into {name}?</h4>
          <p>
            Newlines may execute commands. Review the entire text before
            sending.
          </p>
          <pre>{paste}</pre>
          <button
            className="button"
            type="button"
            onClick={() => finishPaste(true)}
          >
            Send multiline paste
          </button>
          <button
            className="button"
            type="button"
            onClick={() => finishPaste(false)}
          >
            Cancel paste
          </button>
        </div>
      )}
      {error && <p role="alert">{error}</p>}
      <p className="terminal-size">
        {dimensions.columns}×{dimensions.rows}
      </p>
      <section
        ref={surface}
        className="terminal-surface"
        aria-label={`Terminal for ${host} / ${name}`}
      />
      <p className="muted">
        Scrollback: 1,000 lines. Copy only selected text intentionally.
        Clipboard and link escape commands are disabled. Idle terminals close
        after ten minutes.
      </p>
    </section>
  );
}
