# 002 — Tauri React TypeScript scaffold

- Phase: 01 Foundations
- Recommended reasoning: **medium**
- Depends on: **001**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 002`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Create the working desktop shell in the repository root.

## Required implementation

1. Initialize Tauri v2 with React, TypeScript and Vite in the existing root without nesting another app or deleting codex/.
2. Create src-tauri and frontend source folders; add a minimal Rust command returning application version and call it from the UI.
3. Use one JavaScript package manager and commit its lockfile plus Cargo.lock. Record development and production build commands.
4. Add formatting and lint scripts with minimal useful rules; preserve strict TypeScript and avoid an unrestricted shell plugin.

## Acceptance and verification

- Run frontend typecheck/build and cargo check where native libraries are available.
- Launch the native window when a display is available; otherwise record native launch as unverified and verify the IPC contract in a focused check.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/002.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 002 --evidence codex/tracking/evidence/002.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 002 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
