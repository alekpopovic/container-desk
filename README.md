# ContainerDesk

A native desktop client for Docker on Linux servers through your existing OpenSSH configuration. Built with Tauri 2, React/TypeScript and Rust/Tokio. Linux and macOS clients use native SSH, strict host verification and your existing keys/agent; no Docker Desktop, remote agent or exposed Docker TCP listener is required.

Start with the **[Serbian quick start](README_SR.md)** or **[English user guide](docs/user-guide.md)**. Contributors should read [CONTRIBUTING](CONTRIBUTING.md).

Read views include containers, inspect, logs, statistics, Compose groups, images, volumes and networks. Lifecycle/verified existing Compose actions and container terminals require explicit permissions and confirmation. Images/volumes/networks remain read-only. No prune, Compose deployment, Kubernetes, registry credentials, telemetry or automatic updater is included.

[Final handover and package hashes](docs/FINAL_HANDOVER.md) · [Release notes](docs/RELEASE_NOTES.md). All 60 prompts are complete under the documented scope.

[Download ContainerDesk v0.1.0 — unsigned preview](https://github.com/alekpopovic/container-desk/releases/tag/v0.1.0). The owner explicitly authorized this public pre-release after the final handover. Ubuntu 24.04 x86_64 deb/AppImage and macOS 15.7.9 Apple Silicon/Intel app/DMG paths have real execution evidence. The [native platform matrix](docs/platform-matrix.md) also passes actual SSH/Docker/PTY on all three client targets; credentials have not been supplied for Apple signing/notarization. The public-publishing workflow is intentionally omitted. GitHub Actions remain disabled by the owner.

| Install on | Download |
|---|---|
| Ubuntu/Linux x86_64 | [deb](https://github.com/alekpopovic/container-desk/releases/download/v0.1.0/containerdesk-0.1.0-x86_64-unknown-linux-gnu.deb) · [AppImage](https://github.com/alekpopovic/container-desk/releases/download/v0.1.0/containerdesk-0.1.0-x86_64-unknown-linux-gnu.AppImage) |
| Mac Apple Silicon | [DMG](https://github.com/alekpopovic/container-desk/releases/download/v0.1.0/containerdesk-0.1.0-aarch64-apple-darwin.dmg) |
| Mac Intel | [DMG](https://github.com/alekpopovic/container-desk/releases/download/v0.1.0/containerdesk-0.1.0-x86_64-apple-darwin.dmg) |

[SHA256SUMS](https://github.com/alekpopovic/container-desk/releases/download/v0.1.0/SHA256SUMS) · [All assets and install instructions](https://github.com/alekpopovic/container-desk/releases/tag/v0.1.0). Mac app archives are also included. Mac packages are not Developer ID signed/notarized; Gatekeeper may block downloaded apps. See [publication receipt](docs/releases/v0.1.0.md) for exact source, hashes and download verification.

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
