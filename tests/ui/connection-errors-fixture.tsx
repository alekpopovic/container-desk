import { useRef, useState } from "react";
import { mockIPC } from "@tauri-apps/api/mocks";
import { ConnectionPanel } from "../../src/features/hosts/ConnectionPanel";
import type {
  ConnectionDiagnosticCode,
  ConnectionSnapshot,
  DockerOptions,
} from "../../src/lib/ipc/generated";

export function ConnectionErrorsFixture() {
  const [starts, setStarts] = useState(0);
  const generation = useRef(0);
  const code = new URLSearchParams(location.search).get(
    "failure",
  ) as ConnectionDiagnosticCode;
  const allowed = [
    "unknown_host_key",
    "changed_host_key",
    "authentication_failed",
    "timed_out",
  ];
  if (!allowed.includes(code))
    throw new Error("Unknown deterministic connection fixture");
  const selected = {
    alias: "owned-fixture",
    configPath: "/fixture/config",
    useDefaultConfig: false,
  };
  const current = useRef<ConnectionSnapshot | null>(null);
  mockIPC((command, args) => {
    if (command === "begin_ssh_session") {
      generation.current++;
      setStarts(generation.current);
      current.current = {
        token: {
          sessionId: `s_${"a".repeat(32)}`,
          sessionGeneration: generation.current,
        },
        selection: selected,
        state: "connecting",
        durations: [],
        diagnostic: null,
        hasJump: true,
        hostId: null,
        effective: null,
        transportMode: "unconnected",
        dockerOptions: (args as { request: { docker: DockerOptions } }).request
          .docker,
        docker: null,
      };
      return current.current;
    }
    if (command === "get_ssh_session")
      return {
        ...current.current,
        state: "error",
        diagnostic: { stage: "authenticate", code },
      };
    if (command === "disconnect_ssh_session")
      return { ...current.current, state: "disconnected", diagnostic: null };
    throw new Error(`Unexpected connection fixture command ${command}`);
  });
  return (
    <main>
      <output aria-label="Connection starts">{starts}</output>
      <ConnectionPanel selection={selected} />
    </main>
  );
}
