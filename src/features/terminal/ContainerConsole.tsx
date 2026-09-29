import { lazy, Suspense, useId, useState } from "react";
import type { SessionScope } from "../../lib/ipc/generated";
import { LiveLogs } from "../logs/LiveLogs";
const TerminalPanel = lazy(() =>
  import("./TerminalPanel").then((module) => ({
    default: module.TerminalPanel,
  })),
);
export function ContainerConsole({
  scope,
  id,
  name,
  host,
}: {
  scope: SessionScope;
  id: string;
  name: string;
  host: string;
}) {
  const [tab, setTab] = useState<"logs" | "terminal">("logs");
  const uid = useId();
  return (
    <section className="container-console" aria-label="Container console">
      <div
        className="detail-tabs"
        role="tablist"
        aria-label="Container console"
      >
        {(["logs", "terminal"] as const).map((value) => (
          <button
            key={value}
            id={`${uid}-${value}`}
            type="button"
            data-shortcut={value === "logs" ? "logs" : undefined}
            role="tab"
            aria-selected={value === tab}
            aria-controls={`${uid}-panel`}
            tabIndex={value === tab ? 0 : -1}
            onClick={() => setTab(value)}
            onKeyDown={(event) => {
              if (
                ["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)
              ) {
                event.preventDefault();
                const next =
                  event.key === "Home"
                    ? "logs"
                    : event.key === "End"
                      ? "terminal"
                      : value === "logs"
                        ? "terminal"
                        : "logs";
                setTab(next);
                document.getElementById(`${uid}-${next}`)?.focus();
              }
            }}
          >
            {value === "logs" ? "Logs" : "Terminal"}
          </button>
        ))}
      </div>
      <div
        id={`${uid}-panel`}
        role="tabpanel"
        aria-labelledby={`${uid}-${tab}`}
      >
        {tab === "logs" ? (
          <LiveLogs scope={scope} id={id} source={`${host} / ${name}`} />
        ) : (
          <Suspense fallback={<p>Loading terminal…</p>}>
            <TerminalPanel scope={scope} id={id} name={name} host={host} />
          </Suspense>
        )}
      </div>
    </section>
  );
}
