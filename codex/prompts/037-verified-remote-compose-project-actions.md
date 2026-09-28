# 037 — Verified remote Compose project actions

- Phase: 04 Management
- Recommended reasoning: **high**
- Depends on: **036**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 037`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Support start stop and restart for explicitly configured remote Compose projects.

## Required implementation

1. Require a user-confirmed remote project directory, ordered config-file paths and explicit project name; quote each validated path and validate accessibility.
2. Use a controlled remote working directory and explicit Compose arguments; report missing environment/configuration dependencies without exporting resolved secrets.
3. Support start/stop/restart only for existing services in this release, with the same backend mode and confirmation policy.
4. Do not derive commands from untrusted labels alone; retain read-only grouping when executable configuration cannot be verified. Exclude up/build/pull/down and volume deletion.

## Acceptance and verification

- Test remote paths with spaces/apostrophes, multiple config files, absent env file and wrong project name.
- Demonstrate one disposable project restart and verify services afterward without modifying unrelated projects.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/037.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 037 --evidence codex/tracking/evidence/037.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 037 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
