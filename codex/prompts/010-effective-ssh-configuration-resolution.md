# 010 — Effective SSH configuration resolution

- Phase: 02 SSH transport
- Recommended reasoning: **high**
- Depends on: **009**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 010`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Delegate connection semantics to OpenSSH rather than reimplementing its precedence rules.

## Required implementation

1. After explicit host selection, resolve bounded diagnostic output with the chosen ssh executable and ssh -G alias, using the selected -F config path when applicable.
2. Reject aliases that begin with a dash or contain whitespace/control characters; define and document a conservative supported alias grammar.
3. Parse effective host, user, port and jump information for display while retaining the original alias for actual connections.
4. Treat Match exec and ProxyCommand as trusted local configuration code; explain this once when choosing a custom config. Do not expose identity contents or claim ssh -G is side-effect free.

## Acceptance and verification

- Compare effective values with native ssh -G on fixture configs, including overlapping Host entries and Include.
- Verify the alias reaches OpenSSH as an argument, not through a local shell; malformed aliases fail before execution.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/010.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 010 --evidence codex/tracking/evidence/010.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 010 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
