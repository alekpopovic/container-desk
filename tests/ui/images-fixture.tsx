import { useMemo, useRef, useState } from "react";
import { mockIPC } from "@tauri-apps/api/mocks";
import { ImageInventory } from "../../src/features/images/ImageInventory";
import type {
  ImageSummary,
  InspectImageRequest,
  ListImagesRequest,
} from "../../src/lib/ipc/generated";
import fixtures from "../fixtures/ipc.json";
export function ImagesFixture() {
  const [epoch, setEpoch] = useState(1),
    [hold, setHold] = useState(false),
    [fail, setFail] = useState(false),
    [opened, setOpened] = useState("");
  const release = useRef<(() => void) | null>(null);
  const scope = useMemo(
    () => ({
      ...fixtures.detail.summary.scope,
      selection: {
        ...fixtures.detail.summary.scope.selection,
        hostId: `h_${String(epoch).repeat(32)}`,
        selectionGeneration: epoch,
      },
      sessionGeneration: epoch,
      daemonId: `image-host-${epoch}`,
    }),
    [epoch],
  );
  const imageId = `sha256:${(epoch === 1 ? "c" : "f").repeat(64)}`;
  const danglingId = `sha256:${"b".repeat(64)}`;
  const rows: ImageSummary[] = [
    {
      scope,
      id: imageId,
      tags: ["same:latest", "same:stable"],
      digests: [],
      sizeReported: "12.3MB",
      createdAtReported: "2026-09-29",
    },
    {
      scope,
      id: danglingId,
      tags: [],
      digests: [],
      sizeReported: "0B",
      createdAtReported: null,
    },
    ...Array.from({ length: 51 }, (_, index) => ({
      scope,
      id: `sha256:${(index + 1).toString(16).padStart(64, "0")}`,
      tags: [`other:${index}`],
      digests: [],
      sizeReported: null,
      createdAtReported: null,
    })),
  ];
  mockIPC((command, args) => {
    const request = (args as { request: unknown }).request;
    if (command === "list_images") {
      if (fail) throw { code: "permission_denied" };
      const selected = request as ListImagesRequest;
      const response = {
        scope,
        danglingOnly: selected.danglingOnly,
        images: selected.danglingOnly ? [rows[1]] : rows,
      };
      if (hold && epoch === 1)
        return new Promise((resolve) => {
          release.current = () => resolve(response);
        });
      return response;
    }
    if (command === "inspect_image") {
      const selected = request as InspectImageRequest;
      const row = rows.find((row) => row.id === selected.imageId);
      return {
        scope,
        id: selected.imageId,
        tags: row?.tags ?? [],
        digests: [],
        sizeBytes: 0,
        createdAt: null,
        os: "linux",
        architecture: "amd64",
        variant: null,
        labels: [
          { name: "<img src=x onerror=alert(1)>", value: null, masked: true },
        ],
        containers:
          selected.imageId === imageId
            ? [
                {
                  containerId: fixtures.detail.summary.id,
                  name: `Reference host ${epoch}`,
                  state: "created",
                },
              ]
            : [],
      };
    }
    throw Error(`Unexpected image fixture command ${command}`);
  });
  return (
    <main style={{ padding: 24 }}>
      <button type="button" onClick={() => setEpoch((n) => n + 1)}>
        Fixture switch image host
      </button>
      <button type="button" onClick={() => setHold(true)}>
        Fixture hold images
      </button>
      <button type="button" onClick={() => release.current?.()}>
        Fixture release images
      </button>
      <button type="button" onClick={() => setFail(true)}>
        Fixture image failure
      </button>
      <fieldset aria-label="Opened image reference">{opened}</fieldset>
      <ImageInventory
        view={{
          scope,
          rows: [{ ...fixtures.detail.summary, scope }],
          selectedId: null,
          updatedAt: 1,
          stale: false,
          loading: false,
          error: null,
        }}
        native
        refreshContainers={() => {}}
        openContainer={(scope, id) => setOpened(`${scope.daemonId}:${id}`)}
      />
    </main>
  );
}
