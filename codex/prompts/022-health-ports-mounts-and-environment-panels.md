# 022 — Health, ports, mounts and environment panels

- Phase: 03 Read-only MVP
- Recommended reasoning: **medium**
- Depends on: **021**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 022`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Make inspect data easy to read and compare.

## Required implementation

1. Create detail tabs for Overview, Ports, Mounts, Networks, Labels and Environment with copy actions for selected safe fields.
2. Differentiate container port exposure from host bindings; handle IPv4, IPv6 and multiple host bindings.
3. Display healthcheck absence as not configured, and health state separately from running state. Mark exit/OOM details when provided.
4. Require an explicit reveal for environment values; keep content as plain text and clear revealed values when the session closes.

## Acceptance and verification

- Inspect layouts with long mount paths, empty labels, multiple bindings and a failed healthcheck.
- Verify no label/environment content is interpreted as HTML or linked automatically.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/022.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 022 --evidence codex/tracking/evidence/022.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 022 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
