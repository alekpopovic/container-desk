# 013 — SSH authentication and host trust flow

- Phase: 02 SSH transport
- Recommended reasoning: **high**
- Depends on: **012**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 013`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Make authentication failures actionable without collecting credentials in the app.

## Required implementation

1. Use the configured IdentityFile/SSH agent and existing known_hosts with strict host-key verification. Disable interactive password fallback for structured operations.
2. For unknown or changed keys, show the SSH error and a documented terminal setup route; the user verifies fingerprints independently and retries.
3. Account for ProxyJump child processes: destination options are not assumed to configure jump-host authentication. Test unattended behavior on both hops and make missing agent keys fail promptly.
4. Do not enable agent forwarding, disable host checks, store passphrases or automatically edit known_hosts. Explain encrypted-key loading through the normal OS agent.

## Acceptance and verification

- Test known/unknown/changed keys and absent/encrypted keys on a disposable SSH lab.
- Confirm a missing jump-host key cannot leave a hidden password prompt hanging indefinitely.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/013.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 013 --evidence codex/tracking/evidence/013.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 013 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
