import { resourceLimitsReport } from "../lib/resourceLimits";
export function ResourceLimitsSummary() {
  const { limits, configurationIgnored } = resourceLimitsReport();
  return (
    <section className="support-panel" aria-labelledby="resource-limits-title">
      <h3 id="resource-limits-title">Resource limits</h3>
      <p>
        Logs: {limits.logLines.toLocaleString()} lines /{" "}
        {(limits.logBytes / (1024 * 1024)).toFixed(2)} MiB per buffer. Stats:{" "}
        {limits.statsHistory} points per container (up to 12 containers). Active
        live hosts: {limits.activeHosts}. Concurrent SSH jobs per transport:{" "}
        {limits.concurrentJobs}.
      </p>
      <p>
        Only one host is connected at a time. Switching hosts closes the
        previous session. Limits can be reduced in resource-limits.json in the
        application data directory; restart to apply changes. Stream queues keep
        separate bounded buffers and report dropped records.
      </p>
      {configurationIgnored && (
        <p role="alert">
          Resource configuration was unavailable or invalid. Safe default limits
          are in use. Review resource-limits.json and restart.
        </p>
      )}
    </section>
  );
}
