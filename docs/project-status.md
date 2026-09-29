# ContainerDesk project status

Updated 2026-09-29 through completed prompt 058. The native Linux application implements SSH discovery and saved hosts, live Docker read views, explicitly enabled lifecycle/Compose actions, container terminals and bounded recovery. The current feature and platform evidence is maintained in [046 checkpoint](checkpoints/046-feature-complete.md). Earlier checkpoint and tracker evidence describes the implementation at that historical stage.

The application uses the local native OpenSSH executable and the selected trusted SSH configuration. One saved host can be connected at a time. New/recovered sessions start read-only; management and terminal grants are explicit and transient. Images, volumes and networks remain read-only. Compose actions operate only on verified existing projects; deployment, prune, arbitrary scripts and Kubernetes are outside v1.

| Platform / release area | Actual state |
|---|---|
| Ubuntu 26.04.1 x86_64, WebKitGTK 2.52.6, X11 | Native unbundled release application exercised against disposable Docker/strict ProxyJump resources |
| GUI launch, minimal PATH, encrypted agent keys | Native Linux verification in 042 |
| Support Save dialogs / keyboard / screen-reader labels | Native Linux verification in 043–044; audible speech quality not assessed |
| Ubuntu 24.04 baseline and native Wayland | Ubuntu 24.04.5 clean VM deb installation, desktop launch and SSH discovery/resolution passed on X11; Wayland remains pending |
| macOS Apple Silicon and Intel | Native app/DMG, Finder launch, private app data, system SSH and empty-agent access passed on macOS 15.7.9; full direct/private ProxyJump, encrypted-agent, strict trust, Docker, management, terminal and recovery acceptance passed in 058 |
| deb / AppImage / app / DMG installation | deb, AppImage, app and DMG created with verified checksums; deb installed/launched in clean Ubuntu VM; both native Mac app/DMG launch tests passed; ordinary AppImage/FUSE launch passed in 058 |
| Signing / notarization / public distribution | 055 configuration-ready without credentials; actual signing/notarization unverified; no release published |

Commands and pinned toolchains: [development](development.md), [toolchains](toolchains.md). Runtime bounds: [resource limits](resource-limits.md). Security model: [operation policy](operation-policy.md), [remote commands](remote-commands.md), [SSH authentication](ssh-authentication.md). Feature evidence is distinct from packaging and release readiness. Continue sequentially under the user's [execution authorization](execution.md).

The [052 CI matrix](ci.md) passed on Linux x86_64 and both Mac architectures. Prompt 052 is complete under the user's explicit decision to omit the manual public-publisher workflow. No release-write capability was installed and no public release was published. [053 Linux packaging](linux-packages.md) and [054 macOS packaging](macos-packages.md) are complete; [055 signing readiness](signing.md) is complete through its explicit no-credentials branch; [056 manual updates and rollback](updates.md) is complete; [057 quick start and contributor guide](../README_SR.md) is verified through a fresh native profile and dedicated direct/ProxyJump VM; [058 native acceptance](platform-matrix.md) is complete on all three targets; continue with 059 release candidate review.
