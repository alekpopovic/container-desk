# Native CI and artifact boundaries

The [checks workflow](../.github/workflows/ci.yml) runs on pull requests, pushes to main and explicit manual dispatch. It creates CI artifacts; it does not publish a GitHub Release, sign or notarize anything. Standard public-repository runners are used, with no paid service or self-hosted runner.

| Runner label | Native architecture | Rust target | Initial package |
|---|---|---|---|
| `ubuntu-24.04` | x86_64 | x86_64-unknown-linux-gnu | deb |
| `macos-15` | Apple Silicon / arm64 | aarch64-apple-darwin | app archive |
| `macos-15-intel` | Intel / x86_64 | x86_64-apple-darwin | app archive |

The job asserts both `uname -m` and the Rust compiler host triple. These are native builds, not cross-compiles. Native GUI/runtime acceptance is a separate gate. AppImage and DMG expansion and installer smoke tests belong to the subsequent packaging prompts; CI currently builds the formats shown above. macOS minimum is 15.0. Labels follow the [official runner mapping](https://docs.github.com/en/actions/reference/runners/github-hosted-runners); they do not pin an immutable OS image, so actual OS/tool metadata is recorded.

Each isolated job selects the exact Node/npm and Rust toolchains, installs locked dependencies, runs packaging failure-boundary tests and all [standard verification](verification-command.md), then packages the ordinary executable. The verifier records its SHA-256 and Git revision; packaging rejects a different executable/revision or incomplete/failed checks. Build/registry caches are not shared across jobs or trust boundaries. No `--all-features` or native-automation binary enters packaging.

`scripts/package_ci.py` uses the pinned local Tauri CLI's `bundle --ci --no-sign`, refuses signing environment variables and existing nonempty bundle/artifact output, and emits versioned package names, SHA256SUMS and JSON metadata. App archives preserve Unix permissions and symlinks. Metadata includes source revision/dirty state, lock hashes, package and binary hashes, native target/toolchain identity, run ID/attempt and explicit unverified runtime/signing/release fields. A green build is not installation or Gatekeeper proof. Local invocation supports a selected `CARGO_TARGET_DIR` and `--report`/`--output`; run verification first with the same directory. No cleanup is implicit.

## Trust and publication

The only token permission in the build workflow is `contents: read`; unspecified scopes are none. Checkout does not persist credentials. There is no secrets reference, release token, signing key, environment attachment, OIDC grant, `pull_request_target`, `workflow_run`, downloaded executable artifact, or publication step. Fork PR code runs only in its read-only build job and can upload its own explicitly unapproved CI artifacts. Neither PR titles nor branch names are interpolated into shell code. Package selection uses fixed platform mappings.

Public publication is reserved for an explicit trusted workflow action after the final release gate. The proposed separate workflow requires manual dispatch from main, all 60 tracker steps completed with valid evidence, a reviewed approval manifest specifying exact artifact bytes and source commit, a successful same-repository main-push build, an existing matching tag, and an explicit version confirmation. It would create a new release without replacing an existing release or running downloaded code. The workflow and write grant are not currently installed: automatic approval review rejected adding that persistent public-release capability; user authorization is pending. No PR can reach it while absent. Do not weaken this boundary with a tag/push publication trigger or privileged PR event. Signing integration and final release approval remain later gates.

Reports and synthetic fixture failure screenshots are retained with `always()` for 14 days, including failed verification; GitHub also retains job logs according to repository policy. Packages are uploaded only after all required steps succeed. There are no production aliases/credentials/remote raw logs in these jobs. Retained packages are development artifacts, not approved public installers.

## Reviewed action pins (2026-09-29)

| Action | Release and full commit | Review |
|---|---|---|
| checkout | [v7.0.1](https://github.com/actions/checkout/releases/tag/v7.0.1), `3d3c42e5aac5ba805825da76410c181273ba90b1` | Manifest/defaults and PR safety change reviewed; Node 24, unsafe PR checkout remains false; persist-credentials explicitly false. |
| setup-node | [v7.0.0](https://github.com/actions/setup-node/releases/tag/v7.0.0), `820762786026740c76f36085b0efc47a31fe5020` | Manifest/cache defaults reviewed; Node 24; automatic package cache explicitly false; no registry auth configuration. |
| upload-artifact | [v7.0.1](https://github.com/actions/upload-artifact/releases/tag/v7.0.1), `043fb46d1a93c77aae656e7c1c64a875d1fc6a0a` | Manifest/upload defaults reviewed; Node 24; finite retention, no hidden files, no overwrite. |

Pins were resolved against official GitHub tags; this is a bounded manifest/behavior review, not a claim that every bundled dependency was audited. Update full SHAs deliberately with release-note and permission review. Rust uses the committed rustup toolchain on hosted runners; there is no additional toolchain action. [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) inform Linux system package installation.

Local syntax validation uses [actionlint v1.7.12](https://github.com/rhysd/actionlint/releases/tag/v1.7.12), official Linux binary SHA-256 `8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8`. Validate with `actionlint .github/workflows/*.yml`; a syntax pass alone does not prove hosted execution. Actual runs/results are recorded in prompt 052 evidence.

The first real macOS job exposed Linux-only keyboard modifiers in a browser test and an unpaused fake clock whose real elapsed time changed event read counts. Tests now use Playwright `ControlOrMeta` and pause the event clock before mounting the fixture, preserving the original assertions. See [keyboard API](https://playwright.dev/docs/api/class-keyboard) and [clock semantics](https://playwright.dev/docs/clock). These are browser test portability fixes, not native GUI evidence.
