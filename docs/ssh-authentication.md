---
title: "SSH authentication and trust"
section: "Hosts & SSH"
icon: "🔐"
---

# 🔐 SSH authentication and trust

Settings → select a trusted alias → **Check SSH access** explicitly opens a connection and runs one fixed, POSIX-quoted `printf` command. A successful marker means SSH authentication and remote command execution worked. It does not create a persistent host session or prove Docker readiness. The check has a 15-second overall deadline, 1 KiB stdout and 64 KiB stderr limits, shares the single native diagnostic gate, and is disabled by Rust in demo mode. It never retries automatically.

The backend references the original selected configuration from an app-owned private policy file. A leading `Host *` applies strict verification, BatchMode, no password/keyboard-interactive fallback, no agent/X11 forwarding, no inherited control socket and no local-command hook. Native ProxyJump subprocesses inherit this `-F` policy; the live acceptance test verifies destination and jump failures independently, including deliberately permissive original settings. Original IdentityFile/IdentityAgent references are retained. The app never reads or copies private keys, supplies passphrases, or changes SSH config/known_hosts.

The directory uses a random 128-bit name under `/tmp`, mode 0700; the policy file is 0600. A symlink references the selected config without copying its contents or interpolating its filename into Include syntax. The process owner retains this resource until SSH cleanup/reaping. Removal touches only the exact app-owned entries and checks directory identity. Crash-orphan cleanup belongs to the later session/resource-owner increment.

Structured children start in a new session with no controlling terminal, null stdin, and child-local `SSH_ASKPASS_REQUIRE=never`; askpass/display variables are removed while the user's agent socket remains available. Deadline/cancellation kills the owned process group and reaps its leader. The app does not sandbox arbitrary trusted `ProxyCommand`, `Match exec`, agent helpers or programs that deliberately escape this group. Those programs can have their own behavior/side effects. A custom ProxyCommand launching another SSH client must itself preserve strict host checks and unattended behavior; only native ProxyJump propagation is covered by this policy and lab.

Explicit custom configs retain native `-F` behavior (no implicit system config). Default selections include the existing user config and then `/etc/ssh/ssh_config`. Relative user Includes retain the ~/.ssh base. A relative Include within a system config is an implementation limitation of the user-policy overlay: use absolute system Include paths; that nonstandard setup is not covered by the current acceptance. Files on an unresponsive filesystem can stall OS metadata operations; the SSH process deadline starts after policy preparation.

## Terminal setup

1. Use the same native OpenSSH executable and selected config in your own terminal. For a custom config use `ssh -F /absolute/path/to/config jump-alias`; for native defaults use `ssh jump-alias`. These are examples: substitute your own chosen aliases and safely quote paths containing spaces.
2. Check each jump host independently, then the destination. Obtain each expected host-key fingerprint from its administrator through an independent trusted channel. Compare it before accepting a new key. Do not disable verification or treat an unverified key scan as proof.
3. For a changed or revoked key, investigate the cause with the administrator. Only after independently verifying a legitimate rotation, repair the specific known_hosts entry yourself. ContainerDesk does not remove entries or approve keys.
4. Load an encrypted key with your normal OS agent (`ssh-add /absolute/path/to/key`, using your terminal's safe quoting). Make the same agent available to the desktop app. The app does not collect the passphrase or forward the agent to the server.
5. Return to Settings and explicitly press **Check SSH access** again.

The UI shows a bounded recognized SSH diagnostic phrase and a typed result. Arbitrary stderr, banners, key paths and remote output are not returned or persisted. The result may refer to the destination **or** a jump: stderr is not reliable evidence of which hop failed. Changed/unknown/rejected keys take priority over generic connection/authentication errors.

## Verification

`python3 tests/lab/ssh_auth.py` builds a development-only pinned-base Alpine/OpenSSH image, creates two labelled disposable servers on a dedicated internal network, creates temporary host/client keys and a private agent, and invokes the actual Rust `auth::probe` path with native OpenSSH. The local Docker socket is explicit; no user's registry configuration, SSH aliases or identities are used. No ports are published. Docker is a development test requirement only.

The 15 cases cover direct and ProxyJump trust failures, absent keys, encrypted keys without/with agent loading, and an authenticated forced-command failure. The driver also runs native session-state checks. A hostile inherited askpass fixture must never be invoked, every local case must return within eight seconds, and config/known_hosts SHA-256 values must remain identical. The driver removes its exact containers, network, agent and temporary directory even after failure; the labelled image/build cache is retained for reuse. It never prunes unrelated resources. The native lab test is ignored in ordinary `cargo test` and must be run explicitly to claim this acceptance. Browser fixture tests cover presentation only; macOS behavior remains unverified.

References: [OpenSSH configuration and ProxyJump semantics](https://man.openbsd.org/ssh_config.5), [OpenSSH invocation and askpass](https://man.openbsd.org/ssh.1), [ssh-add and agent loading](https://man.openbsd.org/ssh-add.1).
