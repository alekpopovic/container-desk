type Clock = {
  now: () => number;
  schedule: (run: () => void, ms: number) => unknown;
  cancel: (id: unknown) => void;
};
type Signals = {
  visible: () => boolean;
  listen: (run: () => void) => () => void;
};
const clock: Clock = {
  now: () => Date.now(),
  schedule: (run, ms) => setTimeout(run, ms),
  cancel: (id) => clearTimeout(id as ReturnType<typeof setTimeout>),
};
const signals: Signals = {
  visible: () => !document.hidden,
  listen: (run) => {
    window.addEventListener("online", run);
    window.addEventListener("focus", run);
    document.addEventListener("visibilitychange", run);
    return () => {
      window.removeEventListener("online", run);
      window.removeEventListener("focus", run);
      document.removeEventListener("visibilitychange", run);
    };
  },
};
/** OS sleep often suspends timers; a clock jump is only a hint to reconcile the current snapshot. */
export function watchReadRecovery(
  refresh: () => void,
  time: Clock = clock,
  source: Signals = signals,
) {
  let closed = false;
  let previous = time.now();
  let lastRefresh = -Infinity;
  let timer: unknown;
  const recover = () => {
    if (!closed && source.visible() && time.now() - lastRefresh >= 2000) {
      lastRefresh = time.now();
      refresh();
    }
  };
  const tick = () => {
    const now = time.now();
    if (now - previous > 45000) recover();
    previous = now;
    if (!closed) timer = time.schedule(tick, 15000);
  };
  const remove = source.listen(recover);
  timer = time.schedule(tick, 15000);
  return () => {
    closed = true;
    remove();
    time.cancel(timer);
  };
}
