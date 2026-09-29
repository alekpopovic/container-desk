# Linux packages

The initial package target is Ubuntu 24.04 LTS x86_64. CI builds natively on `ubuntu-24.04`; the package command rejects other Linux build baselines. No aarch64 or other distribution compatibility is claimed. [Tauri's baseline guidance](https://v2.tauri.app/distribute/debian/) explains why building on a newer glibc can break older systems.

After all standard checks pass, `python3 scripts/package_ci.py` bundles the ordinary release executable into deb and AppImage. `APPIMAGE_EXTRACT_AND_RUN=1` permits bundling on a CI worker without a FUSE mount. Metadata schema 2 records each package in `packages`, with filename, length and SHA-256; SHA256SUMS covers both packages and metadata. Artifacts remain unsigned development output with `releaseApproved: false`.

The deb declares OpenSSH and the baseline libc requirement in addition to Tauri's native GTK/WebKit dependencies. Tauri generates the ContainerDesk desktop entry, application icon and executable installation from the committed product/category/icon settings. No shell-profile startup or bundled Node/Rust/Docker is required. AppImage still uses the system OpenSSH executable and normal user SSH configuration; see [desktop launch](desktop-launch.md).

Install a verified deb with `sudo apt install ./containerdesk-0.1.0-x86_64-unknown-linux-gnu.deb`, then use ContainerDesk in the desktop application menu. Keep the downloaded checksum file with the package. AppImage is portable: make the downloaded file executable and open it; Ubuntu requires FUSE 2 compatibility (`libfuse2t64`) for normal mounting, or use the AppImage runtime's extraction mode when FUSE is unavailable. The deb smoke result is recorded in 053; ordinary AppImage FUSE launch is verified in [058](verification/058/appimage-vm.json). The tested baseline also has the documented GTK/WebKit runtime libraries; no claim of a completely self-contained runtime is made.

Flatpak, Snap and other sandboxed formats are outside the first package set: access to SSH config/includes, identity paths, the native executable and agent sockets needs an explicit additional design. No package alters SSH trust files or config. Wayland and full native remote-operation acceptance remain separate platform gates.

## Disposable desktop verification

`tests/lab/linux_package_vm.py` uses KVM and the official Ubuntu Noble image dated 2026-09-26, verified against SHA-256 `6a81c37564db9b1ee84e141922625e1d7c5b389b99bb3c572e0243607d5bb4d2` from [Ubuntu's manifest](https://cloud-images.ubuntu.com/noble/20260926/SHA256SUMS). It creates an app-owned overlay and short-lived SSH identities, pins the VM host key before first connection, installs Openbox/Xfce components on Xvfb, then installs the checked deb with apt. The fresh guest has no Node/npm/Rust/Cargo/Docker. Python/AT-SPI and Xvfb are test tools, not app dependencies.

```sh
python3 tests/lab/linux_package_vm.py \
  --qemu-root /path/to/extracted-qemu-packages \
  --image /path/to/noble-server-cloudimg-amd64.img \
  --packages /path/to/checked-ci-artifact \
  --artifacts /path/to/owned-test-results
```

The test launches the installed desktop entry with `gtk-launch`, observes the real production process/window, invokes native accessible controls to browse a synthetic user config plus Include, resolves a selected inert alias with `/usr/bin/ssh`, verifies unchanged config bytes, and closes the window. It does not connect to a remote Docker host. The guest disk/keys/process are discarded; screenshots, exact package hashes, OS/runtime versions and a bounded result report are retained. This proves the exercised X11 virtual desktop path only; physical GPU, Wayland and full distro-family claims need separate evidence.

The generated Debian package name is `container-desk`, while the executable is `/usr/bin/containerdesk` and the launcher is `ContainerDesk.desktop`. The installed icon is a 512×512 PNG. The packaged executable hash differs from the pre-bundle hash because [Tauri 2.12's bundler](https://github.com/tauri-apps/tauri/blob/tauri-v2.12.0/crates/tauri-bundler/src/bundle.rs) replaces its bundle-type marker. Inspection of the actual deb proved that reversing only `DEB` to `UNK` at that marker exactly reproduces the verified production executable hash. The installed-file hash is recorded separately; packages themselves are never modified by this inspection.

Add `--format appimage` to the disposable desktop command to verify the AppImage instead of installing the deb. The test checks the package manifest, extracts only for static executable/desktop inspection, removes that extraction, and launches the ordinary AppImage through FUSE. The running `/proc` executable hash must match the embedded executable and its native FUSE mount must disappear after normal close. [058's result](verification/058/appimage-desktop.json) and [screenshot](verification/058/appimage-discovery.png) use the exact previously built 053 package, with its original source/hash attribution preserved.
