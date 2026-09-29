import test from "node:test";
import assert from "node:assert/strict";
import { HostRecovery } from "../../features/hosts/recovery.ts";
import type {
  HostInventory,
  ConnectionState,
  ConnectionDiagnosticCode,
} from "./generated.ts";
function observed(
  state: ConnectionState,
  code: ConnectionDiagnosticCode | null = null,
  id = "selected-host",
): HostInventory {
  return {
    mode: "live",
    saved: { preferences: { selectedHostId: id } },
    connection: { hostId: id, state, diagnostic: code ? { code } : null },
  } as unknown as HostInventory;
}
test("recovery requires an observed ready selection and never connects saved/offline or authentication-failed hosts", () => {
  const recovery = new HostRecovery();
  assert.equal(
    recovery.observe(observed("error", "connection_failed"), 0).connect,
    null,
  );
  recovery.observe(observed("ready"), 0);
  recovery.observe(observed("degraded", "connection_lost"), 1);
  assert.equal(
    recovery.observe(observed("degraded", "connection_lost"), 1001).connect,
    "selected-host",
  );
  assert.match(
    recovery.observe(observed("error", "changed_host_key"), 1002).message!,
    /stopped/,
  );
  assert.equal(
    recovery.observe(observed("error", "connection_failed"), 90000).connect,
    null,
  );
  recovery.observe(observed("ready"), 100000);
  recovery.observe(observed("disconnected"), 100001);
  assert.equal(
    recovery.observe(observed("error", "connection_failed"), 200000).connect,
    null,
  );
});
test("reconnect backoff is finite, preserves attempts through flapping, and resets after stable readiness or explicit action", () => {
  const recovery = new HostRecovery();
  const lost = observed("degraded", "connection_lost");
  recovery.observe(observed("ready"), 0);
  assert.equal(recovery.observe(lost, 1).connect, null);
  assert.equal(recovery.observe(lost, 1000).connect, null);
  assert.equal(recovery.observe(lost, 1001).connect, "selected-host");
  assert.equal(recovery.observe(observed("connecting"), 1002).connect, null);
  assert.match(recovery.observe(observed("ready"), 1003).message!, /read-only/);
  assert.equal(recovery.observe(lost, 1004).connect, null);
  assert.equal(recovery.observe(lost, 3003).connect, null);
  assert.equal(recovery.observe(lost, 3004).connect, "selected-host");
  recovery.observe(lost, 3005);
  assert.equal(recovery.observe(lost, 7005).connect, "selected-host");
  assert.match(recovery.observe(lost, 90000).message!, /Three/);
  assert.equal(recovery.observe(lost, 100000).connect, null);
  recovery.observe(observed("ready"), 100001);
  assert.equal(recovery.observe(observed("ready"), 160001).message, null);
  recovery.observe(lost, 160002);
  assert.equal(recovery.observe(lost, 161002).connect, "selected-host");
  recovery.reset();
  assert.equal(recovery.observe(lost, 200000).connect, null);
});
test("hidden/wake observations do not spend attempts or connect another selected host", () => {
  const recovery = new HostRecovery();
  recovery.observe(observed("ready"), 0);
  recovery.observe(observed("degraded", "connection_lost"), 1, false);
  assert.equal(
    recovery.observe(observed("degraded", "connection_lost"), 5000, false)
      .connect,
    null,
  );
  assert.equal(
    recovery.observe(observed("degraded", "connection_lost"), 5000, true)
      .connect,
    "selected-host",
  );
  assert.equal(
    recovery.observe(
      observed("degraded", "connection_lost", "other-host"),
      6000,
    ).connect,
    null,
  );
  assert.equal(
    recovery.observe(
      observed("degraded", "connection_lost", "other-host"),
      600000,
    ).connect,
    null,
  );
});
test("a failed reconnect admission keeps its finite budget even if the old session was already invalidated", () => {
  const recovery = new HostRecovery();
  const lost = observed("degraded", "connection_lost");
  recovery.observe(observed("ready"), 0);
  recovery.observe(lost, 1);
  assert.equal(recovery.observe(lost, 1001).connect, "selected-host");
  recovery.failedAttempt();
  recovery.observe(observed("disconnected"), 1002);
  assert.equal(
    recovery.observe(observed("disconnected"), 3002).connect,
    "selected-host",
  );
  recovery.reset();
  assert.equal(recovery.observe(observed("disconnected"), 90000).connect, null);
});
test("a prior admission failure cannot authorize retrying a later authentication failure", () => {
  const recovery = new HostRecovery();
  recovery.observe(observed("ready"), 0);
  recovery.observe(observed("degraded", "connection_lost"), 1);
  recovery.observe(observed("degraded", "connection_lost"), 1001);
  recovery.failedAttempt();
  recovery.observe(observed("disconnected"), 1002);
  recovery.observe(observed("disconnected"), 3002);
  recovery.acceptedAttempt();
  assert.match(
    recovery.observe(observed("error", "authentication_failed"), 3003).message!,
    /stopped/,
  );
  assert.equal(
    recovery.observe(observed("error", "authentication_failed"), 90000).connect,
    null,
  );
});
