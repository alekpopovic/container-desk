import { watchReadRecovery } from "../../lib/reads/recovery";
import { useEffect, useState } from "react";
import { followDockerEvents } from "../../lib/ipc/client";
import type { SessionScope } from "../../lib/ipc/generated";
import { EventHints, InventoryInvalidator } from "./invalidation";
export function useContainerEvents(
  scope: SessionScope | null,
  refresh: () => Promise<boolean>,
) {
  const [status, setStatus] = useState<{
    scope: SessionScope | null;
    text: string;
  }>({ scope: null, text: "" });
  useEffect(() => {
    if (!scope) return;
    const current = scope;
    const lifetime = new AbortController();
    const invalidator = new InventoryInvalidator(refresh);
    const hints = new EventHints();
    let attempt: AbortController | null = null;
    let retry: ReturnType<typeof setTimeout> | null = null;
    let retries = 0;
    const report = (text: string) => {
      if (!lifetime.signal.aborted) setStatus({ scope: current, text });
    };
    const recover = () => {
      if (lifetime.signal.aborted || retry !== null) return;
      invalidator.invalidate();
      if (retries >= 3) {
        report(
          "Event gap · automatic recovery stopped. Refresh containers or reconnect the host; snapshots remain authoritative.",
        );
        return;
      }
      const delay = 1000 * 2 ** retries++;
      report(
        `Event gap · refreshing the snapshot; reconnecting events (${retries}/3).`,
      );
      retry = setTimeout(() => {
        retry = null;
        void start();
      }, delay);
    };
    async function start() {
      attempt?.abort();
      attempt = new AbortController();
      const signal = attempt.signal;
      report(
        "Connecting events · current snapshot requested; event history may have gaps.",
      );
      invalidator.invalidate();
      try {
        await followDockerEvents(
          { scope: current, since: hints.since },
          () => (lifetime.signal.aborted ? null : current),
          (batch) => {
            if (signal.aborted || lifetime.signal.aborted) return;
            if (hints.accept(batch.events) || batch.gap || batch.ended)
              invalidator.invalidate();
            if (batch.ended || batch.error) recover();
            else
              report(
                batch.gap
                  ? `Event gap · ${batch.droppedRecords} records dropped; refreshing current snapshot.`
                  : "Live events · full snapshots remain authoritative; event history is best-effort.",
              );
          },
          recover,
          signal,
        );
        if (!lifetime.signal.aborted) invalidator.invalidate();
      } catch {
        if (!signal.aborted) recover();
      }
    }
    const stopRecovery = watchReadRecovery(() => invalidator.invalidate());
    void start();
    return () => {
      stopRecovery();
      lifetime.abort();
      attempt?.abort();
      if (retry !== null) clearTimeout(retry);
      invalidator.dispose();
    };
  }, [scope, refresh]);
  return status.scope === scope
    ? status.text
    : scope
      ? "Connecting events…"
      : "";
}
