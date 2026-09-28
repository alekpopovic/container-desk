# 030 — Read-only MVP checkpoint

- Phase: 03 Read-only MVP
- Recommended reasoning: **high**
- Depends on: **029**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 030`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Deliver a coherent usable release for inspecting remote Docker hosts.

## Required implementation

1. Integrate host selection, containers, inspect, health, ports, logs, stats, events and Compose grouping into one navigation flow.
2. Review every structured operation for read-only behavior and confirm no action silently changes daemon state.
3. Document supported runtime prerequisites, current limits and the full direct/ProxyJump connection journey in docs/checkpoints/030-read-only.md.
4. Run a real disposable-host walkthrough and capture sanitized evidence/screenshots; distinguish the available OS test from pending cross-platform verification.

## Acceptance and verification

- Demonstrate list, inspect, log follow/cancel and stats through the jump host without local Docker/jq.
- Check empty daemon, denied access and disconnected host UX; block the checkpoint if the integrated live workflow is unavailable.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/030.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 030 --evidence codex/tracking/evidence/030.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 030 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
