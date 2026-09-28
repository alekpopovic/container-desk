# ContainerDesk project status

Updated 2026-09-28 after prompt 012. ContainerDesk is a working native Tauri v2 shell with React, TypeScript, Vite, Tailwind and Rust/Tokio. Foundations and remote command preparation through 012 are implemented; live remote Docker management is not available yet.

## Current implementation

- Responsive host/resource workspace, six routes, empty/error/offline presentation, keyboard navigation and light/dark/system themes.
- Rust-owned generated IPC models with typed errors, validated IDs and host/selection/session/daemon scopes. Late success and error responses are rejected by renderer adapters.
- Private versioned JSON settings, atomic replacement, recoverable previous files, corrupt-original retention, migration, revision checks and owner-only permissions. Theme save/restart and recovery were exercised in the real Linux app.
- Native dependency diagnostics and validated absolute OpenSSH override. Real SSH -V and agent socket presence/accessibility checks have output/time/concurrency limits; keys and identities are not read. The native view works with PATH=/nonexistent.
- Backend operation registry, default read-only access and short-lived one-use confirmation intents bound to exact operations/targets/scopes. Mutation/terminal handlers reject read-only requests directly in Rust and have no production window grant yet. Remote execution remains unavailable.

- Explicit offline demo with persistent labeling, deterministic failure scenarios, bounded synthetic parsing and replaceable read transport; no live-to-fixture fallback or SSH probing in demo.

- SSH config candidate discovery with explicit browse/selection, bounded includes and manual aliases; no automatic resolution or connections.

- Explicit native ssh -G resolution with bounded output/deadline, original-alias/config-policy preservation, trusted-code explanation and safe effective-host/user/port/jump display.

- Shared cancellable process runner with four slots, separate bounded pipes, typed failures, partial-output preservation and cleanup/reaping before acknowledgment. Native structured SSH option policy is prepared; no remote command dispatcher is exposed yet.

- Centralized POSIX remote quoting and validated Docker command preparation, including absolute binary overrides, named contexts and fixed optional sudo -n; hostile arguments checked through an inert shell harness.

Design: [remote commands](remote-commands.md), [SSH runner](ssh-runner.md), [SSH resolution](ssh-resolution.md), [SSH discovery](ssh-discovery.md), [demo mode](demo-mode.md), [IPC](ipc-contract.md), [settings](settings.md), [native dependencies](native-dependencies.md), [operation policy](operation-policy.md), [design system](design-system.md). Commands/pins: [development](development.md), [toolchains](toolchains.md).

Evidence: [001](../codex/tracking/evidence/001.md), [002](../codex/tracking/evidence/002.md), [003](../codex/tracking/evidence/003.md), [004](../codex/tracking/evidence/004.md), [005](../codex/tracking/evidence/005.md), [006](../codex/tracking/evidence/006.md), [007](../codex/tracking/evidence/007.md), [008](../codex/tracking/evidence/008.md), [009](../codex/tracking/evidence/009.md), [010](../codex/tracking/evidence/010.md), [011](../codex/tracking/evidence/011.md), [012](../codex/tracking/evidence/012.md).

## Actual platform status

| Area | Status |
|---|---|
| Linux runtime | Verified on Ubuntu 26.04.1 x86_64, GTK 3.24.52, WebKitGTK 2.52.6, X11/XWayland |
| Native settings/diagnostics | Verified in isolated app-data profiles; unrelated app windows retained |
| Launch environment | Process-local removal of incompatible Snap library/module variables required on this host; no global configuration changed |
| Ubuntu 24.04 baseline / native Wayland | Not verified |
| macOS arm64 / Intel | Not built or executed |
| Installers, signing, notarization | Not performed; native unbundled Linux builds only |
| SSH host sessions / Docker resource reads / mutations / terminal | Not implemented or exercised live yet |

No browser fixture, mock runtime or build is counted as native server-operation proof. Existing bundle-identifier warning and GitHub moderate dependency alert remain open for the relevant later review. [Checkpoints](checkpoints.md) still require actual lab/platform evidence.

Next prompt: **013 — SSH authentication and host trust flow**. Continue sequentially under the user's [execution authorization](execution.md).
