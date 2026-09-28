# ContainerDesk project status

Audit date: 2026-09-28. Prompt 001 establishes the repository and implementation contract. The application is **not runnable yet**; prompt 002 supplies the native shell, frontend, version IPC and build/lint commands.

## Repository audit

- Starting commit: `154ab1027910e5ac092399c382b8d13a30423fee`, branch `main`, upstream `origin/main`.
- Initial `git status --short` was empty. There were no application sources, package manifests, lockfiles or existing implementation evidence. The repository contained the 60-prompt pack, docs, Python tracker/tests and project instructions.
- Existing `AGENTS.md` and `CODEX_START.md` are retained unchanged, including the user-authorized per-prompt commit/push rule. No unrelated changes were found or overwritten. The ephemeral tracker lock was already ignored.
- Historical prompts, manifest hashes, architecture contract and pack checksums are retained. Decisions are recorded in [ADR 0001](decisions/0001-architecture.md), not edits to historical tasks.
- Added exact npm/Cargo manifests and lockfiles plus toolchain pins. The Rust target contains only a documentation comment to allow Cargo dependency resolution. It implements no application feature; the same root and `src-tauri/` are extended by 002.
- Fixed the existing tracker renderer's trailing space after history events with empty notes, found by `git diff --check`. State changes still go exclusively through the tracker.

## Observed local environment

| Item | Observed during audit | Meaning |
|---|---|---|
| OS | Ubuntu 26.04.1 LTS, Linux 7.0.0-34-generic, x86_64, glibc 2.43 | Actual development host; target Linux baseline is Ubuntu 24.04 |
| Initial Rust | `/usr/bin/rustc` 1.93.1 | Cargo/rustup initially absent from this session's PATH |
| User-provided Rust installation | `~/.cargo/bin`, rustup stable Rust/Cargo 1.98.1 | Discovered after the user installed Cargo; session PATH needed a command-local prefix |
| Pinned Rust | Exact toolchain 1.98.1 installed with rustfmt/clippy, default unchanged | Used for lock generation, metadata and formatting |
| Existing Node/npm | Node 26.4.0, npm 11.17.0 | Left unchanged globally |
| Selected Node/npm | Node 24.21.0 LTS, npm 11.19.0 | Official Linux archive verified by published SHA-256 and extracted under `/tmp` for this audit |
| Other package managers | pnpm 11.20.0 present, yarn absent | npm is the project's sole JavaScript package manager |
| Python | 3.14.4 | Ran the tracker and its existing 14 tests |
| C compiler | GCC 15.2.0; build-essential 12.12ubuntu2.26.04.2 | Present; not proof of a native app build |
| OpenSSH | OpenSSH_10.2p1 Ubuntu-2ubuntu3.6 | Version inspected only; no user's SSH alias/config/keys were executed or read |
| Local Docker | CLI 29.8.1 | Version inspected only; daemon and lab availability untested |
| Display | DISPLAY and WAYLAND_DISPLAY set | A native window was not launched; display variables alone do not prove launchability |
| Missing native development packages | pkg-config, libgtk-3-dev, libwebkit2gtk-4.1-dev, libayatana-appindicator3-dev, librsvg2-dev, libxdo-dev | Confirmed absent by dpkg-query; prerequisites for the next native build |
| Present SSL development package | libssl-dev 3.5.5-1ubuntu3.5 | Confirmed installed |
| macOS | No macOS runner or Xcode execution available | Apple Silicon/Intel runtime, packaging and signing remain unverified |

See [toolchains](toolchains.md) for version pins, official sources and setup commands. Temporary Node binaries and the npm cache are audit conveniences, not portable project dependencies; future sessions must select/install the pinned Node. Ensure rustup's bin directory is on PATH before invoking Cargo. The execution sandbox helper failed with `mountinfo path is not absolute`; actual checks ran through reviewed commands outside that broken helper. This is an agent-environment limitation, not an application result.

## Verification status

| Check | Result |
|---|---|
| Exact stable direct versions and compatible declared engine/peer ranges | Selected and checked against official metadata |
| npm lock generation and `npm ci --ignore-scripts` | Passed with Node 24.21.0/npm 11.19.0; 44 local packages installed |
| `npm ls --depth=0` | Passed; all 11 direct packages resolved at exact pins |
| Cargo lock generation | Passed; 419 dependencies plus the application package locked |
| Cargo `metadata --locked --offline --no-deps` and `fmt --check` | Passed; manifest/format checks only |
| Pin/lock consistency | Passed for npm, Cargo and toolchain selections; no direct prereleases |
| Existing Python tracker tests | 14 passed after the renderer fix |
| Tracker integrity | All 60 prompt hashes/dependencies/statuses validated |
| Frontend compilation / native compilation / version IPC / GUI | NOT RUN; runnable scaffold belongs to 002 |
| Live direct/ProxyJump SSH, Docker operations and terminal | NOT RUN; no disposable lab selected or contacted |
| Linux baseline and macOS packages/runtime/signing | NOT RUN |

## Delivery gates and next step

The manual execution procedures, required evidence and blocking criteria for 018, 030, 038, 046 and 060 are defined in [checkpoints](checkpoints.md). Management and terminal are separately opt-in per host and enforced in Rust; all new host sessions begin read-only. Scope and resource/security boundaries follow ADR 0001.

Prompt 001's audit acceptance does not require building a native app that does not yet exist, so missing native libraries/macOS hardware are recorded downstream limitations. Next: **002 — Tauri React TypeScript scaffold**. Reuse these pinned manifests/locks, add the actual frontend and native entry points, install native prerequisites as needed, and execute its typecheck/build/IPC/native checks. Stop after 001; do not start 002 automatically.

Detailed audit evidence: [001](../codex/tracking/evidence/001.md). The tracker remains the authoritative current task status as later prompts progress.
