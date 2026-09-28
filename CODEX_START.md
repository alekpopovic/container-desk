# Start / resume

Run from the repository root. Read `AGENTS.md`, `README_SR.md`, `codex/docs/ARCHITECTURE.md` and `codex/docs/TRACKING.md`.

## Start or continue — paste this into Codex

```text
Implement the next ContainerDesk task in this repository.
Read AGENTS.md and CODEX_START.md, then run:
python3 codex/scripts/track.py validate
python3 codex/scripts/track.py next

Read the exact returned prompt file and the architecture contract.
Inspect existing code, changes and evidence before editing anything.
Use the manifest reasoning recommendation if your interface supports it;
otherwise report it without claiming that you changed a model setting.
Start the task through the tracker, implement the requested increment,
run meaningful checks, write evidence, and mark done or blocked accurately.
Do not execute a second prompt automatically. Return changes, checks,
remaining limitations and the next task ID. Preserve all unrelated user work.
```

## Resume a blocked task

Read the recorded blocker, resolve it, then use `start NNN` again. A completed prerequisite must exist before the next task can start. If a native OS or SSH lab is missing, prepare what is independent, retain the blocker and tell the user what environment is required.

## Completion evidence

Create `codex/tracking/evidence/NNN.md` from the evidence template. Record commands actually run and results; do not paste the tracker example command as proof of application functionality. The initial package contains zero completed application prompts.

Reasoning is advisory. Prefer the prompt recommendation, escalating locally for a concrete security/concurrency/recovery ambiguity. This package does not depend on a particular Codex model name or an undocumented CLI flag.
