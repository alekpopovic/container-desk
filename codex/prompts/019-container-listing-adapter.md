# 019 — Container listing adapter

- Phase: 03 Read-only MVP
- Recommended reasoning: **high**
- Depends on: **018**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 019`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Turn Docker JSON-line output into stable container summaries.

## Required implementation

1. Build docker ps -a --no-trunc --format {{json .}} through the registry and parse one JSON object per line with serde_json.
2. Normalize full IDs, names, image, state, display status, ports and labels while tolerating unknown fields and missing optional values.
3. Treat no lines with exit code zero as an empty result; detect malformed/banners output explicitly rather than silently dropping arbitrary lines.
4. Keep Docker CLI display fields distinct from precise inspect-derived fields and bound response bytes and record counts.

## Acceptance and verification

- Test running/exited/unhealthy containers, Unicode names, empty results, malformed lines and large valid listings.
- Compare normalized IDs and counts with native Docker CLI output against the disposable target.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/019.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 019 --evidence codex/tracking/evidence/019.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 019 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
