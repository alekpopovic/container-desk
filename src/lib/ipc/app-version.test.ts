import assert from "node:assert/strict";
import { afterEach, beforeEach, test } from "node:test";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { getAppVersion } from "./app-version.ts";

beforeEach(() => {
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: { crypto: globalThis.crypto },
  });
});
afterEach(() => {
  clearMocks();
  Reflect.deleteProperty(globalThis, "window");
});

test("requests the registered command and decodes the backend version", async () => {
  const commands: string[] = [];
  mockIPC((command) => {
    commands.push(command);
    return { version: "0.9.2" };
  });
  assert.deepEqual(await getAppVersion(), { version: "0.9.2" });
  assert.deepEqual(commands, ["app_version"]);
});

test("rejects malformed IPC responses instead of showing false success", async () => {
  for (const response of [
    null,
    "0.9.2",
    {},
    { version: 42 },
    { version: "" },
    { version: "x".repeat(129) },
  ]) {
    mockIPC(() => response);
    await assert.rejects(
      getAppVersion(),
      /Invalid application version response/,
    );
  }
});

test("propagates bridge errors without retrying a request", async () => {
  let calls = 0;
  mockIPC(() => {
    calls += 1;
    throw new Error("Bridge unavailable");
  });
  await assert.rejects(getAppVersion(), /Bridge unavailable/);
  assert.equal(calls, 1);
});
