---
title: "ADR 0001 — One native desktop application over OpenSSH"
section: "Build & design"
icon: "🛠️"
---

# 🛠️ ADR 0001 — One native desktop application over OpenSSH

Status: accepted implementation contract for prompt 001, 2026-09-28. This decision specifies the application to build; it does not claim that its runtime exists. Source contract: [original architecture](https://github.com/alekpopovic/container-desk/blob/main/codex/docs/ARCHITECTURE.md).

## Repository and scope

Retain the existing root repository, `AGENTS.md`, `CODEX_START.md`, all 60 immutable prompts, manifest, tracker and tests. The audit started from commit `154ab1027910e5ac092399c382b8d13a30423fee` with a clean working tree and no application sources or manifests. Existing user rules already include per-prompt commit/push and protection of unrelated work; no replacement or merge of `AGENTS.md` is needed. Do not rewrite original prompt hashes or the historical pack verification/checksum report.

Build one Tauri v2 desktop app with React, TypeScript, Vite and Tailwind, backed by Rust/Tokio. The local client runs on Linux x86_64 or macOS arm64/x86_64; remote Docker Engine runs on Linux. Native OpenSSH connects through the user's existing aliases, keys, agent and optional bastion. There is no web backend, remote custom agent or requirement to expose Docker TCP.

The selected prompt and architecture require version resolution in 001, while prompt 002 creates the runnable scaffold. To satisfy both, 001 adds exact dependency manifests, npm/Cargo lockfiles, toolchain pins and a documentation-only Rust library target needed by Cargo resolution. It does not add frontend source, a native entry point, Tauri configuration, build scripts, commands or UI. Prompt 002 extends this baseline in place and retains the locks. Terminal libraries and later tooling are selected when their implementation prompts are reached, rather than installing unused feature dependencies now.

## Boundaries

| Boundary | Owner and contract |
|---|---|
| UI and IPC | React renders untrusted text; narrow typed Tauri commands expose domain operations. No unrestricted shell plugin or renderer-supplied shell commands. |
| Authorization | Rust enforces per-host modes for every operation; frontend visibility is only a convenience. |
| SSH discovery | Bounded parser discovers concrete aliases and Includes without executing config. Wildcards/negations/dynamic Match are not expanded into guessed hosts; manual alias entry remains available. |
| Selected alias resolution | Native `ssh -G` resolves only a user-selected alias. It may evaluate `Match exec`, so SSH config is trusted executable configuration, not inert data. |
| Command construction | Validate the SSH executable, aliases, IDs, enums, integers and paths. Spawn local SSH with argument arrays and no shell; separately encode every remote argument with centralized POSIX single-quote escaping. Preserve Docker templates literally and test hostile input. |
| SSH trust | Strict host verification, existing keys/agent, no key copying or passphrase collection, no agent forwarding and no automatic edits to SSH config/known_hosts. User establishes host trust independently. Destination and jump-host authentication/trust must each be verified. |
| Processes | Rust owns children, cancellation, bounded queues and app-created control sockets. Use paths within both OS socket limits; reap owned children and leave other tools' masters untouched. Structured operations have no PTY. |
| Identity | Host, actual daemon endpoint/context, container ID and session generation scope requests, rows, caches and streams. Explicit Docker contexts apply to every command. Identity changes invalidate caches and action intents. |
| Data | Parse bounded JSON/JSON-lines and tolerate optional/unknown fields. Distinguish successful empty snapshots, errors and stale data. |
| Persistence | Versioned local JSON stores non-secret host references and preferences. Raw logs, terminal transcripts, keys and inspect environment values are excluded from persistence/telemetry/default diagnostics. Mask sensitive metadata before default IPC/export; reveal only by explicit current-session action. |

## Permissions and supported operations

Every host starts read-only. **Management and terminal access are opt-in per host.** Enabling management does not silently enable terminal access. A terminal is write-capable and requires explicit backend authorization for its current host/session. Returning to read-only revokes write/terminal authorization and closes owned terminal sessions. Stored host metadata must never silently grant a new session write access.

Mutations consume short-lived confirmation intents bound to the exact host, daemon, session, action and selected targets. Only explicitly selected stopped containers may be removed; never force removal or remove volumes. Images, volumes and networks remain read-only. Compose grouping can work without the Compose CLI; start/stop/restart requires installed Compose and verified existing remote project paths, config, project name and services. No compose up/down, deployment/build, prune, registry credentials, arbitrary script runner or Kubernetes.

An optional fixed `sudo -n` prefix can use an existing server policy; the application neither installs sudo rules nor weakens permissions. App read-only mode does not reduce the remote account's underlying Docker privileges. Local activity records are not a tamper-proof audit system.

## Resource and recovery contract

| Resource | Initial bound |
|---|---|
| SSH connect | 10 seconds for SSH connection establishment, plus a bounded overall attempt |
| Standard snapshot | 30-second deadline; at most 16 MiB stdout, explicit oversize error |
| Diagnostic stderr | 256 KiB ring, sanitized before any persistence |
| Logs | 20,000 lines or 8 MiB, whichever is reached first |
| Individual log record | 256 KiB; visible truncation marker |
| Stats | Every 5 seconds; one in-flight sample per host; 360 samples per displayed container |
| Read jobs | 4 per active host; separate bounded reserved capacity for user operations |
| Active hosts | Initially 3; saved inventory has separate limits |

Long-lived streams need connection/idle handling and cancellation rather than the finite snapshot deadline. Full queues and truncated output are visible to the user. Stderr alone cannot reliably distinguish application log text from SSH/Docker diagnostics. Stats preserve Docker CLI semantics, including CPU above 100%; histories are in memory unless explicitly exported. Events trigger reconciliation rather than serving as durable history.

Read sessions can reconnect with bounded backoff, gap indicators and refreshed snapshots. **Never replay a dispatched mutation or terminal input.** Losing a response may mean the remote operation completed: report unknown outcome and reconcile by reading current state. Session generations reject late output from a previous connection.

## Platforms and release

Target Ubuntu 24.04 LTS x86_64 as the Linux package build/runtime baseline and macOS 15 or later on Apple Silicon and Intel as the initial Mac baseline. These are project targets pending native acceptance, not verified compatibility claims. Tailwind 4 requires a modern WebKit (Safari 16.4-equivalent or newer); verify actual Linux WebKitGTK and Mac WKWebView behavior at native gates. Build Linux release packages on the oldest supported baseline; this development host's Ubuntu 26.04 builds cannot prove Ubuntu 24.04 compatibility.

Produce deb/AppImage and app/DMG with native platform runners. Local unsigned packaging and public macOS signing/notarization are separate evidence fields. Do not invent credentials, publish artifacts or buy services. Sandboxed stores and automatic updates are deferred. See [toolchains and sources](../toolchains.md), [checkpoint procedures](../checkpoints.md) and [current status](../project-status.md).

## Verification consequences

Test parsing/quoting, backend authorization, child ownership/cancellation and stale-state/unknown-outcome recovery at their implementation steps. Actual native windows and disposable direct/ProxyJump SSH/Docker lab runs are mandatory at the defined gates. Neither fixtures, browser previews, lockfile resolution, cross-compilation nor unexecuted CI definitions prove native operation. All changes to server state during development are confined to explicitly disposable resources; never guess a production SSH alias.
