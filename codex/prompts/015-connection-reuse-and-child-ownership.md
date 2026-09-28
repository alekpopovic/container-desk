# 015 — Connection reuse and child ownership

- Phase: 02 SSH transport
- Recommended reasoning: **high**
- Depends on: **014**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 015`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Reduce SSH overhead while preserving correct process ownership.

## Required implementation

1. Implement an app-owned OpenSSH multiplex master per selected alias/config identity with a private short control-socket path and bounded persistence.
2. Use explicit ControlPath values for app sessions and never issue an exit to a user-owned control master. Keep app instance ownership distinguishable.
3. Ensure log/terminal/command children share a controlled session and can still be cancelled independently; implement a non-multiplex fallback with a visible diagnostic.
4. Apply permissions and path-length limits appropriate to Linux and macOS Unix sockets; clean up only resources created by this app.

## Acceptance and verification

- Confirm repeated read commands reuse the app connection and app exit leaves an unrelated user master running.
- Test a stale socket, long home path, master death and fallback without unbounded reconnect loops.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/015.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 015 --evidence codex/tracking/evidence/015.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 015 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
