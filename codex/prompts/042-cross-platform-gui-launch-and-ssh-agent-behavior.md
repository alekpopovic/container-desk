# 042 — Cross-platform GUI launch and SSH agent behavior

- Phase: 05 Terminal and resilience
- Recommended reasoning: **high**
- Depends on: **041**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 042`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Close the gap between terminal development and installed desktop operation.

## Required implementation

1. Test Linux desktop launcher and macOS Finder launch with minimal environment; resolve SSH through the configured absolute executable.
2. Provide diagnostics for inaccessible agent sockets and passphrase-protected keys; document user setup for their OS agent/Keychain without shelling into profile scripts.
3. Handle spaces/non-ASCII home directories, application data paths and permission errors consistently.
4. Document macOS-only SSH options as optional guarded config fragments, not mandatory syntax for Linux.

## Acceptance and verification

- Run platform-specific manual steps on each available platform and keep missing-platform results explicitly pending.
- Verify the app never depends on local Docker, jq or development toolchains after installation.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/042.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 042 --evidence codex/tracking/evidence/042.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 042 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
