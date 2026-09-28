# 012 — Remote argument quoting and Docker command builders

- Phase: 02 SSH transport
- Recommended reasoning: **high**
- Depends on: **011**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 012`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Prevent remote shell interpretation from changing structured operations.

## Required implementation

1. Document that OpenSSH remote commands still pass through the remote login shell; local argument arrays alone do not prevent remote command injection.
2. Target a POSIX-compatible remote shell and build one remote command from fixed tokens plus a thoroughly tested POSIX single-quote encoder.
3. Constrain container/image IDs, options, numeric limits and paths before quoting; support a validated absolute Docker binary path without evaluating shell startup files.
4. Keep template syntax such as {{json .}} intact. Do not concatenate labels, host display names, pasted scripts or arbitrary renderer text into commands.

## Acceptance and verification

- Test apostrophes, spaces, semicolons, newlines, dollar substitution, backticks and leading dashes with an inert local POSIX-shell harness.
- Verify generated commands preserve arguments exactly and a malicious label cannot spawn a second command.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/012.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 012 --evidence codex/tracking/evidence/012.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 012 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
