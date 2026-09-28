# 028 — Read refresh scheduling and stale data

- Phase: 03 Read-only MVP
- Recommended reasoning: **high**
- Depends on: **027**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 028`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Keep multiple views responsive during slow or unreliable SSH connections.

## Required implementation

1. Create a per-host read scheduler with concurrency limits, single-flight refreshes, cancellation and bounded backoff with jitter.
2. Prioritize user navigation over background stats and avoid one SSH process per table row for inspect.
3. Maintain last-success timestamps and stale indicators; distinguish disconnected from an empty daemon.
4. Retry only idempotent read requests with limits and refresh on wake/network recovery; mutations remain outside automatic retries.

## Acceptance and verification

- Simulate a slow daemon, repeated refresh clicks and a laptop sleep/wake cycle with fake clocks where appropriate.
- Measure maximum concurrent jobs and confirm late data cannot overwrite a newer session.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/028.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 028 --evidence codex/tracking/evidence/028.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 028 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
