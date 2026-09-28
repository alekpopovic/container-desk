# 046 — Feature-complete desktop checkpoint

- Phase: 05 Terminal and resilience
- Recommended reasoning: **high**
- Depends on: **045**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 046`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Review the complete product before release engineering.

## Required implementation

1. Demonstrate the workflow from SSH discovery through live data, management, Compose and container terminal against a disposable jump-host setup.
2. Create a feature matrix marking implemented, verified and platform-pending states; remove placeholder buttons and misleading success messages.
3. Inspect command-policy coverage, resource lifecycle and consistent host identity across tabs.
4. Write docs/checkpoints/046-feature-complete.md with actual defects fixed and explicit remaining release gates.

## Acceptance and verification

- Run the integrated journey on the current OS and record terminal/log cleanup evidence.
- Block this checkpoint for any missing core feature; platform release verification remains mandatory later.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/046.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 046 --evidence codex/tracking/evidence/046.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 046 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
