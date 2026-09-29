# Native acceptance matrix

Prompt 058 is complete. App **0.1.0**, clean artifact source **`ce33d6cd9bba5823642749a26d3e6d367a9bab73`**, [successful native CI run 36594720178](https://github.com/alekpopovic/container-desk/actions/runs/36594720178). All three native client jobs and both prerequisite probes passed; [retained run receipt](verification/058/ci-run.json). Later documentation/test-helper commits do not change these artifact identities.

| Check | Linux x86_64 | macOS arm64 | macOS x86_64 |
|---|---|---|---|
| Actual OS / native OpenSSH | Ubuntu 24.04.5, OpenSSH 9.6p1; additional Ubuntu 26.04.1 / 10.2p1 local run | macOS 15.7.9 (24G830), OpenSSH 9.9p2 | macOS 15.7.9 (24G830), OpenSSH 9.9p2 |
| Native backend report | [Linux](verification/058/linux-integration.json) | [ARM](verification/058/arm64-integration.json) | [Intel](verification/058/intel-integration.json) |
| Actual remote versions | Alpine 3.22.4, kernel 6.12.81, Docker client/server 28.3.3, Compose 2.36.2, OpenSSH 10.0p2 | Same versions, ARM guest | Same versions, x86_64 guest |
| Ordinary package / GUI launch | PASS [deb](verification/058/final-deb-vm.json), [AppImage/FUSE](verification/058/final-appimage-vm.json); acceleration attribution below | PASS DMG/Finder [report](verification/058/arm64-package.json) | PASS DMG/Finder [report](verification/058/intel-package.json) |
| Minimal GUI PATH / system SSH | PASS actual desktop config/Include discovery and selected native resolution | PASS native diagnostics and inherited agent socket | PASS native diagnostics and inherited agent socket |
| Direct connection | PASS native backend; actual GUI also [057](verification/057/quick-start.json) | PASS native backend | PASS native backend |
| ProxyJump via private target | PASS, direct private endpoint independently inaccessible | PASS, same negative control | PASS, same negative control |
| Encrypted key / native agent | PASS empty-agent rejection, loaded encrypted key, public-only IdentityFile | PASS | PASS |
| Unknown / changed host key | PASS direct, destination and bastion rejection | PASS | PASS |
| Container list / masked inspect | PASS exact real four-container inventory and masked values | PASS | PASS |
| Logs follow / cancel / bounds | PASS native finite/follow/cancel; [045 pressure measurements](../codex/tracking/evidence/045.md) | PASS finite/follow/cancel; bound contracts in native Rust tests, no separate pressure measurement | PASS finite/follow/cancel; bound contracts in native Rust tests, no separate pressure measurement |
| Stats / reconnect / stale scopes | PASS actual reads and rejection after generation change | PASS | PASS |
| Start / stop / restart / unknown outcome | PASS actual actions, lost-response Unknown, exact independent event counts | PASS | PASS |
| Compose paths / existing service actions | PASS space/apostrophe path, verified two-service restart | PASS | PASS |
| PTY input / resize / Ctrl-C / cleanup | PASS actual echo, 111×37, interrupted sleep and close | PASS | PASS |
| Network interruption / recovery / no replay | PASS actual guest SSH session termination, revoked generation, new read-only scope | PASS | PASS |
| Unrelated SSH master preserved | PASS; 577 observed app SSH identities reaped | PASS; 483 identities reaped | PASS; 560 identities reaped |
| Redacted support export | PASS native file readback/redaction; native GUI Save [043](../codex/tracking/evidence/043.md) | PASS native Save/readback, mode 0600, plus backend redaction | PASS native Save/readback, mode 0600, plus backend redaction |
| Checksums / source metadata | [manifest](verification/058/linux-SHA256SUMS.txt), [metadata](verification/058/linux-metadata.json) | [manifest](verification/058/arm64-SHA256SUMS.txt), [metadata](verification/058/arm64-metadata.json) | [manifest](verification/058/intel-SHA256SUMS.txt), [metadata](verification/058/intel-metadata.json) |
| All 13 standard checks | [PASS](verification/058/linux-verification.json) | [PASS](verification/058/arm64-verification.json) | [PASS](verification/058/intel-verification.json) |
| Public signing/notarization | Optional owner signature not requested | Developer ID/notarization UNVERIFIED; linker ad-hoc only | Developer ID/notarization UNVERIFIED; unsigned |

The native Rust backend executable, host OpenSSH and host PTY run on the stated native client architecture. Their dedicated remote Docker server runs in an owned QEMU guest (TCG on hosted runners). No browser mocks, cross-compiles or prerequisite-only probes fill a native backend cell. Exact fixed PASS groups and independent Docker event/state oracles are in each report. All four workloads, owned agents/VMs/private keys/seed/disks are cleaned; source SSH/trust files and the independent owned control master remain unchanged. No host Docker socket, user key or guessed production alias is used.

Linux ordinary package launch also has KVM-backed Ubuntu 24.04.5 evidence: [deb 053](../codex/tracking/evidence/053.md), [AppImage 058](verification/058/appimage-vm.json). The final packages were installed/launched again in separate fresh Ubuntu 24.04.5 **TCG** desktop guests because local `/dev/kvm` access later returned EACCES. Their installed executables exactly match those earlier KVM-tested binaries: deb `81946e3fe231fcbf6a265ef47a3120bee19e9ac549b6bf82e2a195ea4369253d`, AppImage `9d38769d44ae3e45d9314a5a292feeafe6664486c2be7ad8651401f3cdbd3b52`. These are actual package/window tests with native GTK/WebKit in the guest, with software CPU emulation explicitly recorded. They do not establish hardware graphics behavior. [Deb screenshot](verification/058/final-deb-discovery.png), [AppImage screenshot](verification/058/final-appimage-discovery.png), [ARM support screenshot](verification/058/arm64-support.png), [Intel support screenshot](verification/058/intel-support.png).

Physical sleep/wake is unverified; the required alternative **network-interruption** gate passed on each native host. Wayland, physical GPU coverage, full Mac keyboard/VoiceOver journeys, other OS releases, Apple Keychain integration and quarantined-download Gatekeeper acceptance are not claimed. macOS packaging floor 15.0 is distinct from actual 15.7.9 execution. Signing readiness follows [055](signing.md), without supplied credentials. No public-publisher workflow or substitute capability exists. Failed attempts and corrections are recorded in [058 evidence](../codex/tracking/evidence/058.md).
