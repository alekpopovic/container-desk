---
title: "Native Linux security verification — 047"
section: "Reviews & evidence"
icon: "🧪"
historical: true
---

# 🧪 Native Linux security verification — 047

2026-09-29, Ubuntu 26.04.1 x86_64, WebKitGTK 2.52.6, private Xvfb/D-Bus profile. No SSH/Docker server was selected or changed by this harness.

- Baseline: `python3 tests/lab/security.py --tools-dir /tmp/containerdesk-025-tools --artifacts docs/verification/047-native --baseline`, exit 0 means the vulnerability was **reproduced**, not that the old binary was secure. 046 binary SHA-256 `5765040b70cfdc153f799b8ae763b867e09194ffa6eeda4a3c101223e6065031` loaded the owned HTTP page; `baseline.json` records `/navigate`.
- Fixed: same command without `--baseline`, exit **0**. Final binary SHA-256 `63579e105e1e7e5d7c888e9fa0e5553f445624fcdad2c55c795e030865b85855`. External/data/file/path navigation and popups were rejected, forms/frames made zero requests to the owned server, inline script remained blocked, local Settings routing worked.
- Six malformed aliases, relative config path, invalid session ID and fake-scope requests were rejected by actual native IPC. Generic shell/filesystem/window plugin calls were denied. An HTML-looking saved host name was rendered as text; its seeded private marker/config path/alias were absent from `reviewed-support.json`.
- A separate normal app launch, without Tauri/WebKit automation/inspector environment, owned no TCP listening socket across its process tree and exited normally. WebDriver endpoints belonged only to the external test driver in the automation phase.

`native-security.json` contains bounded outcomes, and the screenshot shows the actual local app after attempted navigation. Hostile text and all test values are synthetic. One initial navigation guard blocked Tauri's no-slash local URL; corrected before the final run. Environment portal/fuse/PipeWire and D-Bus teardown messages do not establish native package support. macOS behavior remains pending.
