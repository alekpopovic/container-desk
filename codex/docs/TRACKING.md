# Tracker usage

Requires Python 3.10+ on Linux/macOS. No network calls or third-party Python packages are used. Run commands from the project root. The script derives the project path from its own location, so `codex/` must remain directly inside that root.

## Commands

```bash
# Verify prompt/state consistency and submitted evidence hashes.
python3 codex/scripts/track.py validate

# Find the earliest unfinished task, including active or blocked work.
python3 codex/scripts/track.py next

# Print one complete prompt (001 or 1 are both accepted).
python3 codex/scripts/track.py show 001

# Begin/resume a task after completed dependencies.
python3 codex/scripts/track.py start 001

# Show counts and refresh the generated Markdown view.
python3 codex/scripts/track.py summary
python3 codex/scripts/track.py render
```

After implementation, write a real evidence file. Then record the actual check, for example:

```bash
python3 codex/scripts/track.py done 001 \
  --evidence codex/tracking/evidence/001.md \
  --check "Repository audit completed; tool versions and architecture decision recorded"
```

The example is appropriate only after that work happened. Pass `--check` repeatedly for multiple real checks. Optional `--commit <sha>` records the implementation commit without creating one.

## Block and resume

```bash
python3 codex/scripts/track.py block 018 --reason "Disposable SSH target is unavailable; live ProxyJump check remains unverified"

# After resolving the recorded blocker:
python3 codex/scripts/track.py start 018
```

Only an in-progress task can be blocked or completed. Later tasks cannot start until their dependencies are completed. Preparing an independent configuration file does not justify marking an unavailable native acceptance test as passed.

## Reopen and correct

```bash
python3 codex/scripts/track.py reopen 018 --reason "Transport defect requires another live check"
```

If downstream work is already tracked, the script rejects a simple reopen. Review the impact, then use:

```bash
python3 codex/scripts/track.py reopen 018 --cascade --reason "Revalidate dependent transport features after the defect fix"
```

Cascade resets tracker status for this task and its descendants; it does not delete source code or evidence files. The previous rows are preserved in history. Revalidate existing work instead of reimplementing it. A fresh evidence file may replace that task's old file after reopening; retain past logs in Git if they matter.

## Data files

| File | Role |
|---|---|
| `manifest.json` | Ordered immutable prompt definitions, prerequisites, reasoning and SHA-256 |
| `tracking/state.json` | Authoritative statuses, check descriptions, history and evidence hashes |
| `tracking/TRACKER.md` | Generated human-readable view |
| `tracking/evidence/NNN.md` | Real results and implementation decisions for that task |
| `tracking/.tracker.lock` | Ephemeral POSIX lock; exclude from Git |

State writes are serialized by POSIX `flock` and replaced atomically. If Markdown rendering is interrupted after state was saved, run `render`; state remains authoritative. Use local disk for the tracker while executing it. Shared-network filesystem locking is outside the tested scope.

## What validation proves

The script checks IDs, dependency ordering, single-active-task status, prompt hashes, evidence existence/content length and evidence hashes. It rejects evidence paths outside the project, including symlink escapes. It cannot prove that a human-written check result is truthful or that a described test really ran. Review the actual evidence.

The tracker is not a Codex API integration. `reasoning` is an advisory value selected for each task. Set it in your Codex environment if supported; otherwise proceed with the available setting and state that limitation accurately.

## Recovery

- Preserve `state.json`, `manifest.json`, evidence and application source in Git or your normal backup.
- If evidence changed after completion, reopen the task (and dependent work if necessary), review and complete it again.
- If a prompt hash changed accidentally, restore the original prompt from this ZIP. Put approved changes in decisions/evidence.
- If state is corrupt, restore its last known valid Git/backup version. Do not overwrite it with a fabricated completed state.
- Run `python3 -m unittest discover -s codex/tests -v` to verify the tracker against isolated temporary copies.
