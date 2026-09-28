import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { afterEach, beforeEach, test } from "node:test";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { inspectContainer } from "./client.ts";
import type { ContainerDetail, SessionScope } from "./generated.ts";
const detail: ContainerDetail = JSON.parse(
  readFileSync(
    new URL("../../../tests/fixtures/ipc.json", import.meta.url),
    "utf8",
  ),
).detail;
const scope = detail.summary.scope;
const request = {
  scope,
  containerId: detail.summary.id,
  revealSensitive: false,
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
test("inspect default projection is decoded and fails closed if values arrive without reveal", async () => {
  mockIPC((command, args) => {
    assert.equal(command, "inspect_container");
    assert.deepEqual(args, { request });
    return structuredClone(detail);
  });
  assert.deepEqual(await inspectContainer(request, () => scope), detail);
  const leaked = structuredClone(detail);
  const env = leaked.environment[0];
  assert.ok(env);
  env.value = "synthetic-forbidden";
  mockIPC(() => leaked);
  await assert.rejects(
    inspectContainer(request, () => scope),
    { code: "invalid_response" },
  );
  env.masked = false;
  leaked.environmentValuesMasked = false;
  assert.equal(
    (await inspectContainer({ ...request, revealSensitive: true }, () => scope))
      .environment[0]?.value,
    "synthetic-forbidden",
  );
});
test("inspect response is bound to exact full ID and current session even after explicit reveal", async () => {
  const wrong = structuredClone(detail);
  wrong.summary.id = "f".repeat(64);
  mockIPC(() => wrong);
  await assert.rejects(
    inspectContainer(request, () => scope),
    { code: "invalid_response" },
  );
  let current: SessionScope | null = scope;
  let finish: ((v: unknown) => void) | undefined;
  mockIPC(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  const pending = inspectContainer(
    { ...request, revealSensitive: true },
    () => current,
  );
  current = null;
  assert.ok(finish);
  finish(detail);
  await assert.rejects(pending, { code: "stale_session" });
});
test("inspect limits reject malformed precise data and suppress raw remote errors", async () => {
  for (const change of [
    (v: ContainerDetail) => {
      v.networks = Array(129).fill({});
    },
    (v: ContainerDetail) => {
      v.resources.memoryBytes = "9007199254740993.1";
    },
    (v: ContainerDetail) => {
      v.mounts = Array(129).fill({});
    },
  ]) {
    const v = structuredClone(detail);
    change(v);
    mockIPC(() => v);
    await assert.rejects(
      inspectContainer(request, () => scope),
      { code: "invalid_response" },
    );
  }
  mockIPC(() => {
    throw { code: "container_not_found", message: "synthetic-error-secret" };
  });
  await assert.rejects(
    inspectContainer(request, () => scope),
    (e: unknown) =>
      e instanceof Error &&
      !e.message.includes("synthetic") &&
      e.message.includes("no longer exists"),
  );
});
