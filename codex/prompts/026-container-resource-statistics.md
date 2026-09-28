# 026 — Container resource statistics

- Phase: 03 Read-only MVP
- Recommended reasoning: **high**
- Depends on: **025**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 026`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Show CPU, memory and I/O with explicit sampling semantics.

## Required implementation

1. Use bounded docker stats --no-stream --no-trunc --format JSON-template snapshots at a configurable modest interval; allow only one stats request per host at a time.
2. Parse units and missing values carefully, retaining raw strings for diagnostics and normalized values for charts.
3. Label CPU as Docker-reported percent, which may exceed 100 on multicore hosts, and memory as Docker CLI semantics; do not reinterpret it as raw API memory.
4. Use finite in-memory history keyed by host and full container ID; show stopped/unavailable samples as gaps rather than zeros.

## Acceptance and verification

- Check unit conversion, percentages above 100, unavailable values and container disappearance during sampling.
- Verify polling stops on disconnect and slows/pauses for inactive windows according to documented settings.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/026.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 026 --evidence codex/tracking/evidence/026.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 026 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
