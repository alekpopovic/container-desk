# 052 — Linux and macOS CI build matrix

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **051**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 052`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Automate reproducible checks and package creation without publishing automatically.

## Required implementation

1. Add GitHub Actions with pinned/action-reviewed dependencies and Linux/macOS jobs, least-privilege token permissions and documented architecture mapping.
2. Run checks before producing versioned artifacts, checksums and metadata; retain failures and test reports.
3. Separate trusted release jobs from pull-request builds so forked changes never access signing keys or release tokens.
4. Keep publishing as an explicit workflow action after the release gate; artifact generation should work without Apple credentials.

## Acceptance and verification

- Validate workflow syntax and run available jobs; document unavailable remote CI execution as pending.
- Inspect permissions and prove untrusted PR events cannot reach secrets or publishing steps.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/052.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 052 --evidence codex/tracking/evidence/052.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 052 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
