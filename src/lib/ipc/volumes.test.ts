import assert from "node:assert/strict";
import { afterEach, beforeEach, test } from "node:test";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { inspectVolume, listVolumes } from "./client.ts";
const scope = {
  selection: { hostId: `h_${"a".repeat(32)}`, selectionGeneration: 1 },
  sessionId: `s_${"b".repeat(32)}`,
  sessionGeneration: 1,
  daemonId: "volume-engine",
};
const summary = {
  scope,
  name: "named-data",
  driver: "local",
  volumeScope: "local",
};
const detail = {
  summary,
  createdAt: null,
  mountpointReported: null,
  labels: [{ name: "token", value: null, masked: true }],
  options: [],
  references: [
    {
      containerId: "c".repeat(64),
      name: "using-data",
      state: "created",
      destination: "/data",
      readOnly: true,
    },
  ],
  referenceObservation: "referenced",
  unresolvedContainerIds: [],
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
test("volume scope and valid unique names are required before display", async () => {
  mockIPC((command, args) => {
    assert.equal(command, "list_volumes");
    assert.deepEqual(args, { request: { scope } });
    return { scope, volumes: [summary] };
  });
  assert.deepEqual((await listVolumes(scope, () => scope)).volumes, [summary]);
  for (const volumes of [
    [summary, summary],
    [{ ...summary, name: "../data" }],
    [{ ...summary, scope: { ...scope, daemonId: "other" } }],
  ]) {
    mockIPC(() => ({ scope, volumes }));
    await assert.rejects(
      listVolumes(scope, () => scope),
      { code: "invalid_response" },
    );
  }
});
test("volume detail rejects secret values, duplicate mounts and inconsistent reference observations", async () => {
  mockIPC(() => detail);
  assert.deepEqual(
    await inspectVolume({ scope, name: summary.name }, () => scope),
    detail,
  );
  for (const value of [
    { ...detail, labels: [{ name: "token", value: "private", masked: true }] },
    {
      ...detail,
      options: [{ name: "password", value: "private", masked: false }],
    },
    { ...detail, references: [...detail.references, ...detail.references] },
    { ...detail, referenceObservation: "unreferenced" },
    { ...detail, unresolvedContainerIds: [detail.references[0]!.containerId] },
    { ...detail, summary: { ...summary, name: "wrong-volume" } },
  ]) {
    mockIPC(() => value);
    await assert.rejects(
      inspectVolume({ scope, name: summary.name }, () => scope),
      { code: "invalid_response" },
    );
  }
  const incomplete = {
    ...detail,
    referenceObservation: "incomplete",
    unresolvedContainerIds: ["d".repeat(64)],
  };
  mockIPC(() => incomplete);
  assert.deepEqual(
    await inspectVolume({ scope, name: summary.name }, () => scope),
    incomplete,
  );
});
test("a late volume response cannot be attached to another selected daemon", async () => {
  let selected = scope;
  let finish: (value: unknown) => void = () => {};
  mockIPC(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  const request = listVolumes(scope, () => selected);
  selected = { ...scope, daemonId: "other" };
  finish({ scope, volumes: [summary] });
  await assert.rejects(request, { code: "stale_session" });
});
