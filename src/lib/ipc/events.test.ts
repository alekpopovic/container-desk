import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { afterEach, beforeEach, test } from "node:test";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import type { Channel } from "@tauri-apps/api/core";
import { followDockerEvents } from "./client.ts";
import type { ContainerEvent, EventBatch } from "./generated.ts";
import {
  EventHints,
  InventoryInvalidator,
} from "../../features/events/invalidation.ts";
const scope = JSON.parse(
  readFileSync(
    new URL("../../../tests/fixtures/ipc.json", import.meta.url),
    "utf8",
  ),
).logs.scope;
const id = `sub_${"a".repeat(32)}`;
const event: ContainerEvent = {
  actorId: "a".repeat(64),
  action: "destroy",
  timestampUnixNanos: "1760000000123456789",
};
const batch = (): EventBatch => ({
  scope,
  subscriptionId: id,
  sequence: 1,
  events: [event],
  droppedRecords: 0,
  gap: true,
  ended: false,
  error: null,
});
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
test("event channel validates scope, IDs, actions, nanoseconds and bounded payload before ACK", async () => {
  for (const invalid of [
    { ...batch(), scope: { ...scope, sessionGeneration: 999 } },
    { ...batch(), events: Array(65).fill(event) },
    { ...batch(), events: [{ ...event, actorId: "short" }] },
    { ...batch(), events: [{ ...event, action: "exec_create" }] },
    {
      ...batch(),
      events: [{ ...event, timestampUnixNanos: "18446744073709551616" }],
    },
    {
      ...batch(),
      events: [{ ...event, timestampUnixNanos: Number("1760000000123456789") }],
    },
  ]) {
    let channel: Channel<unknown> | undefined;
    const acks: number[] = [];
    let cancelled = 0;
    const errors: string[] = [];
    mockIPC((command, args) => {
      const a = args as {
        onBatch: Channel<unknown>;
        request: { sequence: number };
      };
      if (command === "follow_docker_events") {
        channel = a.onBatch;
        return { scope, subscriptionId: id };
      }
      if (command === "ack_docker_events") acks.push(a.request.sequence);
      if (command === "cancel_subscription") cancelled++;
      return null;
    });
    await followDockerEvents(
      { scope, since: null },
      () => scope,
      () => assert.fail("Invalid event consumed"),
      (e) => errors.push(e.code),
      new AbortController().signal,
    );
    assert.ok(channel);
    channel.onmessage(invalid);
    await turn();
    assert.deepEqual(acks, [0]);
    assert.equal(cancelled, 1);
    assert.deepEqual(errors, ["invalid_response"]);
  }
});
test("event ACK follows consumption and scope disposal cancels the owned stream", async () => {
  let channel: Channel<unknown> | undefined;
  let release: (() => void) | undefined;
  const acks: number[] = [];
  let cancelled = 0;
  mockIPC((command, args) => {
    const a = args as {
      onBatch: Channel<unknown>;
      request: { sequence: number };
    };
    if (command === "follow_docker_events") {
      channel = a.onBatch;
      return { scope, subscriptionId: id };
    }
    if (command === "ack_docker_events") acks.push(a.request.sequence);
    if (command === "cancel_subscription") cancelled++;
    return null;
  });
  const controller = new AbortController();
  await followDockerEvents(
    { scope, since: null },
    () => scope,
    () =>
      new Promise<void>((resolve) => {
        release = resolve;
      }),
    () => assert.fail("Valid event rejected"),
    controller.signal,
  );
  assert.ok(channel);
  channel.onmessage(batch());
  await turn();
  assert.deepEqual(acks, [0]);
  assert.ok(release);
  release();
  await turn();
  assert.deepEqual(acks, [0, 1]);
  controller.abort();
  await turn();
  assert.equal(cancelled, 1);
});
test("event hints deduplicate bounded history and preserve greatest nanosecond timestamp", () => {
  const hints = new EventHints();
  assert.equal(hints.accept([event, event]), true);
  assert.equal(hints.accept([event]), false);
  assert.equal(hints.since, "1760000000.123456789");
  hints.accept([{ ...event, timestampUnixNanos: "1" }]);
  assert.equal(hints.since, "1760000000.123456789");
  for (let n = 0; n < 1100; n++)
    hints.accept([{ ...event, timestampUnixNanos: String(n + 2) }]);
  assert.equal(
    hints.accept([event]),
    true,
    "old dedupe keys are evicted instead of accumulating",
  );
});
test("10000 hints coalesce, in-flight events require one further read, dispose cancels queued work", async () => {
  let now = 0;
  let scheduled: (() => void) | null = null;
  let due = 0;
  let calls = 0;
  let release: ((ok: boolean) => void) | undefined;
  const refresh = new InventoryInvalidator(
    () => {
      calls++;
      return new Promise<boolean>((resolve) => {
        release = resolve;
      });
    },
    {
      now: () => now,
      schedule: (run, ms) => {
        assert.equal(scheduled, null);
        scheduled = run;
        due = now + ms;
        return 1;
      },
      cancel: () => {
        scheduled = null;
      },
    },
  );
  const fire = async () => {
    now = due;
    const run = scheduled;
    scheduled = null;
    assert.ok(run);
    run();
    await turn();
  };
  for (let n = 0; n < 10000; n++) refresh.invalidate();
  assert.equal(due, 500);
  await fire();
  assert.equal(calls, 1);
  for (let n = 0; n < 10000; n++) refresh.invalidate();
  assert.equal(scheduled, null);
  assert.ok(release);
  release(true);
  await turn();
  assert.equal(due, 2500);
  await fire();
  assert.equal(calls, 2);
  assert.ok(release);
  release(false);
  await turn();
  assert.equal(due, 4500, "manual read contention retains one bounded retry");
  refresh.dispose();
  assert.equal(scheduled, null);
  refresh.invalidate();
  assert.equal(scheduled, null);
});
