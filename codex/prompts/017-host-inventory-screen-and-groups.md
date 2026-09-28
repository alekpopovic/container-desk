# 017 — Host inventory screen and groups

- Phase: 02 SSH transport
- Recommended reasoning: **medium**
- Depends on: **016**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 017`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Connect discovery, metadata and connection state into the desktop UI.

## Required implementation

1. Allow selecting config aliases, assigning labels, grouping prod/staging/dev hosts, favoriting hosts and removing app metadata.
2. Display effective destination, jump route summary, Docker endpoint, connection health and per-host read-only mode.
3. Add explicit Connect/Disconnect/Retry actions; connection errors retain the selected host and offer relevant diagnostics.
4. Keep deleting a host from the app separate from editing SSH config or deleting remote resources.

## Acceptance and verification

- Verify duplicate display names still map to distinct stable HostIds and session state cannot leak across hosts.
- Exercise add/connect/disconnect/remove with both direct and ProxyJump aliases in demo and live adapters.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/017.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 017 --evidence codex/tracking/evidence/017.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 017 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
