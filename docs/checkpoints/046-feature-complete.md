---
title: "046 — Feature-complete desktop checkpoint"
section: "Reviews & evidence"
icon: "🧪"
historical: true
---

# 🧪 046 — Feature-complete desktop checkpoint

This checkpoint covers the implemented product on native Linux, before release engineering. It does not certify installers, an older Ubuntu baseline, macOS execution or public signing. The evidence below distinguishes the integrated 046 journey from earlier isolated feature checks.

| Feature | Implemented | Native Linux verification | Platform / release pending |
|---|---|---|---|
| Saved hosts, metadata-only SSH discovery, explicit OpenSSH resolution, strict direct/ProxyJump trust | Yes | 018, 042; 046 discovers three owned aliases and resolves/connects the selected jump route | macOS, packaged launch |
| Containers, inspect/masked environment, health, ports, mounts and labels | Yes | 020–022, 030; 046 live list/inspect of eight owned containers | macOS / native baseline |
| Log follow, bounded retention, pause/search/export, stats and events | Yes | 024–028, 030, 043–045; 046 follows real logs and verifies SSH stream reaping | macOS dialogs and performance |
| Explicit management, single and batch start/stop/restart | Yes | 032–033, 038; 046 cancelled confirmation then exactly one stop/start and observed states | macOS |
| Selected stopped-container removal, no force or volume removal | Yes | 033, 038 actual removal and state-race checks | macOS |
| Images, volumes and networks, read-only references/attachments | Yes | 034–036 and 038 native direct/ProxyJump; no claim that 046 reran all resource views | macOS |
| Compose grouping and verified existing-project start/stop/restart | Yes | 029, 037–038; 046 GUI restart plus Rust restart/stop/start and independent Docker state oracle | macOS |
| Explicit non-root container terminal; resize, Ctrl-C, multiline paste, revocation | Yes | 039–040; 046 actual typed shell output, cancellation, close under output and owned PTY/SSH cleanup | macOS PTY/UI and physical sleep/wake |
| Read reconnect, stale/gap display, no mutation/input replay, shutdown cleanup | Yes | 041 native controlled connection loss and app exit; 046 disconnect/reaping | Physical sleep/wake and macOS |
| Private settings, desktop/agent environment, support export/redaction | Yes | 005, 042–043 actual native GUI launch, encrypted key agent cases and Save dialog | Finder, macOS native dialogs |
| Keyboard, focus dialogs, 200% text, screen-reader labels | Yes | 044 actual keyboard/Orca-generated labels; audible speech quality not assessed | VoiceOver, other desktop assistive technology |
| Configurable bounded retention/jobs; large inventory/inspect load | Yes | 045 native measured pressure; synthetic scale sources explicitly identified | macOS/older baseline performance |
| Native packages and installation | Pending | Unbundled release binary only through 046 | 051–054 and 058 required |
| Public signing/notarization/release | Pending | No credentials or publication used | Separate 055/059/060 gates |

Evidence for historical rows is sealed under `codex/tracking/evidence/NNN.md`; native artifacts are linked there. The app admits one active host at a time; the original architecture's proposed three-host bound is not implemented concurrency. [Resource limits](../resource-limits.md) records the decision and actual measured bounds.

## Integrated journey

`python3 tests/lab/checkpoint.py --tools-dir /tmp/containerdesk-025-tools --artifacts docs/verification/046-native` launches the release binary in an owned Xvfb/private D-Bus session with no client Docker/Node/Python/Cargo on the app's PATH. It creates eight explicitly owned disposable containers, two loopback sshd instances, isolated keys/config/trust, and a forced-command allowlist. This is an actual native WebKitGTK → Tauri IPC → native OpenSSH → Docker journey; no browser IPC fixture or custom production agent is involved.

One app session browses the generated SSH config, selects/resolves `logs-owned` via `logs-jump`, saves that same host, connects, displays live inventory/inspect, follows/stops logs, explicitly enables management, cancels a stop confirmation, then stops/starts one exact container. It revokes management, verifies the existing Compose project's directory with spaces/apostrophes, ordered files, name, service names and full IDs, cancels a restart, then confirms exactly one restart and observes both services running. It returns to Containers, explicitly enables management and terminal, cancels an open, then starts one non-root shell, types benign input, and closes the terminal tab while output is flowing. Finally it disconnects the host.

The harness checks the same displayed host/alias and resolved daemon identity, full action IDs, zero dispatch from cancelled confirmations, exact mutation counts, no terminal replay, stream PID/start-time disappearance and zero owned SSH/zombie processes after disconnect. Ownership includes leased ControlPersist masters and their ProxyJump descendants. It scans isolated app storage and diagnostics for the synthetic terminal transcript marker. The backend terminal and Compose native checkpoints then run separately; an independent Docker oracle checks both unrelated project services retained their original start times, and both container primary processes survived terminal signals. Both SSH-hop known_hosts entries remain byte-identical. The lab removes only its own resources.

## Review and defects fixed

- The header always claimed “Management unavailable” and “Connected · read-only”, including an authorized management session. Removed the static availability label and made the transport label “Connected”; action panels display their actual backend permission state.
- Container and batch confirmations displayed only the user-editable host name, while Compose/terminal included the SSH alias. They now consistently include name plus alias; exact daemon and target IDs remain visible.
- Visual inspection of the native screenshot found dark, proportional terminal text on the dark terminal surface: the production CSP rejected xterm’s generated theme/dimension style elements. Added `style-src-elem` permission for local/inline styles required by the pinned DOM renderer; `script-src`, network origins and style attributes retain their restrictions. The final native journey checks computed terminal colors/font and rejection of an inline script. Browser development fixtures had not exposed this production-only defect.
- Current project/policy/checkpoint documentation incorrectly described the implementation at prompts 001/007/017. Updated those current guides; immutable prompts and historical evidence remain unchanged.
- The first integrated harness attempt incorrectly expected an endpoint URL in a confirmation's daemon-identity field. Corrected the assertion to compare the actual connected daemon ID, and reran the full journey. A later run completed the GUI journey, then exposed a fixed four-container assumption in the reused terminal backend test; it now compares the exact IDs with the owned manifest, supporting the integrated eight-container lab. These failed attempts are not counted as acceptance.

Policy inspection covered `policy/registry.rs`, `policy/mod.rs`, `backend.rs`, `backend/compose_actions.rs`, `backend/terminal.rs`, the main capability and command registration. Read operations are typed; writes consume scoped one-use intents and recheck current access/session after admission; Compose verifies configuration and existing identities; PTY input has a separate scoped owner. No generic shell/process plugin grant or image/volume/network mutation was introduced. The [current policy guide](../operation-policy.md) maps these boundaries.

Lifecycle inspection covered native session generation checks, log/event ACK bounds, terminal reservations/closure, owned SSH cancellation and the bounded shutdown path in `lib.rs`. UI resource views fence results by host/selection/session/daemon; tab changes retain the selected owner while unmounting active stream/terminal owners. Prior 041 recovery and 045 pressure evidence covers interruption and sustained bounds beyond this short integrated journey. This inspection is not the full 047 security review.

## Remaining release gates

047–050: security/regression/lab/integration review. 051–054: reproducible native build and package creation/installation on the documented Linux and both macOS architectures. 055: distinguish local unsigned package evidence from available signing/notarization credentials. 058: execute the full target-platform acceptance matrix, including physical sleep/wake, GUI launch/agent and native dialogs. 059–060: actual release candidate artifacts, checksums, clean source contents and final handover.

Missing macOS hardware/runners, baseline runtime or signing credentials cannot be replaced by cross-compilation, browser mocks, a CI YAML file or an unsigned Linux binary. No public artifacts have been published. Final observed command results and binary digest are in [046 evidence](https://github.com/alekpopovic/container-desk/blob/main/codex/tracking/evidence/046.md) and [native results](../verification/046-native/RESULTS.md).
