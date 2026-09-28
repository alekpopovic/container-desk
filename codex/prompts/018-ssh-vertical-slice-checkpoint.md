# 018 — SSH vertical slice checkpoint

- Phase: 02 SSH transport
- Recommended reasoning: **high**
- Depends on: **017**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 018`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Verify the entire SSH foundation before building resource screens.

## Required implementation

1. Prepare a documented disposable lab with a local client, bastion and private Linux target, using temporary keys and a dedicated known_hosts file.
2. Demonstrate a bounded remote Docker version/list probe through ProxyJump and through a direct alias without requiring Docker or jq on the client.
3. Review command quoting, strict host checks, session ownership, cancellation and credential diagnostics together.
4. Write docs/checkpoints/018-ssh.md with OS, tool versions, commands, observed results and remaining platform checks; keep real hostnames and secrets out of committed fixtures.

## Acceptance and verification

- Run the vertical slice on the available native OS plus deterministic transport tests.
- If no live disposable SSH target exists, mark this checkpoint blocked rather than reporting mock output as SSH evidence.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/018.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 018 --evidence codex/tracking/evidence/018.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 018 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
