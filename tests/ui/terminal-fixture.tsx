import { StrictMode, useMemo, useRef, useState } from "react";
import { mockIPC } from "@tauri-apps/api/mocks";
import { ContainerConsole } from "../../src/features/terminal/ContainerConsole";
import type {
  SessionScope,
  TerminalInputRequest,
  TerminalHandleRequest,
  TerminalResizeRequest,
  PrepareConfirmationRequest,
} from "../../src/lib/ipc/generated";
type Owned = {
  key: TerminalHandleRequest;
  sequence: number;
  initial: boolean;
  revoked: boolean;
  flood: boolean;
};
export function TerminalFixture() {
  const [host, setHost] = useState(1);
  const [closed, setClosed] = useState(0);
  const [opens, setOpens] = useState(0);
  const [inputs, setInputs] = useState<string[]>([]);
  const [sizes, setSizes] = useState("");
  const owners = useRef(new Map<string, Owned>());
  const serial = useRef(0);
  const permissions = useRef(
    new Map<string, { management: boolean; terminal: boolean }>(),
  );
  const scope: SessionScope = useMemo(
    () => ({
      selection: {
        hostId: `h_${String(host).repeat(32)}`,
        selectionGeneration: host,
      },
      sessionId: `s_${String(host + 2).repeat(32)}`,
      sessionGeneration: host,
      daemonId: `fixture-engine-${host}`,
    }),
    [host],
  );
  mockIPC((command, args) => {
    const request = (args as { request: Record<string, unknown> })?.request;
    const selected = request?.scope as SessionScope | undefined;
    const permission = permissions.current.get(
      selected?.selection.hostId ?? "",
    ) ?? { management: false, terminal: false };
    if (command === "get_management" || command === "get_terminal_permission")
      return {
        scope: selected,
        enabled:
          command === "get_management"
            ? permission.management
            : permission.terminal,
      };
    if (command === "set_management" || command === "set_terminal_permission") {
      const value = { ...permission };
      if (command === "set_management") value.management = !!request?.enabled;
      else value.terminal = !!request?.enabled;
      permissions.current.set(selected?.selection.hostId ?? "", value);
      if (!value.terminal)
        for (const owner of owners.current.values())
          if (owner.key.scope.selection.hostId === selected?.selection.hostId)
            owner.revoked = true;
      return request;
    }
    if (command === "prepare_confirmation")
      return {
        ...(request as unknown as PrepareConfirmationRequest),
        id: `i_${"a".repeat(32)}`,
        expiresInMs: 30000,
      };
    if (command === "open_container_terminal") {
      const terminalId = `sub_${(++serial.current).toString(16).padStart(32, "0")}`;
      const key = { scope: selected as SessionScope, terminalId };
      owners.current.set(terminalId, {
        key,
        sequence: 0,
        initial: true,
        revoked: false,
        flood: false,
      });
      setOpens((n) => n + 1);
      return key;
    }
    if (command === "close_terminal") {
      owners.current.delete(String(request?.terminalId));
      setClosed((n) => n + 1);
      return null;
    }
    const owner = owners.current.get(String(request?.terminalId));
    if (command === "read_terminal") {
      if (!owner) throw { code: "terminal_closed" };
      const text = owner.initial
        ? "\x1b]52;c;c3ludGhldGljLWNsaXBib2FyZA==\x07\x1b]2;untrusted-title\x07\x1b]8;;https://example.invalid\x07link\x1b]8;;\x07\r\n<svg onload=alert(1)> synthetic terminal output\r\n"
        : owner.flood
          ? "synthetic-040-stream\r\n".repeat(500)
          : "";
      owner.initial = false;
      return {
        ...owner.key,
        sequence: ++owner.sequence,
        bytes: Array.from(new TextEncoder().encode(text)),
        state: owner.revoked ? "exited" : "running",
        exitCode: null,
        error: owner.revoked ? "permission_denied" : null,
      };
    }
    if (command === "write_terminal") {
      const value = request as unknown as TerminalInputRequest;
      if (!owner) throw { code: "terminal_closed" };
      setInputs((items) => [
        ...items,
        `${value.scope.selection.hostId}:${new TextDecoder().decode(Uint8Array.from(value.bytes))}`,
      ]);
      return null;
    }
    if (command === "resize_terminal") {
      const size = request as unknown as TerminalResizeRequest;
      setSizes(`${size.columns}x${size.rows}`);
      return null;
    }
    return null;
  });
  return (
    <main style={{ padding: 20 }}>
      <button type="button" onClick={() => setHost((n) => (n === 1 ? 2 : 1))}>
        Fixture switch host
      </button>
      <button
        type="button"
        onClick={() => {
          for (const owner of owners.current.values()) owner.flood = true;
        }}
      >
        Fixture output flood
      </button>
      <button
        type="button"
        onClick={() => {
          for (const owner of owners.current.values()) owner.revoked = true;
        }}
      >
        Fixture revoke permission
      </button>
      <output aria-label="Fixture terminal opens">{opens}</output>
      <output aria-label="Fixture terminal closes">{closed}</output>
      <output aria-label="Fixture terminal input">{inputs.join("|")}</output>
      <output aria-label="Fixture terminal size">{sizes}</output>
      <StrictMode>
        <ContainerConsole
          key={host}
          scope={scope}
          id={String(host + 4).repeat(64)}
          name={`container-${host}`}
          host={`Fixture host ${host}`}
        />
      </StrictMode>
    </main>
  );
}
