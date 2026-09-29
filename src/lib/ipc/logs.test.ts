import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { afterEach, beforeEach, test } from "node:test";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { containerLogs } from "./client.ts";
import type { LogSnapshot, SessionScope } from "./generated.ts";
const logs: LogSnapshot = JSON.parse(
  readFileSync(
    new URL("../../../tests/fixtures/ipc.json", import.meta.url),
    "utf8",
  ),
).logs;
const request = {
  scope: logs.scope,
  containerId: logs.containerId,
  tail: 100,
  timeoutSeconds: 30,
  since: null,
  until: null,
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
test("typed log IPC preserves successful stderr data and strict request scope", async () => {
  mockIPC((command, args) => {
    assert.equal(command, "container_logs");
    assert.deepEqual(args, { request });
    return structuredClone(logs);
  });
  assert.deepEqual(await containerLogs(request, () => logs.scope), logs);
  let current: SessionScope | null = logs.scope;
  let finish: ((value: unknown) => void) | undefined;
  mockIPC(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  const pending = containerLogs(request, () => current);
  current = null;
  assert.ok(finish);
  finish(logs);
  await assert.rejects(pending, { code: "stale_session" });
});
test("log IPC enforces UTF-8 byte, count, retention and resource identity bounds", async () => {
  const first = logs.records[0];
  assert.ok(first);
  for (const value of [
    { ...logs, containerId: "b".repeat(64) },
    { ...logs, records: Array(20001).fill(first) },
    { ...logs, records: [{ ...first, text: "é".repeat(131073) }] },
    {
      ...logs,
      records: Array(33).fill({ ...first, text: "x".repeat(262144) }),
    },
    { ...logs, records: [{ ...first, channel: "trusted_application_stderr" }] },
  ]) {
    mockIPC(() => value);
    await assert.rejects(
      containerLogs(request, () => logs.scope),
      { code: "invalid_response" },
    );
  }
});
test("log errors remain static and never replay failed reads implicitly", async () => {
  let calls = 0;
  mockIPC(() => {
    calls += 1;
    throw { code: "log_driver_unsupported", message: "raw-secret-log" };
  });
  await assert.rejects(
    containerLogs(request, () => logs.scope),
    (e: unknown) =>
      e instanceof Error &&
      e.message.includes("logging driver") &&
      !e.message.includes("raw-secret"),
  );
  assert.equal(calls, 1);
});
