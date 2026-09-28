# 032 — Container start stop and restart

- Phase: 04 Management
- Recommended reasoning: **high**
- Depends on: **031**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 032`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Implement the essential container management controls.

## Required implementation

1. Add fixed builders and UI actions for start, stop and restart with a bounded validated stop timeout where supported.
2. Show host, daemon and exact container name/ID in confirmation; use IDs obtained from the current host inventory.
3. After the action, refresh inspect/list to verify observed state and show error or pending convergence rather than assuming exit code alone proves readiness.
4. If connectivity is lost after submission, report unknown outcome and offer a read refresh before the user chooses another action.

## Acceptance and verification

- Run start/stop/restart against disposable containers and verify state transitions plus health-check delay.
- Disconnect after dispatch and confirm there is no automatic second mutation on reconnect.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/032.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 032 --evidence codex/tracking/evidence/032.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 032 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
