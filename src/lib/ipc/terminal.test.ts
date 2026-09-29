import assert from "node:assert/strict";
import { test, beforeEach, afterEach } from "node:test";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import {
  openTerminal,
  readTerminal,
  writeTerminal,
  getTerminalPermission,
} from "./client.ts";
import {
  TerminalSession,
  pastedText,
  type TerminalBridge,
} from "../../features/terminal/session.ts";
import type { SessionScope, TerminalOutput } from "./generated.ts";
const scope: SessionScope = {
  selection: { hostId: `h_${"a".repeat(32)}`, selectionGeneration: 1 },
  sessionId: `s_${"b".repeat(32)}`,
  sessionGeneration: 1,
  daemonId: "terminal-fixture",
};
const key = { scope, terminalId: `sub_${"c".repeat(32)}` };
const request = {
  scope,
  intentId: `i_${"d".repeat(32)}`,
  spec: {
    containerId: "e".repeat(64),
    shell: "sh" as const,
    columns: 80,
    rows: 24,
  },
};
const running: TerminalOutput = {
  ...key,
  sequence: 1,
  bytes: [],
  state: "running",
  exitCode: null,
  error: null,
};
const settle = async () => {
  for (let n = 0; n < 12; n++) await Promise.resolve();
};
beforeEach(() =>
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: { crypto: globalThis.crypto },
  }),
);
afterEach(() => {
  clearMocks();
  Reflect.deleteProperty(globalThis, "window");
});
test("late terminal opening closes its exact old handle once and never reconnects", async () => {
  let current: SessionScope | null = scope;
  let release: ((v: unknown) => void) | undefined;
  const calls: string[] = [];
  mockIPC((command, args) => {
    calls.push(command);
    if (command === "open_container_terminal")
      return new Promise((resolve) => {
        release = resolve;
      });
    assert.equal(command, "close_terminal");
    assert.deepEqual(args, { request: key });
    return null;
  });
  const pending = openTerminal(request, () => current);
  current = null;
  assert.ok(release);
  release(key);
  await assert.rejects(pending, { code: "stale_session" });
  assert.deepEqual(calls, ["open_container_terminal", "close_terminal"]);
});
test("terminal IPC rejects foreign streams and malformed bytes; input transport failures never retry", async () => {
  mockIPC((command, args) => {
    assert.equal(command, "get_terminal_permission");
    assert.deepEqual(args, { request: { scope } });
    return { scope, enabled: false };
  });
  assert.equal(
    (await getTerminalPermission(scope, () => scope)).enabled,
    false,
  );
  for (const value of [
    { ...running, terminalId: `sub_${"f".repeat(32)}` },
    { ...running, bytes: [256] },
    { ...running, bytes: Array(32769).fill(0) },
    { ...running, sequence: 0 },
    { ...running, scope: { ...scope, daemonId: "other" } },
  ]) {
    mockIPC(() => value);
    await assert.rejects(
      readTerminal(key, () => scope),
      { code: "invalid_response" },
    );
  }
  let writes = 0;
  mockIPC(() => {
    writes++;
    throw {
      code: "transport_unavailable",
      message: "must not expose remote data",
    };
  });
  await assert.rejects(
    writeTerminal({ ...key, sequence: 1, bytes: [65] }, () => scope),
    { code: "transport_unavailable" },
  );
  assert.equal(writes, 1);
});
test("tab closure fences pending output and queued input; two host owners cannot share keystrokes", async () => {
  const reads: Array<(output: TerminalOutput) => void> = [];
  const writes: Array<{
    terminalId: string;
    sequence: number;
    bytes: number[];
  }> = [];
  const closes: string[] = [];
  let releaseWrite: (() => void) | undefined;
  const bridge: TerminalBridge = {
    readTerminal: () => new Promise((resolve) => reads.push(resolve)),
    writeTerminal: async (value) => {
      writes.push(value);
      await new Promise<void>((resolve) => {
        releaseWrite = resolve;
      });
    },
    resizeTerminal: async () => {},
    closeTerminal: async (value) => {
      closes.push(value.terminalId);
    },
  };
  let current = scope;
  let rendered = 0;
  const first = new TerminalSession(
    key,
    () => current,
    async () => {
      rendered++;
    },
    () => {},
    bridge,
  );
  first.start();
  reads.shift()?.(running);
  await settle();
  first.input(new Uint8Array([65]));
  first.input(new Uint8Array([66]));
  assert.equal(writes.length, 1);
  await first.stop();
  current = {
    ...scope,
    selection: { ...scope.selection, hostId: `h_${"f".repeat(32)}` },
    sessionGeneration: 2,
  };
  releaseWrite?.();
  await settle();
  first.input(new Uint8Array([67]));
  assert.equal(writes.length, 1);
  const other = { scope: current, terminalId: `sub_${"d".repeat(32)}` };
  const second = new TerminalSession(
    other,
    () => current,
    async () => {
      rendered++;
    },
    () => {},
    bridge,
  );
  second.start();
  reads.shift()?.({ ...running, ...other });
  await settle();
  second.input(new Uint8Array([68]));
  assert.equal(writes[1]?.terminalId, other.terminalId);
  assert.equal(writes[1]?.sequence, 1);
  await second.stop();
  releaseWrite?.();
  await settle();
  assert.deepEqual(closes, [key.terminalId, other.terminalId]);
  const late = new TerminalSession(
    key,
    () => scope,
    async () => {
      rendered++;
    },
    () => {},
    bridge,
  );
  late.start();
  await late.stop();
  reads.shift()?.({ ...running, bytes: [88] });
  await settle();
  assert.equal(rendered, 0);
});
test("lost input response and frontend pressure close without replay; paste is bounded and multiline is explicit", async () => {
  let writes = 0,
    closes = 0;
  const messages: string[] = [];
  const bridge: TerminalBridge = {
    readTerminal: async () => running,
    writeTerminal: async () => {
      writes++;
      throw Error("synthetic");
    },
    resizeTerminal: async () => {},
    closeTerminal: async () => {
      closes++;
    },
  };
  const connection = new TerminalSession(
    key,
    () => scope,
    async () => {},
    (v) => messages.push(v.message),
    bridge,
  );
  connection.start();
  await settle();
  connection.input(new Uint8Array([65]));
  connection.input(new Uint8Array([66]));
  await settle();
  assert.equal(writes, 1);
  assert.equal(closes, 1);
  assert.ok(messages.some((s) => s.includes("unknown")));
  const pressure = new TerminalSession(
    key,
    () => scope,
    async () => {},
    () => {},
    bridge,
  );
  pressure.start();
  await settle();
  pressure.input(new Uint8Array(65537));
  await settle();
  assert.equal(writes, 1);
  assert.equal(closes, 2);
  assert.deepEqual(pastedText("one\r\ntwo\u2028three"), {
    text: "one\ntwo\nthree",
    multiline: true,
  });
  assert.equal(pastedText("one\ttwo").multiline, false);
  for (const text of ["\x1b[200~bad", "bad\0text", "x".repeat(16373)])
    assert.throws(() => pastedText(text));
});
