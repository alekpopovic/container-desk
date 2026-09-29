# 041 native recovery and shutdown

Date: 2026-09-29. Ubuntu 26.04.1 x86_64, WebKitGTK 2.52.6, owned Xvfb/X11 software rendering, release Tauri 2.12.0 executable. Binary SHA-256: `0af8364433add839638d6f8a8913462562a1226e58d9d5c4e472c3c436b2cbe1`.

The final `python3 /tmp/containerdesk-041-xvfb.py` wrapper allocated a private Xvfb display using `-displayfd`, removed WAYLAND_DISPLAY, set GDK_BACKEND=x11 and LIBGL_ALWAYS_SOFTWARE=1, ran this command, then terminated/reaped only that display:

```sh
python3 tests/lab/logs.py \
  --sshd-root /tmp/containerdesk-025-tools/openssh \
  --stream --jump --terminal --recovery \
  --native-driver /tmp/containerdesk-025-tools/bin/tauri-driver \
  --webkit-driver /tmp/containerdesk-025-tools/webkit/usr/bin/WebKitWebDriver \
  --native-artifacts docs/verification/041-native \
  --focus-xdotool /tmp/containerdesk-025-tools/xdotool/usr/bin/xdotool
```

Exit 0. Node 24.21.0/npm 11.19.0 were first on PATH. No production host, user SSH config or private key was used. Two owned loopback SSH servers provided actual strict ProxyJump. All temporary keys/config/data/servers and explicitly owned Docker containers were cleaned.

- Actual native stats sample received before fault injection. At all three faults the restricted remote gate confirmed an exact owned-container stats request was in flight.
- During live logs/stats, only the owned target sshd connection handlers were terminated; its listener remained available. The selected host reconnected read-only, loaded fresh rows, and all old local SSH PIDs/start-times disappeared. Explicit log restart displayed its gap.
- During terminal/stats, only the exact private app master was terminated. The terminal closed, old SSH processes were reaped, the host recovered read-only, and the independent remote exec count remained one until a second explicit terminal confirmation.
- An actual X11 WM_DELETE_WINDOW event closed the native window while terminal/stats/events were active. The native application exited before WebDriver session deletion. Its process and all recorded owned SSH processes were then absent, including zombies, within **0.117 seconds**. No mutation dispatch occurred. See [process audit](process-audit.json).
- Native backend PTY checkpoint: one test passed (15.10 seconds), including exact permission/intent, non-root UID, echo, Ctrl-C, resize, missing shell, revoke/disconnect and no replay.
- Native direct fallback checkpoint: one test passed (0.60 seconds). The bounded marker probe detected loss of the owned SSH server while preserving a foreign socket-path marker.
- Independent remote gate observed four explicit non-root shell attempts in total, exactly one missing-shell attempt, no automatic fallback, and both container primary processes still running. Known-host trust hashes stayed unchanged. App storage/default driver diagnostics contained no terminal marker.

The two screenshots were visually reviewed: recovered logs show the visible gap and read-only connection; the second explicitly opened terminal shows host/alias/full container/daemon/user identity before exit. Screenshots alone do not establish lifecycle correctness; process/start-time checks and the remote gate provide that evidence.

This is actual native Linux network interruption evidence, not physical suspend/resume, macOS, Wayland or installed-package evidence.
