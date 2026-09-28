import assert from "node:assert/strict";
import { beforeEach, afterEach, test } from "node:test";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { readFileSync } from "node:fs";
import {
  discoverSshHosts,
  selectSshAlias,
  getSshConfigPath,
  isConcreteAlias,
} from "./client.ts";
const fixtures = JSON.parse(
  readFileSync(
    new URL("../../../tests/fixtures/ipc.json", import.meta.url),
    "utf8",
  ),
);
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
test("discovery decodes Rust shape and manual selection is explicit without connect", async () => {
  const commands: string[] = [];
  mockIPC((command, args) => {
    commands.push(command);
    if (command === "get_ssh_config_path")
      return { path: fixtures.discovery.configPath };
    if (command === "discover_ssh_hosts") {
      assert.deepEqual(args, { request: { configPath: null } });
      return fixtures.discovery;
    }
    assert.equal(command, "select_ssh_alias");
    assert.deepEqual(args, {
      request: { configPath: null, alias: "manual-01" },
    });
    return { configPath: fixtures.discovery.configPath, alias: "manual-01" };
  });
  assert.equal((await getSshConfigPath()).path, "/fixture/.ssh/config");
  assert.equal(
    (await discoverSshHosts(null)).candidates[0]?.alias,
    "fixture-host",
  );
  assert.equal((await selectSshAlias(null, "manual-01")).alias, "manual-01");
  assert.deepEqual(commands, [
    "get_ssh_config_path",
    "discover_ssh_hosts",
    "select_ssh_alias",
  ]);
});
test("option injection and malformed candidates are rejected without exposing raw errors", async () => {
  let calls = 0;
  mockIPC(() => {
    calls += 1;
    return {
      ...fixtures.discovery,
      candidates: [{ alias: "*.example", source: "/tmp/config", line: 1 }],
    };
  });
  for (const alias of [
    "-F",
    "host name",
    "x;id",
    "user@host",
    "a\nb",
    "a".repeat(257),
  ]) {
    assert.equal(isConcreteAlias(alias), false);
    await assert.rejects(selectSshAlias(null, alias), {
      code: "invalid_alias",
    });
  }
  assert.equal(calls, 0);
  await assert.rejects(discoverSshHosts(null), { code: "invalid_response" });
  mockIPC(() => {
    throw { code: "permission_denied", message: "PRIVATE_CONTENT" };
  });
  await assert.rejects(
    discoverSshHosts(null),
    (error: unknown) =>
      error instanceof Error && !error.message.includes("PRIVATE_CONTENT"),
  );
});
