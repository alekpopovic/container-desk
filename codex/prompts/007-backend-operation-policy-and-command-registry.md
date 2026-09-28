# 007 — Backend operation policy and command registry

- Phase: 01 Foundations
- Recommended reasoning: **high**
- Depends on: **006**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 007`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Make remote operations explicit and enforce host permissions in Rust.

## Required implementation

1. Create typed read, mutation and terminal operation categories with default per-host read-only policy.
2. Define a registry of allowed Docker command builders and validate all IPC arguments in Rust; UI disabled buttons alone must never enforce access.
3. Use fixed supported operation variants, numeric bounds for timeouts/tail counts, validated IDs, and explicit per-operation result types.
4. Prepare confirmation intents bound to host, session generation, operation and target IDs for later mutation prompts.

## Acceptance and verification

- Call a mutation handler directly while read-only mode is enabled and ensure rejection before transport.
- Check invalid operation names, negative limits and malformed IDs; legitimate read operations remain available.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/007.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 007 --evidence codex/tracking/evidence/007.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 007 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
