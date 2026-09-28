# 014 — Host connection state machine

- Phase: 02 SSH transport
- Recommended reasoning: **high**
- Depends on: **013**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 014`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Present connection progress and cancellation reliably.

## Required implementation

1. Implement disconnected, resolving, connecting, probing, ready, degraded and error states with an incrementing session generation.
2. Ensure connect cancellation, repeated clicks and switching hosts cannot attach an old completion to a new session.
3. Add structured stage durations and sanitized diagnostics so jump failure, authentication failure and remote-command failure are understandable.
4. Keep persisted host metadata separate from transient connection state; avoid automatically connecting every saved host at launch.

## Acceptance and verification

- Simulate slow resolution, successful connect followed by disconnect, and switching hosts during a probe.
- Verify the UI and Rust state agree and stale callbacks cannot change the selected host state.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/014.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 014 --evidence codex/tracking/evidence/014.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 014 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
