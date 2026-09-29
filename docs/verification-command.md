# Contributor verification

Select the pinned Node/npm from `.nvmrc` and `package.json`, and Rust from `rust-toolchain.toml`. The runner refuses mismatched Node, npm, rustc or Cargo versions. Python 3.10+ is required for contributor scripts, not the application. It runs every command directly with argument arrays and a bounded deadline, stops on the first failure, returns nonzero, and records later checks as `not_run`.

## Install separately

```sh
npm run verify:install
```

This executes `npm ci`, installs Chromium matching the lockfile's Playwright version, then runs `cargo fetch --locked`. It does not run verification or install system packages. `npm ci` replaces generated `node_modules`; neither command rewrites lockfiles. There is no automatic `cargo clean`, source deletion or cache reset. Network access is needed if dependencies/browser binaries are not already cached.

## Standard checks

```sh
npm run verify
```

Required checks are: Biome formatting, Rust formatting, TypeScript, frontend lint, Node IPC tests, all configured Playwright browser fixtures, production frontend build, default and automation-feature Clippy, Rust tests, native production executable build, tracker tests and tracker validation. Individual commands remain available in `package.json` and [development commands](development.md). Formatting verification never applies fixes.

The default report is `test-results/verification.json` (ignored), with commands, statuses, exit codes, durations and toolchain/platform identity. Override it with `npm run verify -- --report /absolute/path/result.json`. Console output clearly reports skipped native SSH/Docker/desktop integration, other platform/architecture execution, installers and signing. A standard pass does not prove any skipped gate. Browser fixtures use mocked IPC and have no Rust/WebKit window. Their dedicated `browser-test` Vite mode disables file watching/HMR, so development reloads cannot reset a scenario mid-assertion; restart the test command after changing its source. Normal development keeps HMR.

To require compilation into a fresh output directory without deleting an existing build:

```sh
CARGO_TARGET_DIR=/absolute/path/to/a/new/owned-directory CARGO_BUILD_JOBS=2 npm run verify
```

Use an actually empty directory when recording a clean-build claim. Dependency download caches may be reused; compiler output is separate. The runner does not delete the directory afterward. Lower `CARGO_BUILD_JOBS` on memory-constrained machines. The standard native build respects Cargo's selected output directory and creates an unbundled ordinary executable, not a verified installer.

## Opt-in native desktop verification

On a prepared Linux native runner, include the real-window suite:

```sh
npm run verify -- \
  --native-tools /tmp/containerdesk-025-tools \
  --native-artifacts /absolute/path/to/owned-native-results
```

This adds the explicit automation build and [050 desktop suite](native-testing.md) after standard checks. The existing desktop harness uses repository-local binary paths, so the runner rejects this option when `CARGO_TARGET_DIR` is set; unset it for that run. An explicitly requested unsupported native platform or failed native check is an error, never silently skipped. System tools and the disposable Docker fixtures must be available. Do not supply production aliases/resources.

The stronger dedicated-VM direct/private-bastion transport suite is independently opt-in; use the exact command and verified image in [integration lab](integration-lab.md). Both native suites require their own actual execution evidence. macOS's embedded automation route is evaluated in [native testing](native-testing.md), but the current Linux harness is not a WKWebView substitute.

## Native system prerequisites

The only desktop distribution executed so far is Ubuntu **26.04.1 x86_64**. Ubuntu 24.04 LTS remains the planned packaging/runtime baseline and needs its own native gate. Following the current [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/), prepare an Ubuntu development machine with:

```sh
sudo apt update
sudo apt install build-essential pkg-config curl wget file libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

This setup command is documentation, not an action performed by `verify`. Prompt 051 used existing system packages, recorded in [051/system-packages.txt](verification/051/system-packages.txt). Main observed versions: WebKitGTK 2.52.6, GTK 3.24.52, OpenSSL 3.5.5, librsvg 2.61.3. Chromium may need additional distro runtime libraries; use [Playwright's official installation guidance](https://playwright.dev/docs/intro#system-requirements) for the installed release. Native desktop labs additionally use external driver/Xvfb/xdotool/sshd tools described in their own docs, and Docker only for disposable development fixtures.

For macOS desktop development, Tauri documents Xcode Command Line Tools (`xcode-select --install`); full Xcode is needed for additional Apple targets. Use a native Apple Silicon or Intel machine meeting this project's macOS 15+ baseline, the pinned Node/Rust versions, and record `xcode-select -p`, `xcrun --show-sdk-version`, `uname -m`, and the OS version. Command-line tools/frameworks and native OpenSSH must be available. No macOS prerequisite installation or execution was performed here. Public signing/notarization and Apple credentials are separate release gates.

The standard verification does not change user SSH configuration, known_hosts, keys, production server state or global package installations. Native opt-in tests clean only their explicitly owned lab resources.
