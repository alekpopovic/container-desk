import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { ContainerCache } from "../../features/containers/cache.ts";
import type { ContainerSummary, SessionScope } from "./generated.ts";
const fixtures = JSON.parse(
  readFileSync(
    new URL("../../../tests/fixtures/ipc.json", import.meta.url),
    "utf8",
  ),
);
const first: SessionScope = fixtures.success.scope;
function row(scope: SessionScope, id = "a".repeat(64)): ContainerSummary {
  return { ...fixtures.success.containers[0], scope, id, name: "Same name" };
}
test("host and daemon identities isolate identical names/IDs and out-of-order replies", () => {
  let now = 100;
  const cache = new ContainerCache(() => now);
  cache.activate(first);
  const old = cache.begin();
  assert.ok(old);
  const second = {
    ...first,
    selection: { ...first.selection, hostId: `h_${"f".repeat(32)}` },
    sessionId: `s_${"e".repeat(32)}`,
  };
  cache.activate(second);
  const fresh = cache.begin();
  assert.ok(fresh);
  cache.complete(fresh, { scope: second, containers: [row(second)] });
  cache.select("a".repeat(64));
  now = 200;
  cache.complete(old, {
    scope: first,
    containers: [row(first, "b".repeat(64))],
  });
  assert.equal(cache.getSnapshot().updatedAt, 100);
  assert.equal(
    cache.getSnapshot().rows[0]?.scope.selection.hostId,
    second.selection.hostId,
  );
  cache.activate({ ...second, daemonId: "replacement-daemon" });
  assert.equal(cache.getSnapshot().rows.length, 0);
  assert.equal(cache.getSnapshot().selectedId, null);
});
test("refresh retains selection by full ID, exposes stale data on error and clears disappeared resources", () => {
  let now = 10;
  const cache = new ContainerCache(() => now);
  cache.activate(first);
  const initial = cache.begin();
  assert.ok(initial);
  assert.equal(cache.begin(), null, "one inflight refresh");
  cache.complete(initial, { scope: first, containers: [row(first)] });
  cache.select("a".repeat(64));
  const failure = cache.begin();
  assert.ok(failure);
  cache.fail(failure, "Unavailable");
  assert.equal(cache.getSnapshot().stale, true);
  assert.equal(cache.getSnapshot().updatedAt, 10);
  assert.equal(cache.getSnapshot().selectedId, "a".repeat(64));
  const retry = cache.begin();
  assert.ok(retry);
  now = 20;
  cache.complete(retry, { scope: first, containers: [] });
  assert.equal(cache.getSnapshot().selectedId, null);
  assert.equal(cache.getSnapshot().stale, false);
  assert.equal(cache.getSnapshot().updatedAt, 20);
  cache.activate({ ...first, sessionGeneration: first.sessionGeneration + 1 });
  assert.equal(
    cache.getSnapshot().stale,
    true,
    "even an old empty success is stale on reconnect",
  );
  const pending = cache.begin();
  assert.ok(pending);
  cache.unavailable();
  cache.complete(pending, {
    scope: pending.scope,
    containers: [row(pending.scope)],
  });
  assert.equal(cache.getSnapshot().rows.length, 0);
});
test("snapshot retention is limited to three host/daemon pairs", () => {
  const cache = new ContainerCache();
  for (let n = 0; n < 4; n++) {
    const scope = { ...first, daemonId: `daemon-${n}` };
    cache.activate(scope);
    const ticket = cache.begin();
    assert.ok(ticket);
    cache.complete(ticket, { scope, containers: [row(scope)] });
  }
  cache.activate({ ...first, daemonId: "daemon-0" });
  assert.equal(cache.getSnapshot().updatedAt, null);
});

test("demo and live never reuse snapshots even if metadata identities coincide", () => {
  const cache = new ContainerCache();
  cache.setNamespace("demo");
  cache.activate(first);
  const ticket = cache.begin();
  assert.ok(ticket);
  cache.complete(ticket, { scope: first, containers: [row(first)] });
  cache.setNamespace("live");
  cache.activate(first);
  assert.equal(cache.getSnapshot().updatedAt, null);
  assert.equal(cache.getSnapshot().rows.length, 0);
});
