# 053 — Linux package builds

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **052**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 053`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Prepare installable Linux artifacts and define their actual support range.

## Required implementation

1. Build the chosen Tauri Linux packages, initially deb and AppImage, on a pinned supported Ubuntu baseline with required native libraries.
2. Declare x86_64 as the initial verified Linux architecture; do not advertise untested distributions or aarch64 builds.
3. Configure app name, desktop entry, icons, executable permissions and required package dependencies.
4. Document that sandboxed distribution formats need additional SSH file/socket access design and are outside the first package set.

## Acceptance and verification

- Install and launch at least one native package on a clean supported desktop environment.
- Verify host config access and SSH discovery from the desktop launcher, recording artifact hashes and platform versions.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/053.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 053 --evidence codex/tracking/evidence/053.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 053 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
