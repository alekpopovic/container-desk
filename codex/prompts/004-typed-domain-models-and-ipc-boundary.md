# 004 — Typed domain models and IPC boundary

- Phase: 01 Foundations
- Recommended reasoning: **high**
- Depends on: **003**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 004`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Define the shared data contract between the renderer and Rust.

## Required implementation

1. Model HostId, SessionId, ContainerId, ConnectionState, HostCapabilities, ContainerSummary, ContainerDetail and structured AppError codes.
2. Expose narrow commands such as list_hosts, connect_host, list_containers and cancel_subscription; keep an arbitrary executable/command-string API out of the renderer.
3. Choose explicit serialization names and a single maintainable method for keeping Rust and TypeScript DTOs aligned.
4. Include host/session generation identifiers in requests and responses so stale data cannot be applied to a newly selected host.

## Acceptance and verification

- Check serialization against a representative success/error fixture across the boundary.
- Verify invalid IDs and absent sessions return typed errors before any process is spawned.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/004.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 004 --evidence codex/tracking/evidence/004.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 004 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
