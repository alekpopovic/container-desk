# Roadmap

| IDs | Milestone | Completion means |
|---|---|---|
| 001–008 | Foundations | Native shell, typed boundary, policy and fixtures |
| 009–018 | SSH | Direct and ProxyJump probes against a real lab |
| 019–030 | Read-only MVP | Containers, inspect, logs, stats, events, Compose groups |
| 031–038 | Management MVP | Lifecycle actions, resource views, verified Compose actions |
| 039–046 | Feature complete | PTY terminal, recovery, diagnostics, accessibility |
| 047–060 | Release candidate | Reviews, integration tests, native packages, platform evidence |

Dependencies are intentionally sequential. Missing native environments must remain explicit blockers at their acceptance gates.

| ID | Prompt | Reasoning |
|---|---|---|
| 001 | [Repository audit and implementation contract](../prompts/001-repository-audit-and-implementation-contract.md) | high |
| 002 | [Tauri React TypeScript scaffold](../prompts/002-tauri-react-typescript-scaffold.md) | medium |
| 003 | [Application layout and design tokens](../prompts/003-application-layout-and-design-tokens.md) | medium |
| 004 | [Typed domain models and IPC boundary](../prompts/004-typed-domain-models-and-ipc-boundary.md) | high |
| 005 | [Local settings and host metadata store](../prompts/005-local-settings-and-host-metadata-store.md) | high |
| 006 | [Native dependency diagnostics](../prompts/006-native-dependency-diagnostics.md) | medium |
| 007 | [Backend operation policy and command registry](../prompts/007-backend-operation-policy-and-command-registry.md) | high |
| 008 | [Synthetic fixtures and offline development mode](../prompts/008-synthetic-fixtures-and-offline-development-mode.md) | medium |
| 009 | [SSH config discovery and host candidates](../prompts/009-ssh-config-discovery-and-host-candidates.md) | high |
| 010 | [Effective SSH configuration resolution](../prompts/010-effective-ssh-configuration-resolution.md) | high |
| 011 | [OpenSSH subprocess runner](../prompts/011-openssh-subprocess-runner.md) | high |
| 012 | [Remote argument quoting and Docker command builders](../prompts/012-remote-argument-quoting-and-docker-command-builders.md) | high |
| 013 | [SSH authentication and host trust flow](../prompts/013-ssh-authentication-and-host-trust-flow.md) | high |
| 014 | [Host connection state machine](../prompts/014-host-connection-state-machine.md) | high |
| 015 | [Connection reuse and child ownership](../prompts/015-connection-reuse-and-child-ownership.md) | high |
| 016 | [Remote Docker capability probe](../prompts/016-remote-docker-capability-probe.md) | high |
| 017 | [Host inventory screen and groups](../prompts/017-host-inventory-screen-and-groups.md) | medium |
| 018 | [SSH vertical slice checkpoint](../prompts/018-ssh-vertical-slice-checkpoint.md) | high |
| 019 | [Container listing adapter](../prompts/019-container-listing-adapter.md) | high |
| 020 | [Container table and host-scoped cache](../prompts/020-container-table-and-host-scoped-cache.md) | medium |
| 021 | [Container inspect adapter and details](../prompts/021-container-inspect-adapter-and-details.md) | high |
| 022 | [Health, ports, mounts and environment panels](../prompts/022-health-ports-mounts-and-environment-panels.md) | medium |
| 023 | [Bounded log snapshot retrieval](../prompts/023-bounded-log-snapshot-retrieval.md) | high |
| 024 | [Live log subscriptions and cancellation](../prompts/024-live-log-subscriptions-and-cancellation.md) | high |
| 025 | [Log viewer usability and export](../prompts/025-log-viewer-usability-and-export.md) | medium |
| 026 | [Container resource statistics](../prompts/026-container-resource-statistics.md) | high |
| 027 | [Docker event stream and inventory invalidation](../prompts/027-docker-event-stream-and-inventory-invalidation.md) | high |
| 028 | [Read refresh scheduling and stale data](../prompts/028-read-refresh-scheduling-and-stale-data.md) | high |
| 029 | [Compose project discovery and read views](../prompts/029-compose-project-discovery-and-read-views.md) | high |
| 030 | [Read-only MVP checkpoint](../prompts/030-read-only-mvp-checkpoint.md) | high |
| 031 | [Mutation intents and local activity records](../prompts/031-mutation-intents-and-local-activity-records.md) | high |
| 032 | [Container start stop and restart](../prompts/032-container-start-stop-and-restart.md) | high |
| 033 | [Multi-container actions and stopped-container removal](../prompts/033-multi-container-actions-and-stopped-container-removal.md) | high |
| 034 | [Image inventory and inspection](../prompts/034-image-inventory-and-inspection.md) | medium |
| 035 | [Volume inventory and mount relationships](../prompts/035-volume-inventory-and-mount-relationships.md) | medium |
| 036 | [Network inventory and container attachments](../prompts/036-network-inventory-and-container-attachments.md) | medium |
| 037 | [Verified remote Compose project actions](../prompts/037-verified-remote-compose-project-actions.md) | high |
| 038 | [Management MVP checkpoint](../prompts/038-management-mvp-checkpoint.md) | high |
| 039 | [PTY terminal transport](../prompts/039-pty-terminal-transport.md) | high |
| 040 | [Terminal UI with bounded lifecycle](../prompts/040-terminal-ui-with-bounded-lifecycle.md) | high |
| 041 | [Sleep wake network loss and graceful shutdown](../prompts/041-sleep-wake-network-loss-and-graceful-shutdown.md) | high |
| 042 | [Cross-platform GUI launch and SSH agent behavior](../prompts/042-cross-platform-gui-launch-and-ssh-agent-behavior.md) | high |
| 043 | [Support diagnostics and redacted export](../prompts/043-support-diagnostics-and-redacted-export.md) | high |
| 044 | [Accessibility themes and keyboard workflow](../prompts/044-accessibility-themes-and-keyboard-workflow.md) | medium |
| 045 | [Large inventories and stream pressure](../prompts/045-large-inventories-and-stream-pressure.md) | high |
| 046 | [Feature-complete desktop checkpoint](../prompts/046-feature-complete-desktop-checkpoint.md) | high |
| 047 | [Focused security review](../prompts/047-focused-security-review.md) | high |
| 048 | [Frontend and parser regression coverage](../prompts/048-frontend-and-parser-regression-coverage.md) | high |
| 049 | [Disposable direct and bastion integration lab](../prompts/049-disposable-direct-and-bastion-integration-lab.md) | high |
| 050 | [Native desktop integration tests](../prompts/050-native-desktop-integration-tests.md) | high |
| 051 | [Local verification command and clean builds](../prompts/051-local-verification-command-and-clean-builds.md) | medium |
| 052 | [Linux and macOS CI build matrix](../prompts/052-linux-and-macos-ci-build-matrix.md) | high |
| 053 | [Linux package builds](../prompts/053-linux-package-builds.md) | high |
| 054 | [macOS application and DMG builds](../prompts/054-macos-application-and-dmg-builds.md) | high |
| 055 | [Signing and notarization integration](../prompts/055-signing-and-notarization-integration.md) | high |
| 056 | [Versioning updates and rollback guidance](../prompts/056-versioning-updates-and-rollback-guidance.md) | medium |
| 057 | [User and contributor documentation](../prompts/057-user-and-contributor-documentation.md) | medium |
| 058 | [Native platform acceptance matrix](../prompts/058-native-platform-acceptance-matrix.md) | high |
| 059 | [Release candidate review and defect closure](../prompts/059-release-candidate-review-and-defect-closure.md) | high |
| 060 | [Final handover and release gate](../prompts/060-final-handover-and-release-gate.md) | high |
