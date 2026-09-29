# Bounded log snapshots

The live `container_logs` command reads one full container ID in the current host/daemon/session scope. Rust authorizes it as read-only, requires a live session, and verifies the actual Docker binding before and after the operation. Four shared read permits bound concurrent work. A selection/session change fences late IPC replies; disconnect cancels owned SSH channels.

The fixed command is `docker logs --timestamps --tail N [--since S] [--until U] -- ID`, encoded by the existing central POSIX argument encoder. Tail must be 1–20,000, never zero or `all`. The total operation deadline is 1–30 seconds including binding checks. Range values are UTC Unix seconds, optionally with 1–9 decimal fraction digits, at most 253402300799 seconds; since must not exceed until. Relative dates, local date strings, flags and shell syntax are rejected. No follow or details option is enabled.

Docker documents timestamp, tail and range behavior in its [container logs reference](https://docs.docker.com/reference/cli/docker/container/logs/). Omitting details avoids adding configured label/environment metadata. Logs themselves can still contain application secrets; they are returned only as transient text and excluded from Debug output, preferences and diagnostics.

## Channels and limits

A dedicated capture profile allows at most 8 MiB on stdout and 8 MiB on stderr. Crossing either cap terminates/reaps the owned job and reports a resource limit; it does not return an apparently complete partial snapshot. Other structured commands retain their 256 KiB diagnostic stderr limit.

Successful capture is decoded into records with text, optional Docker-format timestamp, channel, truncation and invalid-UTF-8 flags. Stderr is always labelled ambiguous: application stderr and transport/daemon diagnostics share that pipe. A nonzero process exit returns a static scoped error instead of log content. Known unsupported-driver and missing-container errors are distinguished; SSH exit 255 reports disconnection. Other diagnostic variants remain generic transport errors.

Each record retains at most 256 KiB UTF-8 text. Invalid bytes are replaced, with an explicit flag; truncation is separately flagged. Timestamp recognition checks Docker's fixed RFC3339Nano shape, not arbitrary application date semantics. Records are stably sorted by timestamp; absent timestamps follow dated records. Equal timestamps preserve per-channel order with stdout before stderr. Original cross-channel arrival order cannot be reconstructed reliably.

Retention keeps at most 20,000 records and 8 MiB of text plus timestamp bytes, dropping oldest entries and reporting their count. Parsing also bounds intermediate per-channel queues. Rust and the IPC decoder enforce the same output bounds. This is a snapshot API; the next prompt owns presentation, markers and view controls. No export/persistence or live-follow feature is added here.

## Verification laboratory

`tests/lab/logs.py --sshd-root PATH` creates exactly two labelled, network-disabled disposable containers on the explicit local Docker Unix socket, one json-file and one none driver. It uses only their returned full IDs. An owned nonroot loopback OpenSSH server admits fixed read probes and those IDs through a fixture-only ForceCommand gate. The host socket is never mounted into a target; the application does not require the gate or Python at runtime. The application test runs with PATH=/nonexistent and an explicit native SSH executable.

The independent native Docker CLI output is compared with the backend snapshot. Synthetic workload output includes ordered stdout/stderr, a large stderr record and an invalid input byte. Docker json-file itself normalizes that byte; raw invalid-UTF-8 handling is therefore proved by parser tests, not falsely attributed to real raw Engine output. Temporary CLI oracle files and generated SSH keys are deleted along with only owned resources. Client host verification remains strict, and known_hosts is checked unchanged. The temporary server uses StrictModes no solely for its private directory below /tmp; it has no password authentication or forwarding and changes no system service or user SSH configuration.
