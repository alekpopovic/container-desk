import assert from "node:assert/strict";
import { test } from "node:test";
import { ReadScheduler, type ReadCommand } from "../reads/scheduler.ts";
import { watchReadRecovery } from "../reads/recovery.ts";
class FakeClock {
  time = 0;
  serial = 0;
  timers = new Map<number, { at: number; run: () => void }>();
  now = () => this.time;
  random = () => 0;
  schedule = (run: () => void, ms: number) => {
    const id = ++this.serial;
    this.timers.set(id, { at: this.time + ms, run });
    return id;
  };
  cancel = (id: unknown) => {
    this.timers.delete(id as number);
  };
  async tick(ms: number) {
    const end = this.time + ms;
    while (true) {
      const next = [...this.timers].sort((a, b) => a[1].at - b[1].at)[0];
      if (!next || next[1].at > end) break;
      this.time = next[1].at;
      this.timers.delete(next[0]);
      next[1].run();
      await settle();
    }
    this.time = end;
    await settle();
  }
}
const settle = async () => {
  for (let n = 0; n < 8; n++) await Promise.resolve();
};
const error = (code: string) => Object.assign(Error(code), { code });
test("slow host single-flight, maximum two reads, navigation ahead of queued stats and independent cancellation", async () => {
  const clock = new FakeClock();
  const scheduler = new ReadScheduler(error, clock);
  let active = 0,
    maximum = 0;
  const starts: string[] = [];
  const finish = new Map<string, () => void>();
  const run = (key: string, command: ReadCommand) =>
    scheduler.run(
      "host",
      key,
      command,
      () => true,
      () => {
        active++;
        maximum = Math.max(maximum, active);
        starts.push(key);
        return new Promise<string>((resolve) =>
          finish.set(key, () => {
            active--;
            resolve(key);
          }),
        );
      },
    );
  const first = run("list", "list_containers");
  const duplicates = Array.from({ length: 20 }, () =>
    run("list", "list_containers"),
  );
  const stats = run("stats", "container_stats");
  const queued = run("later-stats", "container_stats");
  const inspect = run("selected-inspect", "inspect_container");
  assert.deepEqual(starts, ["list", "stats"]);
  finish.get("stats")?.();
  await settle();
  assert.deepEqual(starts, ["list", "stats", "selected-inspect"]);
  finish.get("list")?.();
  await settle();
  assert.deepEqual(starts, [
    "list",
    "stats",
    "selected-inspect",
    "later-stats",
  ]);
  finish.get("selected-inspect")?.();
  finish.get("later-stats")?.();
  await Promise.all([first, stats, queued, inspect, ...duplicates]);
  assert.equal(maximum, 2);
  assert.equal(clock.timers.size, 0);
});
test("four global reads, bounded host/queue/subscriber counts; abandoned reads keep slots until native completion", async () => {
  const clock = new FakeClock();
  const scheduler = new ReadScheduler(error, clock);
  let alive = true;
  let active = 0;
  let maximum = 0;
  const releases: Array<() => void> = [];
  const run = (host: string, key: string) =>
    scheduler.run(
      host,
      key,
      "list_containers",
      () => alive,
      () => {
        active++;
        maximum = Math.max(maximum, active);
        return new Promise<void>((resolve) =>
          releases.push(() => {
            active--;
            resolve();
          }),
        );
      },
    );
  const pending: Promise<unknown>[] = [];
  for (let n = 0; n < 16; n++) pending.push(run("one", String(n)));
  for (let n = 0; n < 31; n++) pending.push(run("one", "0"));
  await assert.rejects(run("one", "overflow"), { code: "resource_limit" });
  await assert.rejects(run("one", "0"), { code: "resource_limit" });
  pending.push(run("two", "a"), run("two", "b"), run("three", "c"));
  await assert.rejects(run("four", "d"), { code: "resource_limit" });
  assert.equal(maximum, 4);
  const cancelled = Promise.allSettled(pending);
  alive = false;
  await clock.tick(100);
  const outcomes = await cancelled;
  assert.ok(outcomes.every((x) => x.status === "rejected"));
  assert.equal(
    active,
    4,
    "in-flight native work cannot be hidden by cancelling its consumer",
  );
  for (const release of releases) release();
  await settle();
  assert.equal(clock.timers.size, 0);
});
test("only transient reads retry, jitter is bounded, retries release slots, total dispatches stop at three", async () => {
  const clock = new FakeClock();
  const scheduler = new ReadScheduler(error, clock);
  const attempts: number[] = [];
  const request = scheduler.run(
    "host",
    "read",
    "list_containers",
    () => true,
    async () => {
      attempts.push(clock.time);
      throw error("operation_timed_out");
    },
  );
  const failure = assert.rejects(request, { code: "operation_timed_out" });
  await settle();
  await clock.tick(2000);
  await failure;
  assert.deepEqual(attempts, [0, 400, 1200]);
  assert.equal(clock.timers.size, 0);
  let calls = 0;
  await assert.rejects(
    scheduler.run(
      "host",
      "denied",
      "inspect_container",
      () => true,
      async () => {
        calls++;
        throw error("permission_denied");
      },
    ),
    { code: "permission_denied" },
  );
  assert.equal(calls, 1);
  let alive = true;
  const late = scheduler.run(
    "host",
    "late",
    "list_containers",
    () => alive,
    async () => {
      calls++;
      throw error("transport_unavailable");
    },
  );
  const cancelled = assert.rejects(late, { code: "stale_session" });
  await settle();
  alive = false;
  await clock.tick(1000);
  await cancelled;
  assert.equal(calls, 2, "stale requests never dispatch retry");
});
test("wake/network/focus signals reconcile once; hidden and disposed watchers do not refresh", async () => {
  const clock = new FakeClock();
  let visible = true;
  let listener = () => {};
  let removed = false;
  let calls = 0;
  const stop = watchReadRecovery(() => calls++, clock, {
    visible: () => visible,
    listen: (run) => {
      listener = run;
      return () => {
        removed = true;
      };
    },
  });
  listener();
  listener();
  assert.equal(calls, 1);
  visible = false;
  await clock.tick(3000);
  listener();
  assert.equal(calls, 1);
  visible = true;
  clock.time += 60000;
  const tick = [...clock.timers.values()][0];
  assert.ok(tick);
  clock.timers.clear();
  tick.run();
  await settle();
  assert.equal(calls, 2);
  listener();
  assert.equal(calls, 2);
  stop();
  listener();
  assert.equal(calls, 2);
  assert.equal(removed, true);
  assert.equal(clock.timers.size, 0);
});

test("real IPC adapter joins duplicate inventory reads and fences a stale consumer without cancelling the other", async () => {
  const { mockIPC, clearMocks } = await import("@tauri-apps/api/mocks");
  const { listContainers, IpcError } = await import("./client.ts");
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: { crypto: globalThis.crypto },
  });
  try {
    const scope = {
      selection: { hostId: `h_${"a".repeat(32)}`, selectionGeneration: 1 },
      sessionId: `s_${"b".repeat(32)}`,
      sessionGeneration: 1,
      daemonId: "read-fixture",
    };
    let calls = 0;
    let release: ((value: unknown) => void) | undefined;
    mockIPC((command) => {
      assert.equal(command, "list_containers");
      calls++;
      return new Promise((resolve) => {
        release = resolve;
      });
    });
    let current: typeof scope | null = scope;
    const old = listContainers(scope, () => current);
    const fresh = listContainers(scope, () => scope);
    const stale = assert.rejects(
      old,
      (e: unknown) => e instanceof IpcError && e.code === "stale_session",
    );
    assert.equal(calls, 1);
    current = null;
    assert.ok(release);
    release({ scope, containers: [] });
    await stale;
    assert.deepEqual(await fresh, { scope, containers: [] });
  } finally {
    clearMocks();
    Reflect.deleteProperty(globalThis, "window");
  }
});

test("mutation invalidation fences old readers, preserves native slots and other hosts, and never retries discarded reads", async () => {
  const clock = new FakeClock();
  const scheduler = new ReadScheduler(error, clock);
  const finish: Array<(v: string) => void> = [];
  const fail: Array<(e: unknown) => void> = [];
  let starts = 0;
  const run = (host: string, key: string) =>
    scheduler.run(
      host,
      key,
      "inspect_container",
      () => true,
      () => {
        starts++;
        return new Promise<string>((resolve, reject) => {
          finish.push(resolve);
          fail.push(reject);
        });
      },
    );
  const old = run("host", "same");
  const duplicate = run("host", "same");
  const second = run("host", "second");
  const queued = run("host", "queued");
  const other = run("other", "same");
  const cancelled = Promise.all(
    [old, duplicate, second, queued].map((p) =>
      assert.rejects(p, { code: "operation_cancelled" }),
    ),
  );
  scheduler.invalidate("host");
  await cancelled;
  const fresh = run("host", "same");
  assert.equal(
    starts,
    3,
    "old active native reads still occupy both host slots",
  );
  fail[0]?.(error("transport_unavailable"));
  await settle();
  assert.equal(starts, 4, "fresh read starts once a real native slot releases");
  finish[1]?.("old");
  finish[2]?.("independent");
  finish[3]?.("fresh");
  assert.equal(await other, "independent");
  assert.equal(await fresh, "fresh");
  await clock.tick(3000);
  assert.equal(starts, 4);
  assert.equal(clock.timers.size, 0);
});
