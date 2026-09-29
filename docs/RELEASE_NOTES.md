---
title: "ContainerDesk 0.1.0 — unsigned preview release"
section: "Releases & platforms"
icon: "📦"
---

# 📦 ContainerDesk 0.1.0 — unsigned preview release

2026-09-29. **[Public pre-release v0.1.0](https://github.com/alekpopovic/container-desk/releases/tag/v0.1.0)**, manually published at the owner's explicit request after local acceptance. All six verified installers, SHA256SUMS and provenance/build metadata are attached. No publisher workflow or auto-updater was added. Artifact source: `ce33d6cd9bba5823642749a26d3e6d367a9bab73`; [successful native run](https://github.com/alekpopovic/container-desk/actions/runs/36594720178). Later review/documentation commits do not rebuild or relabel these packages.

## Included

Saved SSH aliases/groups and explicit native configuration resolution; direct and private ProxyJump connections with strict host verification and existing native agent/key access. Container inventory, masked inspect, health/ports/mount/environment views, bounded logs/export, resource statistics and event-driven refresh. Read-only image, volume and network metadata and relationships. Explicitly enabled container lifecycle actions, bounded batches and stopped-container removal without force or volume deletion. Verified existing Compose project start/stop/restart. Separately enabled non-root PTY terminal with resize/Ctrl-C, bounded output and safe paste behavior. Session-scoped confirmations, read-only recovery, unknown mutation outcomes without replay, owned-process cleanup, private local settings and reviewed support exports. Light/dark themes, keyboard shortcuts and an isolated labeled offline demo.

## Runtime requirements and actual platforms

| Target | Actual native acceptance | Package |
|---|---|---|
| Linux x86_64 | Ubuntu 24.04.5; additional native Ubuntu 26.04.1 checks | deb, AppImage |
| macOS Apple Silicon | macOS 15.7.9 arm64 | app archive, DMG |
| macOS Intel | macOS 15.7.9 x86_64 | app archive, DMG |

Packaging floors are Ubuntu 24.04/glibc 2.39 and macOS 15.0; other OS versions are not execution claims. Linux requires the documented GTK/WebKit/OpenSSH libraries, plus FUSE 2 compatibility for ordinary AppImage mounting. Mac uses system OpenSSH and WKWebView. No local Docker, Docker Desktop, Node, Rust, Python or jq is required by the installed app. The remote server needs Linux Docker Engine/CLI, a supported POSIX shell and the user's existing access; Compose CLI is required only for Compose actions. Actual matrix used Docker 28.3.3, Compose 2.36.2 and encrypted native-agent keys. Strict trust must already be established independently, including the bastion.

Install/build instructions: [Serbian quick start](https://github.com/alekpopovic/container-desk/blob/main/README_SR.md), [Linux packages](linux-packages.md), [Mac packages](macos-packages.md), [development](development.md). Back up settings before a manual update; [rollback guidance](updates.md) distinguishes an independent backup from the rolling previous file.

## Evidence and limitations

The [native matrix](platform-matrix.md) separates actual native backend/OpenSSH/PTY execution, ordinary package GUI tests, software-emulated remote/desktop guests, browser fixtures and exact hashes. Final Linux package executables match earlier KVM GUI-tested binaries; final fresh package retests explicitly used TCG. All three native clients passed network interruption/recovery, encrypted agent, strict trust, direct/ProxyJump, Docker operations and terminal cleanup. This does not prove physical sleep/wake, Wayland, physical GPU, full Mac keyboard/VoiceOver, Apple Keychain integration or every Docker/context/rootless deployment. One host is active at a time; saved inventory is separate. SSH configuration is trusted executable configuration, and app read-only controls do not reduce the remote account's permissions.

Images/volumes/networks are read-only; there is no prune, Compose deployment/up/down/build, registry credential manager, Kubernetes, arbitrary script runner, remote custom agent, SaaS service or automatic updater. Logs/events can have explicit gaps/drops; unknown mutation results require a fresh read and user decision. Local history is bounded troubleshooting data, not a tamper-proof audit archive. Raw log exports can contain application secrets and remain explicit user actions.

Dependency review retains two upstream warnings: GLib 0.18.5 unsound iterator ([RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html)) and unmaintained build-time proc-macro-error 1.0.4 ([RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html)). The affected iterator has no call found in the reviewed app/framework sources; that bounded search is not a proof of universal unreachability. No advisory suppression or incompatible GTK dependency replacement was made. [Review and audit receipt](release-review.md) record the decision and future reassessment trigger.

## Distribution status

Linux is unsigned. ARM Mac uses linker ad-hoc signing; Intel is unsigned. Neither Mac package is Developer ID signed/notarized, and internet-download Gatekeeper approval is unverified. [Optional signing readiness](signing.md) is configuration-ready without owner credentials. The owner authorized public distribution as an unsigned preview; this does not establish signed-download/Gatekeeper acceptance. No OS protection was disabled to claim a pass. Original CI artifact retention is 14 days; the release assets are separately attached to v0.1.0. Signing credentials, notarization and provider accounts remain owner actions. [Publication/download receipt](releases/v0.1.0.md).
