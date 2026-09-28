# 003 — Application layout and design tokens

- Phase: 01 Foundations
- Recommended reasoning: **medium**
- Depends on: **002**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 003`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Build an original interface with the navigation density expected from a server management desktop app.

## Required implementation

1. Create a sidebar for host groups, a host header with connection state, a resource navigation area and a main table/detail split.
2. Define light/dark tokens, typography, spacing, focus indicators and status colors in Tailwind/CSS variables. Use a clearly distinct ContainerDesk identity.
3. Include Containers, Compose, Images, Volumes, Networks and Settings routes with honest empty states; unavailable features should explain their state.
4. Keep host identity visible in detail panes and action dialogs; make keyboard navigation work from the start.

## Acceptance and verification

- Inspect the interface at 1280x800 and a narrower desktop window; no clipped navigation or unreadable status labels.
- Check keyboard focus and theme contrast using representative empty, loading, offline and error states.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/003.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 003 --evidence codex/tracking/evidence/003.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 003 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
