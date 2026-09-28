# 035 — Volume inventory and mount relationships

- Phase: 04 Management
- Recommended reasoning: **medium**
- Depends on: **034**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 035`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Make persistent storage relationships visible.

## Required implementation

1. List and inspect volumes with name, driver, scope and safe metadata through fixed read builders.
2. Build references from container inspect mounts without copying host volume contents.
3. Display unused-reference status as a snapshot observation, not proof that deletion is safe.
4. Keep volume browsing, deletion and prune outside this release; reuse generic read-only list/detail patterns.

## Acceptance and verification

- Test named/anonymous/external-driver volumes, no mountpoint and containers deleted during refresh.
- Verify volume views never spawn filesystem reads against remote data directories.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/035.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 035 --evidence codex/tracking/evidence/035.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 035 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
