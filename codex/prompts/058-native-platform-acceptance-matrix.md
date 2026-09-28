# 058 — Native platform acceptance matrix

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **057**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 058`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Prove the claimed Linux and macOS workflows with real execution evidence.

## Required implementation

1. Complete docs/platform-matrix.md for Linux x86_64, macOS arm64 and macOS x86_64, including OS/app/OpenSSH/Docker versions and native launch.
2. For each claimed platform verify direct+ProxyJump, encrypted-key agent access, strict trust behavior, list/logs/stats, management and terminal resize/cleanup.
3. Run real tests where practical and use explicitly named CI/native runner evidence; mocks and cross-compiled binaries do not count as runtime verification.
4. Keep any missing platform blocked or narrow the release claim with an explicit documented scope decision; do not silently drop the agreed Linux/macOS targets.

## Acceptance and verification

- Review evidence row by row and link exact logs/screenshots/checksums without secrets.
- Leave the prompt blocked if a required native platform lacks evidence; packaging preparation may already be complete.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/058.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 058 --evidence codex/tracking/evidence/058.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 058 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
