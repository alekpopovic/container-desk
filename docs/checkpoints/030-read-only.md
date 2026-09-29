---
title: "Read-only MVP checkpoint"
section: "Reviews & evidence"
icon: "🧪"
historical: true
---

# 🧪 Read-only MVP checkpoint

The current native application connects a saved SSH alias to a remote Linux Docker daemon and provides container inventory, inspect, health, ports, finite/followed logs, selected-container statistics, event-driven refresh and Compose grouping. This checkpoint verifies the existing integrated application; it introduces no new remote write operation.

## Runtime and connection journey

The client needs a native OpenSSH executable, access to the user's SSH configuration/agent and the operating-system GTK/WebKit libraries on Linux (the system WebView on macOS). Local Docker, Docker Desktop, jq, Python, Node and Rust are not application runtime requirements. Build prerequisites are in [toolchains](../toolchains.md). The remote account needs a POSIX-compatible shell, Docker CLI and permission to its Linux Engine endpoint. Remote jq and a custom agent are unnecessary. Compose is optional for grouping.

1. In Hosts, save a display name, existing SSH config path and concrete alias. Discovery reads configuration without executing it; explicitly resolving an alias uses native `ssh -G`, which may evaluate trusted `Match exec`. Supported aliases use the validated ASCII subset documented by the host form.
2. For a direct connection, confirm that native SSH works with that same configuration/alias. For ProxyJump, configure the target's `ProxyJump` and the jump alias's own identity, trust and forwarding permissions. The application shows the resolved route; it does not generate SSH trust or copy private keys.
3. Establish trust outside the app using independently verified fingerprints. Unknown/changed host keys, unavailable credentials and noninteractive authentication failures show bounded diagnostics. Existing keys/agent are used without passphrase collection or agent forwarding. Fix the configuration in the user's tools, then explicitly reconnect.
4. Select the saved host and connect. Confirm the actual daemon identity, endpoint/context, version and read-only session shown in Hosts. A named context applies to every read. Optional noninteractive sudo requires an already configured policy; the app does not change it.
5. Open Containers, search/select a full-ID-scoped row, then inspect summary/health and tabs. Exposed ports are distinguished from active published bindings. Environment and sensitive label values are masked in Rust before IPC; explicit reveal belongs only to the current detail session.
6. Statistics poll only the selected container while permitted by visibility/pause state. Start logs explicitly, filter/search/select retained lines, stop or resume following, and use the native export preview/dialog if needed. Text is untrusted plain text; stream gaps and truncation are visible.
7. Open Compose to see plugin/label provenance and unverified reported paths, then select an available instance to return to its existing Containers detail/log view. Reported paths do not authorize execution.
8. Events trigger coalesced authoritative inventory reads. A deleted container disappears only after a successful current snapshot. Disconnect removes old rows/live views and revokes the session; reconnect requires explicit user action. Failed reads retain visibly stale data, distinct from a successful empty snapshot.

## Structured-operation audit at 030

| Operation | Remote behavior and boundary |
|---|---|
| Connection/capability/identity checks | Native SSH resolution/connection and fixed shell/Docker version, context and info reads; no installation or daemon configuration |
| Container inventory | Fixed `docker ps --all --no-trunc` JSON template |
| Inspect/health/ports/image entrypoint metadata | Fixed container/image inspect; typed IDs and bounded responses; masked projection before IPC |
| Log snapshot/follow | Fixed `docker logs` with bounded validated tail/time arguments, optional follow; no exec |
| Statistics | `docker stats --no-stream` and narrow state inspect |
| Events | `docker events --filter type=container` JSON stream; only invalidates snapshots |
| Compose discovery | `compose ls --all --format json`, filtered `ps` IDs and batched exact-label inspect projection; reported paths never read or executed |
| Local host/settings/log export | App-owned preferences or explicitly selected native output file; no remote state mutation |

The allowlisted read plans are in `src-tauri/src/policy/registry.rs`; validated remote argv is centrally POSIX-quoted and native SSH is spawned without a local shell. The main-window capability grants narrow read/session/settings/export commands. Mutation/terminal scaffolding has no granted command capability or working dispatcher at this checkpoint: backend methods consume authorization and return `feature_unavailable`. No startup, selection, refresh or retry dispatches a Docker mutation. Trusted user SSH configuration can itself execute programs; application read-only mode cannot reduce the remote account's OS permissions.

## Bounds and current limits

Native reads have four global permits, at most two finite reads and one statistics request per host. The renderer admits three host/daemon groups with bounded queues; only finite transient reads retry, at most twice. Snapshot deadlines, output limits, session/daemon identity fences and cancellation/reaping remain enforced. See [read scheduling](../read-scheduling.md).

Logs retain at most 20,000 lines/8 MiB, truncate individual records at 256 KiB and use ACK-limited delivery; virtual rendering is bounded. Logs written to stderr cannot reliably be distinguished from transport diagnostics by channel alone. Events are hints with bounded replay/deduplication, not durable history. Statistics use Docker CLI semantics, allow CPU above 100%, retain gaps and do not persist history. Compose metadata is bounded and configuration remains unverified; see [Compose discovery](../compose-discovery.md). Images, volumes and networks do not yet have complete resource screens at 030. Lifecycle management and terminals belong to subsequent prompts.

## Real native evidence and platform boundary

[030 evidence](https://github.com/alekpopovic/container-desk/blob/main/codex/tracking/evidence/030.md) records the actual checks and commands. A real release Tauri/WebKitGTK application on Ubuntu 26.04.1 x86_64, X11/Xvfb/software rendering traversed two owned OpenSSH daemons using ProxyJump. It listed and inspected real disposable Docker containers, observed health/exposure, followed/stopped/resumed logs, received statistics, navigated Compose groups and refreshed a deletion from events. Its PATH contained no Docker or jq. The external lab harness used local Docker to provision/reap only explicitly owned fixtures.

The running-workload lab's fixed remote command gate restricted all container operations to those owned IDs. It intentionally denied Compose plugin listing so native UI verified the documented exact-label fallback. Actual plugin listing and plugin-absent grouping against a private Docker Engine were independently verified in [029 evidence](https://github.com/alekpopovic/container-desk/blob/main/codex/tracking/evidence/029.md). A second private empty Engine verified direct and ProxyJump successful empty snapshots plus SSH trust/authentication-denied UX. Authentication-denied evidence is not a claim of native Docker socket-permission-denial coverage.

This proves Linux native execution on the stated host, not package installation, Ubuntu 24.04 compatibility, native Wayland, macOS Intel/Apple Silicon, signing or notarization. Those remain separate native platform gates. No production host or guessed SSH alias was used.
