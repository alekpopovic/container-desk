import assert from "node:assert/strict";
import { afterEach, beforeEach, test } from "node:test";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import {
  getActivity,
  mutateContainer,
  prepareMutation,
  setManagement,
} from "./client.ts";
import type { MutationSpec } from "./generated.ts";
const scope = {
  selection: { hostId: `h_${"a".repeat(32)}`, selectionGeneration: 1 },
  sessionId: `s_${"b".repeat(32)}`,
  sessionGeneration: 1,
  daemonId: "management-fixture",
};
const spec: MutationSpec = {
  operation: "restart",
  containerIds: ["a".repeat(64)],
  timeoutSeconds: 10,
};
const intentId = `i_${"c".repeat(32)}`;
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
test("mutation IPC dispatches once on transient errors and preserves explicit unknown", async () => {
  for (const code of [
    "transport_unavailable",
    "operation_timed_out",
    "resource_limit",
  ]) {
    let calls = 0;
    mockIPC((command, args) => {
      calls++;
      assert.equal(command, "mutate_container");
      assert.deepEqual(args, { request: { scope, spec, intentId } });
      throw { code };
    });
    await assert.rejects(
      mutateContainer(scope, spec, intentId, () => scope),
      { code },
    );
    assert.equal(calls, 1);
  }
  mockIPC(() => ({ scope, spec, outcome: "unknown" }));
  assert.equal(
    (await mutateContainer(scope, spec, intentId, () => scope)).outcome,
    "unknown",
  );
});
test("management and confirmation IPC reject foreign scopes, altered targets and excessive TTL", async () => {
  mockIPC(() => ({ scope: { ...scope, daemonId: "other" }, enabled: true }));
  await assert.rejects(
    setManagement(scope, true, () => scope),
    { code: "invalid_response" },
  );
  const valid = {
    scope,
    id: intentId,
    operation: { category: "mutation", spec },
    expiresInMs: 30000,
  };
  for (const value of [
    { ...valid, expiresInMs: 30001 },
    {
      ...valid,
      operation: {
        category: "mutation",
        spec: { ...spec, containerIds: ["b".repeat(64)] },
      },
    },
    { ...valid, scope: { ...scope, sessionGeneration: 2 } },
  ]) {
    mockIPC(() => value);
    await assert.rejects(
      prepareMutation(scope, spec, () => scope),
      { code: "invalid_response" },
    );
  }
});
test("local activity validation rejects text targets, unsafe timestamps and oversized history", async () => {
  const row = {
    id: intentId,
    hostId: scope.selection.hostId,
    action: "stop",
    targets: spec.containerIds,
    startedAtMs: 100,
    updatedAtMs: 101,
    outcome: "unknown",
  };
  mockIPC(() => [row]);
  assert.equal((await getActivity()).length, 1);
  for (const value of [
    [{ ...row, targets: ["unsafe\ncommand"] }],
    [{ ...row, updatedAtMs: Number.MAX_SAFE_INTEGER }],
    Array(201).fill(row),
  ]) {
    mockIPC(() => value);
    await assert.rejects(getActivity(), { code: "invalid_response" });
  }
});
