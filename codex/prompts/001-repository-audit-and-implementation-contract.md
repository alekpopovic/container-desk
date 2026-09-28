# 001 — Repository audit and implementation contract

- Phase: 01 Foundations
- Recommended reasoning: **high**
- Depends on: **none**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 001`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Establish a reproducible starting point for ContainerDesk, a desktop Docker manager using native OpenSSH.

## Required implementation

1. Inspect the existing repository and uncommitted changes before creating anything; preserve user work and adapt this pack to the actual repository.
2. Write docs/project-status.md and docs/decisions/0001-architecture.md: one Tauri v2 app, React/TypeScript/Vite UI, Rust/Tokio backend, native OpenSSH, remote Linux Docker Engine, Linux/macOS clients.
3. Record installed Rust, Node, package manager and OS versions. Choose supported stable toolchains from official documentation and pin the selected versions; do not silently select prereleases.
4. Define executable MVP checkpoints 018, 030, 038, 046 and release candidate 060. Declare terminal and management access to be opt-in per host.

## Acceptance and verification

- Confirm the repository can retain the pack without overwriting an existing AGENTS.md; document any merged rules.
- Verify every implementation decision agrees with codex/docs/ARCHITECTURE.md; record unavailable build environments honestly.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/001.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 001 --evidence codex/tracking/evidence/001.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 001 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
