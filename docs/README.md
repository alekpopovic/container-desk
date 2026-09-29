---
title: "Documentation"
section: "Start here"
icon: "🧭"
permalink: "/"
home: true
---

# 🧭 Documentation

**Your servers. Your SSH. One workspace.** Find installation steps, workflow guides, architecture notes and the evidence behind ContainerDesk.

![ContainerDesk — native Docker over SSH](assets/brand/banner.svg)

[⬇ Download v0.1.0](https://github.com/alekpopovic/container-desk/releases/tag/v0.1.0) · [🚀 Start using ContainerDesk](user-guide.md) · [🇷🇸 Uputstvo na srpskom](https://github.com/alekpopovic/container-desk/blob/main/README_SR.md) · [🎨 Brand kit](branding.md)

> **Unsigned preview.** Linux x86_64 and macOS Apple Silicon/Intel have recorded native execution evidence. Mac packages are not Developer ID signed or notarized; Gatekeeper may block downloaded apps. See the [platform matrix](platform-matrix.md) and [release receipt](releases/v0.1.0.md).

## Choose your starting point

| I want to… | Go to |
|---|---|
| 🚀 Install and connect a server | [User guide](user-guide.md) · [Linux](linux-packages.md) · [macOS](macos-packages.md) |
| 🔐 Understand SSH and permissions | [Authentication and trust](ssh-authentication.md) · [Operation policy](operation-policy.md) |
| ▦ Inspect workloads | [Containers](container-inventory.md) · [Logs](log-viewer.md) · [Statistics](container-statistics.md) |
| ⚡ Manage an existing workload | [Lifecycle actions](container-management.md) · [Compose](compose-actions.md) · [Terminal](terminal-transport.md) |
| 🛠️ Contribute or troubleshoot | [Development](development.md) · [Verification](verification-command.md) · [Support reports](support-reports.md) |
| 🎨 Use the project identity | [Brand kit](branding.md) · [Website maintenance](github-pages.md) |

## Documentation library

Implementation notes describe the increment in which a feature was added. Use [project status](project-status.md), the [user guide](user-guide.md) and the [platform matrix](platform-matrix.md) for current behavior. Checkpoints and native results preserve their original historical findings.

### 🧭 Start here

- [Using ContainerDesk](user-guide.md)
- [Desktop launch and SSH setup](desktop-launch.md)
- [ContainerDesk project status](project-status.md)
- [Versions, manual updates and rollback](updates.md)
- [Keyboard and accessible inspection](accessibility.md)
- [Local settings](settings.md)
- [Support reports and local troubleshooting data](support-reports.md)
- [Explicit offline demo](demo-mode.md)

### 📦 Releases & platforms

- [ContainerDesk 0.1.0 — unsigned preview release](RELEASE_NOTES.md)
- [v0.1.0 — public unsigned preview](releases/v0.1.0.md)
- [Linux packages](linux-packages.md)
- [macOS app and DMG packages](macos-packages.md)
- [Native acceptance matrix](platform-matrix.md)
- [Optional package signing and notarization](signing.md)
- [Local release candidate checklist](RELEASE_CHECKLIST.md)
- [Release candidate review — 059](release-review.md)
- [ContainerDesk 0.1.0 — final handover](FINAL_HANDOVER.md)

### 🔐 Hosts & SSH

- [Saved host inventory](host-inventory.md)
- [SSH configuration discovery](ssh-discovery.md)
- [Effective SSH configuration](ssh-resolution.md)
- [SSH authentication and trust](ssh-authentication.md)
- [Bounded SSH process runner](ssh-runner.md)
- [Owned SSH connection reuse](ssh-multiplexing.md)
- [Connection attempt state and cancellation](connection-state.md)
- [Connection recovery and exit](connection-recovery.md)
- [Native dependency diagnostics](native-dependencies.md)
- [Remote Docker capabilities and target identity](docker-capabilities.md)
- [Remote command construction](remote-commands.md)

### ▦ Containers & resources

- [Container inventory](container-inventory.md)
- [Docker container listing adapter](container-listing.md)
- [Container inspect details](container-inspect.md)
- [Container detail panels](container-detail-panels.md)
- [Selected-container resource statistics](container-statistics.md)
- [Log viewer and explicit export](log-viewer.md)
- [Bounded log snapshots](log-snapshots.md)
- [Owned live log subscriptions](live-log-streams.md)
- [Docker event hints](docker-events.md)
- [Compose discovery and read views](compose-discovery.md)
- [Read-only image inventory](image-inventory.md)
- [Volume metadata and mount relationships](volume-inventory.md)
- [Network inventory and attachments](network-inventory.md)
- [Read scheduling and stale data](read-scheduling.md)

### ⚡ Management & terminal

- [Backend operation policy](operation-policy.md)
- [Container lifecycle controls](container-management.md)
- [Explicit container batches](batch-actions.md)
- [Mutation confirmation and local activity](mutation-activity.md)
- [Verified remote Compose actions](compose-actions.md)
- [Container terminal transport](terminal-transport.md)

### 🛠️ Build & design

- [Development and build commands](development.md)
- [Pinned development toolchains](toolchains.md)
- [Contributor verification](verification-command.md)
- [Native desktop testing](native-testing.md)
- [Disposable SSH/Docker integration lab](integration-lab.md)
- [Native CI and artifact boundaries](ci.md)
- [IPC contract](ipc-contract.md)
- [Workspace layout and design tokens](design-system.md)
- [ContainerDesk brand kit](branding.md)
- [Documentation website](github-pages.md)
- [ADR 0001 — One native desktop application over OpenSSH](decisions/0001-architecture.md)

### 🧪 Reviews & evidence

- [Focused security review — 047](security-review.md)
- [Frontend and parser regressions — 048](regression-tests.md)
- [Resource limits and pressure checks](resource-limits.md)
- [Executable acceptance checkpoints](checkpoints.md)
- [Execution authorization](execution.md)
- [018 — Native SSH foundation checkpoint](checkpoints/018-ssh.md)
- [Read-only MVP checkpoint](checkpoints/030-read-only.md)
- [038 — Management workflow checkpoint](checkpoints/038-management.md)
- [046 — Feature-complete desktop checkpoint](checkpoints/046-feature-complete.md)
- [040 native terminal UI — Linux](verification/040-native/RESULTS.md)
- [041 native recovery and shutdown](verification/041-native/RESULTS.md)
- [042 native Linux desktop launch](verification/042-native/RESULTS.md)
- [043 native support export and clearing](verification/043-native/RESULTS.md)
- [Native Linux 046 results](verification/046-native/RESULTS.md)
- [Native Linux security verification — 047](verification/047-native/RESULTS.md)

## Project links

[GitHub repository](https://github.com/alekpopovic/container-desk) · [Contributing](https://github.com/alekpopovic/container-desk/blob/main/CONTRIBUTING.md) · [Architecture contract](https://github.com/alekpopovic/container-desk/blob/main/codex/docs/ARCHITECTURE.md)
