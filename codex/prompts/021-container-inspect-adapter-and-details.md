# 021 — Container inspect adapter and details

- Phase: 03 Read-only MVP
- Recommended reasoning: **high**
- Depends on: **020**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 021`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Provide accurate container details from structured inspect output.

## Required implementation

1. Fetch docker container inspect for validated IDs and parse its JSON array into a typed detail DTO.
2. Expose lifecycle timestamps, exit code, restart policy/count, image ID, mounts, labels, resource configuration and network addresses.
3. Mask environment values and sensitive labels by default in Rust before IPC; support explicit per-session reveal without persistence.
4. Handle missing/deleted containers and null/absent health or network data; bound large inspect payloads.

## Acceptance and verification

- Validate detailed fixtures for stopped, rootless, healthcheck-free and multi-network containers.
- Check default IPC responses, app logs and persisted files for synthetic secret leakage.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/021.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 021 --evidence codex/tracking/evidence/021.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 021 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
