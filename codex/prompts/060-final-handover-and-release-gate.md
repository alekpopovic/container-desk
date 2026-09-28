# 060 — Final handover and release gate

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **059**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 060`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Deliver the complete repository and a truthful handover for operating and maintaining the app.

## Required implementation

1. Produce docs/FINAL_HANDOVER.md listing build/run/install commands, architecture decisions, package locations/hashes and native platform evidence.
2. Summarize verified features, local unsigned versus signed distribution status, known limitations and the next optional enhancements.
3. Validate all prompt statuses and evidence, repository cleanliness without deleting user work, and secret-free release contents.
4. Mark complete only when the agreed app/platform gates are satisfied; public release and any provider accounts remain owner actions unless separately authorized.

## Acceptance and verification

- Run tracker validate and the agreed release smoke checklist against the final artifact set.
- If a required gate is unavailable, record the exact blocker and safe next command; do not substitute a mock/demo screenshot for a native release.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/060.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 060 --evidence codex/tracking/evidence/060.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 060 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
