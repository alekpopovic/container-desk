import { useState } from "react";
import { mockIPC } from "@tauri-apps/api/mocks";
import { ContainerStats } from "../../src/features/stats/ContainerStats";
import { StatsHistory } from "../../src/features/stats/sampling";
import type {
  ContainerStatsRequest,
  StatsSample,
} from "../../src/lib/ipc/generated";
import fixture from "../fixtures/ipc.json";
let focused = true;
export function StatsFixture() {
  useState(() => {
    Object.defineProperty(document, "hasFocus", {
      configurable: true,
      value: () => focused,
    });
    return true;
  });
  const [history] = useState(() => new StatsHistory());
  const [connected, connect] = useState(true);
  const [calls, setCalls] = useState(0);
  const [mode, setMode] = useState<"available" | "stopped" | "missing">(
    "available",
  );
  mockIPC((command, args) => {
    if (command !== "container_stats")
      throw Error("Unexpected stats fixture command");
    const request = (args as { request: ContainerStatsRequest }).request;
    setCalls((n) => n + 1);
    const sample = structuredClone(fixture.stats) as StatsSample;
    sample.scope = request.scope;
    sample.containerId = request.containerId;
    sample.capturedAtMs = Date.now();
    sample.availability = mode;
    if (mode !== "available")
      sample.values = {
        cpuPercent: null,
        memoryUsageBytes: null,
        memoryLimitBytes: null,
        memoryPercent: null,
        networkRxBytes: null,
        networkTxBytes: null,
        blockReadBytes: null,
        blockWriteBytes: null,
        pids: null,
      };
    return sample;
  });
  return (
    <main>
      <button
        type="button"
        onClick={() => {
          focused = false;
          window.dispatchEvent(new Event("blur"));
        }}
      >
        Fixture inactive window
      </button>
      <button
        type="button"
        onClick={() => {
          focused = true;
          window.dispatchEvent(new Event("focus"));
        }}
      >
        Fixture active window
      </button>
      <button type="button" onClick={() => setMode("stopped")}>
        Fixture stopped
      </button>
      <button type="button" onClick={() => setMode("missing")}>
        Fixture disappeared
      </button>
      <button type="button" onClick={() => connect(false)}>
        Fixture disconnect
      </button>
      <output aria-label="Stats fixture calls">{calls}</output>
      {connected && (
        <ContainerStats
          scope={fixture.stats.scope}
          id={fixture.stats.containerId}
          history={history}
        />
      )}
    </main>
  );
}
