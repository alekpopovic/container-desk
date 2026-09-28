# 051 — Local verification command and clean builds

- Phase: 06 Quality and delivery
- Recommended reasoning: **medium**
- Depends on: **050**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 051`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Make the agreed verification reproducible for contributors.

## Required implementation

1. Create documented commands for formatting, TypeScript checks, frontend tests/build, cargo fmt/clippy/test and an opt-in native integration run.
2. Use lockfiles and pinned toolchains; separate dependency installation from destructive cleanup.
3. Ensure the verification command exits nonzero on a failed required check and reports skipped optional/platform checks explicitly.
4. Record required system packages per tested distribution and macOS development prerequisites using current official documentation.

## Acceptance and verification

- Run the standard verification from a clean dependency install on the current platform.
- Force one check to fail temporarily and ensure the aggregate command reports failure, then restore it.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/051.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 051 --evidence codex/tracking/evidence/051.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 051 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
