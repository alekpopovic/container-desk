# 027 — Docker event stream and inventory invalidation

- Phase: 03 Read-only MVP
- Recommended reasoning: **high**
- Depends on: **026**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 027`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

React to container lifecycle changes without trusting events as a complete database.

## Required implementation

1. Subscribe to bounded Docker event JSON output for the selected daemon and parse events with timestamps and actor IDs.
2. Debounce inventory invalidation for start/stop/die/destroy events; let full snapshots remain authoritative.
3. On reconnect, request a current snapshot and use any event history only as best-effort recovery; daemon retention and connection gaps can lose events.
4. Bound event buffers and apply the same cancellation/ownership policy as logs; never start hidden streams for all saved hosts.

## Acceptance and verification

- Test burst events, duplicate events, a gap across reconnect and a container deleted before inspect.
- Confirm event storms result in a bounded number of refresh requests.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/027.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 027 --evidence codex/tracking/evidence/027.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 027 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
