import { NetworksFixture } from "./networks-fixture";
import { VolumesFixture } from "./volumes-fixture";
import { ImagesFixture } from "./images-fixture";
import { BatchFixture } from "./batch-fixture";
import { ManagementFixture } from "./management-fixture";
import { ComposeFixture } from "./compose-fixture";
import { EventsFixture } from "./events-fixture";
import { StatsFixture } from "./stats-fixture";
import { LogsFixture } from "./logs-fixture";
import { InspectFixture } from "./inspect-fixture";
import { ContainerFixture } from "./container-fixture";
import { createRoot } from "react-dom/client";
import {
  WorkspaceShell,
  type WorkspaceState,
} from "../../src/components/WorkspaceShell";
import "../../src/styles.css";

if (!import.meta.env.DEV) throw new Error("UI fixtures are development-only");
const kind = new URLSearchParams(window.location.search).get("state");
const host = {
  name: `Fixture host ${"very-long-name-".repeat(6)}`,
  alias: "fixture-only",
  endpoint: "unix:///fixture/docker.sock",
};
let state: WorkspaceState = { kind: "empty" };
if (kind === "loading" || kind === "offline") state = { kind, host };
if (kind === "error")
  state = {
    kind,
    host,
    message:
      'Fixture connection error: <img src="x" onerror="alert(1)">. No host was contacted.',
  };
const root = document.getElementById("root");
if (!root) throw new Error("Fixture root missing");
createRoot(root).render(
  kind === "networks" ? (
    <NetworksFixture />
  ) : kind === "volumes" ? (
    <VolumesFixture />
  ) : kind === "images" ? (
    <ImagesFixture />
  ) : kind === "batch" ? (
    <BatchFixture />
  ) : kind === "management" ? (
    <ManagementFixture />
  ) : kind === "compose" ? (
    <ComposeFixture />
  ) : kind === "events" ? (
    <EventsFixture />
  ) : kind === "stats" ? (
    <StatsFixture />
  ) : kind === "logs" ? (
    <LogsFixture />
  ) : kind === "inspect" ? (
    <InspectFixture />
  ) : kind === "containers" ? (
    <ContainerFixture />
  ) : (
    <WorkspaceShell state={state} />
  ),
);
