import { performance } from "node:perf_hooks";
import { LogBuffer } from "../../src/features/logs/buffer.ts";
const buffer = new LogBuffer();
const samples: { lines: number; bytes: number; heapMiB: number; elapsedMs: number }[] = [];
const begin = performance.now();
for (let round = 0; round < 12; round++) {
  for (let batch = 0; batch < 100; batch++) buffer.append(Array.from({ length: 128 }, (_, index) => ({ text: `synthetic-${round}-${batch}-${index}: ${"ordinary text ".repeat(12)}`, timestamp: null, channel: "stdout" as const, truncated: false, invalidUtf8: false })));
  global.gc?.();
  samples.push({ lines: buffer.snapshot().length, bytes: buffer.bytes, heapMiB: process.memoryUsage().heapUsed / (1024 * 1024), elapsedMs: performance.now() - begin });
}
console.log(JSON.stringify({ node: process.version, records: 12 * 100 * 128, dropped: buffer.dropped, samples }, null, 2));
