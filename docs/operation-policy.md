---
title: "Backend operation policy"
section: "Management & terminal"
icon: "⚡"
---

# ⚡ Backend operation policy

Current through prompt 046. Rust owns the operation registry and per-session policy. New backend-owned sessions always register read-only, independently of saved preferences. Session registration binds host ID, selection generation, session ID/generation and daemon identity. Re-registering a host resets grants/intents. Removing a session validates the full scope so cleanup for an old generation cannot remove its replacement. The policy registry has an internal three-entry ceiling; the implemented workspace admits **one active host**, as documented in [resource limits](resource-limits.md).

| Typed command family | Rust requirement / dispatch boundary |
|---|---|
| Container, image, volume, network and Compose reads; logs, stats, events | Current live scope and allowlisted `ReadOperation`; validated arguments, bounded output and read admission |
| Container start/stop/restart | Explicit management grant; one-use intent for exact IDs/action/timeout; current policy and native session checked at dispatch |
| Stopped-container removal | Same mutation checks plus state revalidation; only selected stopped containers, without force or volume removal |
| Existing Compose start/stop/restart | Explicit management grant, verified directory/ordered files/project/services/container IDs, expiring verification and one-use intent; configuration/identity rechecked before dispatch |
| Container terminal | Management plus separate terminal grant, exact one-use intent, fixed `/bin/sh` or `/bin/bash`, explicit user/size; separate bounded PTY owner |
| Terminal input/resize/read/close | Exact scoped terminal owner, sequence and payload bounds; revocation/disconnect closes it, old owner cannot affect a new terminal |

Plans contain argv and validated structured data; IPC cannot submit arbitrary remote shell text. Local native SSH is spawned without a shell. [Central remote POSIX quoting](remote-commands.md) applies Docker binary/context and optional fixed `sudo -n`; terminal input is exclusively the explicitly authorized interactive path. Images, volumes and networks have no mutation variants. Registry operations exclude prune, compose up/down/build/pull, registry credentials and arbitrary scripts.

Read operations remain available in read-only mode. Mutations require `Manage` or `ManageAndTerminal`; terminals require `ManageAndTerminal`. `set_management` and `set_terminal_permission` validate the currently selected workspace scope in Rust. Saved `readOnly=false` never restores a runtime grant. Returning to read-only or replacing the session invalidates intents and terminates terminal access. See [management](container-management.md), [batch/removal](batch-actions.md), [Compose](compose-actions.md) and [terminal transport](terminal-transport.md).

Confirmation intents use 128 bits of OS randomness, a 30-second monotonic expiry and at most 32 pending intents per session. Each binds the complete scope, operation variant, ordered targets and parameters. Consumption removes the intent before returning authorization, including on mismatched parameters; expiry, replay and cross-host use fail. Intents are never persisted. The owned activity record consumes mutations, and the native dispatch callback rechecks scope/access after admission. A lost dispatched response is an unknown outcome, followed only by a read; no mutation or terminal input is replayed.

The local `main` Tauri capability now grants the narrow implemented handlers registered in `lib.rs` and `build.rs`, including mutation and terminal commands. It grants no generic shell/filesystem/process plugin API. IPC grants do not replace Rust authorization. Contract/policy tests check malformed enums, unknown fields, hostile IDs, limits, stale generations, revocation, expiry, wrong-host and repeated intent use. Real disposable native journeys exercise the outer capability gate; mocked Rust/browser tests alone do not prove it. See [046 checkpoint](checkpoints/046-feature-complete.md) for current native evidence and [007 evidence](https://github.com/alekpopovic/container-desk/blob/main/codex/tracking/evidence/007.md) for the original policy increment.

This protects application workflows; it does not reduce the underlying remote account's SSH/Docker privileges. Local activity history is bounded operational history, not a tamper-proof security audit.
