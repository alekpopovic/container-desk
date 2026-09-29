import test from "node:test";
import assert from "node:assert/strict";
import { defaultLimits, validLimits } from "../resourceLimits.ts";
import { LogBuffer } from "../../features/logs/buffer.ts";
import { StatsHistory } from "../../features/stats/sampling.ts";
import fixture from "../../../tests/fixtures/ipc.json" with { type: "json" };
test("configured retention reduces memory bounds and malformed numbers cannot remove ceilings", () => {
  for (const value of [0, -1, NaN, Infinity, 20001])
    assert.equal(validLimits({ ...defaultLimits, logLines: value }), false);
  assert.equal(validLimits({ ...defaultLimits, activeHosts: 2 }), false);
  const buffer = new LogBuffer({
    ...defaultLimits,
    logLines: 1000,
    logBytes: 256 * 1024,
  });
  buffer.append(
    Array.from({ length: 5000 }, () => ({
      text: "synthetic ".repeat(100),
      channel: "stdout" as const,
      timestamp: null,
      truncated: false,
      invalidUtf8: false,
    })),
  );
  assert.ok(buffer.bytes <= 256 * 1024);
  assert.ok(buffer.snapshot().length <= 1000);
  assert.ok(buffer.dropped > 4000);
  const history = new StatsHistory(60);
  for (let n = 0; n < 1000; n++)
    history.append(fixture.logs.scope, "a".repeat(64), {
      time: n,
      values: null,
    });
  assert.equal(history.get(fixture.logs.scope, "a".repeat(64)).length, 60);
  assert.equal(history.get(fixture.logs.scope, "a".repeat(64))[0]?.time, 940);
  assert.equal(new StatsHistory(Infinity).capacity, 360);
});
