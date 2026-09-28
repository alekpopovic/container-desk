import assert from "node:assert/strict";
import { afterEach, beforeEach, test } from "node:test";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import {
  getHostInventory,
  saveHost,
  connectInventoryHost,
  decodeInventory,
} from "./client.ts";
import type { HostInventory, HostDraft } from "./generated.ts";
const draft: HostDraft = {
  ssh: {
    alias: "fixture",
    configPath: "/fixture/config",
    useDefaultConfig: false,
  },
  docker: { context: "rootless", executable: null, sudo: false },
  displayName: "Same name",
  group: "prod",
  labels: ["<text>"],
  favorite: true,
};
function fixture(): HostInventory {
  return {
    mode: "live",
    saved: {
      writable: true,
      notice: null,
      preferences: {
        schemaVersion: 3,
        revision: 1,
        theme: "system",
        selectedHostId: null,
        hosts: [
          {
            ...draft,
            id: `h_${"1".repeat(32)}`,
            alias: draft.ssh.alias,
            readOnly: true,
          },
        ],
        trustedConfigPath: null,
        sshExecutableOverride: null,
      },
    },
    connection: null,
  };
}
beforeEach(() =>
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: { crypto: globalThis.crypto },
  }),
);
afterEach(() => clearMocks());
test("native inventory writes stay revision-bound and mode mismatches never become demo fallback", async () => {
  let calls = 0;
  mockIPC((command, args) => {
    calls++;
    assert.equal(command, "save_host");
    assert.equal((args as { request: { mode: string } }).request.mode, "live");
    return fixture();
  });
  const saved = await saveHost({
    mode: "live",
    expectedRevision: 0,
    id: null,
    draft,
  });
  assert.equal(saved.saved.preferences.hosts[0]?.favorite, true);
  assert.equal(calls, 1);
  mockIPC(() => ({ ...fixture(), mode: "demo" }));
  await assert.rejects(getHostInventory("live"), { code: "invalid_response" });
  mockIPC(() => fixture());
  await assert.rejects(
    saveHost({ mode: "live", expectedRevision: 2, id: null, draft }),
    { code: "invalid_response" },
  );
});
test("duplicate names preserve distinct IDs and foreign host sessions are rejected", async () => {
  const value = fixture();
  const second = {
    ...value.saved.preferences.hosts[0]!,
    id: `h_${"2".repeat(32)}`,
    alias: "second",
    ssh: { ...draft.ssh, alias: "second" },
  };
  value.saved.preferences.hosts.push(second);
  assert.equal(
    decodeInventory(value, "live").saved.preferences.hosts.length,
    2,
  );
  value.connection = {
    hostId: second.id,
    token: { sessionId: `s_${"3".repeat(32)}`, sessionGeneration: 1 },
    selection: draft.ssh,
    effective: null,
    state: "connecting",
    durations: [],
    diagnostic: null,
    hasJump: false,
    transportMode: "unconnected",
    dockerOptions: draft.docker,
    docker: null,
  };
  assert.throws(() => decodeInventory(value, "live"), {
    code: "invalid_response",
  });
  value.connection.hostId = value.saved.preferences.hosts[0]!.id;
  mockIPC(() => value);
  await assert.rejects(
    connectInventoryHost({ mode: "live", hostId: second.id }),
    { code: "invalid_response" },
  );
  value.saved.preferences.hosts[1]!.id = value.saved.preferences.hosts[0]!.id;
  assert.throws(() => decodeInventory(value, "live"), {
    code: "invalid_response",
  });
});
