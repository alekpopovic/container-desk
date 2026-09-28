# 045 — Large inventories and stream pressure

- Phase: 05 Terminal and resilience
- Recommended reasoning: **high**
- Depends on: **044**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 045`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Measure resource usage under realistic concurrency and impose clear limits.

## Required implementation

1. Create a reproducible synthetic benchmark for 1000 container summaries, a large inspect response and high-rate log/events streams.
2. Measure render latency, queue sizes, child process counts and app memory growth on a documented machine; record observations rather than invented numbers.
3. Set configurable but bounded limits for retained log lines/bytes, stats history, active hosts and concurrent jobs.
4. Use virtualization/batching only where measurement shows benefit; expose truncated/dropped data explicitly.

## Acceptance and verification

- Confirm sustained output reaches a memory plateau under the configured buffers.
- Verify cancelling all subscriptions returns owned child count to baseline and filtering remains responsive.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/045.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 045 --evidence codex/tracking/evidence/045.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 045 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
