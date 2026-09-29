import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { afterEach, beforeEach, test } from "node:test";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { containerStats } from "./client.ts";
import type { StatsSample, SessionScope } from "./generated.ts";
import { SerialSampler, StatsHistory } from "../../features/stats/sampling.ts";
const stats: StatsSample = JSON.parse(
  readFileSync(
    new URL("../../../tests/fixtures/ipc.json", import.meta.url),
    "utf8",
  ),
).stats;
const request = { scope: stats.scope, containerId: stats.containerId };
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
test("stats IPC keeps Docker percent over 100 and fences stale/foreign/non-finite data", async () => {
  mockIPC((command, args) => {
    assert.equal(command, "container_stats");
    assert.deepEqual(args, { request });
    return stats;
  });
  assert.equal(
    (await containerStats(request, () => stats.scope)).values.cpuPercent,
    234.5,
  );
  for (const bad of [
    { ...stats, containerId: "b".repeat(64) },
    { ...stats, values: { ...stats.values, cpuPercent: NaN } },
    { ...stats, values: { ...stats.values, pids: -1 } },
    { ...stats, availability: "stopped" },
    { ...stats, raw: { ...stats.raw, cpu: "\u001bsecret" } },
  ]) {
    mockIPC(() => bad);
    await assert.rejects(
      containerStats(request, () => stats.scope),
      { code: "invalid_response" },
    );
  }
  let current: SessionScope | null = stats.scope;
  mockIPC(() => {
    current = null;
    return stats;
  });
  await assert.rejects(
    containerStats(request, () => current),
    { code: "stale_session" },
  );
});
test("stats history is bounded and isolates host, daemon, session and full ID; gaps stay null", () => {
  const history = new StatsHistory();
  for (let time = 0; time < 400; time++)
    history.append(stats.scope, stats.containerId, {
      time,
      values: time === 399 ? null : stats.values,
    });
  assert.equal(history.get(stats.scope, stats.containerId).length, 360);
  assert.equal(history.get(stats.scope, stats.containerId)[0]?.time, 40);
  assert.equal(
    history.get(stats.scope, stats.containerId).at(-1)?.values,
    null,
  );
  for (const scope of [
    { ...stats.scope, daemonId: "other" },
    { ...stats.scope, sessionGeneration: 99 },
    {
      ...stats.scope,
      selection: { ...stats.scope.selection, hostId: `h_${"f".repeat(32)}` },
    },
  ])
    assert.equal(history.get(scope, stats.containerId).length, 0);
  for (let i = 0; i < 12; i++)
    history.append(stats.scope, i.toString(16).padStart(64, "0"), {
      time: 500,
      values: null,
    });
  assert.equal(history.get(stats.scope, stats.containerId).length, 0);
});
test("single-flight sampler pauses without late delivery, resumes without overlap and stops permanently on dispose", async () => {
  let reads = 0;
  let complete: (value: number) => void = () => {};
  const delivered: number[] = [];
  const timers: Array<{ task: () => void; delay: number }> = [];
  const sampler = new SerialSampler(
    () => {
      reads++;
      return new Promise<number>((r) => {
        complete = r;
      });
    },
    (v) => delivered.push(v),
    () => assert.fail("unexpected error"),
    {
      schedule: (task, delay) => {
        const item = { task, delay };
        timers.push(item);
        return item;
      },
      cancel: (id) => {
        const index = timers.indexOf(id as (typeof timers)[number]);
        if (index >= 0) timers.splice(index, 1);
      },
    },
  );
  const flush = () => new Promise<void>((r) => setImmediate(r));
  sampler.setActive(true);
  sampler.setActive(true);
  assert.equal(reads, 1);
  assert.equal(timers.length, 0);
  sampler.setActive(false);
  sampler.setActive(true);
  assert.equal(reads, 1);
  complete(1);
  await flush();
  assert.deepEqual(delivered, []);
  assert.equal(timers[0]?.delay, 5000);
  timers.shift()?.task();
  assert.equal(reads, 2);
  sampler.setInterval(30000);
  complete(2);
  await flush();
  assert.deepEqual(delivered, [2]);
  assert.equal(timers[0]?.delay, 30000);
  sampler.setActive(false);
  assert.equal(timers.length, 0);
  sampler.setActive(true);
  assert.equal(reads, 3);
  sampler.dispose();
  complete(3);
  await flush();
  assert.deepEqual(delivered, [2]);
  assert.equal(timers.length, 0);
  sampler.setActive(true);
  assert.equal(reads, 3);
});
