# 038 — Management MVP checkpoint

- Phase: 04 Management
- Recommended reasoning: **high**
- Depends on: **037**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 038`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Verify the management workflow and its error handling as a single product.

## Required implementation

1. Walk through toggling host write mode, confirming a lifecycle action, inspecting outcome and returning to read-only mode.
2. Review Images, Volumes, Networks and Compose views with the current container detail navigation.
3. Exercise batch partial failure, stale selection, daemon change and unknown action outcome; document the supported action boundaries.
4. Write docs/checkpoints/038-management.md with sanitized live results and remaining platform verification.

## Acceptance and verification

- Run lifecycle and Compose actions only against explicitly disposable lab resources.
- Check that read-only host settings are enforced in Rust for every mutation endpoint; mark any incomplete gate blocked.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/038.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 038 --evidence codex/tracking/evidence/038.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 038 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
