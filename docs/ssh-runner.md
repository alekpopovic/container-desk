---
title: "Bounded SSH process runner"
section: "Hosts & SSH"
icon: "🔐"
---

# 🔐 Bounded SSH process runner

The Rust SSH module provides a shared Runner, used by the effective-config resolver. It starts a validated absolute executable with argv, null stdin, separate piped stdout/stderr and a dedicated process group. There is no generic execute-command IPC or renderer shell-text parameter. Future remote dispatchers must obtain authorized, scoped operations and use the centralized remote quoting builder before passing argv to this low-level Rust API.

A Runner has four active slots and no waiting queue. An exhausted slot returns Busy immediately. Each Job owns an explicit cancellation sender and a result receiver; cancel(), dropping the Job or dropping its wait future requests cancellation. The background owner holds the slot until cleanup/reaping completes, then returns the result and frees capacity. Cancellation received before dispatch prevents spawning. Completion and cancellation race normally: an already-completed process can return its result.

Snapshot defaults are 30 seconds overall, 16 MiB stdout and 256 KiB stderr. Callers may lower these bounds; zero deadlines, values above these maxima, NUL argv, more than 64 arguments or arguments larger than 128 KiB fail validation. Resolver-specific bounds remain 5 seconds/1 MiB/64 KiB. Readers run concurrently so a full stderr pipe cannot block progress waiting on stdout. Both streams retain partial final records without requiring a newline. Nonzero or signaled exits retain their bounded captured data/status for the internal caller; cancellation, timeout and stream-specific oversized-output errors do not masquerade as empty success. Debug output prints byte counts, not stream contents. No retry exists.

On timeout/cancellation/read failure the owner signals only its still-owned unreaped child's process group, then kills/waits for the direct child. It never signals a numeric group ID after reaping. A drop guard and Tokio kill-on-drop cover interrupted owners; normal acknowledgments await explicit cleanup. Trusted SSH config can still create detached processes or external effects; this is not a sandbox. Cancellation after remote dispatch is not evidence that a future mutation was rolled back, and callers must not replay it.

The structured argv base sets no PTY (-T), null stdin (-n), BatchMode, ConnectTimeout=10, ServerAliveInterval=15 and ServerAliveCountMax=2. It preserves the original alias and exact default/custom -F policy. It explicitly retains strict host checking and disables host-key/IP learning, DNS-only trust, agent/X11/tunnel/extra port forwarding, LocalCommand, inherited control sockets, backgrounding and configured RemoteCommand overrides. Connection reuse must later substitute app-owned control resources through its owner. These options apply to the destination; separate ProxyJump trust/authentication verification remains a later live gate.

The centralized remote quoting/command builder must append exactly one encoded remote command after the base argv. This increment exposes no remote dispatcher, and the base builder was tested using native ssh -G without connecting. The later PTY terminal requires a separate input/resize/permission path and must not use snapshot capture.

The effective resolver's backend owner deliberately retains its diagnostic/mode-transition permit through bounded completion even if its IPC caller disappears. The reusable Job cancellation API is available for upcoming session owners; this increment adds no UI cancel button.

Verification: [011 evidence](https://github.com/alekpopovic/container-desk/blob/main/codex/tracking/evidence/011.md). Synthetic child tests execute a fixed repository fixture through the trusted system /bin/sh interpreter, using script-file arguments, never sh -c. That test-only interpreter is not used by production SSH dispatch. Tests check null stdin/no TTY, large stderr, partial output, nonzero exit, timeout, explicit/drop cancellation, pre-dispatch cancellation, bounds and reaped PIDs. See [Tokio process ownership](https://docs.rs/tokio/latest/tokio/process/struct.Child.html#caveats) and [process groups](https://docs.rs/tokio/latest/tokio/process/struct.Command.html#method.process_group).
