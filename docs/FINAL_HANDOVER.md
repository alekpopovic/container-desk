---
title: "ContainerDesk 0.1.0 — final handover"
section: "Releases & platforms"
icon: "📦"
historical: true
---

# 📦 ContainerDesk 0.1.0 — final handover

**Post-handover update:** the owner committed the previously uncommitted CI relocation as `c2464b8` and then explicitly requested publication. [v0.1.0](https://github.com/alekpopovic/container-desk/releases/tag/v0.1.0) now contains the original verified installers as an unsigned public preview; [publication receipt](releases/v0.1.0.md). No workflow was enabled or added. The 060 handover below and the local candidate document their earlier pre-publication state; original package bytes/hashes remain unchanged.

The agreed application and three native platform gates are complete. All 60 prompts are tracked with evidence; 052 omits public publishing by the user's explicit scope decision and 055 uses its explicitly permitted no-credentials signing-readiness branch. This delivery is a **local unsigned release candidate**, with no public release/tag/publisher or auto-updater.

## Package location and identity

Local candidate: `/home/popac/containerdesk/dist-artifacts/release-candidate-0.1.0-ce33d6c/` (repository-relative `dist-artifacts/release-candidate-0.1.0-ce33d6c/`, intentionally ignored build output). It contains six original packages, original per-target manifests/metadata, sanitized native evidence and review documents. Preserve this directory independently; CI artifacts have 14-day retention.

Every package was built from clean **`ce33d6cd9bba5823642749a26d3e6d367a9bab73`**, [successful CI run 36594720178](https://github.com/alekpopovic/container-desk/actions/runs/36594720178). Later documentation/test-helper/review commits are distinct from this tested artifact source and do not relabel the binaries. Review commits 058 `0b232e684923163ec52e18f09b4e84deba9c976b` and 059 `a25aa56c6a642a2357244ac8161e051ab03e6235` were pushed successfully to configured `origin/main`. The 060 commit is recorded in Git/history and final handover message; no circular self-hash is embedded here.

Paths below are relative to the candidate directory; SHA-256 values cover the original package files.

| Package | SHA-256 |
|---|---|
| `packages/x86_64-unknown-linux-gnu/containerdesk-0.1.0-x86_64-unknown-linux-gnu.deb` | `fb3deee5d2046822c899b99f278b0f500db34fb796b7a13a17625bfb469e82c9` |
| `packages/x86_64-unknown-linux-gnu/containerdesk-0.1.0-x86_64-unknown-linux-gnu.AppImage` | `c5956e3f1e9b7f9c6de759864a3c7fb6b0ba56353d526356ecc37eb309f5c0ae` |
| `packages/aarch64-apple-darwin/containerdesk-0.1.0-aarch64-apple-darwin.app.tar.gz` | `f326b7bc7469de5f1ba93030f798e400a8f3c0d84c8b30a3c92e45deb2f2208c` |
| `packages/aarch64-apple-darwin/containerdesk-0.1.0-aarch64-apple-darwin.dmg` | `ad1cc6f09653785b6a1fa647b83c4e3ba9b15be2b2c385e65bf71ef36320a454` |
| `packages/x86_64-apple-darwin/containerdesk-0.1.0-x86_64-apple-darwin.app.tar.gz` | `c86fdc2f55a65f2b282e33716144918edcedcb557da0b3856152b1b51417adaa` |
| `packages/x86_64-apple-darwin/containerdesk-0.1.0-x86_64-apple-darwin.dmg` | `85610a2061828fd6fa770aa0d25aa9f2c9db12b847aa48407410de96c05329d5` |

Before installation, from the candidate directory:

```sh
sha256sum --check SHA256SUMS
# macOS equivalent:
shasum -a 256 --check SHA256SUMS
```

`ARTIFACTS.json` records the source and all package sizes/hashes. Original packaging metadata retains `runtimeAcceptance: not_run` from the packaging step; the separate actual runtime reports in `evidence/` provide acceptance. It is not rewritten into a false packaging-time claim. [Final smoke receipt](verification/060/release-smoke.json), [release notes](RELEASE_NOTES.md), [review](release-review.md), [checklist](RELEASE_CHECKLIST.md).

## Install and run

Linux x86_64, from the candidate directory:

```sh
sudo apt install ./packages/x86_64-unknown-linux-gnu/containerdesk-0.1.0-x86_64-unknown-linux-gnu.deb
# Or use the AppImage:
chmod +x ./packages/x86_64-unknown-linux-gnu/containerdesk-0.1.0-x86_64-unknown-linux-gnu.AppImage
./packages/x86_64-unknown-linux-gnu/containerdesk-0.1.0-x86_64-unknown-linux-gnu.AppImage
```

Open the installed deb's **ContainerDesk** entry from the desktop menu. The baseline is Ubuntu 24.04 x86_64 with documented GTK/WebKit/OpenSSH runtime libraries; normal AppImage mounting also needs FUSE 2 compatibility (`libfuse2t64` on the tested baseline). [Linux details](linux-packages.md).

On macOS, select `aarch64-apple-darwin` for Apple Silicon or `x86_64-apple-darwin` for Intel, verify the manifest, open the corresponding DMG and copy ContainerDesk.app to Applications. Packaging minimum is 15.0; actual execution is 15.7.9. There is no universal/Rosetta claim. ARM is linker ad-hoc, Intel unsigned; Developer ID/notarization and normal internet-download Gatekeeper approval are unverified. Do not disable OS protection to turn this into a signed-distribution claim. [Mac details](macos-packages.md).

Runtime needs native OpenSSH and the user's existing trusted SSH configuration/agent plus remote Linux Docker Engine/CLI access. The installed app requires no local Docker/Docker Desktop, Node, Rust, Python or jq. Remote Compose CLI is needed only for Compose actions. Follow the [Serbian quick start](https://github.com/alekpopovic/container-desk/blob/main/README_SR.md) or [user guide](user-guide.md): discover aliases without execution, explicitly select/resolve/save a host, connect, then inspect read-only. Enable management/terminal separately and confirm exact targets when needed. Establish host trust and load encrypted keys independently in the user's own environment. No production server was used during development verification.

## Build and maintain

Use Node **24.21.0**, npm **11.19.0**, Rust/Cargo **1.98.1**, Python 3.10+ for the tracker/verifier (3.12+ for optional package/native helpers), and the documented native development libraries. Run from the repository root:

```sh
npm run verify:install
npm run desktop:dev
# Required checks, including the ordinary production executable:
npm run verify
# Ordinary executable only when full verification is not needed:
npm run desktop:build
./src-tauri/target/release/containerdesk
python3 codex/scripts/track.py validate
```

`desktop:dev` runs a native Tauri window and loopback Vite; Ctrl-C stops development. `npm run dev`/browser preview alone has no native Rust bridge. `desktop:build` creates the ordinary host-architecture executable without installers. On a supported native packaging host, after successful `npm run verify`, `python3 scripts/package_ci.py` creates checked unsigned packages in a fresh `dist-artifacts/` output; use `--output` for a new empty directory if the default already contains this candidate. Do not point packaging cleanup at the retained candidate. [Development commands](development.md), [verification](verification-command.md), [packaging/CI contract](ci.md).

Use the [disposable VM lab](integration-lab.md) for native SSH/Docker checks; never guess a production alias or run old host-Docker labs against unreviewed resources. Native GUI tests and browser fixtures are distinct. Ordinary builds disable test automation; the explicit automation build is solely for Linux testing. Keep lockfiles and original prompt hashes intact. Review actual failing scope and rerun the relevant gate after app/dependency/package changes.

## Architecture and verified behavior

One Tauri v2 app uses React/TypeScript/Vite/Tailwind and Rust/Tokio. Rust owns typed IPC, validation, host/daemon/session generations, operation permissions, one-use confirmations, bounded jobs/queues and native child cleanup. A validated local OpenSSH executable is spawned with argument arrays; centralized POSIX quoting protects the remote shell. The selected user SSH configuration remains trusted executable configuration; discovery and selected resolution are deliberately separate. There is no custom remote agent or exposed Docker TCP listener requirement, generic script API, SaaS backend or Kubernetes layer.

One host is active at a time. New/recovered sessions begin read-only. Verified features include container list/inspect with default masking, logs/follow/export, stats/events/stale recovery; read-only images/volumes/networks; explicit lifecycle/batches/stopped-only removal without force/volumes; existing verified Compose start/stop/restart; separately authorized non-root PTY with resize/Ctrl-C; private versioned local settings, manual backup/rollback and reviewed support Save. Dispatched mutations and terminal input are never automatically replayed; a lost response can remain Unknown pending a fresh read. [Resource bounds](resource-limits.md), [operation policy](operation-policy.md), [security review](security-review.md), [manual updates](updates.md).

## Native verification and limits

| Platform | Actual evidence |
|---|---|
| Ubuntu 24.04.5 x86_64; additional Ubuntu 26.04.1 | Native host backend/OpenSSH/PTY, all eight direct/private Docker groups; ordinary deb/AppImage GUI with exact binaries bound to earlier KVM evidence and final fresh TCG guest retests |
| macOS 15.7.9 ARM | Native Finder/DMG/system SSH/support Save 0600 and all eight real backend/OpenSSH/PTY groups |
| macOS 15.7.9 Intel | Same native package and backend acceptance on actual Intel host |

Each native client verified encrypted-key agent access, strict unknown/changed trust, inventory/masked inspect/logs/stats, exact management and Compose actions, PTY resize/Ctrl-C, real network interruption, new read-only scope, no replay/stale input, unrelated SSH master survival and owned cleanup. Separate disposable remote guests used Alpine 3.22.4, Docker 28.3.3, Compose 2.36.2. Full versions, executable hashes, exact event oracles and screenshots: [058 matrix](platform-matrix.md). Guest emulation is attributed explicitly; browser mocks and cross-compiles do not count as native acceptance.

All three native jobs passed 13 standard checks. Final local 059 verification also passed all 13: 87 Node, 348 browser fixture, 152 Rust and 14 tracker tests; 29 opt-in native tests are explicitly ignored by that standard command and separately covered where recorded. No additional native run is invented by the final artifact provenance smoke.

Physical sleep/wake (network interruption was the selected gate), Wayland, physical GPU, full Mac keyboard/VoiceOver, Apple Keychain behavior, other OS versions and quarantined signed-download acceptance remain unverified. Logs/events have explicit bounded drops/gaps. Local history is not a tamper-proof audit archive; raw log exports can contain application secrets. SSH/Docker account privileges are not reduced by app read-only mode.

The reviewed dependency tree retains GLib's unsound iterator and proc-macro-error maintenance warnings; bounded source review found no affected iterator call in the app/framework selection, not a universal unreachability proof. Exact audit/reassessment and the inaccessible GitHub alert limitation remain visible in [release review](release-review.md). No unresolved blocking app defect was identified within this bounded review.

## Repository state and optional next work

Prompt-owned changes are committed and pushed to the existing `origin/main`; Git's old remote name redirects to `alekpopovic/container-desk`. No force push or hook bypass was used. Concurrent unrelated local changes are deliberately retained outside these commits: deletion of `.github/workflows/ci.yml`, byte-identical untracked `.github/ci.yml`, and empty `.github/workflows/.keep`. Their intent was queried but not assumed. **The entire working tree is therefore not clean**; [recorded snapshot](verification/059/worktree-note.json) makes the exception explicit. Nothing was deleted/restored/committed merely to hide it. The current committed workflow remains read-only; the original tested run is pinned above regardless of newer CI runs.

Optional next work: owner-supplied signing/notarization and downloaded-artifact acceptance; additional physical sleep/Wayland/Mac accessibility and OS/Docker/context coverage; compatible native dependency remediation when available. Any expansion such as simultaneous active hosts or auto-updates needs a separate scope decision. Public release, provider accounts and distribution remain owner actions. No manual public-publishing workflow or equivalent substitute was added.
