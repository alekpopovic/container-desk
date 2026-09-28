# 056 — Versioning updates and rollback guidance

- Phase: 06 Quality and delivery
- Recommended reasoning: **medium**
- Depends on: **055**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 056`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Make manual upgrades predictable and keep runtime updates optional.

## Required implementation

1. Use one authoritative application version and ensure frontend, Cargo and bundle versions stay aligned with a documented release script.
2. Default the first release to manual installer updates with a verified download location, published hashes and settings backup/migration guidance.
3. Document rollback limitations for schema migrations and preserve compatible settings backups.
4. If an updater is later enabled, require signed update metadata/artifacts and actual hosting credentials; do not ship placeholder update endpoints or auto-install logic now.

## Acceptance and verification

- Test version mismatch detection and a settings migration/rollback scenario with fixture files.
- Check the production build performs no unsolicited update or telemetry requests.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/056.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 056 --evidence codex/tracking/evidence/056.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 056 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
