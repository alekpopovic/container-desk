import assert from "node:assert/strict";
import { afterEach, beforeEach, test } from "node:test";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import fixture from "../../demo/workspace.generated.json" with { type: "json" };
import * as browser from "../../demo/browser.ts";
import { getWorkspaceMode, switchWorkspace, listContainers } from "./client.ts";
import type { SessionScope } from "./generated.ts";
const scope: SessionScope = fixture.inventory.scope;
beforeEach(() =>
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: { crypto: globalThis.crypto },
  }),
);
afterEach(() => {
  clearMocks();
  Reflect.deleteProperty(globalThis, "window");
});

test("Rust-generated demo DTOs travel through the same native IPC decoder", async () => {
  mockIPC((command, args) => {
    if (command === "get_workspace_mode")
      return { mode: "live", scenario: null, scope: null, host: null };
    if (command === "switch_workspace") {
      assert.deepEqual(args, {
        request: { mode: "demo", scenario: "standard" },
      });
      return fixture.workspace;
    }
    assert.equal(command, "list_containers");
    return fixture.inventory;
  });
  assert.equal((await getWorkspaceMode()).mode, "live");
  assert.equal(
    (await switchWorkspace({ mode: "demo", scenario: "standard" })).mode,
    "demo",
  );
  const response = await listContainers(scope, () => scope);
  assert.equal(response.containers.length, 4);
  assert.equal(response.containers[0]?.ports[0]?.hostIp, "::1");
  assert.equal(response.containers[0]?.compose?.project, "demo-stack");
});

test("browser demo is explicit, never invokes native IPC, and never supplies live fallback", async () => {
  let nativeCalls = 0;
  mockIPC(() => {
    nativeCalls += 1;
    throw { code: "permission_denied" };
  });
  await browser.switchWorkspace({ mode: "live" });
  await assert.rejects(
    browser.listContainers(scope, () => scope),
    { code: "feature_unavailable" },
  );
  const demo = await browser.switchWorkspace({
    mode: "demo",
    scenario: "standard",
  });
  assert.ok(demo.scope);
  const response = await browser.listContainers(demo.scope, () => demo.scope);
  assert.equal(response.containers.length, 4);
  assert.equal(nativeCalls, 0);
  // Native request failure must still reject even while a separate browser provider has demo data.
  await assert.rejects(
    listContainers(scope, () => scope),
    { code: "permission_denied" },
  );
  assert.equal(nativeCalls, 1);
  await browser.switchWorkspace({ mode: "live" });
  await assert.rejects(
    browser.listContainers(demo.scope, () => demo.scope),
    { code: "feature_unavailable" },
  );
});

test("bad ports or mismatched mode responses fail validation", async () => {
  mockIPC(() => ({
    ...fixture.inventory,
    containers: [
      {
        ...fixture.inventory.containers[0],
        ports: [
          { hostIp: "::1", publicPort: -1, privatePort: 80, protocol: "tcp" },
        ],
      },
    ],
  }));
  await assert.rejects(
    listContainers(scope, () => scope),
    { code: "invalid_response" },
  );
  mockIPC(() => ({ mode: "live", scenario: null, scope: null, host: null }));
  await assert.rejects(
    switchWorkspace({ mode: "demo", scenario: "standard" }),
    { code: "invalid_response" },
  );
});
