# 050 — Native desktop integration tests

- Phase: 06 Quality and delivery
- Recommended reasoning: **high**
- Depends on: **049**
- Scope: one increment in the existing ContainerDesk application.

## Context and start

Read `AGENTS.md`, `CODEX_START.md`, `codex/docs/ARCHITECTURE.md`, the current repository and tracker state. Preserve existing work. Reuse working implementations and adjust paths to the repository; do not rebuild completed features.

Run `python3 codex/scripts/track.py start 050`. If it rejects unfinished prerequisites, resolve that earlier step first. The reasoning level is a recommendation, not a claim that a prompt can change the model setting.

## Goal

Validate the Tauri boundary in real desktop windows.

## Required implementation

1. Choose the currently supported Tauri/WebdriverIO path from official documentation; distinguish browser-only mocked UI tests from native application tests.
2. For macOS, evaluate the documented embedded WebDriver service if suitable; do not assume the standalone tauri-driver supports WKWebView.
3. Keep automation plugins, test server and mock-control APIs out of production builds using explicit build features.
4. Test host selection, native IPC, connection errors, log cancellation and a bounded terminal interaction on available native runners.

## Acceptance and verification

- Record Linux/macOS native automation separately and mark unavailable runners pending.
- Inspect a release build configuration to verify embedded test automation cannot start in production.
- Verify the actual code paths touched here. Record real commands, exit/results and platform limitations. Do not infer native behavior from browser mocks.
- Keep process/resource bounds, session identity, remote quoting and secret handling consistent with the architecture contract.

## Evidence and finish

Write `codex/tracking/evidence/050.md` using `codex/tracking/EVIDENCE_TEMPLATE.md`. Include files changed, check results, acceptance items, platform, remaining limits and recommended next step. Never copy credentials or production logs into evidence.

If acceptance is met, run:

```bash
python3 codex/scripts/track.py done 050 --evidence codex/tracking/evidence/050.md --check "REPLACE with actual verification command and result"
```

Replace the check text; do not execute the placeholder literally. If an essential check is unavailable or fails, use `python3 codex/scripts/track.py block 050 --reason "Specific missing prerequisite or failed check"` and state the exact next action. Do not mark blocked work done. Stop after this prompt and summarize changes, verification and limitations.
