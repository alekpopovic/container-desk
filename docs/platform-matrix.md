# Native acceptance matrix

Prompt 058 is in progress. Linux results below are real native client execution against an owned disposable Linux Docker VM. Both Mac package/launch paths already have native evidence from 054; the extended direct/ProxyJump/agent/Docker matrix is being executed on actual Mac CI runners. Pending cells are not runtime claims. No agreed architecture is removed.

| Check | Linux x86_64 | macOS arm64 | macOS x86_64 |
|---|---|---|---|
| OS / client | Ubuntu 26.04.1, app 0.1.0; [058 local result](verification/058/linux-local-integration.json) | macOS 15.7.9, app 0.1.0; [054](verification/054/arm64-native.json) | macOS 15.7.9, app 0.1.0; [054](verification/054/intel-native.json) |
| Native package / GUI launch | Ubuntu 24.04.5 deb [053](../codex/tracking/evidence/053.md) | Finder/DMG PASS [054](verification/054/arm64-native.json) | Finder/DMG PASS [054](verification/054/intel-native.json) |
| Minimal GUI PATH / SSH | PASS [042](../codex/tracking/evidence/042.md), [053](../codex/tracking/evidence/053.md) | System SSH/agent socket PASS [054](verification/054/arm64-native.json) | System SSH/agent socket PASS [054](verification/054/intel-native.json) |
| Direct and private ProxyJump | PASS actual backend [058](verification/058/linux-local-integration.json), actual GUI [057](verification/057/quick-start.json) | PENDING native VM CI | PENDING native VM CI |
| Encrypted key / native agent | PASS empty-agent denial then loaded encrypted key [058](verification/058/linux-local-integration.json) | PENDING | PENDING |
| Unknown / changed host key | PASS direct, destination, bastion [058](verification/058/linux-local-integration.json) | PENDING | PENDING |
| List / masked inspect / logs / stats | PASS actual Docker [058](verification/058/linux-local-integration.json) | PENDING | PENDING |
| Management / existing Compose | PASS exact independent event/state oracle [058](verification/058/linux-local-integration.json) | PENDING | PENDING |
| PTY input / resize / Ctrl-C / cleanup | PASS actual PTY [058](verification/058/linux-local-integration.json) | PENDING | PENDING |
| Connection loss / stale scopes / no replay | PASS guest-side SSH termination and recovery [058](verification/058/linux-local-integration.json) | PENDING | PENDING |
| Unrelated SSH master preserved | PASS independent owned root master [058](verification/058/linux-local-integration.json) | PENDING | PENDING |
| Redacted support export | Native GUI [043](../codex/tracking/evidence/043.md) | Standard native backend tests; GUI export not yet exercised | Standard native backend tests; GUI export not yet exercised |
| Checksums | [053 manifest](verification/053/SHA256SUMS.txt) | [054 manifest](verification/054/arm64-SHA256SUMS.txt) | [054 manifest](verification/054/intel-SHA256SUMS.txt) |
| Public signing/notarization | Optional owner signature not requested | UNVERIFIED without credentials [055](signing.md) | UNVERIFIED without credentials [055](signing.md) |

The native Rust backend test is separate from native GUI/package evidence. Browser mocks do not fill any native cell. Full encrypted-agent runs reject an empty agent, prove the generated private key rejects an empty passphrase, load it only into an owned native agent and point SSH at its public identity file. They inspect native Docker results on both routes and prove the private target is unreachable directly. Only owned workloads are mutated. No host Docker socket, user key or guessed server alias is used.

Linux uses KVM where available; Mac runs a real Linux guest through QEMU TCG. The **client executable, Rust runtime, native OpenSSH and PTY are native to the claimed host architecture**. Emulating the remote server does not establish any other client platform. Temporary QEMU disks, seed/key files, agents and SSH processes are removed; only sanitized reports are retained. No signing, publication or release permission is added by these checks.
