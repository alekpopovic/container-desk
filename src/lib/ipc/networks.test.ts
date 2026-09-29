import assert from "node:assert/strict";
import { afterEach, beforeEach, test } from "node:test";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { inspectNetwork, listNetworks } from "./client.ts";
const scope = {
  selection: { hostId: `h_${"a".repeat(32)}`, selectionGeneration: 1 },
  sessionId: `s_${"b".repeat(32)}`,
  sessionGeneration: 1,
  daemonId: "network-engine",
};
const summary = {
  scope,
  id: "c".repeat(64),
  name: "custom",
  driver: "bridge",
  networkScope: "local",
  internal: true,
  ipv6: true,
};
const detail = {
  summary,
  createdAt: null,
  ipamDriver: null,
  ipamConfig: [],
  labels: [],
  options: [],
  ipamOptions: [],
  attachments: [
    {
      endpointKey: "d".repeat(64),
      containerId: "d".repeat(64),
      name: null,
      endpointId: null,
      ipv4Address: null,
      ipv6Address: "fd36::1/64",
    },
  ],
  attachmentsReported: true,
  metadataIncomplete: false,
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
test("network identity and scoped snapshot reject duplicate, hostile and foreign data", async () => {
  mockIPC(() => ({ scope, networks: [summary] }));
  assert.deepEqual((await listNetworks(scope, () => scope)).networks, [
    summary,
  ]);
  for (const networks of [
    [summary, summary],
    [{ ...summary, id: "--all" }],
    [{ ...summary, scope: { ...scope, daemonId: "other" } }],
    [{ ...summary, internal: "true" }],
  ]) {
    mockIPC(() => ({ scope, networks }));
    await assert.rejects(
      listNetworks(scope, () => scope),
      { code: "invalid_response" },
    );
  }
});
test("network optional metadata and unknown endpoints remain safe; secrets and wrong endpoint identity fail", async () => {
  mockIPC(() => detail);
  assert.deepEqual(
    await inspectNetwork({ scope, networkId: summary.id }, () => scope),
    detail,
  );
  const unknown = {
    ...detail,
    attachments: [
      {
        ...detail.attachments[0],
        endpointKey: "lb-endpoint",
        containerId: null,
      },
    ],
  };
  mockIPC(() => unknown);
  assert.deepEqual(
    await inspectNetwork({ scope, networkId: summary.id }, () => scope),
    unknown,
  );
  for (const response of [
    { ...detail, attachmentsReported: false },
    { ...detail, options: [{ name: "token", value: "private", masked: true }] },
    {
      ...detail,
      attachments: [{ ...detail.attachments[0], containerId: "e".repeat(64) }],
    },
    { ...detail, attachments: [...detail.attachments, ...detail.attachments] },
  ]) {
    mockIPC(() => response);
    await assert.rejects(
      inspectNetwork({ scope, networkId: summary.id }, () => scope),
      { code: "invalid_response" },
    );
  }
});
test("late network details never cross a host/session boundary", async () => {
  let selected = scope;
  let finish: (v: unknown) => void = () => {};
  mockIPC(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  const request = inspectNetwork(
    { scope, networkId: summary.id },
    () => selected,
  );
  selected = { ...scope, daemonId: "other" };
  finish(detail);
  await assert.rejects(request, { code: "stale_session" });
});
