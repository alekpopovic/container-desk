---
title: "040 native terminal UI — Linux"
section: "Reviews & evidence"
icon: "🧪"
historical: true
---

# 🧪 040 native terminal UI — Linux

2026-09-29; Ubuntu 26.04.1 x86_64, WebKitGTK 2.52.6, Tauri 2.12.0, OpenSSH 10.2p1, host Docker 29.8.1. Release binary SHA-256 `1a76b6c757462ebc835d7782ce87606a173acbe238b1cbaafe4e11077c4e9086`. Built with actual Node 24.21.0/npm 11.19.0 and Rust 1.98.1.

The wrapper `/tmp/containerdesk-040-xvfb.py` allocated an owned 1440×1000 Xvfb display using `-displayfd`, no TCP listener, `GDK_BACKEND=x11`, `LIBGL_ALWAYS_SOFTWARE=1`, with WAYLAND_DISPLAY removed. It executed this lab and terminated/reaped its own X server:

```sh
python3 tests/lab/logs.py \
  --sshd-root /tmp/containerdesk-025-tools/openssh \
  --stream --jump --terminal \
  --native-driver /tmp/containerdesk-025-tools/bin/tauri-driver \
  --webkit-driver /tmp/containerdesk-025-tools/webkit/usr/bin/WebKitWebDriver \
  --native-artifacts docs/verification/040-native \
  --focus-xdotool /tmp/containerdesk-025-tools/xdotool/usr/bin/xdotool
```

**Exit 0.** The actual production application ran with app-owned data and PATH excluding Docker, Node, Cargo and Python. Only explicit owned container IDs passed the remote gate. Two owned loopback SSH hops enforced strict pinned host keys, with no agent forwarding. Local shell user input used real X11 key events; native paste used trusted clipboard events and Ctrl+Shift+V. No application IPC was mocked.

- Release UI: management plus separate terminal grant, target confirmation and cancel without dispatch; non-root shell, distinct output marker; fitted terminal dimensions observed by remote `stty` (24×97 after window resize); multiline paste Cancel sent nothing, Send executed the exact two synthetic lines; tab closure under continuous output; second explicit saved host alias/session without old output; permission revocation disabled input and did not reopen.
- Native backend: one checkpoint passed, including waits while both native read slots were intentionally occupied, one-use intent, UID 1000, Ctrl-C, 111×37 resize, exit 7, real missing shell, scope/sequence checks, concurrent structured JSON without PTY and owner reaping on revoke/disconnect.
- Independent remote oracle: four explicit non-root `/bin/sh` execs, exactly one missing-shell attempt, no fallback. Both running fixtures' primary processes remained alive. Client trust unchanged. App data, activity and default diagnostics contained no synthetic transcript markers.
- Screenshots: `native-terminal-connected.png` and `native-terminal-revoked.png` show only owned synthetic resources/output.
- Cleanup removed/reaped only lab-owned containers, servers, keys, app data and X server.

Earlier failed attempts and fixes are recorded in [040 evidence](https://github.com/alekpopovic/container-desk/blob/main/codex/tracking/evidence/040.md). This is not macOS, Wayland, Ubuntu 24.04 baseline, installed package or signing/notarization evidence. Two saved host sessions are sequential, both reaching the same isolated server through different explicit aliases; concurrent two-owner fencing is also covered by synthetic ownership tests.
