# 034 — Image inventory and inspection

- Phase: 04 Management
- Recommended reasoning: **medium**
- Depends on: **033**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 034`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Add read-only image visibility for the selected daemon.

## Required implementation

1. List image IDs, tags/digests, size and creation data through structured CLI output; deduplicate image identity while preserving multiple tags.
2. Expose image metadata and links to containers using it; keep secret-like labels masked in default export/diagnostics.
3. Support filtering dangling images for inspection only and clearly handle missing tags.
4. Reuse host/session scoping and bounds from container views; do not implement pulling arbitrary registries or image deletion in this step.

## Acceptance and verification

- Test dangling images, multi-tag images and identical tags on different hosts.
- Compare image identity and references with the disposable daemon inventory.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/034.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 034 --evidence codex/tracking/evidence/034.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 034 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
