# Verification of this prompt package

Date: 2026-09-28

## Executed checks

- Python runtime: 3.12.14 on the Linux preparation environment.
- `python3 -m unittest discover -s codex/tests -v`: **14 tests passed**.
- `python3 codex/scripts/track.py validate`: prompt IDs, dependencies, hashes and initial state passed.
- `render`, `next`, `summary` and `show` paths were exercised; initial next task is 001.
- All 60 individual prompt bodies are present verbatim in `codex/ALL_PROMPTS.md`.
- Required prompt sections and all relative Markdown links were checked.
- Distribution state contains **60 pending**, **0 done**, **0 in progress**, **0 blocked** tasks and an empty history.

Tracker tests cover dependency blocking, one active task, start/block/resume/done, missing/short evidence, path and symlink escape rejection, changed evidence hashes, prompt tampering, cascading reopen with history, concurrent writes and Markdown rendering. Tests use isolated temporary project copies and do not touch SSH or Docker.

## Package integrity

`SHA256SUMS.txt` lists the original package file hashes. These describe the downloaded package before normal tracker updates. After implementation starts, state/evidence/Markdown changes are expected. `manifest.json` separately retains original prompt hashes for the tracker.

## Boundaries of this verification

This is a prompt-and-tracker package. The Tauri application has not been implemented or built by creating this ZIP. No live SSH server, Docker daemon, GUI window, macOS runner, Apple signing identity or production host was used for this package verification.

The tracker was executed on Linux. It uses Python standard-library APIs available on Linux/macOS, including POSIX locking, but this package preparation did not independently run the tracker on macOS. Application/native platform acceptance is explicitly assigned to implementation prompts and must retain truthful results.

The package's tests validate tracker mechanics; they do not certify claims written in future evidence files. A task can only be considered functionally complete after its real implementation criteria have been checked.
