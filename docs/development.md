# Development and build commands

Run from the repository root. Use the pinned Node 24.21.0/npm 11.19.0 (`.nvmrc`) and Rust/Cargo 1.98.1 (`rust-toolchain.toml`). Make sure rustup's bin directory is on PATH. See [toolchains](toolchains.md) for native platform prerequisites. Install dependencies with `npm ci`; only npm is used. Keep both `package-lock.json` and `src-tauri/Cargo.lock` committed.

Use `npm run verify:install` for locked dependencies, then `npm run verify` for the fail-fast required checks. Native tests remain explicit opt-in. See [contributor verification](verification-command.md) for clean output directories, reports and system prerequisites.

## Commands

| Command | Purpose |
|---|---|
| `npm run desktop:dev` | Start Vite on loopback port 1420 and launch the native Tauri window; Ctrl-C stops development |
| `npm run dev` | Frontend-only Vite server; a browser explicitly reports that the desktop version bridge is unavailable |
| `npm run check` | Strict TypeScript, Biome lint/format and focused Node IPC contract tests |
| `npm run test:ui` | Browser fixture checks; no native Rust/WebKit or real SSH proof |
| `npm run format` | Format frontend/configuration files without touching historical prompts or tracker files |
| `npm run rust:format:check` | Check Rust formatting |
| `npm run rust:check` | Locked native Cargo check; requires platform development libraries |
| `npm run rust:lint` | Locked Clippy for all targets with warnings treated as errors |
| `npm run build` | Typecheck and build the production frontend to `dist/` |
| `npm run desktop:build` | Build the ordinary native executable with automation disabled, without installers |
| `npm run desktop:build:automation` | Build an explicit native test artifact and restore the ordinary release; see [native testing](native-testing.md) |
| `npm run preview` | Serve built frontend locally for browser inspection; no Rust bridge |

On Linux, the unbundled release executable is `src-tauri/target/release/containerdesk`. On macOS, build natively for the host architecture; the unbundled executable has the same relative path. Run the binary to inspect the production shell. Installer formats, native platform acceptance and signing/notarization belong to later prompts; `bundle.active` is deliberately false for this increment.

## Application entry points and native verification

The entry points are `src/main.tsx`, `src-tauri/src/main.rs` and `src-tauri/src/lib.rs`. The executable disables automation/inspector activation unless built with the explicit `native-automation` feature, before initializing the Tauri runtime. React uses typed IPC, with Rust owning SSH/Docker sessions and permission enforcement. `build.rs` declares the application handlers and `capabilities/main.json` grants the local window's explicit allowlist. No generic shell or filesystem plugin capability is granted.

Use [native testing](native-testing.md) for real-window checks and [the disposable integration lab](integration-lab.md) for isolated direct/ProxyJump backend coverage. Browser fixtures remain separate. Current feature status and platform limits are in [project status](project-status.md); the original scaffold evidence is historical.

Production CSP allows bundled scripts/styles and Tauri IPC only; development separately allows loopback HMR and Vite's inline styles. Vite binds to 127.0.0.1 and fails on an occupied port rather than exposing a different development URL. No hosted fonts or external runtime assets are used. The code-native icon source is `src-tauri/icons/source.svg`; its PNG is generated using the pinned Tauri CLI. The reverse-DNS identifier `dev.containerdesk.app` is an internal working identifier, not a domain-ownership claim.

Biome 2.5.14 supplies recommended lint rules and formatting without adding a second formatter. Node types 24.19.0 match the selected Node major. Other application dependency pins from 001 remain unchanged. Tests use Node's built-in runner and Tauri's official `mockIPC`; they prove frontend invocation/decoding/error behavior, not native transport. Native operation needs its own launch evidence.

## Reference material

- [Tauri command invocation](https://v2.tauri.app/develop/calling-rust/)
- [Tauri Vite integration](https://v2.tauri.app/start/frontend/vite/)
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)
- [Biome setup](https://biomejs.dev/guides/getting-started/)

Original scaffold verification is recorded in [prompt 002 evidence](../codex/tracking/evidence/002.md); current native automation is tracked separately in [prompt 050 evidence](../codex/tracking/evidence/050.md).

## Launching from a Snap-hosted terminal

The audit session inherited incompatible Snap library/module paths. The native executable was verified with this process-local workaround, which does not alter global settings:

```sh
env -u LD_LIBRARY_PATH -u LD_PRELOAD -u GTK_PATH -u GIO_MODULE_DIR GDK_BACKEND=x11 src-tauri/target/release/containerdesk
```

This evidence covers GTK on X11/XWayland. It does not establish native Wayland or macOS behavior. Prefer a normal host terminal with the project toolchain selected for everyday development.

## Workspace interface

The workspace provides host selection, connection context, resource routes and responsive inventory/details. See [design system](design-system.md) for tokens and accessibility behavior.

UI checks use pinned Playwright 1.63.0. Run `npm exec playwright -- install chromium` once if its browser is missing, then `npm run test:ui`. The runner starts and stops a dedicated development server on loopback port 1431. Screenshots/traces under test-results are ignored; curated verification images are committed separately. Isolated synthetic fixtures are excluded from the production entry point and never represent real SSH/native results.

## IPC contract maintenance

Edit Rust domain DTOs, run `npm run ipc:generate`, review generated types and fixtures, then run `npm run rust:test` and `npm run check`. Ordinary tests fail on drift without rewriting files. `ts-rs` and Tauri mock-runtime helpers are development-only dependencies. See [IPC contract](ipc-contract.md).
