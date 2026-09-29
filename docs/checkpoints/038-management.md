---
title: "038 — Management workflow checkpoint"
section: "Reviews & evidence"
icon: "🧪"
historical: true
---

# 🧪 038 — Management workflow checkpoint

The management workflow was exercised in the actual Linux Tauri/WebKit application and through the same Rust backend using disposable SSH/Docker labs. This checkpoint fixes a stale-read race found during the integrated native journey; dependencies are unchanged. [Execution evidence](https://github.com/alekpopovic/container-desk/blob/main/codex/tracking/evidence/038.md) distinguishes native runs from browser fixtures.

## Permission and action boundaries

Every new resource session starts read-only. A saved preference does not grant management automatically. The selected, current host must be explicitly enabled; disabling management clears outstanding intents. Re-enabling does not restore them. The 038 native Rust checks call endpoints directly with read-only sessions, then with valid confirmations whose grants were revoked. Denials occur before any Docker mutation, independent of disabled frontend buttons.

| Endpoint / operation | Authoritative Rust gate | Supported boundary |
|---|---|---|
| `mutate_container`: start, stop, restart, remove | `PolicyEngine::consume` inside activity admission, plus current permission/session checks before each dispatch | Exact full IDs, one-use intent, at most 20 targets, one active batch per host. Removal only for selected stopped containers; no force or volume removal. |
| `mutate_compose_project`: start, stop, restart | Exact cached verification and scope; `PolicyEngine::consume` inside Compose activity admission; fresh configuration, identities and grant rechecked before dispatch | Explicit trusted remote directory, ordered files and project name; existing verified services only. No up/down/build/pull/deployment. |
| `open_container_terminal` | Terminal-specific policy requires `ManageAndTerminal` | Unavailable at 038 and absent from main-window capability. Ordinary management cannot grant terminal access. Transport follows in 039. |
| Images, volumes, networks | Read registry and scoped read admission | No mutation endpoints. Volume mountpoints are display metadata, never local or remote file-read targets. |

`prepare_confirmation` also checks permission before its native preflight. `set_management` requires the selected current session. `cancel_mutation` can cancel pending work for its exact operation scope; it cannot issue a Docker mutation or reverse an already dispatched command. Shared dispatch ownership keeps locks until the SSH child has been reaped. Confirmation expires after 30 seconds; Compose verification after five minutes.

## Integrated observations

- Native lifecycle journey: explicit enablement, host/daemon/full-ID confirmation, cancellation without dispatch, stop/start/restart, real state and delayed health observation, explicit return to read-only. The Rust journey additionally exercises all four read-only container operations and terminal denial, revocation of a valid stop intent, and rejection of that old intent after re-enabling.
- Native Compose journey: explicit paths including spaces/apostrophes, ordered overrides, verification, enablement, exact target confirmation, cancelled confirmation, one restart, both services observed running and explicit read-only return. Rust directly rejects all three Compose actions in read-only mode and a revoked valid intent; a second project with identical service names remains unchanged.
- Native batch journey: two selected full IDs and separate results; running removal unavailable; stopped removal requires a separate acknowledgement. Backend tests combine actual success and actual target disappearance with a deliberately injected permission refusal, preserve partial results, cancel pending targets, and reject a target started after removal confirmation.
- Images, Volumes and Compose grouping are traversed in one application journey against a private Engine through direct SSH and ProxyJump. Shared-tag/dangling images, named/anonymous/unused volumes, actual RO/RW references and separate same-service Compose projects link to the current full-ID container detail. Metadata values remain masked; unverified label paths remain descriptive.
- Networks use a separate explicitly owned internal dual-stack bridge and running attachment. Native IPAM and endpoint addresses are compared with independent Docker inspect; the link waits for a current container snapshot before becoming available.
- Browser fixtures independently cover selection clearing on host change, expiry, unknown outcomes and six width/theme combinations. Native backend scopes reject old/disconnected sessions and controlled daemon-ID drift before dispatch. The identity-drift test changes the probe projection at the owned SSH gate; it does not replace a real Engine.

## Failure semantics and limits

A successful CLI exit is followed by a fresh observation, not a readiness promise. Health may still be starting, and convergence may remain pending. A lost response after dispatch is recorded as unknown; reconnect does not replay the command or terminal input. Reads can reconcile current state without proving exactly what happened. Batch results remain per target, and nonzero/interrupted Compose commands cannot reliably attribute individual service outcomes.

The 038 native journey reproduced a stale post-action observation: the UI showed exited after start while independent Docker inspect showed running, no OOM, exit code zero and healthy. A read started before mutation completion could still be joined by the result view. Both container and Compose mutation adapters now invalidate pending read consumers for that host/daemon in a `finally` block, including lost-response cases. Existing native work retains its scheduler slots until completion; fresh reads have a separate epoch and cannot join the old request. Discarded reads do not retry. Deterministic tests hold the old read open across successful and failed mutation responses and prove fresh observation without mutation replay. Native WebKit also omitted permission-toggle clicks and exposed an asynchronous network-link assertion: the harness now checks trusted delivery to the intended element and waits for current read state. Lifecycle observation waits up to 50 seconds, covering the actual 30-second preflight plus command deadline; an earlier 30-second assertion could expire while the action was still pending. No application mutation retry was introduced.

The actual client platform is Ubuntu 26.04.1 x86_64, X11/Xvfb/software rendering and WebKitGTK 2.52.6. Host Docker/Compose are 29.8.1/v5.5.1; the private Engine is 28.3.3 with its lab Compose plugin. This is not Ubuntu 24.04 baseline, Wayland, macOS, package-install, signing or notarization evidence. Existing Compose configuration/hash conservatism and the external file/service-change race remain documented in [Compose actions](../compose-actions.md). Local bounded activity history is editable and is not an audit log.
