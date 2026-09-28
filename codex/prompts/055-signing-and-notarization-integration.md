# 055 — Signing and notarization integration

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **054**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 055`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Prepare optional public distribution credentials without making them prerequisites for local development.

## Required implementation

1. Implement documented macOS signing/notarization steps using CI secrets and current Tauri/Apple guidance, with cleanup of temporary keychains.
2. Verify signing identity, entitlements, notarization result and stapling before labeling an artifact publicly verified.
3. Provide a clear branch of evidence: configuration-ready without credentials, or end-to-end signing-verified with supplied credentials.
4. Define Linux checksum/signature publication policy using owner-provided signing material if desired; never generate or upload private keys on the users behalf silently.

## Acceptance and verification

- Without credentials, verify missing-secret checks and configuration and explicitly record notarization as unverified.
- With authorized credentials, validate the produced artifact using native verification tools and retain redacted results.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/055.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 055 --evidence codex/tracking/evidence/055.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 055 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
