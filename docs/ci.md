# Native CI and artifact boundaries

The [checks workflow](../.github/workflows/ci.yml) runs on pull requests, pushes to main and explicit manual dispatch. It creates CI artifacts; it does not publish a GitHub Release, sign or notarize anything. Standard public-repository runners are used, with no paid service or self-hosted runner.

| Runner label | Native architecture | Rust target | Initial package |
|---|---|---|---|
| `ubuntu-24.04` | x86_64 | x86_64-unknown-linux-gnu | deb + AppImage |
| `macos-15` | Apple Silicon / arm64 | aarch64-apple-darwin | app archive + DMG |
| `macos-15-intel` | Intel / x86_64 | x86_64-apple-darwin | app archive + DMG |

The job asserts both `uname -m` and the Rust compiler host triple. These are native builds, not cross-compiles. Native GUI/runtime acceptance is a separate gate. Linux deb/AppImage packaging and the clean Ubuntu deb smoke test are recorded in [053](../codex/tracking/evidence/053.md); native Mac app/DMG launch and SSH diagnostics are recorded in [054](../codex/tracking/evidence/054.md); extended native SSH/Docker acceptance is executed by the 058 suite and recorded separately in the [platform matrix](platform-matrix.md). CI builds the formats shown above. macOS minimum is 15.0. Labels follow the [official runner mapping](https://docs.github.com/en/actions/reference/runners/github-hosted-runners); they do not pin an immutable OS image, so actual OS/tool metadata is recorded.

Each isolated job selects the exact Node/npm and Rust toolchains, installs locked dependencies, runs packaging failure-boundary tests and all [standard verification](verification-command.md), then packages the ordinary executable. The verifier records its SHA-256 and Git revision; packaging rejects a different executable/revision or incomplete/failed checks. Build/registry caches are not shared across jobs or trust boundaries. No `--all-features` or native-automation binary enters packaging.

`scripts/package_ci.py` uses the pinned local Tauri CLI's `bundle --ci --no-sign`, refuses signing environment variables and existing nonempty bundle/artifact output, and emits versioned package names, SHA256SUMS and JSON metadata. App archives preserve Unix permissions and symlinks. Metadata includes source revision/dirty state, lock hashes, package and binary hashes, native target/toolchain identity, run ID/attempt and explicit unverified runtime/signing/release fields. A green build is not installation or Gatekeeper proof. Local invocation supports a selected `CARGO_TARGET_DIR` and `--report`/`--output`; run verification first with the same directory. No cleanup is implicit.

## Trust and publication

The only explicit token permission in the build workflow is `contents: read`; other configurable scopes are none. The actual first main-push job reported Contents read and implicit Metadata read. GitHub documents [fork pull-request token/secret restrictions](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#workflows-in-forked-repositories). Checkout does not persist credentials. There is no secrets reference, release token, signing key, environment attachment, OIDC grant, `pull_request_target`, `workflow_run`, downloaded executable artifact, or publication step. Fork PR code runs only in its read-only build job and can upload its own explicitly unapproved CI artifacts. Neither PR titles nor branch names are interpolated into shell code. Package selection uses fixed platform mappings.

Public publishing is excluded from implementation by the user's explicit 2026-09-29 scope decision: complete the remaining prompts without the manual public-publisher workflow. No public-publisher workflow, release-creation script or `contents: write` grant is installed. The earlier automatic approval rejection is retained in [052 evidence](../codex/tracking/evidence/052.md); the scope decision resolves that blocker without adding the rejected capability. CI artifact generation remains available. Signing readiness and the final release assessment remain separate later gates; any future public distribution requires a separate decision.

Reports and synthetic fixture failure screenshots are retained with `always()` for 14 days, including failed verification; GitHub also retains job logs according to repository policy. Packages are uploaded only after all required steps succeed. After successful packaging, the independent SSH/Docker matrix still runs if the separate package GUI test fails, so both results remain visible; such a run cannot upload the unsigned package set. There are no production aliases/credentials/remote raw logs in these jobs. Retained packages are development artifacts, not approved public installers.

## Reviewed action pins (2026-09-29)

| Action | Release and full commit | Review |
|---|---|---|
| checkout | [v7.0.1](https://github.com/actions/checkout/releases/tag/v7.0.1), `3d3c42e5aac5ba805825da76410c181273ba90b1` | Manifest/defaults and PR safety change reviewed; Node 24, unsafe PR checkout remains false; persist-credentials explicitly false. |
| setup-node | [v7.0.0](https://github.com/actions/setup-node/releases/tag/v7.0.0), `820762786026740c76f36085b0efc47a31fe5020` | Manifest/cache defaults reviewed; Node 24; automatic package cache explicitly false; no registry auth configuration. |
| upload-artifact | [v7.0.1](https://github.com/actions/upload-artifact/releases/tag/v7.0.1), `043fb46d1a93c77aae656e7c1c64a875d1fc6a0a` | Manifest/upload defaults reviewed; Node 24; finite retention, no hidden files, no overwrite. |

Pins were resolved against official GitHub tags; this is a bounded manifest/behavior review, not a claim that every bundled dependency was audited. Update full SHAs deliberately with release-note and permission review. Rust uses the committed rustup toolchain on hosted runners; there is no additional toolchain action. [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) inform Linux system package installation.

Local syntax validation uses [actionlint v1.7.12](https://github.com/rhysd/actionlint/releases/tag/v1.7.12), official Linux binary SHA-256 `8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8`. Validate with `actionlint .github/workflows/*.yml`; a syntax pass alone does not prove hosted execution. Actual runs/results are recorded in prompt 052 evidence.

The first real macOS job exposed Linux-only keyboard modifiers in a browser test and an unpaused fake clock whose real elapsed time changed event read counts. Tests now use Playwright `ControlOrMeta` and pause the event clock before mounting the fixture, preserving the original assertions. See [keyboard API](https://playwright.dev/docs/api/class-keyboard) and [clock semantics](https://playwright.dev/docs/clock). These are browser test portability fixes, not native GUI evidence.

A subsequent native macOS Rust test exposed a diagnostic-agent fixture socket exceeding `sockaddr_un` path limits under macOS TMPDIR. Its exclusively created mode-0700 fixture directory now uses the short `/tmp` root; production SSH/runtime paths are unchanged. Event tests pause a full day ahead of the initial fake instant before mounting application timers, avoiding a one-second setup assumption on slow runners.

## Current verified result

[Run 36594720178](https://github.com/alekpopovic/container-desk/actions/runs/36594720178), clean source `ce33d6cd9bba5823642749a26d3e6d367a9bab73`, passed all five jobs. Each native client passed all 13 standard checks, 17 packaging/signing/version script tests and the full encrypted-agent SSH/Docker/PTY VM suite. Standard suites include 87 Node, 348 browser fixture, 152 Rust (29 separate opt-in tests ignored) and 14 tracker tests. Both Mac jobs also passed ordinary DMG/Finder/diagnostics/native support Save acceptance. The exact Linux packages subsequently passed clean Ubuntu desktop deb and AppImage/FUSE tests. [058 matrix](platform-matrix.md) separates native backend, package GUI, software-emulated guests and unverified signing/physical-platform limits. Downloaded package/metadata/report/installed-executable hashes were independently checked. Original 052 scope approval and historical CI evidence remain in [052](../codex/tracking/evidence/052.md).
