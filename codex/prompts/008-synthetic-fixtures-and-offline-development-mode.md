# 008 — Synthetic fixtures and offline development mode

- Phase: 01 Foundations
- Recommended reasoning: **medium**
- Depends on: **007**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 008`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Enable useful development without connecting to a real server.

## Required implementation

1. Create a replaceable transport trait/interface and synthetic fixtures for healthy, exited, restarting and unhealthy containers, IPv6 ports and Compose labels.
2. Provide an explicit demo mode with a persistent DEMO label; never switch a failed live connection into demo data.
3. Include empty outputs, permission failures, invalid JSON, huge records, disconnects and command timeouts in deterministic fixtures.
4. Set up focused Rust parser/transport tests and frontend component tests for state handling, using current compatible packages.

## Acceptance and verification

- Verify demo mode never starts SSH and live mode never reads fixture data as a fallback.
- Run the initial tests and demonstrate the same DTO shapes in demo and native IPC.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/008.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 008 --evidence codex/tracking/evidence/008.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 008 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
