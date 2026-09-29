import type { SessionScope, StatsValues } from "../../lib/ipc/generated.ts";
export type StatsPoint = { time: number; values: StatsValues | null };
export class StatsHistory {
  private entries = new Map<string, StatsPoint[]>();
  private key(scope: SessionScope, id: string) {
    return JSON.stringify([
      scope.selection.hostId,
      scope.daemonId,
      scope.sessionId,
      scope.sessionGeneration,
      scope.selection.selectionGeneration,
      id,
    ]);
  }
  get(scope: SessionScope, id: string): StatsPoint[] {
    return this.entries.get(this.key(scope, id)) ?? [];
  }
  append(scope: SessionScope, id: string, point: StatsPoint): StatsPoint[] {
    const key = this.key(scope, id);
    const points = [...this.get(scope, id).slice(-359), point];
    this.entries.delete(key);
    this.entries.set(key, points);
    while (this.entries.size > 12) {
      const oldest = this.entries.keys().next().value;
      if (oldest) this.entries.delete(oldest);
    }
    return points;
  }
}
type Clock = {
  schedule: (task: () => void, delay: number) => unknown;
  cancel: (id: unknown) => void;
};
const defaultClock: Clock = {
  schedule: (task, delay) => setTimeout(task, delay),
  cancel: (id) => clearTimeout(id as ReturnType<typeof setTimeout>),
};
/** An interval starts after completion, never while an earlier request is pending. */
export class SerialSampler<T> {
  private active = false;
  private busy = false;
  private disposed = false;
  private epoch = 0;
  private timer: unknown = null;
  private interval = 5000;
  private read: () => Promise<T>;
  private receive: (value: T) => void;
  private fail: (error: unknown) => void;
  private clock: Clock;
  constructor(
    read: () => Promise<T>,
    receive: (value: T) => void,
    fail: (error: unknown) => void,
    clock: Clock = defaultClock,
  ) {
    this.read = read;
    this.receive = receive;
    this.fail = fail;
    this.clock = clock;
  }
  setInterval(ms: number) {
    this.interval = [5000, 10000, 30000, 60000].includes(ms) ? ms : 5000;
  }
  setActive(active: boolean) {
    if (this.disposed || this.active === active) return;
    this.active = active;
    this.epoch++;
    if (this.timer !== null) this.clock.cancel(this.timer);
    this.timer = null;
    if (active) void this.run();
  }
  dispose() {
    this.setActive(false);
    this.disposed = true;
  }
  private async run() {
    if (!this.active || this.busy) return;
    this.busy = true;
    const epoch = this.epoch;
    try {
      const value = await this.read();
      if (this.active && epoch === this.epoch) this.receive(value);
    } catch (error) {
      if (this.active && epoch === this.epoch) this.fail(error);
    } finally {
      this.busy = false;
      if (this.active)
        this.timer = this.clock.schedule(() => {
          this.timer = null;
          void this.run();
        }, this.interval);
    }
  }
}
