import { useState } from "react";
import { mockIPC } from "@tauri-apps/api/mocks";
import { ComposeInventory } from "../../src/features/compose/ComposeInventory";
import type {
  ListComposeResponse,
  ContainerSummary,
  SessionScope,
} from "../../src/lib/ipc/generated";
import type { InventoryView } from "../../src/features/containers/cache";
const scope: SessionScope = {
  selection: { hostId: `h_${"a".repeat(32)}`, selectionGeneration: 1 },
  sessionId: `s_${"b".repeat(32)}`,
  sessionGeneration: 1,
  daemonId: "compose-fixture",
};
const rows: ContainerSummary[] = ["alpha", "beta"].map((name, index) => ({
  scope,
  id: (index + 1).toString(16).padStart(64, "0"),
  name: `${name}-web-1`,
  image: "fixture",
  state: index ? "exited" : "running",
  status: "fixture",
  health: null,
  ports: [],
  compose: { project: name, service: "web" },
  cli: null,
}));
export function ComposeFixture() {
  const [plugin, setPlugin] = useState<"available" | "absent">("available");
  const [connected, connect] = useState(true);
  const [opened, open] = useState("");
  const [revision, revise] = useState(1);
  mockIPC((command, args) => {
    if (command === "get_management") return { scope, enabled: false };
    if (command !== "list_compose")
      throw Error(`Unexpected transport session or command ${command}`);
    const current = (args as { request: { scope: SessionScope } }).request
      .scope;
    return {
      scope: current,
      plugin,
      listingError: null,
      projects: rows.map((row) => ({
        name: row.compose?.project ?? "",
        status: "fixture",
        fromPlugin: plugin === "available",
        fromLabels: true,
        configuration: "unverified",
        configFilesReported: row.name.startsWith("alpha")
          ? ["/remote/deleted/compose.yml,<img src=x onerror=alert(1)>"]
          : [],
        workingDirectoriesReported: ["/remote/missing"],
        instances: [
          {
            containerId: row.id,
            name: row.name,
            service: "web",
            state: row.state,
          },
        ],
      })),
    } satisfies ListComposeResponse;
  });
  const view: InventoryView = {
    scope: connected ? scope : null,
    rows: connected ? rows : [],
    selectedId: null,
    updatedAt: revision,
    loading: false,
    stale: false,
    error: null,
  };
  return (
    <main>
      <button
        type="button"
        onClick={() => {
          setPlugin("absent");
          revise((n) => n + 1);
        }}
      >
        Fixture plugin absent
      </button>
      <button type="button" onClick={() => connect(false)}>
        Fixture disconnect
      </button>
      <output aria-label="Opened container">{opened}</output>
      <ComposeInventory
        native
        view={view}
        refreshContainers={() => revise((n) => n + 1)}
        openContainer={(selected, id) => {
          if (selected === scope) open(id);
        }}
      />
    </main>
  );
}
