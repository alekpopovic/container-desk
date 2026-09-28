# 024 — Live log subscriptions and cancellation

- Phase: 03 Read-only MVP
- Recommended reasoning: **high**
- Depends on: **023**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 024`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Stream logs efficiently through Rust to the frontend.

## Required implementation

1. Spawn docker logs --follow with an owned subscription ID scoped to a host/session/container and send bounded batches through Tauri channels.
2. Use bounded queues, byte/line retention limits and explicit dropped-data markers so a slow renderer cannot exhaust memory.
3. Implement stop/unmount/disconnect cleanup and child reaping; avoid replacing a log stream with a PTY that merges channels or injects terminal escapes.
4. Define reconnect behavior as best-effort: resume with timestamps where available, show gaps and do not promise lossless ordering or perfect deduplication.

## Acceptance and verification

- Test cancellation during heavy output and after network loss, confirming no orphan SSH children.
- Pause consumption under a large synthetic stream and verify bounded memory plus a visible drop marker.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/024.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 024 --evidence codex/tracking/evidence/024.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 024 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
