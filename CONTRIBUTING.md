# Contributing to ContainerDesk

Read [AGENTS.md](AGENTS.md), the [architecture contract](codex/docs/ARCHITECTURE.md) and [current status](docs/project-status.md) before changing code. Preserve unrelated work. The immutable original prompt pack is in `codex/`; implementation decisions and approved scope changes belong in application docs/evidence. During a prompt run use `track.py next/start/done/block` and real verification evidence; never manually bypass dependencies.

## Setup and checks

Use the pinned Node 24.21.0/npm 11.19.0 and Rust/Cargo 1.98.1, Python 3.10+ for the tracker/verifier, and the native prerequisites in [toolchains](docs/toolchains.md). Use Python 3.12+ for optional native CI/package/signing helpers. Install the current platform's system dependencies, then:

```sh
npm run verify:install
npm run desktop:dev
```

Before submitting code:

```sh
python3 scripts/version.py
python3 -m unittest discover -s scripts/tests -v
npm run verify
```

`verify:install` only installs locked dependencies; it is not validation. `verify` checks exact toolchain and application version alignment, then runs formatting, types, lint, unit/browser tests, both Rust feature lint modes, Rust tests, an ordinary native release build and tracker checks. It fails fast and writes `test-results/verification.json`. Do not claim skipped opt-in native tests passed. Keep both lockfiles; use [versioning](docs/updates.md) for a controlled application version change.

The browser suite uses explicit fixtures and cannot prove Rust/SSH/WebKit behavior. Real native UI automation uses an explicit test feature/artifact; the default distribution executable ignores automation/inspector activation variables. Build the separate artifact with `npm run desktop:build:automation`; its helper restores an ordinary default-feature release last. Never package the test artifact. See [native testing](docs/native-testing.md).

## Safe integration environment

The preferred native backend lab is [the dedicated disposable VM](docs/integration-lab.md), which does not touch a host Docker socket or personal SSH configuration. Use the supplied pinned image and explicit tool paths. The 057 quick-start extension exercises a new app profile with generated agent keys, direct/ProxyJump aliases and actual UI controls:

```sh
python3 tests/lab/integration.py \
  --qemu-root /path/to/extracted-qemu \
  --image /path/to/pinned-alpine.qcow2 \
  --quick-start-tools /path/to/native-test-tools \
  --artifacts /path/to/owned-results
```

Build the separate native automation artifact first. The GUI test requires the existing tauri-driver/WebKitWebDriver/Xvfb tool layout. Real guest Docker state and trust-file checks remain independent of UI assertions. Older `logs.py`/checkpoint helpers explicitly address a host Docker socket; do not run them against an unreviewed/shared daemon. They require an independently owned isolated lab. Never guess a production alias, mount a host Docker socket into a container, enable privileged Docker-in-Docker on a shared host or relax SSH trust to get a green test.

Only created fixture resources may be changed. Record source/binary/package hashes, OS/architecture/tool versions, actual passed checks, skipped cases and cleanup. Delete only exact app-owned resources. Native package and platform gates require execution on that OS; cross compilation and browser screenshots are not sufficient. Do not introduce signing credentials, paid infrastructure, public publishers or placeholder updater endpoints. Current CI has read-only contents permission and no publishing workflow, by user decision.

## Review boundaries

Keep native OpenSSH invocation and remote argument validation/quoting centralized. Rust must enforce read-only/mutation/terminal policies and session/daemon identity. Do not replay mutations/input after a lost response. Bound queues, captures, retention and process lifetimes; test cancellation/reaping and stale-reply handling when those paths change. Treat logs/labels as untrusted text and mask inspect environment values before IPC. Keep secrets, raw logs and terminal contents out of persistence/default diagnostics.

Changes to Rust IPC DTOs require regenerated reviewed TypeScript contracts through `npm run ipc:generate`, followed by matching tests. Update user-facing docs when labels or behavior change. Prefer focused failure/boundary tests to implementation snapshots. PRs should describe the trigger, resulting behavior, real validation and remaining limits. Never present fixture evidence as native acceptance or a signed build as an approved public release.
