# Repository instructions — ContainerDesk

## Goal and stack

Build a single Tauri v2 application with React, TypeScript, Vite, Tailwind and a Rust/Tokio backend. Use the local native OpenSSH executable to operate Docker on Linux servers through the user's existing SSH configuration. Deliver Linux and macOS builds. Treat `ContainerDesk` as a working product name.

Read `codex/docs/ARCHITECTURE.md` and the selected prompt before implementation. Resolve supported package versions in prompt 001 and commit lockfiles. Do not replace working architecture merely because another library is familiar.

## Execution discipline

- Inspect existing files and uncommitted changes. Preserve unrelated user work.
- Run `python3 codex/scripts/track.py next`; execute one prompt at a time.
- Use `start`, `done` and `block` through the script; do not manually change state to bypass dependencies.
- Evidence is mandatory. Record real checks and platform limitations. Never claim that a fixture, browser preview, cross-compile or unexecuted CI file proves native operation.
- A recommended reasoning level does not change the actual model configuration. Be accurate about what your environment can control.
- Stop after the selected prompt with changes, verification, limits and next step.
- Keep immutable original prompt hashes. Put implementation decisions or approved scope changes in application docs and evidence rather than rewriting historical tasks.

## Transport and security boundaries

- Spawn a validated local SSH executable with argument arrays. Use no local shell for structured commands.
- Remote OpenSSH commands still need POSIX-shell-safe quoting plus validation. Centralize this logic and test hostile arguments.
- Treat the user's SSH config as trusted executable configuration. Discover aliases without executing it; resolve selected aliases with native OpenSSH.
- Preserve strict host verification. Use the existing agent/keys. Do not copy private keys, collect passphrases, enable agent forwarding or mutate known_hosts/config automatically.
- Backend permissions enforce read-only versus mutation/terminal access; frontend controls alone are insufficient.
- No retries of dispatched mutations or terminal input. A lost response can mean an unknown outcome.
- Use bounded queues, output limits, timeouts and session generations. Reap child processes and clean only app-owned resources.
- Render logs/labels as untrusted text. Mask inspect environment values before IPC. Keep credentials, raw logs and terminal output out of telemetry, persistence and default diagnostics.
- No exposed Docker TCP port or custom remote agent is needed for this architecture.

## Product scope

Implement the agreed read views, lifecycle actions, Compose start/stop/restart for verified existing projects, terminal and native packaging. Images/volumes/networks are read-only in v1. Container removal is limited to explicitly selected stopped containers without force or volume removal. No prune, registry credentials, compose up/down, arbitrary script runner, SaaS backend or Kubernetes integration is required.

Changes to real server state require the app user to select the host, enable management and invoke the action. During implementation use disposable test resources. Never run development checks against a production host by guessing an SSH alias.

## Verification and handover

Use targeted tests for parsing, quoting, IPC authorization, ownership/cancellation and state recovery; avoid tests that merely snapshot the implementation. Native package and platform gates require real native execution evidence. Do not invent signing credentials, publish artifacts or add paid services as part of implementation. Separate local package verification from public signing/notarization readiness.
