import { useEffect, useRef, useState } from "react";
import { containerStats, IpcError } from "../../lib/ipc/client";
import type { SessionScope, StatsSample } from "../../lib/ipc/generated";
import { SerialSampler, type StatsHistory, type StatsPoint } from "./sampling";
const number = new Intl.NumberFormat(undefined, { maximumFractionDigits: 2 });
function value(n: number | null | undefined, suffix = ""): string {
  return n === null || n === undefined
    ? "Unavailable"
    : `${number.format(n)}${suffix}`;
}
function bytes(n: number | null | undefined): string {
  if (n === null || n === undefined) return "Unavailable";
  const units = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
  const exponent =
    n === 0
      ? 0
      : Math.min(
          units.length - 1,
          Math.max(0, Math.floor(Math.log(n) / Math.log(1024))),
        );
  return `${number.format(n / 1024 ** exponent)} ${units[exponent]}`;
}
function Chart({
  points,
  metric,
  title,
  unit,
}: {
  points: StatsPoint[];
  metric: "cpuPercent" | "memoryUsageBytes";
  title: string;
  unit: string;
}) {
  const normalized = points.map((p) => ({
    time: p.time,
    value:
      p.values?.[metric] === null || p.values?.[metric] === undefined
        ? null
        : (p.values[metric] ?? 0) /
          (metric === "memoryUsageBytes" ? 1024 * 1024 : 1),
  }));
  const maximum = Math.max(
    metric === "cpuPercent" ? 100 : 1,
    ...normalized.map((p) => p.value ?? 0),
  );
  const first = points[0]?.time ?? 0;
  const span = Math.max(1, (points.at(-1)?.time ?? first) - first);
  const segments: Array<Array<[number, number]>> = [];
  let segment: Array<[number, number]> = [];
  for (const p of normalized) {
    if (p.value === null) {
      if (segment.length) segments.push(segment);
      segment = [];
    } else
      segment.push([
        8 + ((p.time - first) / span) * 304,
        92 - (p.value / maximum) * 80,
      ]);
  }
  if (segment.length) segments.push(segment);
  return (
    <figure className="stats-chart">
      <figcaption>
        {title} · scale 0–{number.format(maximum)} {unit}
      </figcaption>
      <svg
        viewBox="0 0 320 100"
        role="img"
        aria-label={`${title}; ${points.length} samples, unavailable samples break the line`}
      >
        <title>{title}</title>
        <path d="M8 12V92H312" className="stats-axis" />
        {segments.map((part) => (
          <g key={part[0]?.[0]}>
            <polyline points={part.map((p) => p.join(",")).join(" ")} />
            {part.length === 1 && (
              <circle cx={part[0]?.[0]} cy={part[0]?.[1]} r="2" />
            )}
          </g>
        ))}
      </svg>
    </figure>
  );
}
export function ContainerStats({
  scope,
  id,
  history,
}: {
  scope: SessionScope;
  id: string;
  history: StatsHistory;
}) {
  const [sample, setSample] = useState<StatsSample | null>(null);
  const [points, setPoints] = useState(() => history.get(scope, id));
  const [interval, setInterval] = useState(5000);
  const [paused, setPaused] = useState(false);
  const [foreground, setForeground] = useState(
    () => !document.hidden && document.hasFocus(),
  );
  const [status, setStatus] = useState("Waiting for a sample…");
  const sampler = useRef<SerialSampler<StatsSample> | null>(null);
  useEffect(() => {
    let alive = true;
    const append = (point: StatsPoint) =>
      setPoints(history.append(scope, id, point));
    // Returning to a previously viewed container leaves an explicit observation gap.
    if (history.get(scope, id).length)
      append({ time: Date.now(), values: null });
    const worker = new SerialSampler(
      () =>
        containerStats({ scope, containerId: id }, () =>
          alive ? scope : null,
        ),
      (next) => {
        setSample(next);
        setStatus(
          next.availability === "available"
            ? "Sample received"
            : next.availability === "stopped"
              ? "Container stopped · gap"
              : next.availability === "missing"
                ? "Container disappeared · gap; refresh the inventory"
                : "Statistics unavailable · gap",
        );
        append({
          time: next.capturedAtMs,
          values: next.availability === "available" ? next.values : null,
        });
      },
      (error) => {
        setStatus(
          error instanceof IpcError
            ? `${error.message} · gap`
            : "Statistics unavailable · gap",
        );
        setSample(null);
        append({ time: Date.now(), values: null });
      },
    );
    sampler.current = worker;
    return () => {
      alive = false;
      worker.dispose();
      sampler.current = null;
    };
  }, [scope, id, history]);
  useEffect(() => {
    const focus = () => setForeground(!document.hidden && document.hasFocus());
    const blur = () => setForeground(false);
    window.addEventListener("focus", focus);
    window.addEventListener("blur", blur);
    document.addEventListener("visibilitychange", focus);
    return () => {
      window.removeEventListener("focus", focus);
      window.removeEventListener("blur", blur);
      document.removeEventListener("visibilitychange", focus);
    };
  }, []);
  useEffect(() => {
    sampler.current?.setInterval(interval);
  }, [interval]);
  useEffect(() => {
    const active = foreground && !paused;
    if (!active && history.get(scope, id).length)
      setPoints(history.append(scope, id, { time: Date.now(), values: null }));
    sampler.current?.setActive(active);
  }, [foreground, paused, history, scope, id]);
  const values = sample?.availability === "available" ? sample.values : null;
  return (
    <section
      className="container-stats"
      aria-label="Container resource statistics"
    >
      <h3>Resource statistics</h3>
      <div className="stats-tools">
        <label>
          Stats interval{" "}
          <select
            value={interval}
            onChange={(e) => setInterval(Number(e.target.value))}
          >
            {[5, 10, 30, 60].map((n) => (
              <option key={n} value={n * 1000}>
                {n} seconds after each sample
              </option>
            ))}
          </select>
        </label>
        <button
          className="button"
          type="button"
          onClick={() => setPaused(!paused)}
        >
          {paused ? "Resume statistics" : "Pause statistics"}
        </button>
      </div>
      <p role="status">
        {paused
          ? "Statistics paused. "
          : !foreground
            ? "Window inactive · statistics paused. "
            : ""}
        {status}
      </p>
      <p className="muted">
        Only this selected container is sampled. Polling pauses when the window
        is hidden or unfocused and ends on disconnect. Up to 360 points per
        container are kept in memory; gaps mean no reading.
      </p>
      <dl className="stats-values">
        <div>
          <dt>CPU · Docker-reported percent</dt>
          <dd>{value(values?.cpuPercent, "%")}</dd>
        </div>
        <div>
          <dt>Memory · Docker CLI usage / limit</dt>
          <dd>
            {bytes(values?.memoryUsageBytes)} /{" "}
            {bytes(values?.memoryLimitBytes)}
          </dd>
        </div>
        <div>
          <dt>Memory percent</dt>
          <dd>{value(values?.memoryPercent, "%")}</dd>
        </div>
        <div>
          <dt>Network received / sent · cumulative</dt>
          <dd>
            {bytes(values?.networkRxBytes)} / {bytes(values?.networkTxBytes)}
          </dd>
        </div>
        <div>
          <dt>Block read / written · cumulative</dt>
          <dd>
            {bytes(values?.blockReadBytes)} / {bytes(values?.blockWriteBytes)}
          </dd>
        </div>
        <div>
          <dt>Processes and kernel threads</dt>
          <dd>{value(values?.pids)}</dd>
        </div>
      </dl>
      <p>
        CPU can exceed 100% on multicore hosts. Linux memory uses Docker CLI
        semantics (cache subtracted), not raw API memory. I/O values are totals,
        not rates; CLI rounding also applies to charts.
      </p>
      <p className="stats-samples">
        {points.length} history {points.length === 1 ? "point" : "points"} ·
        Last sample:{" "}
        {sample ? new Date(sample.capturedAtMs).toLocaleTimeString() : "None"}
      </p>
      <div className="stats-charts">
        <Chart
          points={points}
          metric="cpuPercent"
          title="CPU history"
          unit="%"
        />
        <Chart
          points={points}
          metric="memoryUsageBytes"
          title="Memory history"
          unit="MiB"
        />
      </div>
      {sample && (
        <details>
          <summary>Raw metric strings for this sample</summary>
          <dl>
            {Object.entries(sample.raw).map(([key, raw]) => (
              <div key={key}>
                <dt>{key}</dt>
                <dd>{raw ?? "Unavailable"}</dd>
              </div>
            ))}
          </dl>
          <p>
            Transient, as reported by Docker; unknown units remain unavailable.
          </p>
        </details>
      )}
    </section>
  );
}
