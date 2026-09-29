import assert from "node:assert/strict";
import { afterEach, beforeEach, test } from "node:test";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { inspectImage, listImages } from "./client.ts";
const scope = {
  selection: { hostId: `h_${"a".repeat(32)}`, selectionGeneration: 1 },
  sessionId: `s_${"b".repeat(32)}`,
  sessionGeneration: 1,
  daemonId: "image-engine",
};
const id = `sha256:${"c".repeat(64)}`;
const summary = {
  scope,
  id,
  tags: ["same:latest", "same:stable"],
  digests: [],
  sizeReported: "1MB",
  createdAtReported: null,
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
test("image inventory preserves full identity, tags and exact scope/filter", async () => {
  const response = { scope, danglingOnly: false, images: [summary] };
  mockIPC((command, args) => {
    assert.equal(command, "list_images");
    assert.deepEqual(args, { request: { scope, danglingOnly: false } });
    return response;
  });
  assert.deepEqual(
    (await listImages({ scope, danglingOnly: false }, () => scope)).images,
    [summary],
  );
  for (const value of [
    { ...response, danglingOnly: true },
    { ...response, images: [summary, summary] },
    {
      ...response,
      images: [{ ...summary, scope: { ...scope, daemonId: "other" } }],
    },
    { ...response, images: [{ ...summary, id: "short" }] },
  ]) {
    mockIPC(() => value);
    await assert.rejects(
      listImages({ scope, danglingOnly: false }, () => scope),
      { code: "invalid_response" },
    );
  }
});
test("image metadata fails closed on unmasked labels and foreign detail/reference", async () => {
  const detail = {
    scope,
    id,
    tags: [],
    digests: [],
    sizeBytes: 0,
    createdAt: null,
    os: "linux",
    architecture: "amd64",
    variant: null,
    labels: [{ name: "token", value: null, masked: true }],
    containers: [
      { containerId: "d".repeat(64), name: "using-image", state: "created" },
    ],
  };
  mockIPC(() => detail);
  assert.deepEqual(
    await inspectImage({ scope, imageId: id }, () => scope),
    detail,
  );
  for (const value of [
    { ...detail, labels: [{ name: "token", value: "private", masked: true }] },
    { ...detail, id: `sha256:${"f".repeat(64)}` },
    { ...detail, sizeBytes: Number.MAX_SAFE_INTEGER + 1 },
    { ...detail, containers: [...detail.containers, ...detail.containers] },
  ]) {
    mockIPC(() => value);
    await assert.rejects(
      inspectImage({ scope, imageId: id }, () => scope),
      { code: "invalid_response" },
    );
  }
});
test("identical tag on a newly selected host cannot receive an old image response", async () => {
  let selected = scope;
  let finish: (v: unknown) => void = () => {};
  mockIPC(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  const request = listImages({ scope, danglingOnly: false }, () => selected);
  selected = { ...scope, daemonId: "other-image-engine" };
  finish({ scope, danglingOnly: false, images: [summary] });
  await assert.rejects(request, { code: "stale_session" });
});
