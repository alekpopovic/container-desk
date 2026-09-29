# ContainerDesk project status

Updated 2026-09-29 for prompt 046. The native Linux application implements SSH discovery and saved hosts, live Docker read views, explicitly enabled lifecycle/Compose actions, container terminals and bounded recovery. The current feature and platform evidence is maintained in [046 checkpoint](checkpoints/046-feature-complete.md). Earlier checkpoint and tracker evidence describes the implementation at that historical stage.

The application uses the local native OpenSSH executable and the selected trusted SSH configuration. One saved host can be connected at a time. New/recovered sessions start read-only; management and terminal grants are explicit and transient. Images, volumes and networks remain read-only. Compose actions operate only on verified existing projects; deployment, prune, arbitrary scripts and Kubernetes are outside v1.

| Platform / release area | Actual state |
|---|---|
| Ubuntu 26.04.1 x86_64, WebKitGTK 2.52.6, X11 | Native unbundled release application exercised against disposable Docker/strict ProxyJump resources |
| GUI launch, minimal PATH, encrypted agent keys | Native Linux verification in 042 |
| Support Save dialogs / keyboard / screen-reader labels | Native Linux verification in 043–044; audible speech quality not assessed |
| Ubuntu 24.04 baseline and native Wayland | Pending native execution |
| macOS Apple Silicon and Intel | Pending native build and execution |
| deb / AppImage / app / DMG installation | Pending release engineering and actual package smoke checks |
| Signing / notarization / public distribution | No credentials used or release published; separate release gates |

Commands and pinned toolchains: [development](development.md), [toolchains](toolchains.md). Runtime bounds: [resource limits](resource-limits.md). Security model: [operation policy](operation-policy.md), [remote commands](remote-commands.md), [SSH authentication](ssh-authentication.md). Feature evidence is distinct from packaging and release readiness. Continue sequentially under the user's [execution authorization](execution.md).
