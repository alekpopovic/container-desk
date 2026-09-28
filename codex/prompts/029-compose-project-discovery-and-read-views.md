# 029 — Compose project discovery and read views

- Phase: 03 Read-only MVP
- Recommended reasoning: **high**
- Depends on: **028**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 029`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Group containers by Compose project without requiring local Compose files.

## Required implementation

1. When the remote Compose plugin exists, parse docker compose ls --all --format json and normalize its project metadata.
2. Supplement project/service grouping from com.docker.compose labels on inspected containers; expose label-based grouping even if the plugin is absent.
3. Do not assume a remotely reported ConfigFiles path is usable, trusted, readable or complete; label discovery and executable project configuration separately.
4. Display service instances and states and link to the same container detail/log views without duplicating transport sessions.

## Acceptance and verification

- Test two projects with identical service names, absent Compose plugin and stale/missing config files.
- Verify no local filesystem path is mistaken for a remote Compose path.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/029.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 029 --evidence codex/tracking/evidence/029.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 029 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
