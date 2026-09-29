import assert from "node:assert/strict";
import { afterEach, beforeEach, test } from "node:test";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { listCompose } from "./client.ts";
import type { ListComposeResponse } from "./generated.ts";
const scope = {
  selection: { hostId: `h_${"a".repeat(32)}`, selectionGeneration: 1 },
  sessionId: `s_${"b".repeat(32)}`,
  sessionGeneration: 1,
  daemonId: "compose-test",
};
export const response: ListComposeResponse = {
  scope,
  plugin: "absent",
  listingError: null,
  projects: ["a", "b"].map((name, index) => ({
    name,
    status: null,
    fromPlugin: false,
    fromLabels: true,
    configuration: "unverified",
    configFilesReported: ["/remote/missing/compose.yml"],
    workingDirectoriesReported: [],
    instances: [
      {
        containerId: (index + 1).toString(16).padStart(64, "0"),
        name: `${name}-web-1`,
        state: "created",
        service: "web",
      },
    ],
  })),
};
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
test("Compose IPC preserves distinct project/service identity and treats paths only as unverified metadata", async () => {
  mockIPC((command, args) => {
    assert.equal(command, "list_compose");
    assert.deepEqual(args, { request: { scope } });
    return response;
  });
  const value = await listCompose(scope, () => scope);
  assert.equal(value.projects.length, 2);
  assert.equal(
    value.projects[0]?.instances[0]?.service,
    value.projects[1]?.instances[0]?.service,
  );
  assert.equal(value.projects[0]?.configuration, "unverified");
});
test("Compose IPC rejects foreign scope, duplicate resource IDs, unverified-status forgery and excessive metadata", async () => {
  const first = response.projects[0];
  assert.ok(first);
  for (const value of [
    { ...response, scope: { ...scope, daemonId: "foreign" } },
    { ...response, projects: [first, first] },
    { ...response, projects: [{ ...first, configuration: "verified" }] },
    {
      ...response,
      projects: [{ ...first, configFilesReported: Array(129).fill("/remote") }],
    },
    {
      ...response,
      projects: [
        {
          ...first,
          instances: [{ ...first.instances[0], containerId: "short" }],
        },
      ],
    },
  ]) {
    mockIPC(() => value);
    await assert.rejects(
      listCompose(scope, () => scope),
      { code: "invalid_response" },
    );
  }
});
