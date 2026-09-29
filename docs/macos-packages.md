# macOS app and DMG packages

ContainerDesk targets macOS 15.0 or later on two explicit native architectures: Apple Silicon `aarch64-apple-darwin` and Intel `x86_64-apple-darwin`. There is no universal or Rosetta compatibility claim. Native GitHub jobs assert the machine and compiler architecture and use the committed Node/npm/Rust/Tauri versions. Runtime claims require the corresponding executed package test, recorded in [054 evidence](../codex/tracking/evidence/054.md) and the final [058 matrix](platform-matrix.md).

After the standard verification gate, `scripts/package_ci.py` creates a versioned `.app.tar.gz` and `.dmg` for the current native architecture. Metadata schema 2 lists both hashes/lengths and the source revision; SHA256SUMS also covers metadata. It is emitted before runtime checks, so its `runtimeAcceptance: not_run` is the packaging-stage snapshot. The separate native acceptance report binds its results to the exact DMG and installed-executable hashes; it is retained with the CI reports. Neither report grants public release approval. Bundle identity is `dev.containerdesk.app`, minimum system version is `15.0`, and Tauri converts the committed ContainerDesk icon to ICNS. The app uses ordinary user SSH files and the system `/usr/bin/ssh`; no shell-profile startup, embedded SSH key or remote agent is required.

The DMG presents ContainerDesk and an Applications link, with a configured 660×400 window and icon positions. [Tauri documents a CI limitation](https://v2.tauri.app/distribute/dmg/) affecting icon position/size styling; the actual DMG structure is checked independently and pixel-perfect Finder layout is not assumed. The usual installation copies the app into Applications, then opens it with Finder. The acceptance test launches a copy made from the actual mounted DMG.

## Native package test

`python3 tests/lab/macos_package_smoke.py --packages dist-artifacts --artifacts test-results/macos-package` is opt-in locally and runs after macOS CI packaging. It requires a fresh profile and an existing OS accessibility grant. It never changes OS access controls or links its external Swift client into the application.

The test verifies hashes, mounts the DMG read-only, checks the Applications link, copies the actual app into an owned temporary Applications directory, unmounts the image, checks Mach-O architecture/permissions/Info.plist/icon/signature state, and opens the copied app using Finder through AppleScript. Its native accessibility tree must expose a real window and application controls. Startup must create `~/Library/Application Support/dev.containerdesk.app` with private preferences/lock permissions.

A second LaunchServices launch uses a minimal PATH and an empty temporary `/usr/bin/ssh-agent`. Native Settings → Run diagnostics must show available `/usr/bin/ssh` and reachable inherited agent socket. This checks agent access, not loaded-key authentication or a remote Docker connection. The test requests normal app termination, reaps its agent, removes only its previously absent owned profile and discards the copied application/mount. A third ordinary launch exercises the real native support Save panel, reads back the saved JSON, requires mode 0600 and removes only its newly created export. Screenshots and per-launch results are kept with the CI reports. Absence of native permissions is a failed/pending gate, not grounds to bypass them.

## Distribution status

These are local unsigned or linker-ad-hoc-signed development packages. No Developer ID certificate, Apple account, notarization ticket or release token is supplied. The packager refuses signing-related environment values. Local Finder execution of an unquarantined CI-created app does not establish Gatekeeper acceptance for an internet download. Optional [signing/notarization readiness](signing.md) is configuration-ready without credentials in 055; do not prescribe Gatekeeper/security-control bypasses as installation requirements.

No public publishing workflow is included, by the user's explicit scope decision. A CI artifact is not an approved public installer. Flatpak/Snap/store sandboxing and Mac App Store entitlement design are outside this first package set.

## Verified execution

[Run 36574232761](https://github.com/alekpopovic/container-desk/actions/runs/36574232761) passed both native Mac package tests on macOS 15.7.9, after all standard checks. The downloaded app/DMG/report hashes were independently verified. Apple Silicon was linker-ad-hoc signed; Intel was unsigned. Finder launch, app-data permissions, system SSH readiness, empty-agent socket access and normal cleanup passed on each architecture. This is the recorded local development-package result; full remote Docker acceptance remains 058.

The final [run 36594720178](https://github.com/alekpopovic/container-desk/actions/runs/36594720178), clean source `ce33d6cd9bba5823642749a26d3e6d367a9bab73`, passed ordinary package launch/support export and the separate full native SSH/Docker/PTY suite on both architectures. [ARM package result](verification/058/arm64-package.json), [Intel package result](verification/058/intel-package.json), [complete matrix](platform-matrix.md). Source/report/archive/installed executable hashes were independently verified. Developer ID, notarization and quarantined-download Gatekeeper acceptance remain unverified.
