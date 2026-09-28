# 057 — User and contributor documentation

- Phase: 06 Quality and delivery
- Recommended reasoning: **medium**
- Depends on: **056**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 057`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Document the product exactly as implemented.

## Required implementation

1. Write Serbian quick start and English technical docs for install, trusted SSH config, direct/ProxyJump aliases, agent setup and Docker access.
2. Explain remote Docker context selection, rootless and configured sudo -n modes, unsupported remote shells and Compose path requirements.
3. Provide troubleshooting for unknown host keys, failed jump authentication, PATH differences, missing logging driver support and unknown mutation outcomes.
4. Describe read-only mode, terminal access, local data locations and optional export; avoid instructions that disable SSH host verification.

## Acceptance and verification

- Follow the quick start in a fresh test user profile and correct missing steps.
- Cross-check every documented control/command with the current application and remove aspirational features.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/057.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 057 --evidence codex/tracking/evidence/057.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 057 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
