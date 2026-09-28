# ContainerDesk project status

Updated 2026-09-28 after prompt 006. The repository now has a working Tauri v2 desktop shell with React, strict TypeScript, Vite and Tailwind. Its backend now has generated Rust/TypeScript domain types and four narrow host/session commands in addition to the version query. The workspace now includes host groups, host identity/status, six resource/settings routes, responsive inventory/details and light/dark themes. Native Linux wide and narrow layouts have been visually inspected.

## Implemented and verified

- One application in the existing root; the original prompt pack, AGENTS.md and immutable hashes remain intact. Prompt 002 started from clean commit `2f598de89c7f9e2386a79f03ad67cb1296e3faf5`.
- React handles loading, actual version, bridge error/retry and browser-only states. Rust supplies version metadata through a narrow typed command. Prompt 002 initially granted only `allow-app-version`; prompt 004 extends the local capability with the four narrow host/session commands.
- Node 24.21.0/npm 11.19.0 and Rust/Cargo 1.98.1 remain pinned. Biome 2.5.14 and Node types 24.19.0 were added with exact versions. npm's lock was updated; the committed Cargo.lock remains unchanged.
- Strict TypeScript, recommended Biome lint, formatting and three focused IPC bridge tests passed. Rust `cargo check --locked` and Clippy with warnings denied passed.
- Production frontend and native release builds passed. The unbundled Linux executable embeds its frontend and opens a real GTK/WebKit window; the screenshot records actual Rust-to-React version IPC, not browser mock output.

Commands: [development](development.md). Sources/pins: [toolchains](toolchains.md). Initial audit: [001 evidence](../codex/tracking/evidence/001.md). Scaffold checks: [002 evidence](../codex/tracking/evidence/002.md). Current layout checks: [003 evidence](../codex/tracking/evidence/003.md).

Prompt 003 adds pinned Playwright 1.63.0. All 18 browser checks passed across light/dark and 1280×800, 800×700, 640×480, including keyboard navigation, state fixtures, text safety and token contrast. Production native build and real wide/narrow Linux captures passed; fixtures are excluded from the built app. See [design system](design-system.md).

## Native environment and limits

| Area | Actual status |
|---|---|
| Linux host | Ubuntu 26.04.1 LTS x86_64, glibc 2.43 |
| Installed development libraries | User installed missing packages; pkg-config 2.5.1, GTK 3.24.52, WebKitGTK 2.52.6 confirmed |
| Native launch | PASS on X11/XWayland with Snap-inherited library/module paths removed for this process |
| Native screenshot | [Wide workspace](verification/003-native-wide.png) / [narrow workspace](verification/003-native-narrow.png) |
| Native Wayland / Ubuntu 24.04 baseline | NOT RUN; current host execution is not baseline certification |
| macOS arm64 / Intel | NOT RUN; no Mac native build or runtime evidence |
| Installers / public signing / notarization | NOT RUN; no packages published; bundling disabled for scaffold |
| SSH / Docker resource views / management / terminal | Not implemented or exercised in this prompt |

The first launch inherited incompatible Snap library paths and failed before opening the window. A process-local clean environment resolved the conflict; no system or user shell configuration was changed. Non-fatal missing `canberra-gtk-module` messages remain in this environment. The captured test process was terminated and reaped after inspection; no persistent development server is needed for the release binary.

Management and terminal remain planned per-host opt-ins enforced in Rust, as specified in [ADR 0001](decisions/0001-architecture.md). There is no unrestricted shell plugin, host discovery or remote mutation endpoint in the scaffold.

Prompt 004 adds validated opaque IDs, scoped requests/responses, structured errors, generated TypeScript/JSON fixtures and stale-result guards. Storage and SSH transport are still unavailable; the new commands report this honestly. See [IPC contract](ipc-contract.md) and [004 evidence](../codex/tracking/evidence/004.md).

Prompt 005 adds private, bounded, versioned settings with atomic writes, backup/recovery, schema migration, file locking and revision checks. Native theme saving and recovery notices are wired through narrow IPC commands. Linux native recovery and theme persistence across restart passed; host editing and connections are still later work. See [settings](settings.md) and [005 evidence](../codex/tracking/evidence/005.md).

Prompt 006 adds actual native OpenSSH/agent diagnostics and a validated executable override. Linux native diagnostics passed with PATH=/nonexistent; the view showed real OpenSSH version, linux/x86_64, app 0.1.0 and absent-agent guidance. Probes have bounded output/time and one concurrent slot. An inherited-descriptor storage-lock race was fixed and regression-tested. See [native dependencies](native-dependencies.md) and [006 evidence](../codex/tracking/evidence/006.md).

Next prompt: **007 — SSH config alias discovery**. Do not treat this shell as the remote-Docker MVP; checkpoints [018/030/038/046/060](checkpoints.md) still require their actual feature, lab and platform acceptance work.
