# 023 — Bounded log snapshot retrieval

- Phase: 03 Read-only MVP
- Recommended reasoning: **high**
- Depends on: **022**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 023`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Retrieve recent container logs without unbounded output.

## Required implementation

1. Implement docker logs with bounded positive tail, timestamps and an optional validated since/until range.
2. Treat log content as untrusted text. Capture Docker/SSH exit errors separately while acknowledging remote log stderr shares the SSH stderr channel.
3. Do not classify all stderr as failure: container application stderr is valid log data. Distinguish nonzero process exit and mark ambiguous diagnostic lines honestly.
4. Handle unsupported logging drivers, exited containers, invalid UTF-8 and oversized lines; keep data transient unless the user exports it.

## Acceptance and verification

- Test interleaved application stdout/stderr, daemon failure, unsupported log driver and a line larger than the limit.
- Compare a known fixture container log sequence with the native docker logs command.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/023.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 023 --evidence codex/tracking/evidence/023.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 023 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
