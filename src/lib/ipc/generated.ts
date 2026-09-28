// Generated from src-tauri/src/domain.rs with ts-rs. Do not edit.
export type HostId = string;
export type SessionId = string;
export type ContainerId = string;
export type SubscriptionId = string;
export type HostSelection = { hostId: HostId,
/**
 * Renderer selection epoch. Nonzero u32 is exact in JavaScript.
 */
selectionGeneration: number, };
export type SessionScope = { selection: HostSelection, sessionId: SessionId,
/**
 * Backend-issued epoch; changes after reconnect or daemon/context changes.
 */
sessionGeneration: number, daemonId: string, };
export type ErrorCode = "invalid_id" | "invalid_generation" | "host_not_found" | "session_not_found" | "stale_session" | "subscription_not_found" | "feature_unavailable" | "permission_denied" | "resource_limit" | "transport_unavailable" | "invalid_response" | "internal";
export type AppError = { code: ErrorCode, message: string, scope: SessionScope | null, };
export type ConnectionState = "disconnected" | "connecting" | "connected" | "reconnecting" | "failed";
export type HostCapabilities = { docker: boolean, compose: boolean, management: boolean, terminal: boolean, };
export type HostSummary = { id: HostId, alias: string, displayName: string, group: string, readOnly: boolean, connectionState: ConnectionState, };
export type ContainerSummary = { scope: SessionScope, id: ContainerId, name: string, image: string,
/**
 * Preserve unknown Docker states as untrusted text.
 */
state: string, status: string, health: string | null, };
export type ContainerDetail = { summary: ContainerSummary,
/**
 * No environment values or unrestricted inspect JSON cross default IPC.
 */
environmentNames: Array<string>, environmentValuesMasked: boolean, };
export type ListHostsResponse = { hosts: Array<HostSummary>, };
export type ConnectHostRequest = { selection: HostSelection, };
export type ConnectHostResponse = { scope: SessionScope, capabilities: HostCapabilities, };
export type ListContainersRequest = { scope: SessionScope, };
export type ListContainersResponse = { scope: SessionScope, containers: Array<ContainerSummary>, };
export type CancelSubscriptionRequest = { scope: SessionScope, subscriptionId: SubscriptionId, };
export type CancelSubscriptionResponse = { scope: SessionScope, subscriptionId: SubscriptionId, };
export type AppVersion = { version: string, };
