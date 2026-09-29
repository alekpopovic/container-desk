"""Release failure boundaries; isolated Git remotes and mocked GitHub, no public writes."""
import copy
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import release
from version import FILES, ROOT


class VersionAndDraftTests(unittest.TestCase):
    def test_strict_version_input(self):
        for value in ['0.1.2', 'v0.1.2']:
            self.assertEqual(release.version_number(value), '0.1.2')
        for value in ['1.2', '01.2.3', 'v1.2.3\n', '1.2.3;echo bad', '$(whoami)', '../x', '1.2.3-rc.1', ' 1.2.3']:
            with self.subTest(value=value), self.assertRaises(ValueError):
                release.version_number(value)

    def test_previous_release_includes_preview_and_ignores_draft(self):
        entries = [dict(id=1, draft=False, prerelease=False, published_at='2026-01-01', tag_name='v0.1.0'),
                   dict(id=2, draft=False, prerelease=True, published_at='2026-02-01', tag_name='v0.2.0'),
                   dict(id=3, draft=True, published_at='2026-03-01', tag_name='v0.3.0')]
        self.assertEqual(release.previous_release(entries)['tag_name'], 'v0.2.0')
        self.assertIsNone(release.previous_release([]))

    def test_only_same_run_draft_can_resume(self):
        draft = dict(draft=True, body='Notes\nmarker', target_commitish='a'*40, prerelease=True)
        release.owned_draft(draft, 'marker', 'a'*40, True)
        for key, value in [('draft', False), ('body', 'other run'), ('target_commitish', 'b'*40), ('prerelease', False)]:
            with self.subTest(key=key), self.assertRaises(ValueError):
                release.owned_draft({**draft, key: value}, 'marker', 'a'*40, True)

    def test_dispatch_scope(self):
        good = dict(GITHUB_REPOSITORY='alekpopovic/container-desk', GITHUB_RUN_ID='12',
                    GITHUB_EVENT_NAME='workflow_dispatch', GITHUB_REF='refs/heads/main')
        with patch.dict(os.environ, good):
            self.assertEqual(release.context(), ('alekpopovic/container-desk', '12'))
        for key, value in [('GITHUB_REPOSITORY', 'fork/container-desk'), ('GITHUB_EVENT_NAME', 'pull_request'),
                           ('GITHUB_REF', 'refs/heads/feature'), ('GITHUB_RUN_ID', '../run')]:
            with patch.dict(os.environ, {**good, key: value}), self.assertRaises(ValueError):
                release.context()


class GitPreparationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='cd-release-test-')
        self.folder = Path(self.temp.name)
        self.root = self.folder/'repo'
        self.root.mkdir()
        (self.root/'src-tauri').mkdir()
        for name in (*FILES, 'src-tauri/tauri.conf.json'):
            shutil.copy2(ROOT/name, self.root/name)
        self.git('init', '-b', 'main')
        self.git('config', 'user.name', 'Fixture')
        self.git('config', 'user.email', 'fixture@example.invalid')
        # Normalize test fixture irrespective of the repository's future version.
        release.set_version('0.1.0', self.root)
        self.git('add', '.')
        self.git('commit', '-m', 'Initial release')
        self.initial = self.git('rev-parse', 'HEAD')
        self.git('tag', 'v0.1.0')
        self.git('init', '--bare', str(self.folder/'origin.git'))
        self.git('remote', 'add', 'origin', str(self.folder/'origin.git'))
        (self.root/'feature.txt').write_text('Feature')
        self.git('add', '.')
        self.git('commit', '-m', 'Feature <script> [link](bad)')
        self.source = self.git('rev-parse', 'HEAD')
        self.git('push', 'origin', 'main', '--tags')
        self.root_patch = patch.object(release, 'ROOT', self.root)
        self.root_patch.start()
        self.releases = [dict(id=1, tag_name='v0.1.0', draft=False, prerelease=True, published_at='2026-01-01')]
        self.pages_patch = patch.object(release, 'pages', return_value=self.releases)
        self.pages_patch.start()

    def tearDown(self):
        self.pages_patch.stop()
        self.root_patch.stop()
        self.temp.cleanup()

    def git(self, *args):
        return subprocess.check_output(['git', *args], cwd=self.root, text=True, stderr=subprocess.DEVNULL).strip()

    def prepare(self, selected=None):
        return release.prepare('v0.1.1', 'alekpopovic/container-desk', '123', selected or self.source)

    def test_bump_exact_source_and_safe_retry(self):
        result = self.prepare()
        self.assertEqual(release.check(self.root), '0.1.1')
        self.assertEqual(result['previous_sha'], self.initial)
        self.assertEqual(self.git('rev-parse', 'HEAD^'), self.source)
        self.assertEqual(self.git('rev-parse', 'refs/remotes/origin/main'), result['source_sha'])
        self.assertEqual(set(self.git('diff-tree', '--no-commit-id', '--name-only', '-r', 'HEAD').splitlines()), set(FILES))
        self.assertEqual(self.prepare(), result)
        notes, count = release.commit_notes('alekpopovic/container-desk', 'v0.1.0', self.initial, result['source_sha'])
        self.assertEqual(count, 2)
        self.assertNotIn('Initial release', notes)
        self.assertIn('&lt;script&gt;', notes)
        self.assertIn(r'\[link\]', notes)
        self.assertIn('prepare v0.1.1', notes)

    def test_moving_main_is_rejected_without_bump(self):
        (self.root/'feature.txt').write_text('Concurrent change')
        self.git('commit', '-am', 'Concurrent')
        self.git('push', 'origin', 'main')
        with self.assertRaisesRegex(ValueError, 'main advanced'):
            self.prepare()
        self.assertEqual(release.check(self.root), '0.1.0')

    def test_same_version_after_failed_build_can_start_new_run(self):
        first = self.prepare()
        second = release.prepare('0.1.1', 'alekpopovic/container-desk', '456', first['source_sha'])
        self.assertEqual(first, second)

    def test_existing_tag_never_replaced(self):
        self.git('tag', 'v0.1.1')
        with self.assertRaisesRegex(ValueError, 'Tag already exists'):
            self.prepare()

    def test_existing_release_prevents_version_commit(self):
        self.releases.append(dict(id=2, tag_name='v0.1.1', draft=True, published_at=None))
        with self.assertRaisesRegex(ValueError, 'Release already exists'):
            self.prepare()
        self.assertEqual(self.git('rev-parse', 'HEAD'), self.source)

    def test_full_commit_range_includes_merged_branch_and_merge_commit(self):
        self.git('checkout', '-b', 'feature')
        (self.root/'branch.txt').write_text('Branch')
        self.git('add', '.')
        self.git('commit', '-m', 'Branch change')
        self.git('checkout', 'main')
        self.git('merge', '--no-ff', 'feature', '-m', 'Merge feature')
        notes, count = release.commit_notes('alekpopovic/container-desk', 'v0.1.0', self.initial, self.git('rev-parse', 'HEAD'))
        self.assertEqual(count, 3)
        self.assertIn('Branch change', notes)
        self.assertIn('Merge feature', notes)

    def test_previous_tag_move_is_rejected(self):
        self.git('tag', '-f', 'v0.1.0')
        with self.assertRaisesRegex(ValueError, 'tag moved'):
            release.commit_notes('alekpopovic/container-desk', 'v0.1.0', self.initial, self.source)


class ArtifactTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='cd-release-artifact-')
        self.folder = Path(self.temp.name)
        self.target = 'x86_64-unknown-linux-gnu'
        self.stem = 'containerdesk-0.1.1-' + self.target
        self.metadata = dict(schemaVersion=2, version='0.1.1', target=self.target, sourceCommit='a'*40,
                             runId='123', sourceDirty=False, signing='disabled', notarization='not_requested')
        self.metadata['packages'] = []
        for suffix in release.TARGETS[self.target]:
            path = self.folder/(self.stem + suffix)
            path.write_bytes(b'owned test package')
            self.metadata['packages'].append(dict(file=path.name, bytes=path.stat().st_size, sha256=release.digest(path)))
        self.write_manifest()

    def tearDown(self): self.temp.cleanup()

    def write_manifest(self):
        (self.folder/(self.stem + '.json')).write_text(json.dumps(self.metadata))
        (self.folder/'SHA256SUMS').write_text(''.join(f'{release.digest(p)}  {p.name}\n' for p in self.folder.iterdir() if p.name != 'SHA256SUMS'))

    def validate(self):
        return release.validate_artifact(self.folder, '0.1.1', self.target, 'a'*40, '123')

    def test_complete_artifact_is_accepted(self): self.assertEqual(self.validate(), self.metadata)

    def test_wrong_source_target_version_run_and_dirty_are_rejected(self):
        original = copy.deepcopy(self.metadata)
        for key, value in [('sourceCommit', 'b'*40), ('target', 'aarch64-apple-darwin'), ('version', '0.1.2'), ('runId', '999'), ('sourceDirty', True)]:
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.metadata = {**original, key: value}
                self.write_manifest()
                self.validate()

    def test_corrupt_installer_is_rejected_even_with_new_manifest(self):
        (self.folder/(self.stem + '.deb')).write_bytes(b'corrupt')
        self.write_manifest()
        with self.assertRaisesRegex(ValueError, 'checksum mismatch'): self.validate()

    def test_missing_package_is_rejected(self):
        (self.folder/(self.stem + '.deb')).unlink()
        with self.assertRaises(ValueError): self.validate()

    def test_extra_file_and_symlink_are_rejected(self):
        extra = self.folder/'unexpected'
        extra.write_text('extra')
        with self.assertRaises(ValueError): self.validate()
        extra.unlink()
        target = self.folder/(self.stem + '.deb')
        target.unlink()
        target.symlink_to(self.folder/(self.stem + '.AppImage'))
        with self.assertRaises(ValueError): self.validate()

    def test_manifest_tampering_and_duplicates_are_rejected(self):
        sums = self.folder/'SHA256SUMS'
        sums.write_text(sums.read_text() + sums.read_text().splitlines()[0] + '\n')
        with self.assertRaises(ValueError): self.validate()

    def test_remote_assets_must_be_complete_and_match_server_hash(self):
        path = self.folder/(self.stem + '.deb')
        expected = {path.name: path}
        asset = dict(name=path.name, state='uploaded', size=path.stat().st_size, digest='sha256:' + release.digest(path))
        release.verify_assets([asset], expected, complete=True)
        for assets in [[], [asset, asset], [{**asset, 'digest': 'sha256:' + '0'*64}], [{**asset, 'name': 'unexpected'}]]:
            with self.assertRaises(ValueError): release.verify_assets(assets, expected, complete=True)


class PublishFlowTests(unittest.TestCase):
    git = GitPreparationTests.git
    prepare = GitPreparationTests.prepare
    def setUp(self):
        GitPreparationTests.setUp(self)
        self.prepared = self.prepare()
        self.sha = self.prepared['source_sha']
        self.downloads = {}
        self.artifacts = []
        self.remote_assets = []
        self.draft = None
        self.did_publish = False
        self.fail_upload = False
        self.api_writes = []
        self.real_command = release.command
        for target, suffixes in release.TARGETS.items():
            folder = self.folder/target
            folder.mkdir()
            stem = f'containerdesk-0.1.1-{target}'
            packages = []
            for suffix in suffixes:
                path = folder/(stem+suffix)
                path.write_bytes((target+suffix).encode())
                packages.append(dict(file=path.name, bytes=path.stat().st_size, sha256=release.digest(path)))
            metadata = dict(schemaVersion=2, version='0.1.1', target=target, sourceCommit=self.sha, sourceDirty=False,
                            runId='123', signing='disabled', notarization='not_requested', packages=packages,
                            locks={name: release.digest(self.root/name) for name in ['package-lock.json','src-tauri/Cargo.lock']})
            (folder/(stem+'.json')).write_text(json.dumps(metadata))
            (folder/'SHA256SUMS').write_text(''.join(f'{release.digest(p)}  {p.name}\n' for p in folder.iterdir()))
            name = stem+'-'+self.sha+'-1'
            self.artifacts.append(dict(name=name, expired=False))
            self.downloads[name] = folder
        self.pages_patch.stop()
        self.pages_patch = patch.object(release, 'pages', side_effect=self.fake_pages)
        self.pages_patch.start()
        self.api_patch = patch.object(release, 'api', side_effect=self.fake_api)
        self.api_patch.start()
        self.command_patch = patch.object(release, 'command', side_effect=self.fake_command)
        self.command_patch.start()

    def tearDown(self):
        self.command_patch.stop()
        self.api_patch.stop()
        GitPreparationTests.tearDown(self)

    def fake_pages(self, endpoint):
        if endpoint.endswith('/artifacts'): return self.artifacts
        if endpoint.endswith('/assets'): return self.remote_assets
        if endpoint.endswith('/releases'): return self.releases + ([self.draft] if self.draft else [])
        raise AssertionError(endpoint)

    def fake_api(self, endpoint, method='GET', data=None):
        if method == 'POST':
            self.api_writes.append((method, endpoint))
            self.draft = {**data, 'id':22, 'published_at':None, 'html_url':'https://github.com/alekpopovic/container-desk/releases/tag/v0.1.1'}
        elif method == 'PATCH':
            self.assertEqual(len(self.remote_assets), 12)
            self.api_writes.append((method, endpoint))
            self.did_publish = True
            self.draft.update(data, published_at='2026-02-01')
            self.git('tag', 'v0.1.1', self.sha)
            self.git('push', 'origin', 'refs/tags/v0.1.1')
        return copy.deepcopy(self.draft)

    def fake_command(self, argv, **kwargs):
        if argv[:3] == ['gh', 'run', 'download']:
            source = self.downloads[argv[argv.index('--name')+1]]
            shutil.copytree(source, argv[argv.index('--dir')+1])
            return ''
        if argv[:3] == ['gh', 'release', 'upload']:
            if self.fail_upload and len(self.remote_assets) == 1:
                self.fail_upload = False
                raise subprocess.CalledProcessError(1, argv)
            path = Path(argv[4])
            self.remote_assets.append(dict(name=path.name, size=path.stat().st_size,
                                           digest='sha256:'+release.digest(path), state='uploaded'))
            return ''
        return self.real_command(argv, **kwargs)

    def publish(self):
        release.publish('0.1.1', 'alekpopovic/container-desk', '123', self.sha, 'v0.1.0', self.initial, True)

    def test_publish_requires_all_twelve_assets_before_publication(self):
        self.publish()
        self.assertTrue(self.did_publish)
        self.assertEqual(len(self.remote_assets), 12)
        self.assertIn('Feature &lt;script&gt;', self.draft['body'])
        self.assertEqual(self.git('rev-parse','v0.1.1'), self.sha)

    def test_failed_upload_leaves_draft_and_same_run_resumes(self):
        self.fail_upload = True
        with self.assertRaises(subprocess.CalledProcessError): self.publish()
        self.assertFalse(self.did_publish)
        self.assertTrue(self.draft['draft'])
        self.publish()
        self.assertTrue(self.did_publish)
        self.assertEqual(len([x for x in self.api_writes if x[0]=='POST']), 1)
        self.assertEqual(len(self.remote_assets), 12)

    def test_missing_target_never_creates_draft(self):
        self.artifacts.pop()
        with self.assertRaisesRegex(ValueError, 'Missing CI package artifact'): self.publish()
        self.assertEqual(self.api_writes, [])


if __name__ == '__main__': unittest.main()
