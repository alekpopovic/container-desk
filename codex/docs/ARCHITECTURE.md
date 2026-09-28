# Architecture contract

This is the proposed implementation design for this package, not a claim that the application already exists. Official API references are listed in SOURCES.md. Verify exact dependency versions during scaffold work.

## Process topology

The React webview invokes typed Tauri Rust commands. Rust owns host metadata, operation policies and an asynchronous process manager. The process manager invokes native OpenSSH using a saved alias. OpenSSH applies the user's configuration, including jump hosts, and runs the remote Docker CLI. Rust parses bounded output and sends typed data or bounded stream batches to the webview.

There is one app repository and no web backend. The target daemon is Linux Docker Engine. The local client can run on Linux or macOS. A network route to a bastion is sufficient when that bastion can reach the private target SSH port and permits TCP forwarding.

## Proposed components

| Component | Choice | Responsibility |
|---|---|---|
| Desktop shell | Tauri v2 | Window, native IPC and packaging |
| UI | React + TypeScript + Vite + Tailwind | Host/resource screens and state |
| Async backend | Rust + Tokio + serde | Processes, typed models, bounded parsing |
| SSH | Installed native OpenSSH | Config semantics, identities, jumps, known_hosts |
| Terminal | Maintained cross-platform PTY crate + xterm.js | Interactive input, resize and lifecycle |
| Storage | Versioned local JSON | Non-secret host references and preferences |
| Docker transport | Remote Docker CLI via SSH | Structured snapshots and live streams |
| Development tracker | Python 3.10+ standard library | Prompt states and evidence |

Select exact supported package versions during prompt 001. Pin lockfiles and documented native development toolchains. Local Docker, jq and Python are not runtime app dependencies. Remote Compose is optional for grouping, required for Compose CLI actions. No remote jq is needed.

## Proposed repository layout

| Path | Purpose |
|---|---|
| `src/features/hosts/` | Discovery, metadata, connections |
| `src/features/containers/` | Inventory/details/lifecycle |
| `src/features/logs/`, `stats/`, `terminal/` | Streaming views |
| `src/lib/ipc/` | Typed backend contracts |
| `src-tauri/src/ssh/` | Resolver, runner, sessions, quoting |
| `src-tauri/src/docker/` | Fixed builders and parsers |
| `src-tauri/src/policy/` | Host modes and action intents |
| `src-tauri/src/storage/` | Preferences and migrations |
| `src-tauri/src/commands/` | Narrow IPC handlers |
| `docs/` | Implemented design and verification evidence |
| `codex/` | This prompt pack and execution state |

Adapt names to an existing repository; these are proposed boundaries, not mandatory scaffolding generators.

## SSH selection and trust

Alias discovery is a convenience parser. It lists concrete Host tokens and follows includes with recursion/size limits, without evaluating executable configuration. Wildcards, negation and dynamic Match conditions are not a list of concrete hosts. Manual alias selection remains available.

OpenSSH resolves the selected alias. Use the original alias for connection, with the same selected config/executable each time. `ssh -G` can evaluate `Match exec`; do not run it on every discovered entry automatically or describe it as pure parsing. Config paths are chosen by the local user, not supplied by a remote container.

Reject option-like aliases, whitespace and control characters. The MVP may restrict aliases to ASCII letters/digits followed by letters/digits/dot/underscore/dash, with a finite length. Document this subset and suggest a simple config alias for unsupported names.

Use strict host-key validation. Missing trust/key setup is completed in the user's terminal with independent fingerprint verification. Background operations must fail with a useful diagnosis instead of requesting passwords invisibly. Test the jump host independently because destination options do not necessarily configure the jump subprocess. No automatic `StrictHostKeyChecking=no`, `UserKnownHostsFile=/dev/null` or key forwarding.

For multiple operations, own the control master/socket explicitly. Paths must be short enough for Unix-domain sockets on both target client OSes. Never terminate a control master created by the user's other tools. Track children and handle stale sockets, app crashes and retry.

## Command construction

The backend accepts typed operations, never arbitrary renderer-provided shell text. Spawn local SSH directly with an argument array. Remote command execution uses a shell even when local spawning does not. Encode remote arguments with one central POSIX single-quote function and validate their meaning first. Validate IDs, operation enums, integers and optional remote paths. Preserve Docker Go templates as literal arguments.

Structured operations use no PTY. A separate permission-gated path manages PTY terminal sessions. The supported remote shell is POSIX-compatible; unsupported shells receive a compatibility error. Optional sudo mode uses a fixed `sudo -n` prefix and an already configured policy. Never install sudo rules or weaken permissions from the app.

## Daemon identity and context

By default use the remote user's configured Docker endpoint and show its identity. An explicit named remote context may be selected and must be applied to every operation. A rootless Unix socket and a Docker context that routes somewhere else are not interchangeable. Display SSH target and actual daemon endpoint so actions are understandable. If a context/daemon changes, invalidate confirmation intents and caches.

## Data and operations

Snapshots use JSON templates or native JSON output, not human-aligned tables. `docker ps` yields JSON lines; inspect yields arrays. Normalize optional fields and preserve unknown values safely. An empty successful result differs from an error or stale cache. Every request, stream and row is scoped to host, daemon identity, container ID and session generation.

Read-only operations include inventory, inspect, logs, stats, events and Compose grouping. Management supports lifecycle operations and carefully limited stopped-container deletion. Images, networks and volumes are read views in v1. Compose actions require explicit verified remote project paths/config/name and are limited to start/stop/restart of existing services.

## Streaming and recovery

Use bounded queues and retention buffers with visible truncation/drop markers. Log text is not HTML. A container may write valid log data to stderr, while SSH/Docker diagnostics also use stderr; don't claim reliable origin classification from that channel alone. Process exit status and structured operation context inform errors.

Polling stats starts with bounded non-streaming snapshots and one active request per host. CPU can exceed 100%; memory is reported with Docker CLI semantics. Historical samples remain local/in-memory unless explicitly exported. Events are hints to refresh snapshots, not durable change history.

Reconnect read sessions with bounded backoff. Show gaps for logs/events and refresh state. Never replay a mutation or terminal input after connection loss. Write operations can finish remotely before a local timeout; report unknown outcome and reconcile with a read.

## Concrete initial bounds (tunable after measurement)

| Resource | Initial target |
|---|---|
| Connect stage | 10-second SSH connection timeout, bounded overall attempt |
| Standard snapshot | 30-second operation deadline |
| Captured snapshot | 16 MiB total stdout with explicit oversized-response error |
| Diagnostic stderr | 256 KiB ring buffer, sanitized before persistence |
| Log retention | 20,000 lines and 8 MiB, whichever is reached first |
| Individual log record | 256 KiB, truncated with a visible marker |
| Stats interval | 5 seconds, one inflight sample per selected host |
| Stats history | 360 samples per displayed container |
| Active host jobs | 4 read jobs; reserve bounded slots for user operations |
| Initial active hosts | 3 connected hosts; saved host inventory has separate limits |

These are design defaults to implement and measure, not benchmarks already achieved. Long-lived streams have connect/idle handling and cancellation instead of the finite snapshot deadline. The final UI must explain exhausted limits instead of silently omitting results.

## Read-only and secrets

Rust enforces host modes. Write actions consume a short-lived confirmation intent bound to exact targets/session/action. Terminal access is write-capable. This protects app workflows but does not reduce permissions of the remote SSH/Docker account. Local activity history is not a tamper-proof audit trail.

Mask environment values and sensitive metadata before default IPC/export. Reveal only on explicit user action for the current session. Application logs and crash diagnostics exclude private keys, passwords, container environment values, raw logs and terminal transcripts. Custom exports are previewed and chosen by the user.

## Platform and release boundaries

Initial targets: Linux x86_64 on a documented Ubuntu baseline, macOS Apple Silicon and Intel on documented supported versions. Use native runners to package/test. Build output alone is not runtime proof. The first distribution formats are deb/AppImage and macOS app/DMG. Sandboxed stores require extra SSH file/agent access work and are not part of v1.

Package creation without signing credentials is useful local output. Public macOS signing/notarization is a separate evidence field. Missing hardware, credentials and runners must be reported, not bypassed. Tauri native test support depends on the chosen driver; use current official documentation and exclude embedded automation controls from production binaries.

## Deferred scope

Kubernetes, Swarm administration, registry credentials, compose deployment/build/down, volume deletion/prune, arbitrary script automation, SFTP file managers, cloud discovery, SaaS accounts, team sync and automatic updates are future work. Keep the first application focused on the agreed remote Docker workflow.
