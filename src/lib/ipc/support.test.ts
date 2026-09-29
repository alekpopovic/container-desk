import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { afterEach, beforeEach, test } from "node:test";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import {
  prepareSupportReport,
  saveSupportReport,
  clearSupportData,
  IpcError,
} from "./client.ts";
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
test("support preview is bounded; save sends only the frozen identifier and clearing never retries", async () => {
  const id = "a".repeat(32);
  let response: unknown = {
    id,
    report: '{"schemaVersion":1}',
    expiresInSeconds: 300,
  };
  const calls: unknown[] = [];
  mockIPC((command, args) => {
    calls.push([command, args]);
    if (command === "prepare_support_report") return response;
    if (command === "save_support_report") return false;
    throw { code: "storage_unavailable", message: "SYNTHETIC_SECRET" };
  });
  const preview = await prepareSupportReport();
  assert.equal(await saveSupportReport(preview.id), false);
  assert.deepEqual(calls[1], [
    "save_support_report",
    { request: { previewId: id } },
  ]);
  await assert.rejects(
    clearSupportData(),
    (error) =>
      error instanceof IpcError && !error.message.includes("SYNTHETIC_SECRET"),
  );
  assert.equal(calls.length, 3);
  for (const report of [
    "not json",
    "x".repeat(65537),
    '"' + "é".repeat(32768) + '"',
  ]) {
    response = { id, report, expiresInSeconds: 300 };
    await assert.rejects(prepareSupportReport(), { code: "invalid_response" });
  }
});
test("every typed support error hides raw synthetic error text", async () => {
  const generated = readFileSync(
    new URL("./generated.ts", import.meta.url),
    "utf8",
  );
  const declaration = generated.match(/^export type ErrorCode = (.+);$/m)?.[1];
  assert.ok(declaration);
  const codes = declaration
    .split(" | ")
    .map((value) => JSON.parse(value) as string);
  for (const code of codes) {
    mockIPC(() => {
      throw { code, message: "SYNTHETIC_SECRET_CONFIG_KEY_LOG_TERMINAL" };
    });
    await assert.rejects(
      prepareSupportReport(),
      (error) =>
        error instanceof IpcError &&
        error.code === code &&
        !error.message.includes("SYNTHETIC_SECRET"),
    );
  }
});
