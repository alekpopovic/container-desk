import assert from "node:assert/strict";
import { test } from "node:test";
import {
  LogBuffer,
  MAX_BYTES,
  MAX_LINES,
  filterLogs,
  plainLog,
  selectedText,
  visibleLog,
} from "../../features/logs/buffer.ts";
import type { LogRecord } from "./generated.ts";
function row(text: string): LogRecord {
  return {
    text,
    timestamp: "2026-09-29T10:00:00.000000001Z",
    channel: "stdout",
    truncated: false,
    invalidUtf8: false,
  };
}
test("log control parser neutralizes escape links, C0/C1 and bidi while preserving Unicode", () => {
  const cases = [
    ["\x1b[31mčćž 🚢\x1b[0m", "čćž 🚢"],
    ["\x1b]8;;https://secret.invalid\x07link\x1b]8;;\x1b\\", "link"],
    ["\x9d8;;https://secret.invalid\x9clink\x9d8;;\x9c", "link"],
    ["abc\rde\b\u202eevil\x1b[2J", "abcdeevil"],
    ["before\x1bPprivate\x1b\\after", "beforeafter"],
    ["before\x1b]unterminated", "before"],
    ["single\u2028line\u2029only", "singlelineonly"],
    ["emoji 👩‍💻 and 中文\ttext", "emoji 👩‍💻 and 中文\ttext"],
  ];
  for (const [raw, expected] of cases) {
    assert.ok(raw !== undefined);
    assert.equal(plainLog(raw).text, expected);
  }
});
test("retention bounds include exported prefixes and discarded records release their references", () => {
  const buffer = new LogBuffer();
  buffer.append(Array.from({ length: MAX_LINES + 10 }, () => row("small")));
  assert.equal(buffer.snapshot().length, MAX_LINES);
  assert.equal(buffer.dropped, 10);
  for (let i = 0; i < 100; i++) buffer.append([row("é".repeat(130000))]);
  assert.ok(buffer.bytes <= MAX_BYTES);
  assert.ok(buffer.snapshot().length <= 33);
  // Byte count is independently calculated from exactly the copied/exported visible representation.
  assert.equal(
    buffer.bytes,
    buffer
      .snapshot()
      .reduce(
        (sum, r) => sum + new TextEncoder().encode(visibleLog(r)).length + 1,
        0,
      ),
  );
  const oldKey = buffer.snapshot().at(-1)?.key ?? 0;
  buffer.clear();
  buffer.append([row("new")]);
  assert.equal(buffer.snapshot().length, 1);
  assert.ok((buffer.snapshot()[0]?.key ?? 0) > oldKey);
  assert.equal(buffer.dropped, 0);
});
test("bounded literal filtering and selected export keep buffer order without other lines", () => {
  const buffer = new LogBuffer();
  buffer.append([
    row("first"),
    row("unrelated"),
    row("\x1b[32mthird čćž\x1b[0m"),
  ]);
  const rows = buffer.snapshot();
  const keys = rows.map((r) => r.key);
  assert.ok(keys[0] !== undefined && keys[2] !== undefined);
  const selected = new Set([keys[2], keys[0]]);
  assert.deepEqual(selectedText(rows, selected, false), [
    "first",
    "third čćž [controls removed]",
  ]);
  assert.equal(filterLogs(rows, ".*").length, 0);
  assert.equal(filterLogs(rows, "third").length, 1);
  buffer.append([row("x".repeat(256))]);
  assert.equal(filterLogs(buffer.snapshot(), "x".repeat(100000)).length, 1);
});
