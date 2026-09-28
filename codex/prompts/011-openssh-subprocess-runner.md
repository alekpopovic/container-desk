# 011 — OpenSSH subprocess runner

- Phase: 02 SSH transport
- Recommended reasoning: **high**
- Depends on: **010**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 011`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Create the cancellable transport foundation for remote commands.

## Required implementation

1. Use Tokio process APIs with a resolved SSH executable and argument arrays; build no local sh -c command.
2. Implement separate stdout/stderr readers, bounded capture, exit-code handling, command deadlines and cancellation with child cleanup/reaping.
3. Use noninteractive stdin and no PTY for structured operations; retain a separate execution path for the later interactive terminal.
4. Add connection timeout and keepalive options with documented defaults. Do not let a noisy stderr pipe deadlock a stdout reader.

## Acceptance and verification

- With a synthetic child program, test timeout, cancellation, large stderr, nonzero exit and partial output.
- Verify children are reaped and bounded-output failure is distinguishable from an empty successful response.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/011.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 011 --evidence codex/tracking/evidence/011.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 011 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
