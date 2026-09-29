# Container terminal transport

Prompt 039 adds a separate Rust PTY transport. The terminal screen follows in 040. The app launches the validated native OpenSSH executable directly in a local PTY provided by pinned `portable-pty 0.9.0`. It reuses only its own SSH connection identity/socket, requests the remote PTY with `-tt`, enables stdin explicitly and disables OpenSSH escape commands with `EscapeChar=none`. Structured JSON commands retain `-T`/`-n`, pipe capture and their existing bounds.

## Permission, identity and command

Management must first be enabled for the selected live host. A second explicit terminal grant is required; ordinary management and read-only mode cannot prepare or open a terminal. Opening consumes a 30-second one-use intent bound to the full host/session/daemon/container/shell/size tuple. Preparation and dispatch verify that the exact container is running; dispatch obtains a fresh daemon binding. No arbitrary command text, user name, executable or environment is accepted from the renderer at opening.

The remote command is centrally POSIX-quoted and fixed to Docker exec with interactive and TTY flags, `--user 1000:1000`, the full validated container ID, and either `/bin/sh` or `/bin/bash`. UID/GID 1000 is deliberately non-root even when the image defaults to root; a suitable installed shell and readable working directory are required. User selection/root override is not offered in this increment. Missing shell startup returns a typed error (initial exit 127); the app does not inject a fallback script or automatically try another shell. Other exits retain the numeric status and terminal output for the active user.

Terminal permission does not reduce the SSH/Docker account's underlying rights. Once enabled, interactive input can change container state. SSH configuration remains the user's trusted executable configuration. Existing keys/agent and strict host verification are preserved; no passphrases, agent forwarding or automatic known_hosts changes are introduced.

## Bounded owner and IPC

| Resource | Bound / behavior |
|---|---|
| Terminal admission | One per host, three globally, including starting/closing sessions. Admission is reserved before native preflight and released only after worker completion or failed preparation. |
| Local processes | One OpenSSH client in a new local PTY session; no local command shell. One owner thread performs nonblocking PTY reads/writes, resize, cancellation and child reaping. |
| Input | At most 16 KiB per request, 16 queued requests plus one partially written request. Exactly increasing `u32` sequence; duplicate/out-of-order input is rejected. Bytes already written are never written again. |
| Output | 256 KiB in memory, at most 32 KiB per explicit IPC read. Backpressure stops PTY reads; five seconds continuously full closes the terminal with a resource-limit error. No unbounded push channel or transcript retention. |
| Consumer / idle lease | Ten seconds without output polling closes an abandoned consumer. Ten minutes without accepted input closes an idle terminal. Polling and input do not create another terminal. |
| Size | Columns 20–500, rows 5–300. Resize requests coalesce to the most recent size; the PTY's kernel resize reaches OpenSSH and the remote Docker exec terminal. |
| Close | Stops accepting input, signals only the owned local SSH process group and waits/reaps the child. IPC close waits up to three seconds; an unfinished worker keeps admission and reports timeout. |
| Exit | Output records contain state, sequence, optional exit code and static error. Consumers drain remaining buffered output after `exited` before finishing. No terminal restart/input replay. |

The generated contracts expose `get/set_terminal_permission`, `open_container_terminal`, `read_terminal`, `write_terminal`, `resize_terminal` and `close_terminal` through narrow main-window capabilities. Every handle carries its complete session scope. Input and resize recheck the live terminal grant; the worker rechecks before writes and on every loop. Revocation, session invalidation, disconnect, abandoned consumption and app shutdown close the local owner. An exact old handle may still close its own resource after disconnect. Failed/unknown input responses require explicit closure and a newly confirmed terminal, never resubmission.

PTY stdout and stderr are one byte stream. Bytes remain untrusted terminal data for the renderer to interpret; they are not HTML or application diagnostics. No terminal transcript or input enters preferences, activity history, telemetry or default diagnostics. Rust Debug representations expose byte counts, not payloads. Closing SSH cannot undo commands already sent or guarantee termination of independently detached remote workloads.

## Evidence and platform scope

[039 evidence](../codex/tracking/evidence/039.md) records actual Linux PTY tests over both direct OpenSSH and ProxyJump. They verify output distinct from echoed input, UID 1000, Ctrl-C cancelling sleep, resize observed through remote stty, exit 7, an actually shell-less running container, rejection after permission revocation, disconnect/reaping and a concurrent JSON inventory request without a PTY. The lab keeps both primary container processes running; only test-created resources are removed.

The native implementation is tested on Ubuntu 26.04.1 x86_64. The crate supports Unix PTYs, including macOS, but this is not macOS runtime, desktop terminal UI, package-install, Wayland or signing evidence. Production compilation and native backend execution are recorded separately. The selected UID can be unsuitable for an image, and other operators can stop/replace the container after preflight; failures remain visible and are never replayed.

References checked for this increment: [portable-pty 0.9.0 source/API](https://docs.rs/crate/portable-pty/0.9.0/source/src/lib.rs), [OpenSSH terminal and escape options](https://man.openbsd.org/ssh.1), [Docker exec options](https://docs.docker.com/reference/cli/docker/container/exec/).
