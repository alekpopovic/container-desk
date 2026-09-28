# 049 — Disposable direct and bastion integration lab

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **048**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 049`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Provide repeatable end-to-end SSH and Docker transport verification.

## Required implementation

1. Add an opt-in lab using disposable VMs or containers with a dedicated test Docker daemon; never mount a production host Docker socket into the test fixture.
2. Create direct and ProxyJump topology, ephemeral keys, strict known_hosts and a private target inaccessible directly from the test client.
3. Cover handshake, list/inspect/logs/stats, mutation, Compose, terminal, key mismatch and reconnect with bounded timeouts.
4. If Docker-in-Docker requires privileged mode, keep it inside a dedicated disposable test VM and document the requirement; include complete cleanup.

## Acceptance and verification

- Run the integration suite and prove the private target is reached through the bastion.
- Check cleanup of test keys, resources and child processes; record versions and sanitized logs.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/049.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 049 --evidence codex/tracking/evidence/049.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 049 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
