# 005 — Local settings and host metadata store

- Phase: 01 Foundations
- Recommended reasoning: **high**
- Depends on: **004**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 005`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Persist application preferences without duplicating SSH secrets.

## Required implementation

1. Store schema-versioned JSON in the platform application data directory with atomic writes and owner-only permissions; retain a recoverable previous version.
2. Persist theme, selected host aliases, labels, host group, read-only mode, trusted config path and SSH executable override.
3. Store only identity references; do not copy private keys, passwords, raw inspect responses or terminal output into preferences.
4. Implement schema migration, corrupt-file recovery with visible feedback and a storage adapter injectable in tests.

## Acceptance and verification

- Exercise interrupted write/corrupt JSON and a schema migration without losing the original file.
- Check persisted settings for secret material and verify two hosts keep independent modes and display names.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/005.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 005 --evidence codex/tracking/evidence/005.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 005 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
