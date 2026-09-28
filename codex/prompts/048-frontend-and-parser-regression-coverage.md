# 048 — Frontend and parser regression coverage

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **047**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 048`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Protect the important behavior without creating tests that only mirror implementation.

## Required implementation

1. Select regression scenarios for host isolation, stale data, error mapping, log buffers, mutation uncertainty and null Docker fields.
2. Use contract fixtures tied to documented remote CLI observations and test desired behavior through public adapters.
3. Add deterministic mock IPC component coverage for connection errors and confirmation dialogs.
4. Keep network tests explicit and bounded; avoid requiring access to production hosts for normal test runs.

## Acceptance and verification

- Run the frontend/Rust test suites with fixed fixtures and document exact commands.
- Demonstrate at least one failure the regression tests catch by a controlled temporary fault, then restore the implementation.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/048.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 048 --evidence codex/tracking/evidence/048.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 048 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
