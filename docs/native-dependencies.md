---
title: "Native dependency diagnostics"
section: "Hosts & SSH"
icon: "🔐"
---

# 🔐 Native dependency diagnostics

Settings → Native dependencies runs a fixed local OpenSSH version probe and a Unix agent-socket reachability check. The renderer receives app version, OS/architecture, selected executable path, bounded OpenSSH version and safe status messages. Local Docker, jq, Python, Rust and Node are not runtime dependencies; Python is only used by the development tracker.

The default path is `/usr/bin/ssh`; no shell or PATH lookup is used. A user may explicitly enter an absolute override or clear it to restore the default. Before saving, Rust resolves the canonical file, requires a regular executable owned by the effective user or root, rejects group/world write access and unsafe parent directories, and checks `ssh -V`. Root-owned sticky directories such as /tmp are allowed above a private owned directory. Canonical paths are used for execution so changing the original symlink does not redirect that particular probe. This validates local trust/availability, not a cryptographic identity; users must choose executables they trust. Host SSH operations will reuse the validator and still require their separate session/policy gates.

The process is spawned directly with the one-element `-V` argument array, null stdin and piped stdout/stderr. There is a three-second timeout, 4 KiB per output channel and one shared diagnostic/override slot per app (excess calls fail without queueing). Failed probes are killed and reaped; no retry occurs. Only a UTF-8 single-line OpenSSH version up to 512 bytes is returned. Unrecognized output and raw failure stderr never enter IPC, persistence or app logs. This local version check is not the later general SSH process/session manager; it does not connect to a host or resolve SSH config.

`SSH_AUTH_SOCK` is checked for absence, missing path, socket type and bounded local connection accessibility (500 ms). The connection is closed without sending agent protocol bytes or reading identities/keys. Reachability does not prove a usable identity or server authentication. The socket path is not included in the report.

`dependency_diagnostics` and `set_ssh_executable` are the only new capabilities, scoped to the local main window. Invalid overrides return a safe diagnosis without updating preferences. Successful saves use the existing store revision check, so a concurrent settings edit cannot be overwritten after the version probe.

Adding the first child-process probe exposed a storage-lock lifetime race in parallel tests: a just-forked child can briefly inherit a flock descriptor before exec closes it. FileStorage now explicitly unlocks on owner drop. A deterministic duplicated-descriptor regression test covers this window, while normal concurrent-store rejection remains tested.

[006 evidence](https://github.com/alekpopovic/container-desk/blob/main/codex/tracking/evidence/006.md) records native Linux execution with PATH=/nonexistent and SSH_AUTH_SOCK unset. No production host, SSH configuration or private keys were accessed. macOS and baseline Ubuntu 24.04 remain separate native gates.
