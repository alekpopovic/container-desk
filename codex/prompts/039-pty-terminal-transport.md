# 039 — PTY terminal transport

- Phase: 05 Terminal and resilience
- Recommended reasoning: **high**
- Depends on: **038**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 039`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Implement a real interactive container shell with a separate transport path.

## Required implementation

1. Use a maintained PTY library selected for Linux/macOS and spawn OpenSSH in its local PTY; request a remote PTY for docker exec -it on a validated running container.
2. Support an explicit fixed shell choice such as /bin/sh or /bin/bash and a default non-root container user; report a missing shell rather than injecting fallback scripts.
3. Implement resize, stdin bytes, stdout bytes, close and child reaping with one session-bound terminal ID.
4. Gate opening the terminal behind host write/terminal permission because interactive input can perform arbitrary changes. Read-only mode must reject it in Rust.

## Acceptance and verification

- Test interactive echo, Ctrl-C, resize, exit and a container without a shell in the disposable lab.
- Ensure noninteractive JSON commands never inherit terminal settings or the PTY output path.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/039.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 039 --evidence codex/tracking/evidence/039.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 039 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
