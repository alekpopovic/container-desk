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
        value: request.revealSensitive ? "synthetic-ui-secret" : null,
        masked: !request.revealSensitive,
      },
    ];
    detail.environmentNames = ["SYNTHETIC_TOKEN"];
    detail.labels = [
      {
        name: "innocent",
        value: request.revealSensitive ? "synthetic-ui-label" : null,
        masked: !request.revealSensitive,
      },
    ];
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
    <div className="container-split">
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
      <ContainerDetails key={`${id}-${epoch}`} scope={scope} id={id} />
    </div>
  );
}
