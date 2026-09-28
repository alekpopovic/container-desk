# 044 — Accessibility themes and keyboard workflow

- Phase: 05 Terminal and resilience
- Recommended reasoning: **medium**
- Depends on: **043**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 044`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Make routine inspection efficient without a mouse.

## Required implementation

1. Audit semantic tables, labels, keyboard focus, dialog traps, escape handling and screen-reader status announcements.
2. Support platform-appropriate shortcuts for search, refresh, host selection and log focus without capturing terminal keystrokes.
3. Ensure dark/light themes communicate state with text/icons in addition to color, and respect reduced motion.
4. Inspect dense views at different font scales and window sizes; keep actionable error text readable.

## Acceptance and verification

- Run a keyboard-only host-to-container-to-logs workflow and a screen-reader spot check where available.
- Fix verified focus/contrast issues and attach representative screenshots to the evidence.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/044.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 044 --evidence codex/tracking/evidence/044.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 044 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
