#!/usr/bin/env python3
"""Local prompt tracker. Python 3.10+, Linux/macOS, standard library only."""
import argparse
import copy
import datetime as dt
import fcntl
import hashlib
import json
import os
from pathlib import Path
import sys
import tempfile

PACK = Path(__file__).resolve().parents[1]
PROJECT = PACK.parent
TRACK = PACK / "tracking"
STATE = TRACK / "state.json"
MANIFEST = PACK / "manifest.json"
STATUSES = {"pending", "in_progress", "blocked", "done"}


def now():
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds")


def atomic_write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix=".tracker-", dir=path.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as out:
            out.write(text)
            out.flush()
            os.fsync(out.fileno())
        os.replace(tmp, path)
    finally:
        if os.path.exists(tmp):
            os.unlink(tmp)


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8"))


def clean(value):
    return str(value).replace("|", "\\|").replace("\n", " ")


def dependency_errors(manifest, state):
    errors = []
    for task in manifest["prompts"]:
        row = state["tasks"][task["id"]]
        if row["status"] in {"in_progress", "done"}:
            missing = [x for x in task["depends_on"] if state["tasks"][x]["status"] != "done"]
            if missing:
                errors.append(f'{task["id"]}: unfinished dependencies {", ".join(missing)}')
    return errors


def checked_evidence(relative):
    given = Path(relative)
    if given.is_absolute():
        raise ValueError("Evidence must be a path relative to the project root.")
    resolved = (PROJECT / given).resolve()
    if not resolved.is_relative_to(PROJECT.resolve()):
        raise ValueError("Evidence cannot escape the project root.")
    if not resolved.is_file():
        raise ValueError(f"Evidence file not found: {relative}")
    data = resolved.read_bytes()
    if len(data.strip()) < 80:
        raise ValueError("Evidence is too short; record actual changes, checks and limitations.")
    return str(resolved.relative_to(PROJECT.resolve())), hashlib.sha256(data).hexdigest()


def validate(manifest, state, evidence=True):
    errors = []
    prompts = manifest.get("prompts", [])
    ids = [t["id"] for t in prompts]
    if ids != [f"{i:03d}" for i in range(1, len(ids) + 1)] or not ids:
        errors.append("Prompt IDs must be sequential, unique, and nonempty.")
    if state.get("schema_version") != 1:
        errors.append("Unsupported state schema.")
    if set(state.get("tasks", {})) != set(ids):
        errors.append("Manifest/state IDs differ.")
        return errors
    active = []
    for task in prompts:
        tid = task["id"]
        if any(d not in ids or d >= tid for d in task["depends_on"]):
            errors.append(f"{tid}: invalid dependency graph")
        path = (PACK / task["file"]).resolve()
        if not path.is_relative_to(PACK.resolve()) or not path.is_file():
            errors.append(f"{tid}: missing or invalid prompt path")
        elif hashlib.sha256(path.read_bytes()).hexdigest() != task["sha256"]:
            errors.append(f"{tid}: prompt differs from manifest hash")
        row = state["tasks"][tid]
        if row.get("status") not in STATUSES:
            errors.append(f"{tid}: invalid status")
        if row.get("status") == "in_progress":
            active.append(tid)
        if row.get("status") == "done":
            if not row.get("evidence") or not row.get("checks"):
                errors.append(f"{tid}: done without evidence/checks")
            elif evidence:
                try:
                    _, digest = checked_evidence(row["evidence"])
                    if digest != row.get("evidence_sha256"):
                        errors.append(f"{tid}: evidence changed after completion; reopen and reverify")
                except ValueError as err:
                    errors.append(f"{tid}: {err}")
    if len(active) > 1:
        errors.append("More than one prompt is in progress.")
    if not errors:
        errors.extend(dependency_errors(manifest, state))
    return errors


def render(manifest, state):
    counts = {s: sum(t["status"] == s for t in state["tasks"].values()) for s in sorted(STATUSES)}
    lines = ["# Execution tracker", "", f'Updated: {state["updated_at"]}', "",
             " | ".join(f"{k}: **{v}**" for k, v in counts.items()), "",
             "Generated from `state.json`. Edit status through `python3 codex/scripts/track.py`.", "",
             "`done` records submitted evidence; it is not independent certification of the application.", "",
             "| ID | Phase | Task | Reasoning | Status | Evidence |", "|---|---|---|---|---|---|"]
    for task in manifest["prompts"]:
        row = state["tasks"][task["id"]]
        evidence = row.get("evidence") or "—"
        lines.append(f'| {task["id"]} | {clean(task["phase"])} | {clean(task["title"])} | {task["reasoning"]} | {row["status"]} | {clean(evidence)} |')
    lines += ["", "## History", ""]
    for event in state["history"]:
        lines.append(f'- {event["at"]} — {event["id"]}: {event["action"]}; {clean(event.get("note", ""))}')
    atomic_write(TRACK / "TRACKER.md", "\n".join(lines) + "\n")


def parser():
    p = argparse.ArgumentParser(description=__doc__)
    sub = p.add_subparsers(dest="command", required=True)
    for name in ("next", "summary", "validate", "render"):
        sub.add_parser(name)
    q = sub.add_parser("show")
    q.add_argument("id")
    q = sub.add_parser("start")
    q.add_argument("id")
    q.add_argument("--note", default="")
    q = sub.add_parser("done")
    q.add_argument("id")
    q.add_argument("--evidence", required=True)
    q.add_argument("--check", action="append", required=True)
    q.add_argument("--commit", default="")
    for name in ("block", "reopen"):
        q = sub.add_parser(name)
        q.add_argument("id")
        q.add_argument("--reason", required=True)
        if name == "reopen":
            q.add_argument("--cascade", action="store_true")
    return p


def main():
    args = parser().parse_args()
    TRACK.mkdir(parents=True, exist_ok=True)
    # Linux and macOS share POSIX flock. This lock serializes all state writes.
    with (TRACK / ".tracker.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        manifest, state = read_json(MANIFEST), read_json(STATE)
        tasks = {t["id"]: t for t in manifest["prompts"]}
        tid = getattr(args, "id", "")
        if tid:
            if not tid.isdigit():
                raise ValueError("Prompt ID must be numeric.")
            tid = f"{int(tid):03d}"
            if tid not in tasks:
                raise ValueError(f"Unknown prompt: {tid}")
        errors = validate(manifest, state, evidence=args.command not in {"reopen", "show"})
        if errors:
            raise ValueError("\n".join(errors))
        if args.command == "validate":
            print(f'OK: {len(tasks)} prompts, dependency graph, statuses, hashes and submitted evidence.')
            return
        if args.command == "show":
            print((PACK / tasks[tid]["file"]).read_text(encoding="utf-8"))
            return
        if args.command == "summary":
            print(json.dumps({s: sum(t["status"] == s for t in state["tasks"].values()) for s in sorted(STATUSES)}, indent=2))
            return
        if args.command == "render":
            render(manifest, state)
            print("Updated codex/tracking/TRACKER.md")
            return
        if args.command == "next":
            for task in manifest["prompts"]:
                row = state["tasks"][task["id"]]
                if row["status"] != "done":
                    print(f'{task["id"]} [{row["status"]}] {task["title"]}\nReasoning: {task["reasoning"]}\nPrompt: codex/{task["file"]}')
                    if row["status"] == "blocked":
                        print("Blocker: " + row.get("note", ""))
                    print("Read CODEX_START.md and AGENTS.md; execute only this prompt.")
                    return
            print("All prompts are recorded as done. Check release evidence before distributing.")
            return
        original = copy.deepcopy(state)
        row = state["tasks"][tid]
        note = getattr(args, "note", "") or getattr(args, "reason", "")
        if args.command == "start":
            if row["status"] == "done":
                raise ValueError("Reopen a completed prompt before starting it again.")
            other = [x for x, r in state["tasks"].items() if r["status"] == "in_progress" and x != tid]
            if other:
                raise ValueError("Another prompt is in progress: " + ", ".join(other))
            missing = [x for x in tasks[tid]["depends_on"] if state["tasks"][x]["status"] != "done"]
            if missing:
                raise ValueError("Unfinished prerequisites: " + ", ".join(missing))
            row.update(status="in_progress", started_at=now(), note=note, finished_at=None)
        elif args.command == "block":
            if row["status"] != "in_progress":
                raise ValueError("Only an in-progress prompt can be blocked.")
            if not note.strip():
                raise ValueError("A blocker reason is required.")
            row.update(status="blocked", note=note)
        elif args.command == "done":
            if row["status"] != "in_progress":
                raise ValueError("Start this prompt before marking it done.")
            if any(not c.strip() for c in args.check):
                raise ValueError("Checks must describe actual commands/results.")
            path, digest = checked_evidence(args.evidence)
            row.update(status="done", finished_at=now(), evidence=path,
                       evidence_sha256=digest, checks=args.check, commit=args.commit, note="")
            note = "; ".join(args.check)
        elif args.command == "reopen":
            if not note.strip():
                raise ValueError("A reopen reason is required.")
            affected = {tid}
            for task in manifest["prompts"]:
                if any(d in affected for d in task["depends_on"]):
                    affected.add(task["id"])
            touched = {x for x in affected if state["tasks"][x]["status"] != "pending"}
            descendants = touched - {tid}
            if descendants and not args.cascade:
                raise ValueError("Later work depends on this prompt. Use --cascade to reset its tracked descendants: " + ", ".join(sorted(descendants)))
            for x in (affected if args.cascade else {tid}):
                state["tasks"][x].update(status="pending", note=note, evidence=None,
                                        evidence_sha256=None, checks=[], commit="", started_at=None, finished_at=None)
            note += " | Reset statuses: " + ", ".join(sorted(affected if args.cascade else {tid}))
        state["updated_at"] = now()
        state["history"].append({"at": now(), "id": tid, "action": args.command,
                                 "note": note, "previous": original["tasks"][tid],
                                 "reset_previous": {x: original["tasks"][x] for x in affected} if args.command == "reopen" else {}})
        errors = validate(manifest, state)
        if errors:
            raise ValueError("\n".join(errors))
        atomic_write(STATE, json.dumps(state, indent=2, ensure_ascii=False) + "\n")
        render(manifest, state)
        print(f'{tid}: {state["tasks"][tid]["status"]}')


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        print(f"Tracker error: {error}", file=sys.stderr)
        sys.exit(1)
