import "./inspect-fixture.css";
import { useMemo, useRef, useState } from "react";
import { mockIPC } from "@tauri-apps/api/mocks";
import { ContainerDetails } from "../../src/features/containers/ContainerDetails";
import type {
  ContainerDetail,
  InspectContainerRequest,
} from "../../src/lib/ipc/generated";
import fixtures from "../fixtures/ipc.json";
export function InspectFixture() {
  const [id, setId] = useState("a".repeat(64));
  const [snapshotVersion, setSnapshotVersion] = useState(0);
  const [epoch, setEpoch] = useState(0);
  const [pending, setPending] = useState(false);
  const deferred = useRef<(() => void) | null>(null);
  const fail = useRef(false);
  mockIPC((command, args) => {
    if (command !== "inspect_container")
      throw new Error("Unexpected fixture command");
    if (!args || Array.isArray(args) || !("request" in args))
      throw new Error("Invalid fixture request");
    const request = args.request as InspectContainerRequest;
    if (fail.current) throw { code: "container_not_found" };
    const detail = structuredClone(fixtures.detail) as ContainerDetail;
    detail.summary.scope = request.scope;
    detail.summary.id = request.containerId;
    detail.imageId = `sha256:${"c".repeat(64)}`;
    detail.exitCode = 137;
    detail.oomKilled = true;
    detail.healthcheckConfigured = true;
    detail.summary.state = "running";
    detail.summary.health = "unhealthy";
    detail.exposedPorts = [
      { privatePort: 80, protocol: "tcp" },
      { privatePort: 53, protocol: "udp" },
    ];
    detail.summary.ports = [
      { privatePort: 80, protocol: "tcp", hostIp: "0.0.0.0", publicPort: 8080 },
      { privatePort: 80, protocol: "tcp", hostIp: "::", publicPort: 8080 },
      {
        privatePort: 80,
        protocol: "tcp",
        hostIp: "127.0.0.1",
        publicPort: 18080,
      },
      { privatePort: 53, protocol: "udp", hostIp: null, publicPort: null },
    ];
    detail.mounts = [
      {
        kind: "bind",
        name: null,
        source: "/fixture/" + "long-directory-".repeat(30),
        destination: "/data/" + "long-destination-".repeat(20),
        readWrite: false,
        propagation: "rprivate",
      },
    ];
    detail.networks = [
      {
        name: "<img src=x onerror=alert(1)>",
        networkId: "fixture-network",
        ipv4: "10.1.0.2",
        ipv6: "fd00::2",
        gateway: null,
        macAddress: null,
        aliases: [],
      },
    ];
    detail.environmentValuesMasked = !request.revealSensitive;
    detail.environment = [
      {
        name: "SYNTHETIC_TOKEN",
        value: request.revealSensitive
          ? "synthetic-ui-secret <img src=x onerror=alert(1)> https://example.invalid"
          : null,
        masked: !request.revealSensitive,
      },
    ];
    detail.environmentNames = ["SYNTHETIC_TOKEN"];
    detail.labels = [
      {
        name: "innocent",
        value: request.revealSensitive
          ? 'synthetic-ui-label <a href="https://example.invalid">link</a>'
          : null,
        masked: !request.revealSensitive,
      },
    ];
    if (new URLSearchParams(location.search).has("emptyLabels"))
      detail.labels = [];
    if (request.revealSensitive && deferred.current === null) {
      setPending(true);
      return new Promise((resolve) => {
        deferred.current = () => {
          resolve(detail);
          setPending(false);
        };
      });
    }
    return detail;
  });
  const scope = useMemo(
    () => ({
      ...fixtures.detail.summary.scope,
      sessionGeneration: 3 + epoch,
    }),
    [epoch],
  );
  return (
    <div className="container-split inspect-fixture-shell">
      <button type="button" onClick={() => setSnapshotVersion((v) => v + 1)}>
        Refresh fixture inventory
      </button>
      <button
        type="button"
        onClick={() => {
          deferred.current?.();
        }}
      >
        Finish fixture reveal
      </button>
      <button type="button" onClick={() => setId("b".repeat(64))}>
        Switch fixture container
      </button>
      <button type="button" onClick={() => setEpoch((v) => v + 1)}>
        Reconnect fixture session
      </button>
      <button
        type="button"
        onClick={() => {
          fail.current = true;
        }}
      >
        Delete fixture container
      </button>
      <p>{pending ? "Fixture reveal pending" : "Fixture ready"}</p>
      <div className="detail-panel inspect-fixture-panel">
        <ContainerDetails
          snapshotVersion={snapshotVersion}
          key={`${id}-${epoch}`}
          scope={scope}
          id={id}
        />
      </div>
    </div>
  );
}
