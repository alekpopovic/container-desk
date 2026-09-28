# 031 — Mutation intents and local activity records

- Phase: 04 Management
- Recommended reasoning: **high**
- Depends on: **030**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 031`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Prepare controlled start/stop/restart actions.

## Required implementation

1. Implement backend-issued short-lived confirmation intents tied to host, daemon identity, session generation, action and exact target IDs.
2. Require host write mode plus a consumed confirmation intent for mutations; prevent double-click duplication with an operation lock.
3. Record a bounded sanitized local activity history with action, time, target and outcome, including unknown outcome after transport loss.
4. Do not call this history a tamper-proof audit log; do not put environment values, full commands or terminal content in it.

## Acceptance and verification

- Test expired/reused intents, host switches after confirmation and direct IPC calls that bypass UI.
- Verify interrupted mutations are marked unknown and are never automatically replayed.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/031.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 031 --evidence codex/tracking/evidence/031.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 031 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
