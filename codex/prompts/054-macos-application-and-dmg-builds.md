# 054 — macOS application and DMG builds

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **053**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 054`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Prepare macOS desktop artifacts for Apple Silicon and Intel.

## Required implementation

1. Build on macOS using the pinned Tauri/Rust toolchains and selected deployment target; produce explicit arm64 and x86_64 artifacts or a tested universal bundle.
2. Configure bundle identifier, application icon, permissions and DMG layout with no embedded SSH credentials.
3. Test Finder launch, app data paths, agent access and /usr/bin/ssh usage in the packaged app.
4. Record local unsigned/ad-hoc builds separately from publicly distributable signed/notarized builds; do not describe Gatekeeper bypasses as installation requirements.

## Acceptance and verification

- Verify package contents and launch on each architecture claimed as tested; cross-compilation alone does not prove runtime support.
- If macOS hardware/runner is unavailable, complete build configuration and mark this platform verification prompt blocked.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/054.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 054 --evidence codex/tracking/evidence/054.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 054 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
