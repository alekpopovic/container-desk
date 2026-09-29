import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import {
  inspectContainer,
  mutateContainer,
  mutateComposeProject,
} from "./client.ts";
import type {
  ContainerDetail,
  MutationSpec,
  ComposeActionSpec,
} from "./generated.ts";

test("container and Compose completion or lost response cannot reuse a read begun before the action", async () => {
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: { crypto: globalThis.crypto },
  });
  try {
    for (const compose of [false, true])
      for (const lost of [false, true]) {
        const detail: ContainerDetail = JSON.parse(
          readFileSync(
            new URL("../../../tests/fixtures/ipc.json", import.meta.url),
            "utf8",
          ),
        ).detail;
        const scope = detail.summary.scope;
        const id = detail.summary.id;
        const request = { scope, containerId: id, revealSensitive: false };
        let release: ((v: unknown) => void) | undefined;
        let reads = 0,
          writes = 0;
        const spec: MutationSpec = {
          operation: "start",
          containerIds: [id],
          timeoutSeconds: 1,
        };
        const project: ComposeActionSpec = {
          ...spec,
          verificationId: `v_${"a".repeat(32)}`,
          configuration: {
            projectName: "owned",
            workingDirectory: "/srv/owned",
            configFiles: ["/srv/owned/compose.yml"],
          },
          services: ["web"],
          operation: "start",
        };
        mockIPC((command) => {
          if (command === "inspect_container") {
            if (++reads === 1)
              return new Promise((resolve) => {
                release = resolve;
              });
            const fresh = structuredClone(detail);
            fresh.summary.state = "running";
            return fresh;
          }
          assert.equal(
            command,
            compose ? "mutate_compose_project" : "mutate_container",
          );
          writes++;
          if (lost) throw { code: "transport_unavailable" };
          return {
            scope,
            spec: compose ? project : spec,
            outcome: "succeeded",
            results: [
              {
                containerId: id,
                outcome: "succeeded",
                dispatched: true,
                error: null,
              },
            ],
          };
        });
        const old = inspectContainer(request, () => scope);
        const fenced = assert.rejects(old, { code: "operation_cancelled" });
        const intentId = `i_${"b".repeat(32)}`;
        const mutation = compose
          ? mutateComposeProject(
              { scope, spec: project, intentId },
              () => scope,
            )
          : mutateContainer(scope, spec, intentId, () => scope);
        if (lost)
          await assert.rejects(mutation, { code: "transport_unavailable" });
        else await mutation;
        await fenced;
        const fresh = inspectContainer(request, () => scope);
        assert.equal(
          reads,
          2,
          "post-action observation must invoke a fresh native read",
        );
        detail.summary.state = "exited";
        assert.ok(release);
        release(detail);
        assert.equal((await fresh).summary.state, "running");
        assert.equal(writes, 1);
        clearMocks();
      }
  } finally {
    clearMocks();
    Reflect.deleteProperty(globalThis, "window");
  }
});
