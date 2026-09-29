import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { afterEach, beforeEach, test } from "node:test";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import type { Channel } from "@tauri-apps/api/core";
import { followContainerLogs } from "./client.ts";
import type { LogBatch, LogSnapshot } from "./generated.ts";
const snapshot: LogSnapshot = JSON.parse(
  readFileSync(
    new URL("../../../tests/fixtures/ipc.json", import.meta.url),
    "utf8",
  ),
).logs;
const id = `sub_${"a".repeat(32)}`;
const request = {
  scope: snapshot.scope,
  containerId: snapshot.containerId,
  tail: 100,
  since: null,
};
const turn = () => new Promise((resolve) => setTimeout(resolve, 0));
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
function batch(sequence = 1): LogBatch {
  return {
    scope: request.scope,
    containerId: request.containerId,
    subscriptionId: id,
    sequence,
    records: snapshot.records,
    droppedRecords: 17,
    gap: true,
    ended: false,
    error: null,
  };
}
test("channel ACK waits for consumption, preserves visible drop metadata and cancels on unmount", async () => {
  let channel: Channel<unknown> | undefined;
  const acks: number[] = [];
  let cancelled = 0;
  mockIPC((command, args) => {
    const payload = args as {
      request: { sequence: number };
      onBatch: Channel<unknown>;
    };
    if (command === "follow_container_logs") {
      channel = payload.onBatch;
      return { scope: request.scope, subscriptionId: id };
    }
    if (command === "ack_container_logs") {
      acks.push(payload.request.sequence);
      return null;
    }
    if (command === "cancel_subscription") {
      cancelled++;
      return { scope: request.scope, subscriptionId: id };
    }
    throw Error("Unexpected command");
  });
  const abort = new AbortController();
  let consume: (() => void) | undefined;
  let received: LogBatch | undefined;
  const errors: unknown[] = [];
  await followContainerLogs(
    request,
    () => request.scope,
    (value) => {
      received = value;
      return new Promise((resolve) => {
        consume = resolve;
      });
    },
    (error) => errors.push(error),
    abort.signal,
  );
  assert.deepEqual(acks, [0]);
  assert.ok(channel);
  channel.onmessage(batch());
  await turn();
  assert.deepEqual(acks, [0]);
  assert.equal(received?.droppedRecords, 17);
  assert.equal(received?.gap, true);
  assert.ok(consume);
  consume();
  await turn();
  assert.deepEqual(acks, [0, 1]);
  abort.abort();
  await turn();
  assert.equal(cancelled, 1);
  channel.onmessage(batch(2));
  await turn();
  assert.deepEqual(acks, [0, 1]);
  assert.deepEqual(errors, []);
});
test("late subscription is cancelled using its returned ID and never acknowledged", async () => {
  let finish: ((value: unknown) => void) | undefined;
  let cancelled = 0;
  mockIPC((command) => {
    if (command === "follow_container_logs")
      return new Promise((resolve) => {
        finish = resolve;
      });
    if (command === "cancel_subscription") {
      cancelled++;
      return { scope: request.scope, subscriptionId: id };
    }
    throw Error("Late subscription must not get ACK");
  });
  const abort = new AbortController();
  const pending = followContainerLogs(
    request,
    () => (abort.signal.aborted ? null : request.scope),
    () => {},
    () => {},
    abort.signal,
  );
  abort.abort();
  assert.ok(finish);
  finish({ scope: request.scope, subscriptionId: id });
  await assert.rejects(pending, { code: "stale_session" });
  assert.equal(cancelled, 1);
});
test("foreign or oversized channel data never reaches consumption", async () => {
  for (const invalid of [
    { ...batch(), containerId: "b".repeat(64) },
    { ...batch(), records: Array(129).fill(snapshot.records[0]) },
    { ...batch(), sequence: 2 },
    {
      ...batch(),
      records: [{ ...snapshot.records[0], text: "é".repeat(140000) }],
    },
  ]) {
    let channel: Channel<unknown> | undefined;
    let consumed = false;
    const errors: string[] = [];
    mockIPC((command, args) => {
      if (command === "follow_container_logs") {
        channel = (args as { onBatch: Channel<unknown> }).onBatch;
        return { scope: request.scope, subscriptionId: id };
      }
      return null;
    });
    const abort = new AbortController();
    await followContainerLogs(
      request,
      () => request.scope,
      () => {
        consumed = true;
      },
      (error) => errors.push(error.code),
      abort.signal,
    );
    assert.ok(channel);
    channel.onmessage(invalid);
    await turn();
    assert.equal(consumed, false);
    assert.deepEqual(errors, ["invalid_response"]);
  }
});

test("next batch may arrive after ACK dispatch but before its invoke promise resolves", async () => {
  let channel: Channel<unknown> | undefined;
  let finishAck: (() => void) | undefined;
  const consumed: number[] = [];
  const errors: string[] = [];
  mockIPC((command, args) => {
    if (command === "follow_container_logs") {
      channel = (args as { onBatch: Channel<unknown> }).onBatch;
      return { scope: request.scope, subscriptionId: id };
    }
    if (
      command === "ack_container_logs" &&
      (args as { request: { sequence: number } }).request.sequence === 1
    )
      return new Promise<void>((resolve) => {
        finishAck = resolve;
      });
    return null;
  });
  const abort = new AbortController();
  await followContainerLogs(
    request,
    () => request.scope,
    (value) => {
      consumed.push(value.sequence);
    },
    (error) => errors.push(error.code),
    abort.signal,
  );
  assert.ok(channel);
  channel.onmessage(batch(1));
  await turn();
  assert.ok(finishAck);
  channel.onmessage(batch(2));
  await turn();
  finishAck();
  await turn();
  assert.deepEqual(consumed, [1, 2]);
  assert.deepEqual(errors, []);
  abort.abort();
  await turn();
});
