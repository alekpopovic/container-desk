import { useMemo, useRef, useState } from "react";
import { mockIPC } from "@tauri-apps/api/mocks";
import { VolumeInventory } from "../../src/features/volumes/VolumeInventory";
import type {
  InspectVolumeRequest,
  VolumeSummary,
} from "../../src/lib/ipc/generated";
import fixtures from "../fixtures/ipc.json";
export function VolumesFixture() {
  const [epoch, setEpoch] = useState(1),
    [hold, setHold] = useState(false),
    [fail, setFail] = useState(false),
    [missing, setMissing] = useState(false),
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
      daemonId: `volume-host-${epoch}`,
    }),
    [epoch],
  );
  const rows: VolumeSummary[] = [
    { scope, name: "named-data", driver: "local", volumeScope: "local" },
    { scope, name: "a".repeat(64), driver: "local", volumeScope: "local" },
    {
      scope,
      name: "plugin-data",
      driver: "vendor/plugin:latest",
      volumeScope: "global",
    },
    ...Array.from({ length: 50 }, (_, index) => ({
      scope,
      name: `unused-${index}`,
      driver: null,
      volumeScope: null,
    })),
  ];
  mockIPC((command, args) => {
    if (command === "list_volumes") {
      if (fail) throw { code: "permission_denied" };
      const response = { scope, volumes: rows };
      if (hold && epoch === 1)
        return new Promise((resolve) => {
          release.current = () => resolve(response);
        });
      return response;
    }
    if (command === "inspect_volume") {
      const request = (args as { request: InspectVolumeRequest }).request;
      const summary = rows.find((row) => row.name === request.name)!;
      const hasRef =
        request.name === "named-data" || request.name === "a".repeat(64);
      return {
        summary,
        createdAt: null,
        mountpointReported:
          request.name === "plugin-data"
            ? null
            : "/var/lib/docker/volumes/metadata-only/_data",
        labels: [
          { name: "<img src=x onerror=alert(1)>", value: null, masked: true },
        ],
        options: [{ name: "password", value: null, masked: true }],
        references:
          hasRef && !missing
            ? [
                {
                  containerId: fixtures.detail.summary.id,
                  name: `Reference host ${epoch}`,
                  state: "created",
                  destination: "/data",
                  readOnly: true,
                },
              ]
            : [],
        referenceObservation: hasRef
          ? missing
            ? "incomplete"
            : "referenced"
          : "unreferenced",
        unresolvedContainerIds:
          hasRef && missing ? [fixtures.detail.summary.id] : [],
      };
    }
    throw Error(`Unexpected volume fixture command ${command}`);
  });
  return (
    <main style={{ padding: 24 }}>
      <button type="button" onClick={() => setEpoch((n) => n + 1)}>
        Fixture switch volume host
      </button>
      <button type="button" onClick={() => setHold(true)}>
        Fixture hold volumes
      </button>
      <button type="button" onClick={() => release.current?.()}>
        Fixture release volumes
      </button>
      <button type="button" onClick={() => setFail(true)}>
        Fixture volume failure
      </button>
      <button type="button" onClick={() => setMissing(true)}>
        Fixture container disappeared
      </button>
      <fieldset aria-label="Opened volume reference">{opened}</fieldset>
      <VolumeInventory
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
