# 047 — Focused security review

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **046**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 047`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Check the concrete trust boundaries implemented by this application.

## Required implementation

1. Review renderer IPC validation, remote quoting, host/config trust, StrictHostKeyChecking, app-owned sockets, read-only policy, confirmation intents and terminal access.
2. Test malformed IDs and host aliases, hostile labels/logs, stale mutation tokens and changed daemon/session identities.
3. Check CSP and Tauri capabilities, filesystem scopes, navigation rules and protocol handlers; remove generic shell execution exposed to the renderer.
4. Record findings and fixes in docs/security-review.md; app read-only mode is not server-side authorization for a Docker-privileged account.

## Acceptance and verification

- Run regression checks for each discovered exploitable path and verify default exports omit seeded secrets.
- Confirm release configuration cannot load arbitrary remote UI content or expose an unauthenticated control endpoint.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/047.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 047 --evidence codex/tracking/evidence/047.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 047 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
