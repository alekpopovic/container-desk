---
title: "043 native support export and clearing"
section: "Reviews & evidence"
icon: "🧪"
historical: true
---

# 🧪 043 native support export and clearing

2026-09-29, Ubuntu 26.04.1 x86_64, UID 1000, WebKitGTK 2.52.6, Tauri 2.12.0, owned Xvfb/X11 display. Final production binary SHA-256: `3ae8f2e37a3be3c85a25a0af5a1c2a38d51108be9e91a5a86d0f1242a0ac19b3`.

Final `python3 /tmp/containerdesk-043-xvfb.py` exit 0. The wrapper allocated and reaped its private Xvfb display and ran:

```sh
python3 tests/lab/ssh_auth.py --engine --launch --support \
  --native-driver /tmp/containerdesk-025-tools/bin/tauri-driver \
  --webkit-driver /tmp/containerdesk-025-tools/webkit/usr/bin/WebKitWebDriver \
  --native-artifacts docs/verification/043-native
```

The external harness uses xdotool/import to operate and inspect the real GTK file chooser. Its minimal native application PATH contains only bwrap, required by this distribution's GTK icon loader; Docker, jq, Node, Python and Cargo are absent from that PATH. The separate real Engine Rust checkpoint uses PATH=/nonexistent. The harness itself requires development tools and Docker.

- Seven owned native integration tests passed, including 16 direct/jump authentication cases. Their Docker classification matrix uses a synthetic API and is not Engine runtime proof.
- One separate actual Engine 28.3.3 checkpoint passed (1.61 seconds), direct and ProxyJump, native SSH with no client tool PATH.
- The release UI performed four real encrypted-key/agent cases, then explicitly retried the owned unloaded-key host to obtain an actual authentication-stage failure. It prepared a pseudonymous report containing that stage/code and a seeded synthetic local activity record. The seeded activity was not a real dispatched remote mutation.
- Native preview was scanned for fixture secrets, real owned paths/aliases, daemon identity and full target/host IDs. None appeared. The backend's operation code and coarse duration remained useful. See the actual [reviewed export](reviewed-support.json).
- Native GTK Cancel wrote nothing and retained the same preview. Native GTK Save wrote bytes exactly equal to the frozen reviewed preview, with mode 0600; the consumed preview disappeared.
- Clearing Cancel retained history. Explicit confirmation emptied the actual persisted activity history and invalidated the preview. A new report had an empty activity array. The exported file remained unchanged.
- SHA-256 hashes of owned home `.ssh` config/key/trust markers and actual disposable config/private keys/known_hosts stayed unchanged. No production SSH resource was used.
- A separate actual permission-denied settings lock still allowed a report with a typed preferences/storage error and `unavailable` executable configuration, without guessing defaults. Settings saving stayed disabled.
- The underlying native launch, Unicode path, actual gio desktop entry, no-profile-script and private storage checks also passed. All owned lab containers, network, agent, keys and data were cleaned.

The final report, preview, clear confirmation/cleared state and inaccessible-settings screenshots were reviewed. Native visual review found an initially unstyled confirmation; the final binary uses the existing theme styles and a centered dialog with initial focus on Cancel.

Earlier attempts failed due to XPath Unicode escaping and missing Tauri native command grants. The former was fixed in the harness; the latter was fixed in the actual app manifest/capability using exactly three named commands. Browser mocks had not established those native permissions. Only the final successful run above establishes native acceptance.

Native Linux X11 only; macOS file chooser, Wayland, installed packages and signing are not proven by this run.
