# ContainerDesk — all 60 implementation prompts

Version 1.0 • 2026-09-28. Execute one prompt at a time from the project root.

Read `AGENTS.md`, `CODEX_START.md` and `codex/docs/ARCHITECTURE.md` first.

# 001 — Repository audit and implementation contract

- Phase: 01 Foundations
- Recommended reasoning: **high**
- Depends on: **none**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 001`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Establish a reproducible starting point for ContainerDesk, a desktop Docker manager using native OpenSSH.

## Required implementation

1. Inspect the existing repository and uncommitted changes before creating anything; preserve user work and adapt this pack to the actual repository.
2. Write docs/project-status.md and docs/decisions/0001-architecture.md: one Tauri v2 app, React/TypeScript/Vite UI, Rust/Tokio backend, native OpenSSH, remote Linux Docker Engine, Linux/macOS clients.
3. Record installed Rust, Node, package manager and OS versions. Choose supported stable toolchains from official documentation and pin the selected versions; do not silently select prereleases.
4. Define executable MVP checkpoints 018, 030, 038, 046 and release candidate 060. Declare terminal and management access to be opt-in per host.

## Acceptance and verification

- Confirm the repository can retain the pack without overwriting an existing AGENTS.md; document any merged rules.
- Verify every implementation decision agrees with codex/docs/ARCHITECTURE.md; record unavailable build environments honestly.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/001.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 001 --evidence codex/tracking/evidence/001.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 001 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 002 — Tauri React TypeScript scaffold

- Phase: 01 Foundations
- Recommended reasoning: **medium**
- Depends on: **001**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 002`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Create the working desktop shell in the repository root.

## Required implementation

1. Initialize Tauri v2 with React, TypeScript and Vite in the existing root without nesting another app or deleting codex/.
2. Create src-tauri and frontend source folders; add a minimal Rust command returning application version and call it from the UI.
3. Use one JavaScript package manager and commit its lockfile plus Cargo.lock. Record development and production build commands.
4. Add formatting and lint scripts with minimal useful rules; preserve strict TypeScript and avoid an unrestricted shell plugin.

## Acceptance and verification

- Run frontend typecheck/build and cargo check where native libraries are available.
- Launch the native window when a display is available; otherwise record native launch as unverified and verify the IPC contract in a focused check.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/002.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 002 --evidence codex/tracking/evidence/002.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 002 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 003 — Application layout and design tokens

- Phase: 01 Foundations
- Recommended reasoning: **medium**
- Depends on: **002**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 003`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Build an original interface with the navigation density expected from a server management desktop app.

## Required implementation

1. Create a sidebar for host groups, a host header with connection state, a resource navigation area and a main table/detail split.
2. Define light/dark tokens, typography, spacing, focus indicators and status colors in Tailwind/CSS variables. Use a clearly distinct ContainerDesk identity.
3. Include Containers, Compose, Images, Volumes, Networks and Settings routes with honest empty states; unavailable features should explain their state.
4. Keep host identity visible in detail panes and action dialogs; make keyboard navigation work from the start.

## Acceptance and verification

- Inspect the interface at 1280x800 and a narrower desktop window; no clipped navigation or unreadable status labels.
- Check keyboard focus and theme contrast using representative empty, loading, offline and error states.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/003.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 003 --evidence codex/tracking/evidence/003.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 003 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 004 — Typed domain models and IPC boundary

- Phase: 01 Foundations
- Recommended reasoning: **high**
- Depends on: **003**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 004`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Define the shared data contract between the renderer and Rust.

## Required implementation

1. Model HostId, SessionId, ContainerId, ConnectionState, HostCapabilities, ContainerSummary, ContainerDetail and structured AppError codes.
2. Expose narrow commands such as list_hosts, connect_host, list_containers and cancel_subscription; keep an arbitrary executable/command-string API out of the renderer.
3. Choose explicit serialization names and a single maintainable method for keeping Rust and TypeScript DTOs aligned.
4. Include host/session generation identifiers in requests and responses so stale data cannot be applied to a newly selected host.

## Acceptance and verification

- Check serialization against a representative success/error fixture across the boundary.
- Verify invalid IDs and absent sessions return typed errors before any process is spawned.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/004.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 004 --evidence codex/tracking/evidence/004.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 004 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 005 — Local settings and host metadata store

- Phase: 01 Foundations
- Recommended reasoning: **high**
- Depends on: **004**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 005`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Persist application preferences without duplicating SSH secrets.

## Required implementation

1. Store schema-versioned JSON in the platform application data directory with atomic writes and owner-only permissions; retain a recoverable previous version.
2. Persist theme, selected host aliases, labels, host group, read-only mode, trusted config path and SSH executable override.
3. Store only identity references; do not copy private keys, passwords, raw inspect responses or terminal output into preferences.
4. Implement schema migration, corrupt-file recovery with visible feedback and a storage adapter injectable in tests.

## Acceptance and verification

- Exercise interrupted write/corrupt JSON and a schema migration without losing the original file.
- Check persisted settings for secret material and verify two hosts keep independent modes and display names.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/005.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 005 --evidence codex/tracking/evidence/005.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 005 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 006 — Native dependency diagnostics

- Phase: 01 Foundations
- Recommended reasoning: **medium**
- Depends on: **005**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 006`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Detect the tools the installed application actually needs.

## Required implementation

1. Resolve OpenSSH from a trusted absolute path, normally /usr/bin/ssh, with a user-selectable validated override; do not rely on an interactive shell PATH.
2. Inspect executable permissions and record ssh -V output, platform, architecture and app version in a diagnostics view.
3. Detect the presence/accessibility of SSH_AUTH_SOCK without reading keys. Keep frontend dependencies out of runtime requirements.
4. Show that local Docker, jq, Python, Rust and Node are not required by the built app; Python is used only by this development tracker.

## Acceptance and verification

- Launch diagnostics with a deliberately minimal PATH and confirm native SSH is found.
- Test missing executable, non-executable override and missing agent socket with useful error messages.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/006.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 006 --evidence codex/tracking/evidence/006.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 006 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 007 — Backend operation policy and command registry

- Phase: 01 Foundations
- Recommended reasoning: **high**
- Depends on: **006**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 007`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Make remote operations explicit and enforce host permissions in Rust.

## Required implementation

1. Create typed read, mutation and terminal operation categories with default per-host read-only policy.
2. Define a registry of allowed Docker command builders and validate all IPC arguments in Rust; UI disabled buttons alone must never enforce access.
3. Use fixed supported operation variants, numeric bounds for timeouts/tail counts, validated IDs, and explicit per-operation result types.
4. Prepare confirmation intents bound to host, session generation, operation and target IDs for later mutation prompts.

## Acceptance and verification

- Call a mutation handler directly while read-only mode is enabled and ensure rejection before transport.
- Check invalid operation names, negative limits and malformed IDs; legitimate read operations remain available.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/007.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 007 --evidence codex/tracking/evidence/007.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 007 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 008 — Synthetic fixtures and offline development mode

- Phase: 01 Foundations
- Recommended reasoning: **medium**
- Depends on: **007**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 008`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Enable useful development without connecting to a real server.

## Required implementation

1. Create a replaceable transport trait/interface and synthetic fixtures for healthy, exited, restarting and unhealthy containers, IPv6 ports and Compose labels.
2. Provide an explicit demo mode with a persistent DEMO label; never switch a failed live connection into demo data.
3. Include empty outputs, permission failures, invalid JSON, huge records, disconnects and command timeouts in deterministic fixtures.
4. Set up focused Rust parser/transport tests and frontend component tests for state handling, using current compatible packages.

## Acceptance and verification

- Verify demo mode never starts SSH and live mode never reads fixture data as a fallback.
- Run the initial tests and demonstrate the same DTO shapes in demo and native IPC.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/008.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 008 --evidence codex/tracking/evidence/008.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 008 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 009 — SSH config discovery and host candidates

- Phase: 02 SSH transport
- Recommended reasoning: **high**
- Depends on: **008**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 009`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Read host candidates from the user-selected OpenSSH configuration.

## Required implementation

1. Default to ~/.ssh/config and show its resolved local path. Parse literal Host tokens, quoted paths, comments and Include references with bounded recursion and cycle detection.
2. Keep discovery separate from effective configuration resolution. Do not pretend wildcard/negated patterns or conditional blocks enumerate concrete destinations.
3. Allow manually entered concrete aliases for wildcard or dynamic setups; display candidates for explicit user selection instead of scanning or connecting automatically.
4. Never rewrite SSH config. Treat it as trusted user configuration that may contain executable directives; do not execute Match exec while merely browsing candidates.

## Acceptance and verification

- Test multiple aliases on one Host line, wildcard/negative tokens, missing includes, spaces in include paths and include cycles.
- Check that discovery itself spawns no subprocess and that unsupported discovery cases have a manual-alias path.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/009.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 009 --evidence codex/tracking/evidence/009.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 009 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 010 — Effective SSH configuration resolution

- Phase: 02 SSH transport
- Recommended reasoning: **high**
- Depends on: **009**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 010`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Delegate connection semantics to OpenSSH rather than reimplementing its precedence rules.

## Required implementation

1. After explicit host selection, resolve bounded diagnostic output with the chosen ssh executable and ssh -G alias, using the selected -F config path when applicable.
2. Reject aliases that begin with a dash or contain whitespace/control characters; define and document a conservative supported alias grammar.
3. Parse effective host, user, port and jump information for display while retaining the original alias for actual connections.
4. Treat Match exec and ProxyCommand as trusted local configuration code; explain this once when choosing a custom config. Do not expose identity contents or claim ssh -G is side-effect free.

## Acceptance and verification

- Compare effective values with native ssh -G on fixture configs, including overlapping Host entries and Include.
- Verify the alias reaches OpenSSH as an argument, not through a local shell; malformed aliases fail before execution.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/010.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 010 --evidence codex/tracking/evidence/010.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 010 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 011 — OpenSSH subprocess runner

- Phase: 02 SSH transport
- Recommended reasoning: **high**
- Depends on: **010**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 011`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Create the cancellable transport foundation for remote commands.

## Required implementation

1. Use Tokio process APIs with a resolved SSH executable and argument arrays; build no local sh -c command.
2. Implement separate stdout/stderr readers, bounded capture, exit-code handling, command deadlines and cancellation with child cleanup/reaping.
3. Use noninteractive stdin and no PTY for structured operations; retain a separate execution path for the later interactive terminal.
4. Add connection timeout and keepalive options with documented defaults. Do not let a noisy stderr pipe deadlock a stdout reader.

## Acceptance and verification

- With a synthetic child program, test timeout, cancellation, large stderr, nonzero exit and partial output.
- Verify children are reaped and bounded-output failure is distinguishable from an empty successful response.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/011.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 011 --evidence codex/tracking/evidence/011.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 011 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 012 — Remote argument quoting and Docker command builders

- Phase: 02 SSH transport
- Recommended reasoning: **high**
- Depends on: **011**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 012`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Prevent remote shell interpretation from changing structured operations.

## Required implementation

1. Document that OpenSSH remote commands still pass through the remote login shell; local argument arrays alone do not prevent remote command injection.
2. Target a POSIX-compatible remote shell and build one remote command from fixed tokens plus a thoroughly tested POSIX single-quote encoder.
3. Constrain container/image IDs, options, numeric limits and paths before quoting; support a validated absolute Docker binary path without evaluating shell startup files.
4. Keep template syntax such as {{json .}} intact. Do not concatenate labels, host display names, pasted scripts or arbitrary renderer text into commands.

## Acceptance and verification

- Test apostrophes, spaces, semicolons, newlines, dollar substitution, backticks and leading dashes with an inert local POSIX-shell harness.
- Verify generated commands preserve arguments exactly and a malicious label cannot spawn a second command.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/012.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 012 --evidence codex/tracking/evidence/012.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 012 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 013 — SSH authentication and host trust flow

- Phase: 02 SSH transport
- Recommended reasoning: **high**
- Depends on: **012**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 013`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Make authentication failures actionable without collecting credentials in the app.

## Required implementation

1. Use the configured IdentityFile/SSH agent and existing known_hosts with strict host-key verification. Disable interactive password fallback for structured operations.
2. For unknown or changed keys, show the SSH error and a documented terminal setup route; the user verifies fingerprints independently and retries.
3. Account for ProxyJump child processes: destination options are not assumed to configure jump-host authentication. Test unattended behavior on both hops and make missing agent keys fail promptly.
4. Do not enable agent forwarding, disable host checks, store passphrases or automatically edit known_hosts. Explain encrypted-key loading through the normal OS agent.

## Acceptance and verification

- Test known/unknown/changed keys and absent/encrypted keys on a disposable SSH lab.
- Confirm a missing jump-host key cannot leave a hidden password prompt hanging indefinitely.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/013.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 013 --evidence codex/tracking/evidence/013.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 013 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 014 — Host connection state machine

- Phase: 02 SSH transport
- Recommended reasoning: **high**
- Depends on: **013**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 014`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Present connection progress and cancellation reliably.

## Required implementation

1. Implement disconnected, resolving, connecting, probing, ready, degraded and error states with an incrementing session generation.
2. Ensure connect cancellation, repeated clicks and switching hosts cannot attach an old completion to a new session.
3. Add structured stage durations and sanitized diagnostics so jump failure, authentication failure and remote-command failure are understandable.
4. Keep persisted host metadata separate from transient connection state; avoid automatically connecting every saved host at launch.

## Acceptance and verification

- Simulate slow resolution, successful connect followed by disconnect, and switching hosts during a probe.
- Verify the UI and Rust state agree and stale callbacks cannot change the selected host state.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/014.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 014 --evidence codex/tracking/evidence/014.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 014 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 015 — Connection reuse and child ownership

- Phase: 02 SSH transport
- Recommended reasoning: **high**
- Depends on: **014**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 015`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Reduce SSH overhead while preserving correct process ownership.

## Required implementation

1. Implement an app-owned OpenSSH multiplex master per selected alias/config identity with a private short control-socket path and bounded persistence.
2. Use explicit ControlPath values for app sessions and never issue an exit to a user-owned control master. Keep app instance ownership distinguishable.
3. Ensure log/terminal/command children share a controlled session and can still be cancelled independently; implement a non-multiplex fallback with a visible diagnostic.
4. Apply permissions and path-length limits appropriate to Linux and macOS Unix sockets; clean up only resources created by this app.

## Acceptance and verification

- Confirm repeated read commands reuse the app connection and app exit leaves an unrelated user master running.
- Test a stale socket, long home path, master death and fallback without unbounded reconnect loops.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/015.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 015 --evidence codex/tracking/evidence/015.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 015 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 016 — Remote Docker capability probe

- Phase: 02 SSH transport
- Recommended reasoning: **high**
- Depends on: **015**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 016`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Identify the Docker daemon and operations available to the SSH user.

## Required implementation

1. Probe remote Docker client/server versions, daemon identity, OS, access permission and Compose plugin availability with bounded structured output.
2. Support the remote user default Docker context or an explicitly selected named remote context; pass it consistently and show the actual endpoint/daemon identity.
3. Detect local Unix/rootless socket versus another remote context endpoint, and make endpoint identity visible. Restrict the supported MVP target to Linux Docker Engine.
4. Allow an explicit fixed sudo -n Docker execution mode for existing administrator configuration; never request/store sudo passwords or change server permissions automatically.

## Acceptance and verification

- Test missing docker, stopped daemon, permission denied, rootless context, Compose absent and sudo requiring a password.
- Verify every later builder uses the selected context and does not silently switch daemon or privilege mode.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/016.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 016 --evidence codex/tracking/evidence/016.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 016 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 017 — Host inventory screen and groups

- Phase: 02 SSH transport
- Recommended reasoning: **medium**
- Depends on: **016**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 017`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Connect discovery, metadata and connection state into the desktop UI.

## Required implementation

1. Allow selecting config aliases, assigning labels, grouping prod/staging/dev hosts, favoriting hosts and removing app metadata.
2. Display effective destination, jump route summary, Docker endpoint, connection health and per-host read-only mode.
3. Add explicit Connect/Disconnect/Retry actions; connection errors retain the selected host and offer relevant diagnostics.
4. Keep deleting a host from the app separate from editing SSH config or deleting remote resources.

## Acceptance and verification

- Verify duplicate display names still map to distinct stable HostIds and session state cannot leak across hosts.
- Exercise add/connect/disconnect/remove with both direct and ProxyJump aliases in demo and live adapters.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/017.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 017 --evidence codex/tracking/evidence/017.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 017 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 018 — SSH vertical slice checkpoint

- Phase: 02 SSH transport
- Recommended reasoning: **high**
- Depends on: **017**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 018`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Verify the entire SSH foundation before building resource screens.

## Required implementation

1. Prepare a documented disposable lab with a local client, bastion and private Linux target, using temporary keys and a dedicated known_hosts file.
2. Demonstrate a bounded remote Docker version/list probe through ProxyJump and through a direct alias without requiring Docker or jq on the client.
3. Review command quoting, strict host checks, session ownership, cancellation and credential diagnostics together.
4. Write docs/checkpoints/018-ssh.md with OS, tool versions, commands, observed results and remaining platform checks; keep real hostnames and secrets out of committed fixtures.

## Acceptance and verification

- Run the vertical slice on the available native OS plus deterministic transport tests.
- If no live disposable SSH target exists, mark this checkpoint blocked rather than reporting mock output as SSH evidence.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/018.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 018 --evidence codex/tracking/evidence/018.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 018 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 019 — Container listing adapter

- Phase: 03 Read-only MVP
- Recommended reasoning: **high**
- Depends on: **018**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 019`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Turn Docker JSON-line output into stable container summaries.

## Required implementation

1. Build docker ps -a --no-trunc --format {{json .}} through the registry and parse one JSON object per line with serde_json.
2. Normalize full IDs, names, image, state, display status, ports and labels while tolerating unknown fields and missing optional values.
3. Treat no lines with exit code zero as an empty result; detect malformed/banners output explicitly rather than silently dropping arbitrary lines.
4. Keep Docker CLI display fields distinct from precise inspect-derived fields and bound response bytes and record counts.

## Acceptance and verification

- Test running/exited/unhealthy containers, Unicode names, empty results, malformed lines and large valid listings.
- Compare normalized IDs and counts with native Docker CLI output against the disposable target.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/019.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 019 --evidence codex/tracking/evidence/019.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 019 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 020 — Container table and host-scoped cache

- Phase: 03 Read-only MVP
- Recommended reasoning: **medium**
- Depends on: **019**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 020`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Show an efficient, usable container inventory.

## Required implementation

1. Build sortable columns for name, state, health when available, image, ports and age; add search and state filters.
2. Key queries and row selection by HostId plus daemon identity and full ContainerId; cache timestamps with every successful result.
3. Virtualize large lists if measurement requires it; keep selection stable across refreshes and clear it when a resource disappears.
4. Render loading, empty, error and stale-data states visibly; avoid showing cached data as a live successful refresh.

## Acceptance and verification

- Test switching hosts with identical container names and out-of-order responses.
- Inspect a synthetic 1000-container table for responsive filtering and correct selection.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/020.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 020 --evidence codex/tracking/evidence/020.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 020 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 021 — Container inspect adapter and details

- Phase: 03 Read-only MVP
- Recommended reasoning: **high**
- Depends on: **020**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 021`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Provide accurate container details from structured inspect output.

## Required implementation

1. Fetch docker container inspect for validated IDs and parse its JSON array into a typed detail DTO.
2. Expose lifecycle timestamps, exit code, restart policy/count, image ID, mounts, labels, resource configuration and network addresses.
3. Mask environment values and sensitive labels by default in Rust before IPC; support explicit per-session reveal without persistence.
4. Handle missing/deleted containers and null/absent health or network data; bound large inspect payloads.

## Acceptance and verification

- Validate detailed fixtures for stopped, rootless, healthcheck-free and multi-network containers.
- Check default IPC responses, app logs and persisted files for synthetic secret leakage.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/021.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 021 --evidence codex/tracking/evidence/021.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 021 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 022 — Health, ports, mounts and environment panels

- Phase: 03 Read-only MVP
- Recommended reasoning: **medium**
- Depends on: **021**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 022`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Make inspect data easy to read and compare.

## Required implementation

1. Create detail tabs for Overview, Ports, Mounts, Networks, Labels and Environment with copy actions for selected safe fields.
2. Differentiate container port exposure from host bindings; handle IPv4, IPv6 and multiple host bindings.
3. Display healthcheck absence as not configured, and health state separately from running state. Mark exit/OOM details when provided.
4. Require an explicit reveal for environment values; keep content as plain text and clear revealed values when the session closes.

## Acceptance and verification

- Inspect layouts with long mount paths, empty labels, multiple bindings and a failed healthcheck.
- Verify no label/environment content is interpreted as HTML or linked automatically.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/022.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 022 --evidence codex/tracking/evidence/022.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 022 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 023 — Bounded log snapshot retrieval

- Phase: 03 Read-only MVP
- Recommended reasoning: **high**
- Depends on: **022**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 023`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Retrieve recent container logs without unbounded output.

## Required implementation

1. Implement docker logs with bounded positive tail, timestamps and an optional validated since/until range.
2. Treat log content as untrusted text. Capture Docker/SSH exit errors separately while acknowledging remote log stderr shares the SSH stderr channel.
3. Do not classify all stderr as failure: container application stderr is valid log data. Distinguish nonzero process exit and mark ambiguous diagnostic lines honestly.
4. Handle unsupported logging drivers, exited containers, invalid UTF-8 and oversized lines; keep data transient unless the user exports it.

## Acceptance and verification

- Test interleaved application stdout/stderr, daemon failure, unsupported log driver and a line larger than the limit.
- Compare a known fixture container log sequence with the native docker logs command.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/023.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 023 --evidence codex/tracking/evidence/023.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 023 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 024 — Live log subscriptions and cancellation

- Phase: 03 Read-only MVP
- Recommended reasoning: **high**
- Depends on: **023**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 024`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Stream logs efficiently through Rust to the frontend.

## Required implementation

1. Spawn docker logs --follow with an owned subscription ID scoped to a host/session/container and send bounded batches through Tauri channels.
2. Use bounded queues, byte/line retention limits and explicit dropped-data markers so a slow renderer cannot exhaust memory.
3. Implement stop/unmount/disconnect cleanup and child reaping; avoid replacing a log stream with a PTY that merges channels or injects terminal escapes.
4. Define reconnect behavior as best-effort: resume with timestamps where available, show gaps and do not promise lossless ordering or perfect deduplication.

## Acceptance and verification

- Test cancellation during heavy output and after network loss, confirming no orphan SSH children.
- Pause consumption under a large synthetic stream and verify bounded memory plus a visible drop marker.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/024.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 024 --evidence codex/tracking/evidence/024.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 024 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 025 — Log viewer usability and export

- Phase: 03 Read-only MVP
- Recommended reasoning: **medium**
- Depends on: **024**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 025`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Provide a practical desktop log viewer.

## Required implementation

1. Add follow/pause, clear-view, plain-text search, bounded filtering, timestamps and a capped virtualized log display.
2. Keep pause-rendering behavior distinct from stopping the remote stream; explain retained buffer limits in the UI.
3. Export only selected buffered lines through a native save dialog and warn that log contents may contain application secrets at the export action.
4. Neutralize terminal control sequences and clickable escape links; preserve readable Unicode and copy exact visible text.

## Acceptance and verification

- Test follow scrolling while selecting text, an oversized search term and clearing the view during streaming.
- Export a known buffer and verify ordering/content and that unrelated containers are absent.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/025.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 025 --evidence codex/tracking/evidence/025.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 025 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 026 — Container resource statistics

- Phase: 03 Read-only MVP
- Recommended reasoning: **high**
- Depends on: **025**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 026`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Show CPU, memory and I/O with explicit sampling semantics.

## Required implementation

1. Use bounded docker stats --no-stream --no-trunc --format JSON-template snapshots at a configurable modest interval; allow only one stats request per host at a time.
2. Parse units and missing values carefully, retaining raw strings for diagnostics and normalized values for charts.
3. Label CPU as Docker-reported percent, which may exceed 100 on multicore hosts, and memory as Docker CLI semantics; do not reinterpret it as raw API memory.
4. Use finite in-memory history keyed by host and full container ID; show stopped/unavailable samples as gaps rather than zeros.

## Acceptance and verification

- Check unit conversion, percentages above 100, unavailable values and container disappearance during sampling.
- Verify polling stops on disconnect and slows/pauses for inactive windows according to documented settings.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/026.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 026 --evidence codex/tracking/evidence/026.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 026 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 027 — Docker event stream and inventory invalidation

- Phase: 03 Read-only MVP
- Recommended reasoning: **high**
- Depends on: **026**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 027`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

React to container lifecycle changes without trusting events as a complete database.

## Required implementation

1. Subscribe to bounded Docker event JSON output for the selected daemon and parse events with timestamps and actor IDs.
2. Debounce inventory invalidation for start/stop/die/destroy events; let full snapshots remain authoritative.
3. On reconnect, request a current snapshot and use any event history only as best-effort recovery; daemon retention and connection gaps can lose events.
4. Bound event buffers and apply the same cancellation/ownership policy as logs; never start hidden streams for all saved hosts.

## Acceptance and verification

- Test burst events, duplicate events, a gap across reconnect and a container deleted before inspect.
- Confirm event storms result in a bounded number of refresh requests.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/027.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 027 --evidence codex/tracking/evidence/027.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 027 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 028 — Read refresh scheduling and stale data

- Phase: 03 Read-only MVP
- Recommended reasoning: **high**
- Depends on: **027**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 028`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Keep multiple views responsive during slow or unreliable SSH connections.

## Required implementation

1. Create a per-host read scheduler with concurrency limits, single-flight refreshes, cancellation and bounded backoff with jitter.
2. Prioritize user navigation over background stats and avoid one SSH process per table row for inspect.
3. Maintain last-success timestamps and stale indicators; distinguish disconnected from an empty daemon.
4. Retry only idempotent read requests with limits and refresh on wake/network recovery; mutations remain outside automatic retries.

## Acceptance and verification

- Simulate a slow daemon, repeated refresh clicks and a laptop sleep/wake cycle with fake clocks where appropriate.
- Measure maximum concurrent jobs and confirm late data cannot overwrite a newer session.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/028.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 028 --evidence codex/tracking/evidence/028.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 028 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 029 — Compose project discovery and read views

- Phase: 03 Read-only MVP
- Recommended reasoning: **high**
- Depends on: **028**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 029`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Group containers by Compose project without requiring local Compose files.

## Required implementation

1. When the remote Compose plugin exists, parse docker compose ls --all --format json and normalize its project metadata.
2. Supplement project/service grouping from com.docker.compose labels on inspected containers; expose label-based grouping even if the plugin is absent.
3. Do not assume a remotely reported ConfigFiles path is usable, trusted, readable or complete; label discovery and executable project configuration separately.
4. Display service instances and states and link to the same container detail/log views without duplicating transport sessions.

## Acceptance and verification

- Test two projects with identical service names, absent Compose plugin and stale/missing config files.
- Verify no local filesystem path is mistaken for a remote Compose path.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/029.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 029 --evidence codex/tracking/evidence/029.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 029 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 030 — Read-only MVP checkpoint

- Phase: 03 Read-only MVP
- Recommended reasoning: **high**
- Depends on: **029**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 030`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Deliver a coherent usable release for inspecting remote Docker hosts.

## Required implementation

1. Integrate host selection, containers, inspect, health, ports, logs, stats, events and Compose grouping into one navigation flow.
2. Review every structured operation for read-only behavior and confirm no action silently changes daemon state.
3. Document supported runtime prerequisites, current limits and the full direct/ProxyJump connection journey in docs/checkpoints/030-read-only.md.
4. Run a real disposable-host walkthrough and capture sanitized evidence/screenshots; distinguish the available OS test from pending cross-platform verification.

## Acceptance and verification

- Demonstrate list, inspect, log follow/cancel and stats through the jump host without local Docker/jq.
- Check empty daemon, denied access and disconnected host UX; block the checkpoint if the integrated live workflow is unavailable.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/030.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 030 --evidence codex/tracking/evidence/030.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 030 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 031 — Mutation intents and local activity records

- Phase: 04 Management
- Recommended reasoning: **high**
- Depends on: **030**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 031`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Prepare controlled start/stop/restart actions.

## Required implementation

1. Implement backend-issued short-lived confirmation intents tied to host, daemon identity, session generation, action and exact target IDs.
2. Require host write mode plus a consumed confirmation intent for mutations; prevent double-click duplication with an operation lock.
3. Record a bounded sanitized local activity history with action, time, target and outcome, including unknown outcome after transport loss.
4. Do not call this history a tamper-proof audit log; do not put environment values, full commands or terminal content in it.

## Acceptance and verification

- Test expired/reused intents, host switches after confirmation and direct IPC calls that bypass UI.
- Verify interrupted mutations are marked unknown and are never automatically replayed.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/031.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 031 --evidence codex/tracking/evidence/031.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 031 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 032 — Container start stop and restart

- Phase: 04 Management
- Recommended reasoning: **high**
- Depends on: **031**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 032`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Implement the essential container management controls.

## Required implementation

1. Add fixed builders and UI actions for start, stop and restart with a bounded validated stop timeout where supported.
2. Show host, daemon and exact container name/ID in confirmation; use IDs obtained from the current host inventory.
3. After the action, refresh inspect/list to verify observed state and show error or pending convergence rather than assuming exit code alone proves readiness.
4. If connectivity is lost after submission, report unknown outcome and offer a read refresh before the user chooses another action.

## Acceptance and verification

- Run start/stop/restart against disposable containers and verify state transitions plus health-check delay.
- Disconnect after dispatch and confirm there is no automatic second mutation on reconnect.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/032.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 032 --evidence codex/tracking/evidence/032.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 032 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 033 — Multi-container actions and stopped-container removal

- Phase: 04 Management
- Recommended reasoning: **high**
- Depends on: **032**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 033`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Handle deliberate small batches without hiding partial failures.

## Required implementation

1. Add bounded explicit selections for start/stop/restart with a confirmation listing affected IDs and host.
2. Execute with conservative concurrency and individual outcomes; allow cancellation of pending work and report operations already dispatched.
3. Add optional removal only for currently stopped containers with a separate confirmation, no force flag and no volume-removal flag.
4. Prevent selection across hosts and recheck container state immediately before removal; keep partial outcomes in activity history.

## Acceptance and verification

- Test a batch with success, disappearance and permission error and ensure each result remains visible.
- Attempt removing a running container and verify the application refuses its supported removal flow.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/033.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 033 --evidence codex/tracking/evidence/033.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 033 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 034 — Image inventory and inspection

- Phase: 04 Management
- Recommended reasoning: **medium**
- Depends on: **033**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 034`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Add read-only image visibility for the selected daemon.

## Required implementation

1. List image IDs, tags/digests, size and creation data through structured CLI output; deduplicate image identity while preserving multiple tags.
2. Expose image metadata and links to containers using it; keep secret-like labels masked in default export/diagnostics.
3. Support filtering dangling images for inspection only and clearly handle missing tags.
4. Reuse host/session scoping and bounds from container views; do not implement pulling arbitrary registries or image deletion in this step.

## Acceptance and verification

- Test dangling images, multi-tag images and identical tags on different hosts.
- Compare image identity and references with the disposable daemon inventory.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/034.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 034 --evidence codex/tracking/evidence/034.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 034 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 035 — Volume inventory and mount relationships

- Phase: 04 Management
- Recommended reasoning: **medium**
- Depends on: **034**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 035`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Make persistent storage relationships visible.

## Required implementation

1. List and inspect volumes with name, driver, scope and safe metadata through fixed read builders.
2. Build references from container inspect mounts without copying host volume contents.
3. Display unused-reference status as a snapshot observation, not proof that deletion is safe.
4. Keep volume browsing, deletion and prune outside this release; reuse generic read-only list/detail patterns.

## Acceptance and verification

- Test named/anonymous/external-driver volumes, no mountpoint and containers deleted during refresh.
- Verify volume views never spawn filesystem reads against remote data directories.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/035.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 035 --evidence codex/tracking/evidence/035.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 035 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 036 — Network inventory and container attachments

- Phase: 04 Management
- Recommended reasoning: **medium**
- Depends on: **035**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 036`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Show Docker networks and their attached containers.

## Required implementation

1. List/inspect networks using typed adapters and show driver, internal flag, IPAM summary and attachment relationships.
2. Handle IPv4/IPv6, host/none networks, missing IPAM and stale endpoint data.
3. Link attached container IDs back to the correct host inventory and show unknown/deleted endpoints without crashing.
4. Keep network mutation out of this milestone and render returned names/options as plain text.

## Acceptance and verification

- Test bridge, host, none, IPv6 and custom network fixtures plus malformed optional fields.
- Compare attachment counts against inspect output on the disposable target.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/036.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 036 --evidence codex/tracking/evidence/036.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 036 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 037 — Verified remote Compose project actions

- Phase: 04 Management
- Recommended reasoning: **high**
- Depends on: **036**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 037`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Support start stop and restart for explicitly configured remote Compose projects.

## Required implementation

1. Require a user-confirmed remote project directory, ordered config-file paths and explicit project name; quote each validated path and validate accessibility.
2. Use a controlled remote working directory and explicit Compose arguments; report missing environment/configuration dependencies without exporting resolved secrets.
3. Support start/stop/restart only for existing services in this release, with the same backend mode and confirmation policy.
4. Do not derive commands from untrusted labels alone; retain read-only grouping when executable configuration cannot be verified. Exclude up/build/pull/down and volume deletion.

## Acceptance and verification

- Test remote paths with spaces/apostrophes, multiple config files, absent env file and wrong project name.
- Demonstrate one disposable project restart and verify services afterward without modifying unrelated projects.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/037.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 037 --evidence codex/tracking/evidence/037.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 037 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 038 — Management MVP checkpoint

- Phase: 04 Management
- Recommended reasoning: **high**
- Depends on: **037**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 038`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Verify the management workflow and its error handling as a single product.

## Required implementation

1. Walk through toggling host write mode, confirming a lifecycle action, inspecting outcome and returning to read-only mode.
2. Review Images, Volumes, Networks and Compose views with the current container detail navigation.
3. Exercise batch partial failure, stale selection, daemon change and unknown action outcome; document the supported action boundaries.
4. Write docs/checkpoints/038-management.md with sanitized live results and remaining platform verification.

## Acceptance and verification

- Run lifecycle and Compose actions only against explicitly disposable lab resources.
- Check that read-only host settings are enforced in Rust for every mutation endpoint; mark any incomplete gate blocked.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/038.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 038 --evidence codex/tracking/evidence/038.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 038 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 039 — PTY terminal transport

- Phase: 05 Terminal and resilience
- Recommended reasoning: **high**
- Depends on: **038**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 039`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Implement a real interactive container shell with a separate transport path.

## Required implementation

1. Use a maintained PTY library selected for Linux/macOS and spawn OpenSSH in its local PTY; request a remote PTY for docker exec -it on a validated running container.
2. Support an explicit fixed shell choice such as /bin/sh or /bin/bash and a default non-root container user; report a missing shell rather than injecting fallback scripts.
3. Implement resize, stdin bytes, stdout bytes, close and child reaping with one session-bound terminal ID.
4. Gate opening the terminal behind host write/terminal permission because interactive input can perform arbitrary changes. Read-only mode must reject it in Rust.

## Acceptance and verification

- Test interactive echo, Ctrl-C, resize, exit and a container without a shell in the disposable lab.
- Ensure noninteractive JSON commands never inherit terminal settings or the PTY output path.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/039.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 039 --evidence codex/tracking/evidence/039.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 039 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 040 — Terminal UI with bounded lifecycle

- Phase: 05 Terminal and resilience
- Recommended reasoning: **high**
- Depends on: **039**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 040`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Connect the PTY to a usable terminal tab.

## Required implementation

1. Use xterm.js with fit support, bounded scrollback and explicit connect/disconnect status; keep host and container identity always visible.
2. Forward input only to the active owned terminal. Disable automatic clipboard writes and unsafe escape-sequence integrations; ask before multiline paste.
3. Stop and reap the PTY on tab close, host disconnect or permission revocation; do not reconnect and replay shell input.
4. Do not persist terminal transcripts or command history in app storage; allow intentional copy of selected visible text.

## Acceptance and verification

- Test terminal resize, paste with newlines, tab closure under output and switching between two host sessions.
- Verify a background terminal cannot receive keystrokes from another tab and no transcript appears in diagnostics.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/040.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 040 --evidence codex/tracking/evidence/040.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 040 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 041 — Sleep wake network loss and graceful shutdown

- Phase: 05 Terminal and resilience
- Recommended reasoning: **high**
- Depends on: **040**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 041`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Make the app recover predictably from normal laptop behavior.

## Required implementation

1. Detect stale masters and failed keepalives, invalidate old session generations and cancel owned jobs when connectivity disappears.
2. On wake/retry, reconnect selected hosts with bounded backoff and refresh read snapshots; restart log subscriptions only with an explicit visible gap.
3. Close terminals after broken sessions and never replay typed input or mutations.
4. Implement graceful app exit cleanup with a bounded deadline, ensuring only app-owned sockets and child processes are touched.

## Acceptance and verification

- Use fault injection to break the master during logs/stats/terminal work and inspect the process tree afterward.
- Perform a native suspend/resume or documented network interruption test on an available OS.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/041.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 041 --evidence codex/tracking/evidence/041.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 041 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 042 — Cross-platform GUI launch and SSH agent behavior

- Phase: 05 Terminal and resilience
- Recommended reasoning: **high**
- Depends on: **041**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 042`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Close the gap between terminal development and installed desktop operation.

## Required implementation

1. Test Linux desktop launcher and macOS Finder launch with minimal environment; resolve SSH through the configured absolute executable.
2. Provide diagnostics for inaccessible agent sockets and passphrase-protected keys; document user setup for their OS agent/Keychain without shelling into profile scripts.
3. Handle spaces/non-ASCII home directories, application data paths and permission errors consistently.
4. Document macOS-only SSH options as optional guarded config fragments, not mandatory syntax for Linux.

## Acceptance and verification

- Run platform-specific manual steps on each available platform and keep missing-platform results explicitly pending.
- Verify the app never depends on local Docker, jq or development toolchains after installation.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/042.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 042 --evidence codex/tracking/evidence/042.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 042 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 043 — Support diagnostics and redacted export

- Phase: 05 Terminal and resilience
- Recommended reasoning: **high**
- Depends on: **042**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 043`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Create useful troubleshooting output without dumping sensitive data.

## Required implementation

1. Add an app-controlled diagnostic report containing version, platform, configured transport mode, staged error codes and bounded redacted timings.
2. Exclude keys, raw SSH config, tokens, environment values, log buffers, terminal text and full inspect payloads by default.
3. Provide an export preview and explicit save action; use pseudonyms for hostnames/paths where possible and explain remaining user-supplied text.
4. Implement a finite retention policy for local activity/diagnostic logs and a clear-local-data action that never deletes SSH files.

## Acceptance and verification

- Seed synthetic secrets in all error paths and scan the exported report for leakage.
- Confirm clearing app data leaves ~/.ssh/config, private keys and known_hosts untouched.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/043.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 043 --evidence codex/tracking/evidence/043.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 043 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 044 — Accessibility themes and keyboard workflow

- Phase: 05 Terminal and resilience
- Recommended reasoning: **medium**
- Depends on: **043**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 044`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Make routine inspection efficient without a mouse.

## Required implementation

1. Audit semantic tables, labels, keyboard focus, dialog traps, escape handling and screen-reader status announcements.
2. Support platform-appropriate shortcuts for search, refresh, host selection and log focus without capturing terminal keystrokes.
3. Ensure dark/light themes communicate state with text/icons in addition to color, and respect reduced motion.
4. Inspect dense views at different font scales and window sizes; keep actionable error text readable.

## Acceptance and verification

- Run a keyboard-only host-to-container-to-logs workflow and a screen-reader spot check where available.
- Fix verified focus/contrast issues and attach representative screenshots to the evidence.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/044.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 044 --evidence codex/tracking/evidence/044.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 044 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 045 — Large inventories and stream pressure

- Phase: 05 Terminal and resilience
- Recommended reasoning: **high**
- Depends on: **044**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 045`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Measure resource usage under realistic concurrency and impose clear limits.

## Required implementation

1. Create a reproducible synthetic benchmark for 1000 container summaries, a large inspect response and high-rate log/events streams.
2. Measure render latency, queue sizes, child process counts and app memory growth on a documented machine; record observations rather than invented numbers.
3. Set configurable but bounded limits for retained log lines/bytes, stats history, active hosts and concurrent jobs.
4. Use virtualization/batching only where measurement shows benefit; expose truncated/dropped data explicitly.

## Acceptance and verification

- Confirm sustained output reaches a memory plateau under the configured buffers.
- Verify cancelling all subscriptions returns owned child count to baseline and filtering remains responsive.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/045.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 045 --evidence codex/tracking/evidence/045.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 045 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 046 — Feature-complete desktop checkpoint

- Phase: 05 Terminal and resilience
- Recommended reasoning: **high**
- Depends on: **045**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 046`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Review the complete product before release engineering.

## Required implementation

1. Demonstrate the workflow from SSH discovery through live data, management, Compose and container terminal against a disposable jump-host setup.
2. Create a feature matrix marking implemented, verified and platform-pending states; remove placeholder buttons and misleading success messages.
3. Inspect command-policy coverage, resource lifecycle and consistent host identity across tabs.
4. Write docs/checkpoints/046-feature-complete.md with actual defects fixed and explicit remaining release gates.

## Acceptance and verification

- Run the integrated journey on the current OS and record terminal/log cleanup evidence.
- Block this checkpoint for any missing core feature; platform release verification remains mandatory later.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/046.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 046 --evidence codex/tracking/evidence/046.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 046 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 047 — Focused security review

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **046**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 047`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Check the concrete trust boundaries implemented by this application.

## Required implementation

1. Review renderer IPC validation, remote quoting, host/config trust, StrictHostKeyChecking, app-owned sockets, read-only policy, confirmation intents and terminal access.
2. Test malformed IDs and host aliases, hostile labels/logs, stale mutation tokens and changed daemon/session identities.
3. Check CSP and Tauri capabilities, filesystem scopes, navigation rules and protocol handlers; remove generic shell execution exposed to the renderer.
4. Record findings and fixes in docs/security-review.md; app read-only mode is not server-side authorization for a Docker-privileged account.

## Acceptance and verification

- Run regression checks for each discovered exploitable path and verify default exports omit seeded secrets.
- Confirm release configuration cannot load arbitrary remote UI content or expose an unauthenticated control endpoint.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/047.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 047 --evidence codex/tracking/evidence/047.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 047 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 048 — Frontend and parser regression coverage

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **047**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 048`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Protect the important behavior without creating tests that only mirror implementation.

## Required implementation

1. Select regression scenarios for host isolation, stale data, error mapping, log buffers, mutation uncertainty and null Docker fields.
2. Use contract fixtures tied to documented remote CLI observations and test desired behavior through public adapters.
3. Add deterministic mock IPC component coverage for connection errors and confirmation dialogs.
4. Keep network tests explicit and bounded; avoid requiring access to production hosts for normal test runs.

## Acceptance and verification

- Run the frontend/Rust test suites with fixed fixtures and document exact commands.
- Demonstrate at least one failure the regression tests catch by a controlled temporary fault, then restore the implementation.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/048.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 048 --evidence codex/tracking/evidence/048.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 048 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 049 — Disposable direct and bastion integration lab

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **048**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 049`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Provide repeatable end-to-end SSH and Docker transport verification.

## Required implementation

1. Add an opt-in lab using disposable VMs or containers with a dedicated test Docker daemon; never mount a production host Docker socket into the test fixture.
2. Create direct and ProxyJump topology, ephemeral keys, strict known_hosts and a private target inaccessible directly from the test client.
3. Cover handshake, list/inspect/logs/stats, mutation, Compose, terminal, key mismatch and reconnect with bounded timeouts.
4. If Docker-in-Docker requires privileged mode, keep it inside a dedicated disposable test VM and document the requirement; include complete cleanup.

## Acceptance and verification

- Run the integration suite and prove the private target is reached through the bastion.
- Check cleanup of test keys, resources and child processes; record versions and sanitized logs.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/049.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 049 --evidence codex/tracking/evidence/049.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 049 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 050 — Native desktop integration tests

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **049**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 050`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Validate the Tauri boundary in real desktop windows.

## Required implementation

1. Choose the currently supported Tauri/WebdriverIO path from official documentation; distinguish browser-only mocked UI tests from native application tests.
2. For macOS, evaluate the documented embedded WebDriver service if suitable; do not assume the standalone tauri-driver supports WKWebView.
3. Keep automation plugins, test server and mock-control APIs out of production builds using explicit build features.
4. Test host selection, native IPC, connection errors, log cancellation and a bounded terminal interaction on available native runners.

## Acceptance and verification

- Record Linux/macOS native automation separately and mark unavailable runners pending.
- Inspect a release build configuration to verify embedded test automation cannot start in production.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/050.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 050 --evidence codex/tracking/evidence/050.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 050 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 051 — Local verification command and clean builds

- Phase: 06 Quality and delivery
- Recommended reasoning: **medium**
- Depends on: **050**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 051`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Make the agreed verification reproducible for contributors.

## Required implementation

1. Create documented commands for formatting, TypeScript checks, frontend tests/build, cargo fmt/clippy/test and an opt-in native integration run.
2. Use lockfiles and pinned toolchains; separate dependency installation from destructive cleanup.
3. Ensure the verification command exits nonzero on a failed required check and reports skipped optional/platform checks explicitly.
4. Record required system packages per tested distribution and macOS development prerequisites using current official documentation.

## Acceptance and verification

- Run the standard verification from a clean dependency install on the current platform.
- Force one check to fail temporarily and ensure the aggregate command reports failure, then restore it.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/051.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 051 --evidence codex/tracking/evidence/051.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 051 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 052 — Linux and macOS CI build matrix

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **051**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 052`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Automate reproducible checks and package creation without publishing automatically.

## Required implementation

1. Add GitHub Actions with pinned/action-reviewed dependencies and Linux/macOS jobs, least-privilege token permissions and documented architecture mapping.
2. Run checks before producing versioned artifacts, checksums and metadata; retain failures and test reports.
3. Separate trusted release jobs from pull-request builds so forked changes never access signing keys or release tokens.
4. Keep publishing as an explicit workflow action after the release gate; artifact generation should work without Apple credentials.

## Acceptance and verification

- Validate workflow syntax and run available jobs; document unavailable remote CI execution as pending.
- Inspect permissions and prove untrusted PR events cannot reach secrets or publishing steps.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/052.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 052 --evidence codex/tracking/evidence/052.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 052 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 053 — Linux package builds

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **052**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 053`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Prepare installable Linux artifacts and define their actual support range.

## Required implementation

1. Build the chosen Tauri Linux packages, initially deb and AppImage, on a pinned supported Ubuntu baseline with required native libraries.
2. Declare x86_64 as the initial verified Linux architecture; do not advertise untested distributions or aarch64 builds.
3. Configure app name, desktop entry, icons, executable permissions and required package dependencies.
4. Document that sandboxed distribution formats need additional SSH file/socket access design and are outside the first package set.

## Acceptance and verification

- Install and launch at least one native package on a clean supported desktop environment.
- Verify host config access and SSH discovery from the desktop launcher, recording artifact hashes and platform versions.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/053.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 053 --evidence codex/tracking/evidence/053.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 053 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 054 — macOS application and DMG builds

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **053**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 054`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Prepare macOS desktop artifacts for Apple Silicon and Intel.

## Required implementation

1. Build on macOS using the pinned Tauri/Rust toolchains and selected deployment target; produce explicit arm64 and x86_64 artifacts or a tested universal bundle.
2. Configure bundle identifier, application icon, permissions and DMG layout with no embedded SSH credentials.
3. Test Finder launch, app data paths, agent access and /usr/bin/ssh usage in the packaged app.
4. Record local unsigned/ad-hoc builds separately from publicly distributable signed/notarized builds; do not describe Gatekeeper bypasses as installation requirements.

## Acceptance and verification

- Verify package contents and launch on each architecture claimed as tested; cross-compilation alone does not prove runtime support.
- If macOS hardware/runner is unavailable, complete build configuration and mark this platform verification prompt blocked.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/054.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 054 --evidence codex/tracking/evidence/054.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 054 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 055 — Signing and notarization integration

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **054**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 055`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Prepare optional public distribution credentials without making them prerequisites for local development.

## Required implementation

1. Implement documented macOS signing/notarization steps using CI secrets and current Tauri/Apple guidance, with cleanup of temporary keychains.
2. Verify signing identity, entitlements, notarization result and stapling before labeling an artifact publicly verified.
3. Provide a clear branch of evidence: configuration-ready without credentials, or end-to-end signing-verified with supplied credentials.
4. Define Linux checksum/signature publication policy using owner-provided signing material if desired; never generate or upload private keys on the users behalf silently.

## Acceptance and verification

- Without credentials, verify missing-secret checks and configuration and explicitly record notarization as unverified.
- With authorized credentials, validate the produced artifact using native verification tools and retain redacted results.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/055.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 055 --evidence codex/tracking/evidence/055.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 055 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 056 — Versioning updates and rollback guidance

- Phase: 06 Quality and delivery
- Recommended reasoning: **medium**
- Depends on: **055**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 056`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Make manual upgrades predictable and keep runtime updates optional.

## Required implementation

1. Use one authoritative application version and ensure frontend, Cargo and bundle versions stay aligned with a documented release script.
2. Default the first release to manual installer updates with a verified download location, published hashes and settings backup/migration guidance.
3. Document rollback limitations for schema migrations and preserve compatible settings backups.
4. If an updater is later enabled, require signed update metadata/artifacts and actual hosting credentials; do not ship placeholder update endpoints or auto-install logic now.

## Acceptance and verification

- Test version mismatch detection and a settings migration/rollback scenario with fixture files.
- Check the production build performs no unsolicited update or telemetry requests.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/056.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 056 --evidence codex/tracking/evidence/056.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 056 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 057 — User and contributor documentation

- Phase: 06 Quality and delivery
- Recommended reasoning: **medium**
- Depends on: **056**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 057`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Document the product exactly as implemented.

## Required implementation

1. Write Serbian quick start and English technical docs for install, trusted SSH config, direct/ProxyJump aliases, agent setup and Docker access.
2. Explain remote Docker context selection, rootless and configured sudo -n modes, unsupported remote shells and Compose path requirements.
3. Provide troubleshooting for unknown host keys, failed jump authentication, PATH differences, missing logging driver support and unknown mutation outcomes.
4. Describe read-only mode, terminal access, local data locations and optional export; avoid instructions that disable SSH host verification.

## Acceptance and verification

- Follow the quick start in a fresh test user profile and correct missing steps.
- Cross-check every documented control/command with the current application and remove aspirational features.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/057.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 057 --evidence codex/tracking/evidence/057.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 057 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 058 — Native platform acceptance matrix

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **057**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 058`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Prove the claimed Linux and macOS workflows with real execution evidence.

## Required implementation

1. Complete docs/platform-matrix.md for Linux x86_64, macOS arm64 and macOS x86_64, including OS/app/OpenSSH/Docker versions and native launch.
2. For each claimed platform verify direct+ProxyJump, encrypted-key agent access, strict trust behavior, list/logs/stats, management and terminal resize/cleanup.
3. Run real tests where practical and use explicitly named CI/native runner evidence; mocks and cross-compiled binaries do not count as runtime verification.
4. Keep any missing platform blocked or narrow the release claim with an explicit documented scope decision; do not silently drop the agreed Linux/macOS targets.

## Acceptance and verification

- Review evidence row by row and link exact logs/screenshots/checksums without secrets.
- Leave the prompt blocked if a required native platform lacks evidence; packaging preparation may already be complete.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/058.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 058 --evidence codex/tracking/evidence/058.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 058 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 059 — Release candidate review and defect closure

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **058**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 059`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Prepare a reviewable release candidate with accurate readiness status.

## Required implementation

1. Review all completed prompt evidence, known defects, compatibility matrix, app security boundaries and produced package hashes.
2. Fix release-blocking defects found in the integrated workflow, add only targeted regression tests and update affected evidence honestly.
3. Write release notes covering implemented features, runtime prerequisites, supported OS/architectures, limitations and signing status.
4. Prepare a draft release checklist and local release directory; do not upload/publish artifacts or contact users as part of this prompt.

## Acceptance and verification

- Run the required checks once after final fixes and confirm the release artifacts correspond to the tested commit.
- Verify no pending core bug or unverified claimed platform is hidden by a completed tracker status.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/059.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 059 --evidence codex/tracking/evidence/059.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 059 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---

# 060 — Final handover and release gate

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **059**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 060`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Deliver the complete repository and a truthful handover for operating and maintaining the app.

## Required implementation

1. Produce docs/FINAL_HANDOVER.md listing build/run/install commands, architecture decisions, package locations/hashes and native platform evidence.
2. Summarize verified features, local unsigned versus signed distribution status, known limitations and the next optional enhancements.
3. Validate all prompt statuses and evidence, repository cleanliness without deleting user work, and secret-free release contents.
4. Mark complete only when the agreed app/platform gates are satisfied; public release and any provider accounts remain owner actions unless separately authorized.

## Acceptance and verification

- Run tracker validate and the agreed release smoke checklist against the final artifact set.
- If a required gate is unavailable, record the exact blocker and safe next command; do not substitute a mock/demo screenshot for a native release.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/060.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 060 --evidence codex/tracking/evidence/060.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 060 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.


---
