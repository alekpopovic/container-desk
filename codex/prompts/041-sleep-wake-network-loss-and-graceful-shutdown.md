# 041 — Sleep wake network loss and graceful shutdown

- Phase: 05 Terminal and resilience
- Recommended reasoning: **high**
- Depends on: **040**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 041`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Make the app recover predictably from normal laptop behavior.

## Required implementation

1. Detect stale masters and failed keepalives, invalidate old session generations and cancel owned jobs when connectivity disappears.
2. On wake/retry, reconnect selected hosts with bounded backoff and refresh read snapshots; restart log subscriptions only with an explicit visible gap.
3. Close terminals after broken sessions and never replay typed input or mutations.
4. Implement graceful app exit cleanup with a bounded deadline, ensuring only app-owned sockets and child processes are touched.

## Acceptance and verification

- Use fault injection to break the master during logs/stats/terminal work and inspect the process tree afterward.
- Perform a native suspend/resume or documented network interruption test on an available OS.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/041.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 041 --evidence codex/tracking/evidence/041.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 041 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
