# Execution tracker

Updated: 2026-09-28T17:09:10+00:00

blocked: **0** | done: **1** | in_progress: **0** | pending: **59**

Generated from `state.json`. Edit status through `python3 codex/scripts/track.py`.

`done` records submitted evidence; it is not independent certification of the application.

| ID | Phase | Task | Reasoning | Status | Evidence |
|---|---|---|---|---|---|
| 001 | 01 Foundations | Repository audit and implementation contract | high | done | codex/tracking/evidence/001.md |
| 002 | 01 Foundations | Tauri React TypeScript scaffold | medium | pending | — |
| 003 | 01 Foundations | Application layout and design tokens | medium | pending | — |
| 004 | 01 Foundations | Typed domain models and IPC boundary | high | pending | — |
| 005 | 01 Foundations | Local settings and host metadata store | high | pending | — |
| 006 | 01 Foundations | Native dependency diagnostics | medium | pending | — |
| 007 | 01 Foundations | Backend operation policy and command registry | high | pending | — |
| 008 | 01 Foundations | Synthetic fixtures and offline development mode | medium | pending | — |
| 009 | 02 SSH transport | SSH config discovery and host candidates | high | pending | — |
| 010 | 02 SSH transport | Effective SSH configuration resolution | high | pending | — |
| 011 | 02 SSH transport | OpenSSH subprocess runner | high | pending | — |
| 012 | 02 SSH transport | Remote argument quoting and Docker command builders | high | pending | — |
| 013 | 02 SSH transport | SSH authentication and host trust flow | high | pending | — |
| 014 | 02 SSH transport | Host connection state machine | high | pending | — |
| 015 | 02 SSH transport | Connection reuse and child ownership | high | pending | — |
| 016 | 02 SSH transport | Remote Docker capability probe | high | pending | — |
| 017 | 02 SSH transport | Host inventory screen and groups | medium | pending | — |
| 018 | 02 SSH transport | SSH vertical slice checkpoint | high | pending | — |
| 019 | 03 Read-only MVP | Container listing adapter | high | pending | — |
| 020 | 03 Read-only MVP | Container table and host-scoped cache | medium | pending | — |
| 021 | 03 Read-only MVP | Container inspect adapter and details | high | pending | — |
| 022 | 03 Read-only MVP | Health, ports, mounts and environment panels | medium | pending | — |
| 023 | 03 Read-only MVP | Bounded log snapshot retrieval | high | pending | — |
| 024 | 03 Read-only MVP | Live log subscriptions and cancellation | high | pending | — |
| 025 | 03 Read-only MVP | Log viewer usability and export | medium | pending | — |
| 026 | 03 Read-only MVP | Container resource statistics | high | pending | — |
| 027 | 03 Read-only MVP | Docker event stream and inventory invalidation | high | pending | — |
| 028 | 03 Read-only MVP | Read refresh scheduling and stale data | high | pending | — |
| 029 | 03 Read-only MVP | Compose project discovery and read views | high | pending | — |
| 030 | 03 Read-only MVP | Read-only MVP checkpoint | high | pending | — |
| 031 | 04 Management | Mutation intents and local activity records | high | pending | — |
| 032 | 04 Management | Container start stop and restart | high | pending | — |
| 033 | 04 Management | Multi-container actions and stopped-container removal | high | pending | — |
| 034 | 04 Management | Image inventory and inspection | medium | pending | — |
| 035 | 04 Management | Volume inventory and mount relationships | medium | pending | — |
| 036 | 04 Management | Network inventory and container attachments | medium | pending | — |
| 037 | 04 Management | Verified remote Compose project actions | high | pending | — |
| 038 | 04 Management | Management MVP checkpoint | high | pending | — |
| 039 | 05 Terminal and resilience | PTY terminal transport | high | pending | — |
| 040 | 05 Terminal and resilience | Terminal UI with bounded lifecycle | high | pending | — |
| 041 | 05 Terminal and resilience | Sleep wake network loss and graceful shutdown | high | pending | — |
| 042 | 05 Terminal and resilience | Cross-platform GUI launch and SSH agent behavior | high | pending | — |
| 043 | 05 Terminal and resilience | Support diagnostics and redacted export | high | pending | — |
| 044 | 05 Terminal and resilience | Accessibility themes and keyboard workflow | medium | pending | — |
| 045 | 05 Terminal and resilience | Large inventories and stream pressure | high | pending | — |
| 046 | 05 Terminal and resilience | Feature-complete desktop checkpoint | high | pending | — |
| 047 | 06 Quality and delivery | Focused security review | high | pending | — |
| 048 | 06 Quality and delivery | Frontend and parser regression coverage | high | pending | — |
| 049 | 06 Quality and delivery | Disposable direct and bastion integration lab | high | pending | — |
| 050 | 06 Quality and delivery | Native desktop integration tests | high | pending | — |
| 051 | 06 Quality and delivery | Local verification command and clean builds | medium | pending | — |
| 052 | 06 Quality and delivery | Linux and macOS CI build matrix | high | pending | — |
| 053 | 06 Quality and delivery | Linux package builds | high | pending | — |
| 054 | 06 Quality and delivery | macOS application and DMG builds | high | pending | — |
| 055 | 06 Quality and delivery | Signing and notarization integration | high | pending | — |
| 056 | 06 Quality and delivery | Versioning updates and rollback guidance | medium | pending | — |
| 057 | 06 Quality and delivery | User and contributor documentation | medium | pending | — |
| 058 | 06 Quality and delivery | Native platform acceptance matrix | high | pending | — |
| 059 | 06 Quality and delivery | Release candidate review and defect closure | high | pending | — |
| 060 | 06 Quality and delivery | Final handover and release gate | high | pending | — |

## History

- 2026-09-28T16:55:10+00:00 — 001: start;
- 2026-09-28T17:09:10+00:00 — 001: done; npm ci --ignore-scripts and npm ls --depth=0: PASS with Node 24.21.0/npm 11.19.0; 11 exact direct pins; cargo generate-lockfile, locked offline metadata and fmt --check: PASS with Rust 1.98.1; no native compilation claimed; python3 -m unittest discover -s codex/tests -v: 14 tests passed after tracker renderer whitespace fix; Manifest/lock/toolchain consistency, architecture review, local Markdown links, tracker validate and git diff --check: PASS; native/platform limits recorded
