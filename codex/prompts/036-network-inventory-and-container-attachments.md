# 036 — Network inventory and container attachments

- Phase: 04 Management
- Recommended reasoning: **medium**
- Depends on: **035**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 036`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Show Docker networks and their attached containers.

## Required implementation

1. List/inspect networks using typed adapters and show driver, internal flag, IPAM summary and attachment relationships.
2. Handle IPv4/IPv6, host/none networks, missing IPAM and stale endpoint data.
3. Link attached container IDs back to the correct host inventory and show unknown/deleted endpoints without crashing.
4. Keep network mutation out of this milestone and render returned names/options as plain text.

## Acceptance and verification

- Test bridge, host, none, IPv6 and custom network fixtures plus malformed optional fields.
- Compare attachment counts against inspect output on the disposable target.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/036.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 036 --evidence codex/tracking/evidence/036.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 036 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
