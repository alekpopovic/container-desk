import type { ContainerEvent } from "../../lib/ipc/generated.ts";
type Clock = {
  now: () => number;
  schedule: (run: () => void, ms: number) => unknown;
  cancel: (id: unknown) => void;
};
const defaultClock: Clock = {
  now: () => Date.now(),
  schedule: (run, ms) => setTimeout(run, ms),
  cancel: (id) => clearTimeout(id as ReturnType<typeof setTimeout>),
};
/** One dirty flag, one timer and one in-flight read. Bursts cannot starve the refresh or queue work. */
export class InventoryInvalidator {
  private dirty = false;
  private busy = false;
  private closed = false;
  private timer: unknown = null;
  private lastStart = -Infinity;
  private read: () => Promise<boolean>;
  private clock: Clock;
  constructor(read: () => Promise<boolean>, clock: Clock = defaultClock) {
    this.read = read;
    this.clock = clock;
  }
  invalidate() {
    if (this.closed) return;
    this.dirty = true;
    if (this.busy || this.timer !== null) return;
    const delay = Math.max(500, 2000 - (this.clock.now() - this.lastStart));
    this.timer = this.clock.schedule(() => {
      this.timer = null;
      void this.run();
    }, delay);
  }
  private async run() {
    if (this.closed || !this.dirty) return;
    this.dirty = false;
    this.busy = true;
    this.lastStart = this.clock.now();
    try {
      if (!(await this.read())) this.dirty = true;
    } catch {
      // A manual read already in flight cannot consume this hint.
      /* Read errors are surfaced by the inventory; no unbounded immediate retry. */
    } finally {
      this.busy = false;
      if (this.dirty) this.invalidate();
    }
  }
  dispose() {
    this.closed = true;
    this.dirty = false;
    if (this.timer !== null) this.clock.cancel(this.timer);
    this.timer = null;
  }
}
export class EventHints {
  private recent = new Set<string>();
  since: string | null = null;
  accept(events: ContainerEvent[]): boolean {
    let changed = false;
    for (const event of events) {
      const nanos = BigInt(event.timestampUnixNanos);
      const since = `${nanos / 1000000000n}.${(nanos % 1000000000n).toString().padStart(9, "0")}`;
      // Preserve greatest observed timestamp; replay and out-of-order arrival are possible.
      if (this.since === null || nanos > BigInt(this.since.replace(".", "")))
        this.since = since;
      const key = JSON.stringify([
        event.actorId,
        event.action,
        event.timestampUnixNanos,
      ]);
      if (this.recent.has(key)) continue;
      this.recent.add(key);
      changed = true;
      while (this.recent.size > 1024) {
        const oldest = this.recent.values().next().value;
        if (oldest) this.recent.delete(oldest);
      }
    }
    return changed;
  }
}
