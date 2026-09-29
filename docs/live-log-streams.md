---
title: "Owned live log subscriptions"
section: "Containers & resources"
icon: "▦"
---

# ▦ Owned live log subscriptions

`follow_container_logs` authorizes an ordinary read for the full host/daemon/selection/session/container scope. It reserves one of four read slots and one of two log-stream slots without queuing. Native Docker identity/binding is checked before starting the fixed `docker logs --follow --timestamps --tail N [--since S] -- ID` command. Tail and Unix timestamp validation reuse the snapshot registry. Startup is bounded to 30 seconds. The command uses the pinned Docker endpoint, native SSH arguments, central POSIX quoting, separate stdout/stderr pipes and no PTY.

Streams use an owned random subscription ID. The renderer receives typed [Tauri channel](https://v2.tauri.app/develop/calling-frontend/#channels) batches; application-level acknowledgments bound pending channel work. Tauri channels alone are not treated as backpressure. The backend sends nothing before sequence-0 acknowledgment of the returned ID, then permits only one unacknowledged batch. Each batch holds at most 128 records or 256 KiB + 30 bytes of text/timestamps. JSON encoding and record metadata add bounded overhead. A sequence can be acknowledged once; foreign scopes/IDs and out-of-order/duplicate acknowledgments fail.

## Buffer and process ownership

Each stream retains at most 20,000 records and 8 MiB of text/timestamps. Oldest records are dropped with a saturating count and explicit gap flag in the next batch. Stderr records remain ambiguous application/diagnostic text. There is no ordering claim across the two pipes. Each reader retains at most 256 KiB + 31 bytes of a partial physical line and discards the remainder until newline/EOF, marking truncation. UTF-8 normalization reuses the snapshot decoder. The reader yields between bounded chunks so cancellation is not starved by continuous output.

Batches run at most every 100 ms; a quiet stream sends at most one heartbeat per second after acknowledgment. No acknowledgment for 30 seconds cancels the subscription, including a renderer that disappears during startup. Quiet remote applications otherwise have no arbitrary output-idle timeout; SSH keepalive and the owned session monitor detect connection loss.

Stop is an asynchronous scoped operation that waits for owned process-group termination, direct-child reaping and read/stream permit release before returning. Cleanup accepts the exact original owner scope even after session invalidation. A foreign session cannot cancel another subscription. Unmount/route/container changes send cancellation, and periodic full-scope checks fence stale delivery. Session disconnect/shutdown also cancels the process independently of the renderer. No detached subscriber can keep sending unbounded IPC messages.

The initial view shows at most 100 records with at most 4,000 UTF-16 code units retained per record, visibly marking clipped lines. This is below the backend byte/line ceiling. It renders plain React text and exposes Start/Stop. Full viewer controls, rendering pause, filtering, export and control-sequence normalization belong to 025.

## Recovery semantics

No automatic replay/reconnect is implemented here. Explicit restart within the same selection uses the last received valid timestamp when available and marks a possible gap. After a host/session change the view is cleared and a new subscription has a new scope/ID. Timestamps are a best-effort resume hint, not a durable cursor: rotation, equal timestamps, cross-channel buffering, concurrent output and truncated tails can create gaps or duplicates. Nothing promises perfect deduplication, lossless ordering or durable history. Terminal input and mutations are not involved.

Raw records are excluded from Debug output, settings and diagnostics. Native channel errors produce static application messages. The retention buffer, IPC payload and view are transient; no export path is added in this prompt.

## Actual verification

The existing 023 laboratory gained an optional running workload with bounded json-file disk rotation. Its fixture-only SSH command gate allows logs/inspect for exact owned IDs and filters native inventory to those same IDs; it never returns unrelated host containers. The real release application is tested through external native WebDriver with PATH=/nonexistent. A deliberate 1.8-second webview event-loop stall withholds acknowledgments while real Docker/SSH output continues; the resulting gap/drop count is displayed and captured.

The separate compiled backend test withholds ACK for two seconds under native SSH output, observes no extra delivery, checks the next drop count, stops the child, then cuts only the owned SSH server/process tree during another stream. Unit tests additionally flood a real local child, inspect queue limits, verify the reaped PID, and wait through the real 30-second abandoned-renderer deadline. Browser fixtures prove plain hostile-text rendering and unmount cancellation but are not used as native evidence. See evidence 024 for commands and results.
