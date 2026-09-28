# Development and build commands

Run from the repository root. Use the pinned Node 24.21.0/npm 11.19.0 (`.nvmrc`) and Rust/Cargo 1.98.1 (`rust-toolchain.toml`). Make sure rustup's bin directory is on PATH. See [toolchains](toolchains.md) for native platform prerequisites. Install dependencies with `npm ci`; only npm is used. Keep both `package-lock.json` and `src-tauri/Cargo.lock` committed.

## Commands

| Command | Purpose |
|---|---|
| `npm run desktop:dev` | Start Vite on loopback port 1420 and launch the native Tauri window; Ctrl-C stops development |
| `npm run dev` | Frontend-only Vite server; a browser explicitly reports that the desktop version bridge is unavailable |
| `npm run check` | Strict TypeScript, Biome lint/format and focused Node IPC contract tests |
| `npm run format` | Format frontend/configuration files without touching historical prompts or tracker files |
| `npm run rust:format:check` | Check Rust formatting |
| `npm run rust:check` | Locked native Cargo check; requires platform development libraries |
| `npm run rust:lint` | Locked Clippy for all targets with warnings treated as errors |
| `npm run build` | Typecheck and build the production frontend to `dist/` |
| `npm run desktop:build` | Build a release native executable with embedded frontend and locked Cargo dependencies, without installers |
| `npm run preview` | Serve built frontend locally for browser inspection; no Rust bridge |

On Linux, the unbundled release executable is `src-tauri/target/release/containerdesk`. On macOS, build natively for the host architecture; the unbundled executable has the same relative path. Run the binary to inspect the production shell. Installer formats, native platform acceptance and signing/notarization belong to later prompts; `bundle.active` is deliberately false for this increment.

## Scaffold behavior and boundaries

The entry points are `src/main.tsx`, `src-tauri/src/main.rs` and `src-tauri/src/lib.rs`. React requests `app_version` through `src/lib/ipc/app-version.ts`. Rust returns `{ "version": "…" }` from Tauri package metadata; the Tauri configuration reads the version from `package.json`. The bridge validates the response shape and handles rejection; the UI shows loading, failure/retry, browser-unavailable and actual version states. It ignores responses belonging to an unmounted version request.

`build.rs` declares the app command to the Tauri ACL generator. The capability grants only `allow-app-version` to the local `main` window. Its generated permission file is committed for review; machine-generated schemas in `src-tauri/gen/` stay ignored. No shell plugin, SSH operation, host discovery, persistence or mutation exists in this scaffold. The version query is side-effect free and takes no renderer arguments. Domain models/host modes arrive in their own prompts.

Production CSP allows bundled scripts/styles and Tauri IPC only; development separately allows loopback HMR and Vite's inline styles. Vite binds to 127.0.0.1 and fails on an occupied port rather than exposing a different development URL. No hosted fonts or external runtime assets are used. The code-native icon source is `src-tauri/icons/source.svg`; its PNG is generated using the pinned Tauri CLI. The reverse-DNS identifier `dev.containerdesk.app` is an internal working identifier, not a domain-ownership claim.

Biome 2.5.14 supplies recommended lint rules and formatting without adding a second formatter. Node types 24.19.0 match the selected Node major. Other application dependency pins from 001 remain unchanged. Tests use Node's built-in runner and Tauri's official `mockIPC`; they prove frontend invocation/decoding/error behavior, not native transport. Native operation needs its own launch evidence.

## Reference material

- [Tauri command invocation](https://v2.tauri.app/develop/calling-rust/)
- [Tauri Vite integration](https://v2.tauri.app/start/frontend/vite/)
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)
- [Biome setup](https://biomejs.dev/guides/getting-started/)

Current verification results and limitations are recorded in [prompt 002 evidence](../codex/tracking/evidence/002.md).

## Launching from a Snap-hosted terminal

The audit session inherited incompatible Snap library/module paths. The native executable was verified with this process-local workaround, which does not alter global settings:

```sh
env -u LD_LIBRARY_PATH -u LD_PRELOAD -u GTK_PATH -u GIO_MODULE_DIR GDK_BACKEND=x11 src-tauri/target/release/containerdesk
```

This evidence covers GTK on X11/XWayland. It does not establish native Wayland or macOS behavior. Prefer a normal host terminal with the project toolchain selected for everyday development.
