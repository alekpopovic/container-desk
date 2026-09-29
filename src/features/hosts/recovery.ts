import type { HostInventory } from "../../lib/ipc/generated.ts";
/** Reconnect only a selected host observed ready in this application lifetime. */
export class HostRecovery {
  private host: string | null = null;
  private attempts = 0;
  private due: number | null = null;
  private readySince: number | null = null;
  private dispatchFailed = false;
  acceptedAttempt() {
    this.dispatchFailed = false;
  }
  failedAttempt() {
    this.dispatchFailed = true;
  }
  reset() {
    this.host = null;
    this.attempts = 0;
    this.due = null;
    this.readySince = null;
    this.dispatchFailed = false;
  }
  observe(
    value: HostInventory,
    now: number,
    visible = true,
  ): {
    connect: string | null;
    message: string | null;
  } {
    const connection = value.connection;
    const selected = value.saved.preferences.selectedHostId;
    if (
      value.mode !== "live" ||
      !connection?.hostId ||
      connection.hostId !== selected ||
      (connection.state === "disconnected" && !this.dispatchFailed)
    ) {
      this.reset();
      return { connect: null, message: null };
    }
    if (connection.state === "ready") {
      this.dispatchFailed = false;
      if (this.host !== selected) this.reset();
      this.host = selected;
      this.readySince ??= now;
      this.due = null;
      if (now - this.readySince >= 60000) this.attempts = 0;
      return {
        connect: null,
        message: this.attempts
          ? "Connection restored read-only. Logs have a gap; start them explicitly. Terminals remain closed."
          : null,
      };
    }
    this.readySince = null;
    if (this.host !== selected) {
      this.reset();
      return { connect: null, message: null };
    }
    if (["resolving", "connecting", "probing"].includes(connection.state))
      return {
        connect: null,
        message: `Reconnecting selected host (${this.attempts}/3)…`,
      };
    if (
      !this.dispatchFailed &&
      (!connection.diagnostic ||
        !["connection_lost", "connection_failed", "timed_out"].includes(
          connection.diagnostic.code,
        ))
    ) {
      this.reset();
      return {
        connect: null,
        message:
          "Automatic reconnect stopped. Review this host's connection diagnostics.",
      };
    }
    if (this.attempts >= 3)
      return {
        connect: null,
        message:
          "Connection lost. Three reconnect attempts finished; retry explicitly in Hosts.",
      };
    this.due ??= now + 1000 * 2 ** this.attempts;
    if (!visible || now < this.due)
      return {
        connect: null,
        message: `Connection lost · waiting to reconnect (${this.attempts + 1}/3). Terminals are closed.`,
      };
    this.attempts++;
    this.due = null;
    return {
      connect: this.host,
      message: `Reconnecting selected host (${this.attempts}/3)…`,
    };
  }
}
