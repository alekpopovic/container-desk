import { useMemo, useRef, useState } from "react";
import { mockIPC } from "@tauri-apps/api/mocks";
import { NetworkInventory } from "../../src/features/networks/NetworkInventory";
import type {
  InspectNetworkRequest,
  NetworkSummary,
} from "../../src/lib/ipc/generated";
import fixtures from "../fixtures/ipc.json";
export function NetworksFixture() {
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
      daemonId: `network-host-${epoch}`,
    }),
    [epoch],
  );
  const rows: NetworkSummary[] = [
    {
      scope,
      id: "c".repeat(64),
      name: "custom-bridge",
      driver: "bridge",
      networkScope: "local",
      internal: true,
      ipv6: true,
    },
    {
      scope,
      id: "d".repeat(64),
      name: "host",
      driver: "host",
      networkScope: "local",
      internal: false,
      ipv6: false,
    },
    {
      scope,
      id: "e".repeat(64),
      name: "none",
      driver: "null",
      networkScope: "local",
      internal: null,
      ipv6: null,
    },
    ...Array.from({ length: 50 }, (_, index) => ({
      scope,
      id: (index + 1).toString(16).padStart(64, "0"),
      name: `external-${index}`,
      driver: "vendor/plugin",
      networkScope: "global",
      internal: null,
      ipv6: null,
    })),
  ];
  mockIPC((command, args) => {
    if (command === "list_networks") {
      if (fail) throw { code: "permission_denied" };
      const response = { scope, networks: rows };
      if (hold && epoch === 1)
        return new Promise((resolve) => {
          release.current = () => resolve(response);
        });
      return response;
    }
    if (command === "inspect_network") {
      const request = (args as { request: InspectNetworkRequest }).request;
      const summary = rows.find((row) => row.id === request.networkId)!;
      const custom = summary.name === "custom-bridge";
      return {
        summary,
        createdAt: null,
        ipamDriver: custom ? "default" : null,
        ipamConfig: custom
          ? [
              {
                subnet: "10.36.0.0/24",
                gateway: "10.36.0.1",
                ipRange: null,
                auxiliaryAddresses: [],
              },
              {
                subnet: "fd36::/64",
                gateway: "fd36::1",
                ipRange: null,
                auxiliaryAddresses: [],
              },
            ]
          : [],
        labels: [
          { name: "<img src=x onerror=alert(1)>", value: null, masked: true },
        ],
        options: [{ name: "password", value: null, masked: true }],
        ipamOptions: [],
        attachments: custom
          ? [
              {
                endpointKey: fixtures.detail.summary.id,
                containerId: fixtures.detail.summary.id,
                name: `Reference host ${epoch}`,
                endpointId: "endpoint",
                ipv4Address: "10.36.0.2/24",
                ipv6Address: "fd36::2/64",
              },
              {
                endpointKey: "lb-unknown",
                containerId: null,
                name: null,
                endpointId: null,
                ipv4Address: null,
                ipv6Address: null,
              },
            ]
          : [],
        attachmentsReported: summary.name !== "none",
        metadataIncomplete: custom && missing,
      };
    }
    throw Error(`Unexpected network fixture command ${command}`);
  });
  return (
    <main style={{ padding: 24 }}>
      <button type="button" onClick={() => setEpoch((n) => n + 1)}>
        Fixture switch network host
      </button>
      <button type="button" onClick={() => setHold(true)}>
        Fixture hold networks
      </button>
      <button type="button" onClick={() => release.current?.()}>
        Fixture release networks
      </button>
      <button type="button" onClick={() => setFail(true)}>
        Fixture network failure
      </button>
      <button type="button" onClick={() => setMissing(true)}>
        Fixture stale endpoints
      </button>
      <fieldset aria-label="Opened network reference">{opened}</fieldset>
      <NetworkInventory
        view={{
          scope,
          rows: missing ? [] : [{ ...fixtures.detail.summary, scope }],
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
