# 040 — Terminal UI with bounded lifecycle

- Phase: 05 Terminal and resilience
- Recommended reasoning: **high**
- Depends on: **039**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 040`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Connect the PTY to a usable terminal tab.

## Required implementation

1. Use xterm.js with fit support, bounded scrollback and explicit connect/disconnect status; keep host and container identity always visible.
2. Forward input only to the active owned terminal. Disable automatic clipboard writes and unsafe escape-sequence integrations; ask before multiline paste.
3. Stop and reap the PTY on tab close, host disconnect or permission revocation; do not reconnect and replay shell input.
4. Do not persist terminal transcripts or command history in app storage; allow intentional copy of selected visible text.

## Acceptance and verification

- Test terminal resize, paste with newlines, tab closure under output and switching between two host sessions.
- Verify a background terminal cannot receive keystrokes from another tab and no transcript appears in diagnostics.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/040.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 040 --evidence codex/tracking/evidence/040.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 040 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
