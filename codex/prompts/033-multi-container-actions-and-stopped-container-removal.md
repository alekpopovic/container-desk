# 033 — Multi-container actions and stopped-container removal

- Phase: 04 Management
- Recommended reasoning: **high**
- Depends on: **032**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 033`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Handle deliberate small batches without hiding partial failures.

## Required implementation

1. Add bounded explicit selections for start/stop/restart with a confirmation listing affected IDs and host.
2. Execute with conservative concurrency and individual outcomes; allow cancellation of pending work and report operations already dispatched.
3. Add optional removal only for currently stopped containers with a separate confirmation, no force flag and no volume-removal flag.
4. Prevent selection across hosts and recheck container state immediately before removal; keep partial outcomes in activity history.

## Acceptance and verification

- Test a batch with success, disappearance and permission error and ensure each result remains visible.
- Attempt removing a running container and verify the application refuses its supported removal flow.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/033.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 033 --evidence codex/tracking/evidence/033.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 033 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
