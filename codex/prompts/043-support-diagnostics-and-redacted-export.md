# 043 — Support diagnostics and redacted export

- Phase: 05 Terminal and resilience
- Recommended reasoning: **high**
- Depends on: **042**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 043`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Create useful troubleshooting output without dumping sensitive data.

## Required implementation

1. Add an app-controlled diagnostic report containing version, platform, configured transport mode, staged error codes and bounded redacted timings.
2. Exclude keys, raw SSH config, tokens, environment values, log buffers, terminal text and full inspect payloads by default.
3. Provide an export preview and explicit save action; use pseudonyms for hostnames/paths where possible and explain remaining user-supplied text.
4. Implement a finite retention policy for local activity/diagnostic logs and a clear-local-data action that never deletes SSH files.

## Acceptance and verification

- Seed synthetic secrets in all error paths and scan the exported report for leakage.
- Confirm clearing app data leaves ~/.ssh/config, private keys and known_hosts untouched.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/043.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 043 --evidence codex/tracking/evidence/043.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 043 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
