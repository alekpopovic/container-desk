import * as ipc from "../../lib/ipc/client.ts";
import type {
  SessionScope,
  TerminalHandleRequest,
} from "../../lib/ipc/generated.ts";
export type TerminalBridge = Pick<
  typeof ipc,
  "readTerminal" | "writeTerminal" | "resizeTerminal" | "closeTerminal"
>;
export type TerminalNotice = {
  phase: "connecting" | "connected" | "closing" | "closed";
  message: string;
};
export class TerminalSession {
  private key: TerminalHandleRequest;
  private current: () => SessionScope | null;
  private bridge: TerminalBridge;
  private render: (bytes: Uint8Array) => Promise<void>;
  private notice: (value: TerminalNotice) => void;
  private stopped = false;
  private ready = false;
  private closing: Promise<void> | null = null;
  private queue: Uint8Array[] = [];
  private queuedBytes = 0;
  private sending = false;
  private sequence = 0;
  private outputSequence = 0;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private desired: { columns: number; rows: number } | null = null;
  private resizing = false;
  constructor(
    key: TerminalHandleRequest,
    current: () => SessionScope | null,
    render: (bytes: Uint8Array) => Promise<void>,
    notice: (value: TerminalNotice) => void,
    bridge: TerminalBridge = ipc,
  ) {
    this.key = structuredClone(key);
    this.current = current;
    this.render = render;
    this.notice = notice;
    this.bridge = bridge;
  }
  start() {
    this.notice({ phase: "connecting", message: "Connecting terminal…" });
    void this.poll();
  }
  get active() {
    return (
      !this.stopped &&
      this.ready &&
      ipc.sameScope(this.key.scope, this.current())
    );
  }
  input(bytes: Uint8Array) {
    if (!this.active || !bytes.length) return;
    if (
      this.queuedBytes + bytes.length > 65536 ||
      this.queue.length + Math.ceil(bytes.length / 16384) > 32
    ) {
      void this.stop(
        "Input queue is full. Terminal closed; queued input was not replayed.",
      );
      return;
    }
    for (let start = 0; start < bytes.length; start += 16384)
      this.queue.push(bytes.slice(start, start + 16384));
    this.queuedBytes += bytes.length;
    void this.send();
  }
  private async send() {
    if (this.sending) return;
    this.sending = true;
    try {
      while (this.active && this.queue.length) {
        const bytes = this.queue.shift();
        if (!bytes) break;
        await this.bridge.writeTerminal(
          { ...this.key, sequence: ++this.sequence, bytes: Array.from(bytes) },
          this.current,
        );
        if (this.stopped) break;
        this.queuedBytes -= bytes.length;
      }
    } catch {
      void this.stop(
        "Input outcome is unknown. Terminal closed; input will not be replayed.",
      );
    } finally {
      this.sending = false;
    }
  }
  resize(columns: number, rows: number) {
    this.desired = { columns, rows };
    void this.fit();
  }
  private async fit() {
    if (this.resizing || !this.active) return;
    this.resizing = true;
    try {
      while (this.active && this.desired) {
        const value = this.desired;
        this.desired = null;
        await this.bridge.resizeTerminal(
          { ...this.key, ...value },
          this.current,
        );
      }
    } catch {
      void this.stop("Terminal resize failed. Open a new terminal explicitly.");
    } finally {
      this.resizing = false;
    }
  }
  private async poll() {
    try {
      if (this.stopped) return;
      if (!ipc.sameScope(this.key.scope, this.current())) {
        await this.stop("Terminal selection changed.");
        return;
      }
      const output = await this.bridge.readTerminal(this.key, this.current);
      if (this.stopped) return;
      if (output.sequence !== this.outputSequence + 1)
        throw new ipc.IpcError("invalid_response");
      this.outputSequence = output.sequence;
      const wasReady = this.ready;
      this.ready = output.state === "running";
      if (this.ready && !wasReady) {
        this.notice({
          phase: "connected",
          message: "Connected · input goes only to this container",
        });
        void this.fit();
      }
      if (output.bytes.length) await this.render(Uint8Array.from(output.bytes));
      if (this.stopped) return;
      if (output.state === "exited" && output.bytes.length === 0) {
        await this.stop(
          output.error
            ? new ipc.IpcError(output.error).message
            : `Terminal exited${output.exitCode === null ? "" : ` with code ${output.exitCode}`}.`,
        );
        return;
      }
      this.timer = setTimeout(
        () => {
          this.timer = null;
          void this.poll();
        },
        output.bytes.length ? 0 : 50,
      );
    } catch (error) {
      await this.stop(
        error instanceof ipc.IpcError
          ? error.message
          : "Terminal output stopped.",
      );
    }
  }
  stop(message = "Terminal disconnected."): Promise<void> {
    if (this.closing) return this.closing;
    this.stopped = true;
    this.ready = false;
    this.queue = [];
    this.queuedBytes = 0;
    this.desired = null;
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
    this.notice({ phase: "closing", message: "Closing terminal…" });
    this.closing = this.bridge.closeTerminal(this.key).then(
      () => this.notice({ phase: "closed", message }),
      () =>
        this.notice({
          phase: "closed",
          message:
            "Terminal close could not be confirmed. Input is disabled; no reconnect or replay.",
        }),
    );
    return this.closing;
  }
}
export function pastedText(text: string): { text: string; multiline: boolean } {
  const normalized = text
    .replace(/\r\n?/g, "\n")
    .replace(/[\u2028\u2029]/g, "\n");
  if (
    new TextEncoder().encode(normalized).length > 16372 ||
    Array.from(normalized).some((character) => {
      const code = character.charCodeAt(0);
      return code === 127 || (code < 32 && code !== 9 && code !== 10);
    })
  )
    throw new ipc.IpcError("invalid_limits");
  return { text: normalized, multiline: normalized.includes("\n") };
}
