import type { ErrorCode } from "../ipc/generated.ts";
export type ReadCommand =
  | "list_containers"
  | "inspect_container"
  | "container_logs"
  | "container_stats";
type Clock = {
  now: () => number;
  random: () => number;
  schedule: (run: () => void, ms: number) => unknown;
  cancel: (id: unknown) => void;
};
const defaultClock: Clock = {
  now: () => performance.now(),
  random: () => Math.random(),
  schedule: (run, ms) => setTimeout(run, ms),
  cancel: (id) => clearTimeout(id as ReturnType<typeof setTimeout>),
};
type Consumer = {
  current: () => boolean;
  resolve: (v: unknown) => void;
  reject: (e: unknown) => void;
};
type Job = {
  key: string;
  command: ReadCommand;
  task: () => Promise<unknown>;
  consumers: Consumer[];
  attempt: number;
  readyAt: number;
  deadline: number;
  active: boolean;
};
type Host = { jobs: Map<string, Job>; active: number; background: number };
/** Only finite idempotent reads enter this scheduler. Native Rust still enforces its own limits. */
export class ReadScheduler {
  private hosts = new Map<string, Host>();
  private active = 0;
  private timer: unknown = null;
  private clock: Clock;
  private error: (code: ErrorCode) => Error;
  constructor(error: (code: ErrorCode) => Error, clock: Clock = defaultClock) {
    this.error = error;
    this.clock = clock;
  }
  run<T>(
    hostKey: string,
    key: string,
    command: ReadCommand,
    current: () => boolean,
    task: () => Promise<T>,
  ): Promise<T> {
    if (!current()) return Promise.reject(this.error("stale_session"));
    let host = this.hosts.get(hostKey);
    if (!host) {
      if (this.hosts.size >= 3)
        return Promise.reject(this.error("resource_limit"));
      host = { jobs: new Map(), active: 0, background: 0 };
      this.hosts.set(hostKey, host);
    }
    let job = host.jobs.get(key);
    if (!job) {
      if (host.jobs.size >= 16)
        return Promise.reject(this.error("resource_limit"));
      job = {
        key,
        command,
        task,
        consumers: [],
        attempt: 0,
        readyAt: this.clock.now(),
        deadline: this.clock.now() + 120000,
        active: false,
      };
      host.jobs.set(key, job);
    }
    if (job.consumers.length >= 32)
      return Promise.reject(this.error("resource_limit"));
    const selected = job;
    const result = new Promise<unknown>((resolve, reject) =>
      selected.consumers.push({ current, resolve, reject }),
    );
    this.drain();
    return result as Promise<T>;
  }
  private purge(job: Job) {
    job.consumers = job.consumers.filter((consumer) => {
      if (consumer.current()) return true;
      consumer.reject(this.error("stale_session"));
      return false;
    });
  }
  private drain() {
    if (this.timer !== null) this.clock.cancel(this.timer);
    this.timer = null;
    for (const [key, host] of this.hosts) {
      for (const job of host.jobs.values()) {
        this.purge(job);
        if (
          !job.active &&
          (!job.consumers.length || this.clock.now() >= job.deadline)
        ) {
          for (const c of job.consumers)
            c.reject(this.error("operation_timed_out"));
          host.jobs.delete(job.key);
        }
      }
      const ready = [...host.jobs.values()].filter(
        (j) => !j.active && j.readyAt <= this.clock.now(),
      );
      // Stable order within priority; an active background read never occupies both host slots.
      ready.sort(
        (a, b) =>
          Number(a.command === "container_stats") -
          Number(b.command === "container_stats"),
      );
      for (const job of ready) {
        if (host.active >= 2 || this.active >= 4) break;
        if (job.command === "container_stats" && host.background > 0) continue;
        job.active = true;
        host.active++;
        this.active++;
        if (job.command === "container_stats") host.background++;
        void this.execute(host, job);
      }
      if (host.jobs.size === 0) this.hosts.delete(key);
    }
    if (this.hosts.size)
      this.timer = this.clock.schedule(() => {
        this.timer = null;
        this.drain();
      }, 100);
  }
  private async execute(host: Host, job: Job) {
    let repeat = false;
    try {
      const value = await job.task();
      this.purge(job);
      for (const c of job.consumers) c.resolve(value);
    } catch (error) {
      this.purge(job);
      const code =
        typeof error === "object" && error !== null && "code" in error
          ? error.code
          : null;
      repeat =
        job.consumers.length > 0 &&
        job.attempt < 2 &&
        this.clock.now() < job.deadline &&
        [
          "resource_limit",
          "transport_unavailable",
          "operation_timed_out",
        ].includes(String(code));
      if (repeat) {
        const base = 500 * 2 ** job.attempt++;
        job.readyAt =
          this.clock.now() +
          base * (0.75 + 0.5 * Math.min(1, Math.max(0, this.clock.random())));
      } else for (const c of job.consumers) c.reject(error);
    } finally {
      job.active = false;
      host.active--;
      this.active--;
      if (job.command === "container_stats") host.background--;
      if (!repeat) host.jobs.delete(job.key);
      this.drain();
    }
  }
}
