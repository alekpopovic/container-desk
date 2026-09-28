# 059 — Release candidate review and defect closure

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **058**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 059`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Prepare a reviewable release candidate with accurate readiness status.

## Required implementation

1. Review all completed prompt evidence, known defects, compatibility matrix, app security boundaries and produced package hashes.
2. Fix release-blocking defects found in the integrated workflow, add only targeted regression tests and update affected evidence honestly.
3. Write release notes covering implemented features, runtime prerequisites, supported OS/architectures, limitations and signing status.
4. Prepare a draft release checklist and local release directory; do not upload/publish artifacts or contact users as part of this prompt.

## Acceptance and verification

- Run the required checks once after final fixes and confirm the release artifacts correspond to the tested commit.
- Verify no pending core bug or unverified claimed platform is hidden by a completed tracker status.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/059.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 059 --evidence codex/tracking/evidence/059.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 059 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
