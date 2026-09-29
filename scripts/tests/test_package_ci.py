"""Failure-boundary checks for the CI packager; no native execution claims."""
import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from package_ci import checked_report, sha256

ROOT = Path(__file__).resolve().parents[2]


class PackageGateTests(unittest.TestCase):
    def setUp(self):
        self.folder = tempfile.TemporaryDirectory(prefix='containerdesk-package-gate-')
        self.path = Path(self.folder.name)/'report.json'
        self.report = json.loads((ROOT/'docs/verification/051/standard.json').read_text())

    def tearDown(self): self.folder.cleanup()

    def check(self):
        self.path.write_text(json.dumps(self.report))
        return checked_report(self.path)

    def test_passed_checks_are_accepted(self):
        self.assertTrue(self.check()['passed'])

    def test_aggregate_false_is_refused(self):
        self.report['passed'] = False
        with self.assertRaises(ValueError): self.check()

    def test_each_missing_required_check_is_refused(self):
        original = copy.deepcopy(self.report)
        for entry in original['checks']:
            with self.subTest(check=entry['name']):
                self.report = copy.deepcopy(original)
                self.report['checks'] = [r for r in self.report['checks'] if r['name'] != entry['name']]
                with self.assertRaises(ValueError): self.check()

    def test_falsely_aggregate_passed_failure_or_skip_is_refused(self):
        for state, code in [('failed', 1), ('not_run', None), ('passed', 1)]:
            with self.subTest(state=state, code=code):
                self.report['checks'][0].update(status=state, exitCode=code)
                with self.assertRaises(ValueError): self.check()

    def test_installation_is_not_verification(self):
        self.report['mode'] = 'install'
        with self.assertRaises(ValueError): self.check()

    def test_digest_known_value(self):
        self.path.write_bytes(b'abc')
        self.assertEqual(sha256(self.path), 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad')


if __name__ == '__main__': unittest.main()
