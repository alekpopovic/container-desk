import assert from "node:assert/strict";
import { afterEach, beforeEach, test } from "node:test";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import {
  getActivity,
  cancelMutation,
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
const results = [
  {
    containerId: "a".repeat(64),
    outcome: "unknown",
    dispatched: true,
    error: null,
  },
];
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
  mockIPC(() => ({ scope, spec, outcome: "unknown", results }));
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
    results,
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

test("batch response preserves ordered partial outcomes and rejects aggregate or target corruption", async () => {
  const batch = {
    ...spec,
    containerIds: ["a".repeat(64), "b".repeat(64), "c".repeat(64)],
  };
  const response = {
    scope,
    spec: batch,
    outcome: "partial",
    results: [
      {
        containerId: batch.containerIds[0],
        outcome: "succeeded",
        dispatched: true,
        error: null,
      },
      {
        containerId: batch.containerIds[1],
        outcome: "failed",
        dispatched: false,
        error: "container_not_found",
      },
      {
        containerId: batch.containerIds[2],
        outcome: "failed",
        dispatched: true,
        error: "permission_denied",
      },
    ],
  };
  mockIPC(() => response);
  assert.deepEqual(
    (await mutateContainer(scope, batch, intentId, () => scope)).results,
    response.results,
  );
  for (const invalid of [
    { ...response, outcome: "succeeded" },
    { ...response, results: [...response.results].reverse() },
    {
      ...response,
      results: response.results.map((r) => ({ ...r, dispatched: false })),
    },
  ]) {
    mockIPC(() => invalid);
    await assert.rejects(
      mutateContainer(scope, batch, intentId, () => scope),
      { code: "invalid_response" },
    );
  }
  let calls = 0;
  mockIPC((command, args) => {
    calls++;
    assert.equal(command, "cancel_mutation");
    assert.deepEqual(args, { request: { scope, intentId } });
    return { pendingCancellationRequested: true };
  });
  assert.equal(
    (await cancelMutation(scope, intentId)).pendingCancellationRequested,
    true,
  );
  assert.equal(calls, 1);
});
