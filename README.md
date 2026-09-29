# ContainerDesk

A native desktop client for Docker on Linux servers through your existing OpenSSH configuration. Built with Tauri 2, React/TypeScript and Rust/Tokio. Linux and macOS clients use native SSH, strict host verification and your existing keys/agent; no Docker Desktop, remote agent or exposed Docker TCP listener is required.

Start with the **[Serbian quick start](README_SR.md)** or **[English user guide](docs/user-guide.md)**. Contributors should read [CONTRIBUTING](CONTRIBUTING.md).

Read views include containers, inspect, logs, statistics, Compose groups, images, volumes and networks. Lifecycle/verified existing Compose actions and container terminals require explicit permissions and confirmation. Images/volumes/networks remain read-only. No prune, Compose deployment, Kubernetes, registry credentials, telemetry or automatic updater is included.

[Final handover and package hashes](docs/FINAL_HANDOVER.md) · [Release notes](docs/RELEASE_NOTES.md). All 60 prompts are complete under the documented scope.

Current installers are development artifacts, not an approved public release. Ubuntu 24.04 x86_64 deb/AppImage and macOS 15.7.9 Apple Silicon/Intel app/DMG paths have real execution evidence. The [native platform matrix](docs/platform-matrix.md) also passes actual SSH/Docker/PTY on all three client targets; credentials have not been supplied for Apple signing/notarization. The public-publishing workflow is intentionally omitted.

| Need | Documentation |
|---|---|
| Actual platform/feature state | [Project status](docs/project-status.md) |
| Linux / Mac packages | [Linux](docs/linux-packages.md), [macOS](docs/macos-packages.md) |
| SSH, rootless/context/sudo, troubleshooting | [User guide](docs/user-guide.md) |
| Manual updates and data backup | [Versions and rollback](docs/updates.md) |
| Optional signing readiness | [Signing](docs/signing.md) |
| Reproducible development checks | [Verification command](docs/verification-command.md) |
| Disposable native SSH/Docker lab | [Integration lab](docs/integration-lab.md) |
| Architecture / security boundaries | [Architecture](codex/docs/ARCHITECTURE.md), [security review](docs/security-review.md) |

`codex/` retains the original prompt pack and immutable task hashes. Application version is defined in `package.json`; current distribution/release readiness comes from executed evidence, not the original task descriptions. ContainerDesk is a working name, not a trademark/domain-ownership claim.
