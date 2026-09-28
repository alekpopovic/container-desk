# 006 — Native dependency diagnostics

- Phase: 01 Foundations
- Recommended reasoning: **medium**
- Depends on: **005**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 006`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Detect the tools the installed application actually needs.

## Required implementation

1. Resolve OpenSSH from a trusted absolute path, normally /usr/bin/ssh, with a user-selectable validated override; do not rely on an interactive shell PATH.
2. Inspect executable permissions and record ssh -V output, platform, architecture and app version in a diagnostics view.
3. Detect the presence/accessibility of SSH_AUTH_SOCK without reading keys. Keep frontend dependencies out of runtime requirements.
4. Show that local Docker, jq, Python, Rust and Node are not required by the built app; Python is used only by this development tracker.

## Acceptance and verification

- Launch diagnostics with a deliberately minimal PATH and confirm native SSH is found.
- Test missing executable, non-executable override and missing agent socket with useful error messages.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/006.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 006 --evidence codex/tracking/evidence/006.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 006 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
