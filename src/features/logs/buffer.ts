import { resourceLimits } from "../../lib/resourceLimits.ts";
import type { LogRecord } from "../../lib/ipc/generated.ts";
export const MAX_LINES = 20_000;
export const MAX_BYTES = 8 * 1024 * 1024;
export const MAX_SEARCH = 256;
export type BufferedLog = LogRecord & {
  key: number;
  controlsRemoved: boolean;
  bytes: number;
};
const encoder = new TextEncoder();
/** Linear parser: discard CSI, OSC/DCS/SOS/PM/APC strings and C0/C1/bidi controls. Never interpret links. */
export function plainLog(text: string): { text: string; changed: boolean } {
  let result = "";
  let mode: "text" | "escape" | "csi" | "string" | "stringEscape" = "text";
  for (const char of text) {
    const code = char.codePointAt(0) ?? 0;
    if (mode === "stringEscape") {
      mode =
        char === "\\" ? "text" : char === "\x1b" ? "stringEscape" : "string";
      continue;
    }
    if (mode === "string") {
      if (char === "\x07" || char === "\x9c") mode = "text";
      else if (char === "\x1b") mode = "stringEscape";
      continue;
    }
    if (mode === "csi") {
      if (code >= 0x40 && code <= 0x7e) mode = "text";
      else if (char === "\x1b") mode = "escape";
      continue;
    }
    if (mode === "escape") {
      if (char === "[") mode = "csi";
      else if ("]PX^_".includes(char)) mode = "string";
      else if (!(code >= 0x20 && code <= 0x2f)) mode = "text";
      continue;
    }
    if (char === "\x1b") {
      mode = "escape";
      continue;
    }
    if (char === "\x9b") {
      mode = "csi";
      continue;
    }
    if ([0x90, 0x98, 0x9d, 0x9e, 0x9f].includes(code)) {
      mode = "string";
      continue;
    }
    if (
      (code < 32 && code !== 9) ||
      (code >= 0x7f && code <= 0x9f) ||
      (code >= 0x202a && code <= 0x202e) ||
      (code >= 0x2066 && code <= 0x2069) ||
      code === 0x200e ||
      code === 0x200f ||
      code === 0x2028 ||
      code === 0x2029
    )
      continue;
    result += char;
  }
  return { text: result, changed: result !== text };
}
export function visibleLog(
  row: Pick<
    BufferedLog,
    | "text"
    | "timestamp"
    | "channel"
    | "truncated"
    | "invalidUtf8"
    | "controlsRemoved"
  >,
  timestamps = true,
): string {
  return `${timestamps ? `${row.timestamp ?? "No timestamp"} ` : ""}${row.channel === "stderr_ambiguous" ? "[stderr / diagnostic] " : ""}${row.text}${row.truncated ? " [truncated]" : ""}${row.invalidUtf8 ? " [invalid UTF-8 replaced]" : ""}${row.controlsRemoved ? " [controls removed]" : ""}`;
}
export class LogBuffer {
  readonly maxLines: number;
  readonly maxBytes: number;
  constructor(limits = resourceLimits()) {
    this.maxLines = Math.max(
      1000,
      Math.min(
        MAX_LINES,
        Number.isInteger(limits.logLines) ? limits.logLines : MAX_LINES,
      ),
    );
    this.maxBytes = Math.max(
      256 * 1024,
      Math.min(
        MAX_BYTES,
        Number.isInteger(limits.logBytes) ? limits.logBytes : MAX_BYTES,
      ),
    );
  }
  private rows: Array<BufferedLog | undefined> = [];
  private head = 0;
  private next = 0;
  bytes = 0;
  dropped = 0;
  append(records: LogRecord[]) {
    for (const record of records) {
      const plain = plainLog(record.text);
      const row: BufferedLog = {
        ...record,
        text: plain.text,
        controlsRemoved: plain.changed,
        key: ++this.next,
        bytes: 0,
      };
      row.bytes = encoder.encode(visibleLog(row)).length + 1;
      this.rows.push(row);
      this.bytes += row.bytes;
      while (
        this.rows.length - this.head > this.maxLines ||
        this.bytes > this.maxBytes
      ) {
        const old = this.rows[this.head];
        this.rows[this.head++] = undefined;
        if (old) this.bytes -= old.bytes;
        this.dropped++;
      }
    }
    if (this.head > 4096) {
      this.rows = this.rows.slice(this.head);
      this.head = 0;
    }
  }
  snapshot(): BufferedLog[] {
    return this.rows
      .slice(this.head)
      .filter((row): row is BufferedLog => row !== undefined);
  }
  clear() {
    this.rows = [];
    this.head = 0;
    this.bytes = 0;
    this.dropped = 0;
  }
}
export function filterLogs(rows: BufferedLog[], search: string): BufferedLog[] {
  const query = search.slice(0, MAX_SEARCH).toLocaleLowerCase();
  return query
    ? rows.filter((row) => row.text.toLocaleLowerCase().includes(query))
    : rows;
}
export function selectedText(
  rows: BufferedLog[],
  selected: Set<number>,
  timestamps: boolean,
): string[] {
  return rows
    .filter((row) => selected.has(row.key))
    .map((row) => visibleLog(row, timestamps));
}
