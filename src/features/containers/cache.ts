import type {
  ContainerSummary,
  ListContainersResponse,
  SessionScope,
} from "../../lib/ipc/generated.ts";
import { sameScope } from "../../lib/ipc/client.ts";
export function inventoryKey(scope: SessionScope): string {
  return JSON.stringify([scope.selection.hostId, scope.daemonId]);
}
export type InventoryView = {
  scope: SessionScope | null;
  rows: ContainerSummary[];
  selectedId: string | null;
  updatedAt: number | null;
  loading: boolean;
  stale: boolean;
  error: string | null;
};
type Entry = {
  rows: ContainerSummary[];
  updatedAt: number;
  selectedId: string | null;
};
export type Ticket = { serial: number; scope: SessionScope };
/** Three in-memory host/daemon snapshots; old-session rows remain explicitly stale. */
export class ContainerCache {
  private entries = new Map<string, Entry>();
  private serial = 0;
  private namespace: string | null = null;
  private listeners = new Set<() => void>();
  private view: InventoryView = {
    scope: null,
    rows: [],
    selectedId: null,
    updatedAt: null,
    loading: false,
    stale: false,
    error: null,
  };
  private now: () => number;
  constructor(now = () => Date.now()) {
    this.now = now;
  }
  getSnapshot = () => this.view;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  private publish(view: InventoryView) {
    this.view = view;
    for (const listener of this.listeners) listener();
  }
  setNamespace(namespace: string) {
    if (namespace !== this.namespace) {
      this.namespace = namespace;
      this.entries.clear();
      this.unavailable();
    }
  }
  unavailable(error: string | null = null) {
    this.serial += 1;
    this.publish({
      scope: null,
      rows: [],
      selectedId: null,
      updatedAt: null,
      loading: false,
      stale: false,
      error,
    });
  }
  activate(scope: SessionScope) {
    this.serial += 1;
    const key = inventoryKey(scope);
    const entry = this.entries.get(key);
    if (entry) {
      this.entries.delete(key);
      this.entries.set(key, entry);
    }
    this.publish({
      scope,
      rows: entry?.rows ?? [],
      selectedId: entry?.selectedId ?? null,
      updatedAt: entry?.updatedAt ?? null,
      loading: false,
      stale: !!entry,
      error: null,
    });
  }
  begin(): Ticket | null {
    if (!this.view.scope || this.view.loading) return null;
    const ticket = { serial: ++this.serial, scope: this.view.scope };
    this.publish({
      ...this.view,
      loading: true,
      stale: this.view.updatedAt !== null,
      error: null,
    });
    return ticket;
  }
  private current(ticket: Ticket) {
    return (
      ticket.serial === this.serial && sameScope(ticket.scope, this.view.scope)
    );
  }
  complete(ticket: Ticket, response: ListContainersResponse) {
    if (!this.current(ticket)) return;
    if (
      !sameScope(response.scope, ticket.scope) ||
      response.containers.some((row) => !sameScope(row.scope, ticket.scope))
    ) {
      this.fail(ticket, "The connection changed. Refresh the selected host.");
      return;
    }
    const selectedId = response.containers.some(
      (row) => row.id === this.view.selectedId,
    )
      ? this.view.selectedId
      : null;
    const entry = {
      rows: response.containers,
      updatedAt: this.now(),
      selectedId,
    };
    const key = inventoryKey(ticket.scope);
    this.entries.delete(key);
    this.entries.set(key, entry);
    while (this.entries.size > 3) {
      const oldest = this.entries.keys().next().value;
      if (oldest) this.entries.delete(oldest);
    }
    this.publish({
      scope: ticket.scope,
      ...entry,
      loading: false,
      stale: false,
      error: null,
    });
  }
  fail(ticket: Ticket, error: string) {
    if (this.current(ticket))
      this.publish({
        ...this.view,
        loading: false,
        stale: this.view.updatedAt !== null,
        error,
      });
  }
  select(id: string) {
    if (!this.view.scope || !this.view.rows.some((row) => row.id === id))
      return;
    const entry = this.entries.get(inventoryKey(this.view.scope));
    if (entry) entry.selectedId = id;
    this.publish({ ...this.view, selectedId: id });
  }
}
