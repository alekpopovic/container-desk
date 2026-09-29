import assert from "node:assert/strict";
import { afterEach, beforeEach, test } from "node:test";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import {
  verifyComposeProject,
  prepareComposeAction,
  mutateComposeProject,
} from "./client.ts";
import type { ComposeActionSpec } from "./generated.ts";
const scope = {
  selection: { hostId: `h_${"a".repeat(32)}`, selectionGeneration: 1 },
  sessionId: `s_${"b".repeat(32)}`,
  sessionGeneration: 1,
  daemonId: "compose-engine",
};
const configuration = {
  projectName: "owned",
  workingDirectory: "/srv/project's directory",
  configFiles: ["/srv/project's directory/base.yml", "/srv/override file.yml"],
};
const verified = {
  scope,
  id: `v_${"c".repeat(32)}`,
  configuration,
  services: ["web"],
  containerIds: ["d".repeat(64)],
  expiresInMs: 300000,
};
const spec: ComposeActionSpec = {
  verificationId: verified.id,
  configuration,
  services: verified.services,
  containerIds: verified.containerIds,
  operation: "restart",
  timeoutSeconds: 1,
};
const intentId = `i_${"e".repeat(32)}`;
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
test("Compose verification preserves ordered explicit configuration and exact scoped service identities", async () => {
  mockIPC((command, args) => {
    assert.equal(command, "verify_compose_project");
    assert.deepEqual(args, {
      request: { scope, configuration, acknowledged: true },
    });
    return verified;
  });
  assert.deepEqual(
    await verifyComposeProject(
      { scope, configuration, acknowledged: true },
      () => scope,
    ),
    verified,
  );
  for (const value of [
    {
      ...verified,
      configuration: {
        ...configuration,
        configFiles: [...configuration.configFiles].reverse(),
      },
    },
    { ...verified, containerIds: ["short"] },
    { ...verified, services: ["web", "web"] },
    { ...verified, scope: { ...scope, daemonId: "other" } },
  ]) {
    mockIPC(() => value);
    await assert.rejects(
      verifyComposeProject(
        { scope, configuration, acknowledged: true },
        () => scope,
      ),
      { code: "invalid_response" },
    );
  }
});
test("Compose confirmation cannot change a file, service, action or verification token", async () => {
  const response = {
    scope,
    id: intentId,
    operation: { category: "compose", spec },
    expiresInMs: 30000,
  };
  mockIPC(() => response);
  await prepareComposeAction(scope, spec, () => scope);
  for (const changed of [
    { ...spec, services: ["other"] },
    { ...spec, operation: "stop" },
    { ...spec, verificationId: `v_${"f".repeat(32)}` },
    { ...spec, configuration: { ...configuration, projectName: "unrelated" } },
  ]) {
    mockIPC(() => ({
      ...response,
      operation: { category: "compose", spec: changed },
    }));
    await assert.rejects(
      prepareComposeAction(scope, spec, () => scope),
      { code: "invalid_response" },
    );
  }
});
test("Compose mutation dispatches once and preserves unknown per-target outcomes", async () => {
  let calls = 0;
  mockIPC(() => {
    calls++;
    throw { code: "transport_unavailable" };
  });
  await assert.rejects(
    mutateComposeProject({ scope, intentId, spec }, () => scope),
    { code: "transport_unavailable" },
  );
  assert.equal(calls, 1);
  const response = {
    scope,
    spec,
    outcome: "unknown",
    results: [
      {
        containerId: spec.containerIds[0],
        outcome: "unknown",
        dispatched: true,
        error: "transport_unavailable",
      },
    ],
  };
  mockIPC(() => response);
  assert.deepEqual(
    await mutateComposeProject({ scope, intentId, spec }, () => scope),
    response,
  );
  mockIPC(() => ({ ...response, outcome: "succeeded" }));
  await assert.rejects(
    mutateComposeProject({ scope, intentId, spec }, () => scope),
    { code: "invalid_response" },
  );
});
