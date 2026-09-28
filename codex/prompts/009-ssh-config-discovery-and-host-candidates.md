# 009 — SSH config discovery and host candidates

- Phase: 02 SSH transport
- Recommended reasoning: **high**
- Depends on: **008**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 009`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Read host candidates from the user-selected OpenSSH configuration.

## Required implementation

1. Default to ~/.ssh/config and show its resolved local path. Parse literal Host tokens, quoted paths, comments and Include references with bounded recursion and cycle detection.
2. Keep discovery separate from effective configuration resolution. Do not pretend wildcard/negated patterns or conditional blocks enumerate concrete destinations.
3. Allow manually entered concrete aliases for wildcard or dynamic setups; display candidates for explicit user selection instead of scanning or connecting automatically.
4. Never rewrite SSH config. Treat it as trusted user configuration that may contain executable directives; do not execute Match exec while merely browsing candidates.

## Acceptance and verification

- Test multiple aliases on one Host line, wildcard/negative tokens, missing includes, spaces in include paths and include cycles.
- Check that discovery itself spawns no subprocess and that unsupported discovery cases have a manual-alias path.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/009.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 009 --evidence codex/tracking/evidence/009.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 009 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
