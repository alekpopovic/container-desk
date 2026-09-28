# 025 — Log viewer usability and export

- Phase: 03 Read-only MVP
- Recommended reasoning: **medium**
- Depends on: **024**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 025`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Provide a practical desktop log viewer.

## Required implementation

1. Add follow/pause, clear-view, plain-text search, bounded filtering, timestamps and a capped virtualized log display.
2. Keep pause-rendering behavior distinct from stopping the remote stream; explain retained buffer limits in the UI.
3. Export only selected buffered lines through a native save dialog and warn that log contents may contain application secrets at the export action.
4. Neutralize terminal control sequences and clickable escape links; preserve readable Unicode and copy exact visible text.

## Acceptance and verification

- Test follow scrolling while selecting text, an oversized search term and clearing the view during streaming.
- Export a known buffer and verify ordering/content and that unrelated containers are absent.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/025.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 025 --evidence codex/tracking/evidence/025.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 025 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
