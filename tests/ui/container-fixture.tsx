import { useEffect, useState, useSyncExternalStore } from "react";
import { ContainerCache } from "../../src/features/containers/cache";
import { ContainerInventory } from "../../src/features/containers/ContainerInventory";
import { WorkspaceShell } from "../../src/components/WorkspaceShell";
import type {
  ContainerSummary,
  SessionScope,
} from "../../src/lib/ipc/generated";
const scope: SessionScope = {
  selection: { hostId: `h_${"a".repeat(32)}`, selectionGeneration: 1 },
  sessionId: `s_${"b".repeat(32)}`,
  sessionGeneration: 1,
  daemonId: "synthetic-table",
};
const source: ContainerSummary[] = Array.from({ length: 1000 }, (_, index) => ({
  scope,
  id: (index + 1).toString(16).padStart(64, "0"),
  name: `workload-${index.toString().padStart(4, "0")}`,
  image: `fixture/image-${index % 10}`,
  state: index % 2 ? "running" : "exited",
  status: "Synthetic status",
  health: index % 3 ? null : "healthy",
  ports: [],
  compose: null,
  cli: {
    names: [],
    ports: "127.0.0.1:8080->80/tcp",
    createdAt: `2026-09-${((index % 28) + 1).toString().padStart(2, "0")} 00:00:00 +0000 UTC`,
    runningFor: `${index} minutes ago`,
    labelsPresent: false,
  },
}));
export function ContainerFixture() {
  const [cache] = useState(() => new ContainerCache());
  const [rows, setRows] = useState(source);
  const [fail, setFail] = useState(false);
  const view = useSyncExternalStore(cache.subscribe, cache.getSnapshot);
  useEffect(() => {
    cache.activate(scope);
    const ticket = cache.begin();
    if (ticket) cache.complete(ticket, { scope, containers: source });
  }, [cache]);
  function refresh() {
    const ticket = cache.begin();
    if (ticket)
      setTimeout(() => {
        if (fail) cache.fail(ticket, "Fixture refresh failed");
        else cache.complete(ticket, { scope, containers: rows });
      }, 60);
  }
  return (
    <WorkspaceShell
      state={{
        kind: "connected",
        host: {
          name: "Synthetic 1000-container fixture",
          alias: "fixture-only",
          endpoint: "No SSH",
        },
      }}
      containersExtra={
        <>
          <div className="container-tools">
            <button
              type="button"
              onClick={() =>
                setRows(rows.filter((row) => row.id !== view.selectedId))
              }
            >
              Fixture remove selected
            </button>
            <button type="button" onClick={() => setFail(!fail)}>
              Fixture toggle failure
            </button>
          </div>
          <ContainerInventory
            view={view}
            host={{
              name: "Synthetic 1000-container fixture",
              alias: "fixture-only",
              endpoint: "No SSH",
            }}
            refresh={refresh}
            select={(id) => cache.select(id)}
          />
        </>
      }
    />
  );
}
