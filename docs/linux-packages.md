# Linux packages

The initial package target is Ubuntu 24.04 LTS x86_64. CI builds natively on `ubuntu-24.04`; the package command rejects other Linux build baselines. No aarch64 or other distribution compatibility is claimed. [Tauri's baseline guidance](https://v2.tauri.app/distribute/debian/) explains why building on a newer glibc can break older systems.

After all standard checks pass, `python3 scripts/package_ci.py` bundles the ordinary release executable into deb and AppImage. `APPIMAGE_EXTRACT_AND_RUN=1` permits bundling on a CI worker without a FUSE mount. Metadata schema 2 records each package in `packages`, with filename, length and SHA-256; SHA256SUMS covers both packages and metadata. Artifacts remain unsigned development output with `releaseApproved: false`.

The deb declares OpenSSH and the baseline libc requirement in addition to Tauri's native GTK/WebKit dependencies. Tauri generates the ContainerDesk desktop entry, application icon and executable installation from the committed product/category/icon settings. No shell-profile startup or bundled Node/Rust/Docker is required. AppImage still uses the system OpenSSH executable and normal user SSH configuration; see [desktop launch](desktop-launch.md).

Install a verified deb with `sudo apt install ./containerdesk-0.1.0-x86_64-unknown-linux-gnu.deb`, then use ContainerDesk in the desktop application menu. Keep the downloaded checksum file with the package. AppImage is portable: make the downloaded file executable and open it; Ubuntu requires FUSE 2 compatibility (`libfuse2t64`) for normal mounting, or use the AppImage runtime's extraction mode when FUSE is unavailable. These instructions are subject to the native smoke results recorded in 053 evidence.

Flatpak, Snap and other sandboxed formats are outside the first package set: access to SSH config/includes, identity paths, the native executable and agent sockets needs an explicit additional design. No package alters SSH trust files or config. Wayland and full native remote-operation acceptance remain separate platform gates.
