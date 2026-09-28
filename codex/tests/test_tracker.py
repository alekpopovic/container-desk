"""Meaningful state-machine checks on disposable pack copies; no SSH/network."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

SOURCE = Path(__file__).resolve().parents[1]


class TrackerTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name) / "project with spaces"
        self.pack = self.root / "codex"
        shutil.copytree(SOURCE, self.pack, ignore=shutil.ignore_patterns("__pycache__", ".tracker.lock"))
        # Each test starts from the distributed pending state, even if run later
        # from an implementation repository with completed prompts.
        manifest = json.loads((self.pack / "manifest.json").read_text())
        state = dict(schema_version=1, updated_at="test", history=[], tasks={p["id"]: dict(
            status="pending", started_at=None, finished_at=None, evidence=None,
            evidence_sha256=None, checks=[], commit="", note="") for p in manifest["prompts"]})
        (self.pack / "tracking/state.json").write_text(json.dumps(state))

    def tearDown(self):
        self.tmp.cleanup()

    def run_track(self, *args, ok=True):
        result = subprocess.run([sys.executable, str(self.pack / "scripts/track.py"), *args],
                                cwd=self.root, text=True, capture_output=True, timeout=10)
        if ok:
            self.assertEqual(result.returncode, 0, result.stderr)
        else:
            self.assertNotEqual(result.returncode, 0, result.stdout)
        return result

    def state(self):
        return json.loads((self.pack / "tracking/state.json").read_text())

    def finish(self, tid="001"):
        self.run_track("start", tid)
        evidence = self.pack / f"tracking/evidence/{tid}.md"
        evidence.write_text("# Synthetic test evidence\n\nThis is a tracker unit-test fixture. No application or native SSH work was performed.\nChecks: this fixture exercises state transitions in a disposable copy.\n")
        self.run_track("done", tid, "--evidence", f"codex/tracking/evidence/{tid}.md", "--check", "Synthetic tracker transition check")

    def test_initial_state_and_next(self):
        self.run_track("validate")
        self.assertIn("001 [pending]", self.run_track("next").stdout)
        self.assertEqual(json.loads(self.run_track("summary").stdout)["pending"], 60)

    def test_prerequisites_prevent_skipping(self):
        self.assertIn("prerequisites", self.run_track("start", "002", ok=False).stderr)
        self.assertEqual(self.state()["tasks"]["002"]["status"], "pending")

    def test_start_block_resume_complete(self):
        self.run_track("start", "1")
        self.run_track("block", "001", "--reason", "Test blocker")
        self.assertIn("Test blocker", self.run_track("next").stdout)
        self.finish()
        self.assertIn("002 [pending]", self.run_track("next").stdout)
        self.assertEqual([x["action"] for x in self.state()["history"]], ["start", "block", "start", "done"])

    def test_done_requires_started_task_and_real_file(self):
        self.run_track("done", "001", "--evidence", "missing.md", "--check", "example", ok=False)
        self.run_track("start", "001")
        self.assertIn("not found", self.run_track("done", "001", "--evidence", "missing.md", "--check", "example", ok=False).stderr)

    def test_short_evidence_rejected(self):
        self.run_track("start", "001")
        (self.root / "short.md").write_text("done")
        self.assertIn("too short", self.run_track("done", "001", "--evidence", "short.md", "--check", "example", ok=False).stderr)

    def test_external_and_symlink_evidence_rejected(self):
        self.run_track("start", "001")
        outside = self.root.parent / "outside.md"
        outside.write_text("outside evidence " * 20)
        (self.root / "escape.md").symlink_to(outside)
        for value in ("../outside.md", "escape.md", str(outside)):
            self.run_track("done", "001", "--evidence", value, "--check", "example", ok=False)
        self.assertEqual(self.state()["tasks"]["001"]["status"], "in_progress")

    def test_evidence_hash_detects_later_changes_and_reopen_recovers(self):
        self.finish()
        evidence = self.pack / "tracking/evidence/001.md"
        evidence.write_text(evidence.read_text() + "\nCorrected evidence after completion.\n")
        self.assertIn("evidence changed", self.run_track("validate", ok=False).stderr)
        self.run_track("reopen", "001", "--reason", "Recheck corrected evidence")
        self.run_track("validate")

    def test_cascade_preserves_previous_rows_and_files(self):
        self.finish("001")
        self.finish("002")
        self.run_track("reopen", "001", "--reason", "Review", ok=False)
        self.run_track("reopen", "001", "--cascade", "--reason", "Review")
        state = self.state()
        self.assertEqual(state["tasks"]["002"]["status"], "pending")
        self.assertEqual(state["history"][-1]["reset_previous"]["002"]["status"], "done")
        self.assertTrue((self.pack / "tracking/evidence/002.md").is_file())

    def test_prompt_tampering_detected(self):
        manifest = json.loads((self.pack / "manifest.json").read_text())
        target = self.pack / manifest["prompts"][0]["file"]
        target.write_text(target.read_text() + "modified")
        self.assertIn("prompt differs", self.run_track("validate", ok=False).stderr)

    def test_render_matches_saved_state(self):
        self.finish()
        self.run_track("render")
        markdown = (self.pack / "tracking/TRACKER.md").read_text()
        self.assertIn("done: **1**", markdown)
        self.assertIn("codex/tracking/evidence/001.md", markdown)

    def test_invalid_id_and_empty_reasons(self):
        self.run_track("show", "unknown", ok=False)
        self.run_track("show", "999", ok=False)
        self.run_track("start", "001")
        self.run_track("block", "001", "--reason", " ", ok=False)
        self.run_track("reopen", "001", "--reason", " ", ok=False)

    def test_one_active_task(self):
        self.run_track("start", "001")
        result = self.run_track("start", "002", ok=False)
        self.assertIn("Another prompt", result.stderr)

    def test_concurrent_writes_are_serialized(self):
        command = [sys.executable, str(self.pack / "scripts/track.py"), "start", "001"]
        first = subprocess.Popen(command, cwd=self.root, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        second = subprocess.Popen(command, cwd=self.root, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        first.communicate(timeout=10)
        second.communicate(timeout=10)
        self.assertEqual(first.returncode, 0)
        self.assertEqual(second.returncode, 0)
        self.assertEqual(len(self.state()["history"]), 2)
        self.run_track("validate")

    def test_printed_prompt_and_hash_match(self):
        manifest = json.loads((self.pack / "manifest.json").read_text())
        first = manifest["prompts"][0]
        data = (self.pack / first["file"]).read_bytes()
        self.assertEqual(hashlib.sha256(data).hexdigest(), first["sha256"])
        self.assertIn("Repository audit", self.run_track("show", "1").stdout)


if __name__ == "__main__":
    unittest.main()
