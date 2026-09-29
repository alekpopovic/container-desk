# Native desktop testing

The Linux suite uses a custom Python W3C WebDriver client with official `tauri-driver` and WebKitWebDriver. This is a [documented Tauri integration path](https://v2.tauri.app/develop/tests/webdriver/) for existing non-Node harnesses. It drives real WebKitGTK windows and the real IPC/backend; it does not install a command-mocking plugin or intercept `invoke`. Existing native SSH/log/terminal journeys are reused rather than rewritten into another test framework.

## Production and automation builds

`native-automation` is an explicit Cargo feature, off by default. At the executable's single-threaded entry point, ordinary builds remove Tauri/WebKit automation and remote inspector activation variables before Tauri, GTK or Tokio initialization. Pinned `tauri-runtime-wry` 2.12.0 otherwise honors `TAURI_WEBVIEW_AUTOMATION=true` even in a release build. This build gate therefore matters independently of optimization/debug settings.

No WDIO plugin, embedded HTTP server, test-control command, frontend IPC mock or testing permission is registered in either current build. Normal dependency-tree inspection excludes development-only Tauri mock-runtime and ts-rs dependencies. Production capabilities and CSP remain the same in the automation build. Its sole difference is allowing the external native driver's explicit environment activation.

```sh
npm run desktop:build:automation
python3 tests/lab/desktop.py \
  --tools-dir /tmp/containerdesk-025-tools \
  --artifacts docs/verification/050-native
```

The build helper writes the instrumented executable to `src-tauri/target/native-automation/containerdesk`, records both SHA-256 hashes, removes the intermediate instrumented executable from the shipping path, and builds the default production executable last at `src-tauri/target/release/containerdesk`. All output is ignored. Distribute only a normal build created with `npm run desktop:build`; do not package a `native-automation` build or use `--all-features` for packaging. Native GUI helpers require the explicit automation artifact and never silently fall back to the production binary. `CONTAINERDESK_NATIVE_BINARY` may specify another absolute test artifact path.

The Linux tools directory contains extracted native tools at `bin/tauri-driver`, `webkit/usr/bin/WebKitWebDriver`, `xvfb/usr/bin/Xvfb`, `xdotool/usr/bin/xdotool`, and `openssh/usr/sbin/sshd` plus sshd helpers. Current verification uses tauri-driver 2.0.4 and WebKitGTK/WebKitWebDriver 2.52.6. ImageMagick `import`, native OpenSSH, Docker and the compiler toolchain are development requirements for these particular labs, not application runtime requirements. Each journey gets owned Xvfb/DBus/XDG directories and bounded subprocess cleanup.

## What the suite proves

| Journey | Actual evidence |
|---|---|
| Production refusal | Real ordinary-binary window while all five automation/inspector variables are supplied; WebKit child environments omit them; no application TCP listener; no WebDriver session within the explicit 20-second negative-test deadline; native window closes normally |
| Host/connection errors | Native save/select/connect/disconnect UI; actual direct and ProxyJump daemon identity; unknown/changed host key and absent jump authentication; cancellation during a live probe and explicit reconnect |
| Interactive resources | Real discovery, logs followed and cancelled with SSH PID reaping, exact confirmed actions, Compose, bounded PTY response/close and final connection cleanup |
| IPC/security | Actual malformed aliases/IDs and unauthorized operations rejected in Rust; inert hostile text; masked report; native navigation/CSP guards |

The desktop connection-error lab uses the older dedicated empty Engine in an owned container. The interaction lab uses exact-ID-gated, explicitly disposable workloads on the local Docker daemon; it never mounts that socket into a fixture. The stronger private-target network-isolation and dedicated-VM workload proof is the separate [049 integration lab](integration-lab.md). Keep these claims distinct.

`npm run test:ui` is the separate Playwright **browser fixture** suite, with mocked IPC and no Rust/WebKit process. `cargo test` normally ignores opt-in native lab tests. Neither ordinary suite is a substitute for the native command above.

## macOS native execution

The installed macOS application is exercised through an external Swift accessibility client under the runner's existing OS grant. No embedded WebDriver/plugin or application test-control endpoint is installed. The helper is compiled separately and never enters the ordinary app bundle. Native Finder/DMG launch, app-data permissions, minimal-environment OpenSSH diagnostics and owned-agent reachability passed on actual macOS 15.7.9 ARM and Intel runners in [054](../codex/tracking/evidence/054.md). The native support Save dialog also passed on both architectures in [058](platform-matrix.md), with actual file readback and 0600 permissions.

The [058 matrix](platform-matrix.md) links separate actual native Rust/OpenSSH/PTY execution against a dedicated Linux Docker VM for each client architecture. Guest CPU emulation does not emulate the Mac client or establish another client platform. Browser fixture checks, native backend checks and ordinary-package GUI checks remain distinct evidence. Missing OS accessibility or screen-capture permission is a failed/pending gate; the harness never changes those controls.

Linux WebDriver journeys still require the explicit automation artifact and separate production-refusal check. Mac ordinary-package journeys use the shipping artifact built with default features. Full Mac resource-view keyboard/VoiceOver, physical sleep/wake, other OS versions and quarantined-download Gatekeeper behavior are not inferred from these bounded checks.
