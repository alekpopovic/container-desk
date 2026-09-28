# 020 — Container table and host-scoped cache

- Phase: 03 Read-only MVP
- Recommended reasoning: **medium**
- Depends on: **019**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 020`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Show an efficient, usable container inventory.

## Required implementation

1. Build sortable columns for name, state, health when available, image, ports and age; add search and state filters.
2. Key queries and row selection by HostId plus daemon identity and full ContainerId; cache timestamps with every successful result.
3. Virtualize large lists if measurement requires it; keep selection stable across refreshes and clear it when a resource disappears.
4. Render loading, empty, error and stale-data states visibly; avoid showing cached data as a live successful refresh.

## Acceptance and verification

- Test switching hosts with identical container names and out-of-order responses.
- Inspect a synthetic 1000-container table for responsive filtering and correct selection.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/020.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 020 --evidence codex/tracking/evidence/020.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 020 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
