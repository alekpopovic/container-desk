# Pinned development toolchains

Selected on 2026-09-28 from official stable-release metadata. Exact direct versions live in `package.json` and `src-tauri/Cargo.toml`; transitive resolutions live in `package-lock.json` and `src-tauri/Cargo.lock`. Use npm as the sole JavaScript package manager; keep both lockfiles in Git. Do not run floating scaffolding generators over this repository.

## Versions and provenance

| Tool | Selected | Pin / reason |
|---|---|---|
| Node.js | 24.21.0 LTS (Krypton) | `.nvmrc`, exact `engines.node`; [official release index](https://nodejs.org/dist/index.json), [LTS policy](https://nodejs.org/en/about/previous-releases) |
| npm | 11.19.0 | Bundled with that Node release; `packageManager` and exact `engines.npm` in `package.json` |
| Rust / Cargo | 1.98.1 stable | `rust-toolchain.toml`; Rust edition 2024 and `rust-version` in Cargo manifest; [release](https://blog.rust-lang.org/releases/1.98.1/), [stable channel metadata](https://static.rust-lang.org/dist/channel-rust-stable.toml) |
| Rust tools | rustfmt and clippy from 1.98.1 | Pinned toolchain components; no nightly |
| Tracker | Python 3.10+ | Existing standard-library implementation; observed local version 3.14.4, not an application runtime dependency |

`.npmrc` enforces package engine requirements and exact saved versions. The initially installed Node 26.4.0/npm 11.17.0 remain unchanged globally; select the project Node before installing dependencies. pnpm 11.20.0 was found during the audit but is not used for this application.

| Dependency | Exact stable version | Source queried during 001 |
|---|---|---|
| `npm:react` | `19.3.0` | [Official registry metadata](https://registry.npmjs.org/react/latest) |
| `npm:react-dom` | `19.3.0` | [Official registry metadata](https://registry.npmjs.org/react-dom/latest) |
| `npm:typescript` | `7.0.2` | [Official registry metadata](https://registry.npmjs.org/typescript/latest) |
| `npm:vite` | `8.3.1` | [Official registry metadata](https://registry.npmjs.org/vite/latest) |
| `npm:@vitejs/plugin-react` | `6.1.1` | [Official registry metadata](https://registry.npmjs.org/@vitejs/plugin-react/latest) |
| `npm:tailwindcss` | `4.3.3` | [Official registry metadata](https://registry.npmjs.org/tailwindcss/latest) |
| `npm:@tailwindcss/vite` | `4.3.3` | [Official registry metadata](https://registry.npmjs.org/@tailwindcss/vite/latest) |
| `npm:@tauri-apps/api` | `2.12.0` | [Official registry metadata](https://registry.npmjs.org/@tauri-apps/api/latest) |
| `npm:@tauri-apps/cli` | `2.12.0` | [Official registry metadata](https://registry.npmjs.org/@tauri-apps/cli/latest) |
| `npm:@types/react` | `19.3.0` | [Official registry metadata](https://registry.npmjs.org/@types/react/latest) |
| `npm:@types/react-dom` | `19.3.0` | [Official registry metadata](https://registry.npmjs.org/@types/react-dom/latest) |
| `crate:tauri` | `2.12.0` | [Official registry metadata](https://index.crates.io/ta/ur/tauri) |
| `crate:tauri-build` | `2.7.0` | [Official registry metadata](https://index.crates.io/ta/ur/tauri-build) |
| `crate:tokio` | `1.53.1` | [Official registry metadata](https://index.crates.io/to/ki/tokio) |
| `crate:serde` | `1.0.229` | [Official registry metadata](https://index.crates.io/se/rd/serde) |
| `crate:serde_json` | `1.0.151` | [Official registry metadata](https://index.crates.io/se/rd/serde_json) |

The official crates.io HTTP API was unavailable during the audit; version metadata was successfully resolved from its canonical sparse index instead. All direct selections are stable (no prerelease suffix); Cargo selections are not yanked. Release metadata is a dated selection record, not a guarantee that no future advisories will be published.

React/React DOM and their types share 19.3.0. Vite 8.3.1 and React plugin 6.1.1 satisfy the plugin's Vite 8 peer range. The Tailwind Vite plugin 4.3.3 accepts Vite 8. Node 24.21.0 satisfies the selected Vite/plugin Node requirement (`^20.19.0 || >=22.12.0`). Tauri Rust/API/CLI use v2; tauri-build has its own release version (2.7.0). Rust 1.98.1 exceeds the direct dependencies' declared compiler minimums. Dependency resolution must still be followed by frontend compilation, native compilation and runtime checks in subsequent prompts.

Prompt 039 selects and locks `portable-pty 0.9.0` for Linux/macOS local PTY ownership; xterm dependencies follow in 040. The crate is used directly behind existing backend permissions, with no generic terminal plugin or shell launcher capability. Prompt 002 adds Biome 2.5.14 and @types/node 24.19.0, and uses the built-in Node test runner. Avoid unused plugins and capabilities. Prompt 001 initially used a documentation-only Rust target for dependency resolution. Prompt 002 extends it into the native shell; see [development commands](development.md) and its evidence for actual compilation and launch results.

## Reproduce the dependency baseline

Install Node 24.21.0 with your preferred Node version manager (for example `nvm install` and `nvm use` where nvm is already installed). Confirm `node --version` is `v24.21.0` and `npm --version` is `11.19.0`. `.nvmrc` does not by itself change PATH.

With rustup installed and its bin directory on PATH:

```sh
rustup toolchain install 1.98.1 --profile minimal --component rustfmt --component clippy
rustc --version
cargo --version
npm ci --ignore-scripts --no-audit --no-fund
cargo metadata --manifest-path src-tauri/Cargo.toml --locked --no-deps --format-version 1
python3 codex/scripts/track.py validate
```

The install above verifies the package resolution with lifecycle scripts disabled. Prompt 002 performed the normal npm install and verified the actual frontend/native build; a successful `npm ci` alone does not prove that tool binaries or the app run. Do not regenerate locks during normal installation. After deliberate manifest changes, regenerate with the pinned managers and review the diff. [Cargo's lockfile guidance](https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html) explains why application resolutions are committed.

## Native platforms

Target Linux x86_64 on Ubuntu 24.04 LTS, plus macOS 15+ on arm64 and x86_64. These are planned acceptance baselines, not platforms already verified by 001. Use native runners and record exact OS/SDK/package versions when the builds run. The audit host is Ubuntu 26.04.1 x86_64; a build here does not establish compatibility with Ubuntu 24.04. Tauri recommends building on the oldest supported Linux base because of system-library compatibility: [AppImage guidance](https://v2.tauri.app/distribute/appimage/).

For Ubuntu, the official [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) list development libraries. The following is the planned setup command, **not executed in prompt 001**:

```sh
sudo apt update
sudo apt install build-essential pkg-config curl wget file libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

On macOS, install Xcode Command Line Tools for desktop development (`xcode-select --install`) and use native arm64/Intel runners. Record the actual selected Xcode/SDK release in platform evidence; none was available or validated here. No Apple account or signing credential is required for this initial audit. Public signing/notarization is a separate later gate.

Tailwind 4's documented browser floor includes Safari 16.4, Chrome 111 and Firefox 128: [compatibility](https://tailwindcss.com/docs/compatibility). The project Mac baseline exceeds that WebKit generation. Verify the installed Linux WebKitGTK and both native WebViews at runtime; configuring a JavaScript build target cannot polyfill missing CSS features. Vite's Node requirements are documented in its [getting-started guide](https://vite.dev/guide/).

Native OpenSSH is an application runtime dependency. Node, npm, Rust, Python and local Docker/jq are development tools only. OpenSSH and OS security patches follow the supported OS updates rather than a vendored SSH binary. Release packaging, GTK/WebKit execution and macOS signing are not exercised by dependency resolution.

## Current contributor gate

Prompt 051 adds `npm run verify:install` and `npm run verify`, with exact toolchain enforcement and explicit native/platform skips. Use [contributor verification](verification-command.md) for the current full check sequence; the original 001 dependency-resolution commands above are historical.
