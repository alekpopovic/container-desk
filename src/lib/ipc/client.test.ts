import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { afterEach, beforeEach, test } from "node:test";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import {
  listHosts,
  connectHost,
  listContainers,
  cancelSubscription,
  IpcError,
  getPreferences,
  setTheme,
} from "./client.ts";
import type { SessionScope } from "./generated.ts";

// These values are written by Rust serde, not a hand-maintained JS fixture.
const fixtures = JSON.parse(
  readFileSync(
    new URL("../../../tests/fixtures/ipc.json", import.meta.url),
    "utf8",
  ),
);
const expected: SessionScope = fixtures.success.scope;
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

test("decodes Rust success fixtures through narrow commands and camelCase requests", async () => {
  mockIPC((command, args) => {
    if (command === "list_hosts") return fixtures.hosts;
    if (command === "connect_host") {
      assert.deepEqual(args, { request: { selection: expected.selection } });
      return fixtures.connected;
    }
    assert.equal(command, "list_containers");
    assert.deepEqual(args, { request: { scope: expected } });
    return fixtures.success;
  });
  assert.deepEqual(await listHosts(), fixtures.hosts);
  assert.deepEqual(
    await connectHost(
      { selection: expected.selection },
      () => expected.selection,
    ),
    fixtures.connected,
  );
  assert.deepEqual(
    await listContainers(expected, () => expected),
    fixtures.success,
  );
  assert.equal(fixtures.detail.environmentValuesMasked, true);
  assert.equal(Object.hasOwn(fixtures.detail, "environmentValues"), false);
});

test("Rust error fixture stays structured and is never retried", async () => {
  let calls = 0;
  mockIPC(() => {
    calls += 1;
    throw fixtures.error;
  });
  await assert.rejects(
    listContainers(expected, () => expected),
    (error: unknown) => {
      assert.ok(error instanceof IpcError);
      assert.equal(error.code, "session_not_found");
      assert.deepEqual(error.scope, expected);
      return true;
    },
  );
  assert.equal(calls, 1);
});

test("rejects a late result after selection, session or daemon changes", async () => {
  for (const changed of [
    {
      ...expected,
      selection: { ...expected.selection, hostId: `h_${"3".repeat(32)}` },
    },
    {
      ...expected,
      selection: { ...expected.selection, selectionGeneration: 8 },
    },
    { ...expected, sessionId: `s_${"3".repeat(32)}` },
    { ...expected, sessionGeneration: 4 },
    { ...expected, daemonId: "different" },
    null,
  ]) {
    let active: SessionScope | null = expected;
    mockIPC(async () => {
      await Promise.resolve();
      active = changed;
      return fixtures.success;
    });
    await assert.rejects(
      listContainers(expected, () => active),
      { code: "stale_session" },
    );
  }
  mockIPC(() => fixtures.connected);
  await assert.rejects(
    connectHost({ selection: expected.selection }, () => null),
    { code: "stale_session" },
  );
});

test("rejects malformed or cross-session rows and sanitizes untyped errors", async () => {
  for (const result of [
    null,
    {},
    { scope: expected, containers: [{}] },
    {
      ...fixtures.success,
      containers: [
        {
          ...fixtures.success.containers[0],
          scope: { ...expected, daemonId: "other" },
        },
      ],
    },
  ]) {
    mockIPC(() => result);
    await assert.rejects(
      listContainers(expected, () => expected),
      { code: "invalid_response" },
    );
  }
  mockIPC(() => {
    throw "RAW_SECRET_SENTINEL";
  });
  await assert.rejects(listHosts(), (error: unknown) => {
    assert.ok(error instanceof IpcError);
    assert.equal(error.code, "transport_unavailable");
    assert.ok(!error.message.includes("RAW_SECRET"));
    return true;
  });
});

test("cancellation retains exact scope and subscription ownership", async () => {
  const request = { scope: expected, subscriptionId: `sub_${"4".repeat(32)}` };
  mockIPC((command, args) => {
    assert.equal(command, "cancel_subscription");
    assert.deepEqual(args, { request });
    return request;
  });
  assert.deepEqual(await cancelSubscription(request, () => expected), request);
  mockIPC(() => ({ ...request, subscriptionId: "wrong" }));
  await assert.rejects(
    cancelSubscription(request, () => expected),
    { code: "invalid_response" },
  );
});

test("late errors cannot replace the state of a newly selected host", async () => {
  mockIPC(() => {
    throw fixtures.error;
  });
  await assert.rejects(
    listContainers(expected, () => null),
    { code: "stale_session" },
  );
  await assert.rejects(
    connectHost({ selection: expected.selection }, () => null),
    { code: "stale_session" },
  );
});

test("preferences fixture and theme save preserve revision and reject invalid replies", async () => {
  mockIPC((command, args) => {
    if (command === "get_preferences") return fixtures.preferences;
    assert.equal(command, "set_theme");
    assert.deepEqual(args, { request: { theme: "dark", expectedRevision: 0 } });
    return {
      ...fixtures.preferences,
      preferences: {
        ...fixtures.preferences.preferences,
        theme: "dark",
        revision: 1,
      },
    };
  });
  assert.deepEqual(await getPreferences(), fixtures.preferences);
  assert.equal(
    (await setTheme({ theme: "dark", expectedRevision: 0 })).preferences.theme,
    "dark",
  );
  mockIPC(() => fixtures.preferences);
  await assert.rejects(setTheme({ theme: "dark", expectedRevision: 0 }), {
    code: "invalid_response",
  });
  mockIPC(() => ({ preferences: { theme: "impossible" } }));
  await assert.rejects(getPreferences(), { code: "invalid_response" });
});
