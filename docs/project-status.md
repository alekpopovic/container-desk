---
title: "ContainerDesk project status"
section: "Start here"
icon: "🧭"
---

# 🧭 ContainerDesk project status

Updated 2026-09-29 through completed prompt 060. The native Linux application implements SSH discovery and saved hosts, live Docker read views, explicitly enabled lifecycle/Compose actions, container terminals and bounded recovery. The current feature and platform evidence is maintained in [046 checkpoint](checkpoints/046-feature-complete.md). Earlier checkpoint and tracker evidence describes the implementation at that historical stage.

The application uses the local native OpenSSH executable and the selected trusted SSH configuration. One saved host can be connected at a time. New/recovered sessions start read-only; management and terminal grants are explicit and transient. Images, volumes and networks remain read-only. Compose actions operate only on verified existing projects; deployment, prune, arbitrary scripts and Kubernetes are outside v1.

| Platform / release area | Actual state |
|---|---|
| Ubuntu 26.04.1 x86_64, WebKitGTK 2.52.6, X11 | Native unbundled release application exercised against disposable Docker/strict ProxyJump resources |
| GUI launch, minimal PATH, encrypted agent keys | Native Linux verification in 042 |
| Support Save dialogs / keyboard / screen-reader labels | Native Linux verification in 043–044; audible speech quality not assessed |
| Ubuntu 24.04 baseline and native Wayland | Ubuntu 24.04.5 clean VM deb installation, desktop launch and SSH discovery/resolution passed on X11; Wayland remains pending |
| macOS Apple Silicon and Intel | Native app/DMG, Finder launch, private app data, system SSH and empty-agent access passed on macOS 15.7.9; full direct/private ProxyJump, encrypted-agent, strict trust, Docker, management, terminal and recovery acceptance passed in 058 |
| deb / AppImage / app / DMG installation | deb, AppImage, app and DMG created with verified checksums; deb installed/launched in clean Ubuntu VM; both native Mac app/DMG launch tests passed; ordinary AppImage/FUSE launch passed in 058 |
| Signing / notarization / public distribution | 055 configuration-ready without credentials; actual signing/notarization unverified; [v0.1.0 unsigned preview](https://github.com/alekpopovic/container-desk/releases/tag/v0.1.0) manually published with explicit owner authorization |

Commands and pinned toolchains: [development](development.md), [toolchains](toolchains.md). Runtime bounds: [resource limits](resource-limits.md). Security model: [operation policy](operation-policy.md), [remote commands](remote-commands.md), [SSH authentication](ssh-authentication.md). Feature evidence is distinct from packaging and release readiness. Continue sequentially under the user's [execution authorization](execution.md).

The [052 CI matrix](ci.md) passed on Linux x86_64 and both Mac architectures. Prompt 052 is complete under the user's explicit decision to omit the manual public-publisher workflow. No publisher workflow or CI release-write capability was installed. After completing 060, the owner separately authorized manual publication of the original verified installers; [publication receipt](releases/v0.1.0.md). The owner restored `.github/workflows/ci.yml` in `5636d34` after its earlier relocation in `c2464b8`; documentation publishes separately through GitHub Pages from `main/docs`. [053 Linux packaging](linux-packages.md) and [054 macOS packaging](macos-packages.md) are complete; [055 signing readiness](signing.md) is complete through its explicit no-credentials branch; [056 manual updates and rollback](updates.md) is complete; [057 quick start and contributor guide](https://github.com/alekpopovic/container-desk/blob/main/README_SR.md) is verified through a fresh native profile and dedicated direct/ProxyJump VM; [058 native acceptance](platform-matrix.md) is complete on all three targets; [059 release review](release-review.md) is complete with six original packages in a local candidate directory; [060 final handover](FINAL_HANDOVER.md) records commands, hashes, verified behavior and remaining owner actions. All 60 prompts are complete under the documented scope.
