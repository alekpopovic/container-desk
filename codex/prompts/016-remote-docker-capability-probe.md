# 016 — Remote Docker capability probe

- Phase: 02 SSH transport
- Recommended reasoning: **high**
- Depends on: **015**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 016`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Identify the Docker daemon and operations available to the SSH user.

## Required implementation

1. Probe remote Docker client/server versions, daemon identity, OS, access permission and Compose plugin availability with bounded structured output.
2. Support the remote user default Docker context or an explicitly selected named remote context; pass it consistently and show the actual endpoint/daemon identity.
3. Detect local Unix/rootless socket versus another remote context endpoint, and make endpoint identity visible. Restrict the supported MVP target to Linux Docker Engine.
4. Allow an explicit fixed sudo -n Docker execution mode for existing administrator configuration; never request/store sudo passwords or change server permissions automatically.

## Acceptance and verification

- Test missing docker, stopped daemon, permission denied, rootless context, Compose absent and sudo requiring a password.
- Verify every later builder uses the selected context and does not silently switch daemon or privilege mode.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/016.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 016 --evidence codex/tracking/evidence/016.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 016 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
