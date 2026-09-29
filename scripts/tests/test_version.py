"""Version mismatch and release bump checks against isolated repository fixtures."""
import json
from pathlib import Path
import shutil
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from version import ROOT, FILES, check, set_version


class VersionTests(unittest.TestCase):
    def setUp(self):
        self.folder = tempfile.TemporaryDirectory(prefix='cd-version-')
        self.root = Path(self.folder.name)
        (self.root/'src-tauri').mkdir()
        for name in (*FILES, 'src-tauri/tauri.conf.json'): shutil.copy2(ROOT/name, self.root/name)

    def tearDown(self): self.folder.cleanup()

    def test_authoritative_version_and_all_derived_versions_align(self):
        before = (self.root/'src-tauri/Cargo.lock').read_text()
        set_version('2.3.4', self.root)
        self.assertEqual(check(self.root), '2.3.4')
        # Dependency entries and their checksums must remain byte-for-byte unchanged.
        after = (self.root/'src-tauri/Cargo.lock').read_text()
        old_version = json.loads((ROOT/'package.json').read_text())['version']
        self.assertEqual(after.replace('name = "containerdesk"\nversion = "2.3.4"',
            f'name = "containerdesk"\nversion = "{old_version}"'), before)

    def test_each_derived_version_mismatch_is_rejected(self):
        for name in FILES[1:]:
            with self.subTest(file=name):
                path = self.root/name
                before = path.read_text()
                if name.endswith('.json'):
                    value = json.loads(before)
                    value['version'] = '99.0.0'
                    path.write_text(json.dumps(value))
                else:
                    path.write_text(before.replace('name = "containerdesk"\nversion = "', 'name = "containerdesk"\nversion = "99.', 1))
                with self.assertRaisesRegex(ValueError, 'mismatch'): check(self.root)
                with self.assertRaises(ValueError): set_version('3.0.0', self.root)
                path.write_text(before)

    def test_nested_npm_lock_root_mismatch_is_rejected(self):
        path = self.root/'package-lock.json'
        value = json.loads(path.read_text())
        value['packages']['']['version'] = '99.0.0'
        path.write_text(json.dumps(value))
        with self.assertRaisesRegex(ValueError, 'mismatch'): check(self.root)

    def test_literal_bundle_version_is_rejected(self):
        path = self.root/'src-tauri/tauri.conf.json'
        value = json.loads(path.read_text())
        value['version'] = check(self.root)
        path.write_text(json.dumps(value))
        with self.assertRaisesRegex(ValueError, 'authoritative'): check(self.root)

    def test_invalid_version_does_not_change_any_file(self):
        before = {p: (self.root/p).read_bytes() for p in FILES}
        for bad in ['v1.2.3', '01.2.3', '1.2', '1.2.3-rc.1', '../next', '1.2.3\n']:
            with self.assertRaises(ValueError): set_version(bad, self.root)
        self.assertEqual(before, {p: (self.root/p).read_bytes() for p in FILES})


if __name__ == '__main__': unittest.main()
