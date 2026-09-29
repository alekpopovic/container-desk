# Executable acceptance checkpoints

Defined in prompt 001. These are acceptance procedures, not execution evidence by themselves. Checkpoints [018](checkpoints/018-ssh.md), [030](checkpoints/030-read-only.md), [038](checkpoints/038-management.md) and [046](checkpoints/046-feature-complete.md) have separate native Linux results. Final release checkpoint 060 is complete; [handover](FINAL_HANDOVER.md) and [058 matrix](platform-matrix.md) link actual artifacts and distinguish native backend, package GUI, emulated guests and remaining limits. Procedures below alone are not execution evidence. The immutable prompt remains authoritative. Each run records exact app commit, OS/architecture, OpenSSH/Docker/Compose versions, steps, observed outcomes and sanitized evidence in the stated file plus `codex/tracking/evidence/NNN.md`.

Use an explicitly disposable Linux target and bastion, temporary lab identities and a dedicated known_hosts file outside the repository. Record lab resources and ownership before creating/removing any. Independently verify fingerprints and provision target/jump access; do not disable host verification. Never select or guess a production alias. For tests that exclude local Docker/jq, launch the built app in a controlled client environment without those executables while retaining native SSH and required runtime libraries.

## 018 — SSH foundation

Prerequisite: 017 complete; native application and a live disposable direct/ProxyJump lab available.

1. Start the native app, select the lab's direct alias and connect in read-only mode. Record the resolved SSH target, daemon identity and bounded Docker version/list probe.
2. Connect to the private target through the explicit bastion alias; repeat the probe without Docker/jq on the app client's PATH. Require both aliases to work and retain strict verification on each hop.
3. Using isolated lab trust data, exercise unknown and changed host keys and missing authentication. Require useful bounded failures; no invisible password prompt, trust-file mutation or bypass.
4. Cancel a pending probe, disconnect and reconnect. Verify app-owned children/sockets are cleaned, unrelated SSH masters survive and old-generation results cannot populate the new session.
5. Run the implemented deterministic quoting, policy and transport tests and record their actual commands separately from the live results.

Pass: the complete live slice works on the available native OS; cross-platform cells remain explicit. Missing live lab blocks 018. Output: `docs/checkpoints/018-ssh.md`.

## 030 — Read-only MVP

Prerequisite: 029 complete, 018 lab reusable, known disposable containers (including one generating logs).

1. Walk direct and ProxyJump connections through host selection, container list, inspect, health, ports, logs, stats, events and Compose grouping.
2. Follow logs, cancel, switch hosts and reconnect. Confirm bounded retention and visible gaps/stale state; compare inspect fields and sampled statistics against the lab Docker CLI.
3. Repeat via the jump host without client Docker/jq. Exercise an empty daemon, a denied Docker user and disconnected target. Require distinct empty/error/stale states.
4. Review the registered backend operations and verify attempts to mutate a read-only host are rejected. Confirm inspect environment values are masked before default IPC and no read operation changes daemon state.

Pass: integrated live read workflow and error paths run on the available native OS; missing live workflow blocks 030. Output: `docs/checkpoints/030-read-only.md` with sanitized screenshots/results.

## 038 — Management MVP

Prerequisite: 037 complete; explicitly disposable running/stopped containers and verified existing Compose project.

1. Select the lab host, explicitly enable management, confirm exact lifecycle targets, invoke start/stop/restart, and read back actual state. Return to read-only and prove every mutation endpoint is denied by Rust.
2. Remove only selected stopped containers without force or volume removal. Attempt stale/running selection and confirm rejection. Keep image/volume/network views read-only.
3. Verify the Compose project path/config/name/services; exercise only start/stop/restart of existing services. Require clear failure for unverified project metadata.
4. Exercise batch partial failure, stale selection and changed daemon identity. Drop a dispatched action's response in the disposable lab: require unknown outcome, no replay and explicit read reconciliation.

Pass: live lifecycle/Compose flows and backend policy checks pass; any incomplete gate blocks 038. Output: `docs/checkpoints/038-management.md`.

## 046 — Complete feature workflow

Prerequisite: 045 complete and the same disposable jump-host setup.

1. In one native app journey, discover/select a host, read live resources, enable management, act on verified Compose services and explicitly enable a container terminal for that host.
2. Type benign terminal input, resize, send Ctrl-C, close and reopen. Exercise network loss, sleep/wake and host switching. Require no input replay and cleanup of owned PTY/SSH children and log subscriptions.
3. Confirm switching to read-only revokes mutation/terminal access; verify host/daemon/session identity remains consistent across tabs.
4. Inspect policy coverage, bounded stream behavior, diagnostics redaction and accessibility. Record features as implemented, verified or platform-pending; remove placeholder controls and unsupported success messages.

Pass: all core features run on the available native OS with cleanup evidence; missing core features block 046. Platform release gates remain pending. Output: `docs/checkpoints/046-feature-complete.md` and feature matrix.

## 060 — Release candidate and final handover

Prerequisite: 059 complete, native package artifacts and acceptance evidence for Linux x86_64, macOS arm64 and macOS x86_64.

1. Build on the documented native baselines using the commands established in 002/051–054. Record package paths, SHA-256, exact build toolchains and source commit. Never treat a CI file as an executed build.
2. Install and launch deb/AppImage and Mac app/DMG on their target platforms. Run the agreed direct/ProxyJump, read, management, terminal, interruption/cleanup and redacted-export smoke journey against disposable resources, including GUI launch with a minimal environment.
3. Review each required native acceptance cell from 058 against evidence. Record local unsigned package verification separately from signing/notarization; public distribution remains an owner action unless separately authorized.
4. Run `python3 codex/scripts/track.py validate`; review source/artifact contents for secrets and record `git status --short` without deleting unrelated work. Document verified features, unresolved limits and exact build/run/install commands in `docs/FINAL_HANDOVER.md`.
5. Mark 060 done only when required app/platform gates pass, then commit its changes and push per `AGENTS.md`; validate tracker and repository state again in the final handover.

Pass: actual final artifacts satisfy all required native gates. An unavailable required OS, artifact or smoke check blocks 060 with a specific next action; fixtures cannot substitute.
