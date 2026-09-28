# Official references

Reviewed on 2026-09-28. These pages support the technology choices and CLI contracts. The component boundaries, limits, task order and release gates in this pack are proposed engineering decisions. Exact versions are selected and locked during implementation.

| Area | Official source | Use in this pack |
|---|---|---|
| Tauri native IPC | https://v2.tauri.app/develop/calling-rust/ | Rust commands and streaming channels |
| Tauri prerequisites | https://v2.tauri.app/start/prerequisites/ | Platform development dependencies |
| Tauri macOS bundle | https://v2.tauri.app/distribute/macos-application-bundle/ | Native packaging and GUI environment caveat |
| Tauri native testing | https://v2.tauri.app/develop/tests/webdriver/ | Current driver/service choices; standalone driver differs from embedded route |
| Tauri macOS signing | https://v2.tauri.app/distribute/sign/macos/ | Signing/notarization configuration |
| OpenSSH config | https://man.openbsd.org/ssh_config | Host/Include/ProxyJump/IdentityFile/Match/control settings |
| Docker container inventory | https://docs.docker.com/reference/cli/docker/container/ls/ | JSON-template inventory output |
| Docker logs | https://docs.docker.com/reference/cli/docker/container/logs/ | Tail/follow/timestamps/since behavior |
| Docker stats | https://docs.docker.com/reference/cli/docker/container/stats/ | Snapshot/stream options and reported metrics |
| Docker events | https://docs.docker.com/reference/cli/docker/system/events/ | Event stream and filtering |
| Compose projects | https://docs.docker.com/reference/cli/docker/compose/ls/ | Project inventory in JSON |
| Docker SSH access | https://docs.docker.com/engine/security/protect-access/ | Supported SSH daemon access and user permissions |

Consult the docs for the pinned versions as implementation proceeds. Do not paste undocumented CLI flags or assume behavior of an older Tauri test stack. A newer upstream documentation page does not automatically upgrade the project's locked dependencies.
