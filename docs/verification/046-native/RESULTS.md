---
title: "Native Linux 046 results"
section: "Reviews & evidence"
icon: "🧪"
historical: true
---

# 🧪 Native Linux 046 results

2026-09-29, Ubuntu 26.04.1 x86_64 / WebKitGTK 2.52.6, release binary SHA-256 `5765040b70cfdc153f799b8ae763b867e09194ffa6eeda4a3c101223e6065031`.

Command: `python3 tests/lab/checkpoint.py --tools-dir /tmp/containerdesk-025-tools --artifacts docs/verification/046-native`, exit **0**. The owned app runs without client Docker/Node/Python/Cargo on PATH; the lab creates eight disposable containers and two strict SSH hops. No production alias is selected.

Final native screenshots were visually reviewed: terminal text is readable in its actual dark surface. `native-cleanup.json` records the computed theme/font, blocked inline script, one log/one terminal stream reaped and all nine observed owned SSH identities absent after disconnect, zero zombies. RSS/PSS stage snapshots are diagnostic, not a new benchmark or a claim of memory returning to baseline. One transient process PSS was unavailable.

Sanitized command results (environment portal/fuse/PipeWire and session teardown noise omitted):

```text
PASS native discovery: three literal aliases from owned config; explicitly selected/resolved destination retains ProxyJump.
PASS native Compose UI: explicit quoted remote files/name, verification, management opt-in, full host/daemon/service/ID confirmation, cancel dispatched nothing, exactly one restart, both services observed running, explicit return to read-only.
PASS integrated native session: same saved host/daemon/full container identity; actual logs and reaped stream; cancelled then single stop/start; verified Compose restart; cancelled then non-root PTY; terminal tab closure under output; all observed owned SSH identities reaped after disconnect.
PASS integrated native app storage/diagnostics contain no terminal transcript markers.
PASS native PTY: separate terminal permission and exact one-use intent; real non-root UID 1000, interactive echo, Ctrl-C, 111x37 resize, exit 7, shell-less running container typed failure, duplicate input rejected, JSON inventory stays non-PTY, revoke/disconnect reaps owned local SSH, no replay or transcript in activity/storage; PATH=/nonexistent.
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 175 filtered out; finished in 15.39s
PASS native verified Compose over strict ProxyJump: spaces/apostrophes and ordered files, absent required env file and wrong project rejected, reversed file order rejected, readonly/confirmation enforced, two existing services restarted once, stopped, then started with each actual state observed, consumed intent not replayed, actual config drift after confirmation failed before dispatch, no resolved secret in returned activity.
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 175 filtered out; finished in 22.20s
PASS independent PTY gate: 3 explicit non-root shell command(s), exactly one missing-shell attempt, no fallback, both container primary processes remain running.
PASS independent Compose oracle: 4 explicit lifecycle command(s); both owned services running with newer start times; both unrelated project services retain original start times; no deployment/build/pull/down command admitted by app gate.
PASS actual ProxyJump logs-jump: strict trust on both owned SSH hops, no agent forwarding, same scoped read workflow.
PASS: real non-root PTY, echo, Ctrl-C, resize, exit, missing shell, scope and revocation; structured commands retain no PTY.
Cleaned only owned log containers, loopback sshd and temporary keys/data.
```

Earlier attempts and corrected defects are described in the [checkpoint](../../checkpoints/046-feature-complete.md). Native macOS, baseline installers, physical sleep/wake and public signing remain pending.
