"""Published-download generation and explicit Pages rebuild boundaries; no remote writes."""
import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import release
import release_site

REPO = 'alekpopovic/container-desk'


def fixture(version='0.1.1', identifier=2, date='2026-09-29T18:00:00Z'):
    tag = 'v' + version
    names = {f'containerdesk-{version}-{target}{suffix}' for target, suffixes in release.TARGETS.items() for suffix in suffixes}
    names |= {'SHA256SUMS', 'release-provenance.json'}
    return dict(id=identifier, tag_name=tag, draft=False, prerelease=True, published_at=date,
                assets=[dict(name=name, size=123, state='uploaded', browser_download_url=f'https://github.com/{REPO}/releases/download/{tag}/{name}') for name in names])


class PageGenerationTests(unittest.TestCase):
    def test_all_six_downloads_and_preview_history(self):
        new, old = fixture(), fixture('0.1.0', 1, '2026-09-28T18:00:00Z')
        draft = {**fixture('0.1.2', 3), 'draft': True}
        documents = release_site.render([old, draft, new], REPO)
        self.assertEqual(set(documents), set(release_site.FILES))
        self.assertIn('containerdesk-release:v0.1.1', documents['docs/downloads.md'])
        self.assertIn('Gatekeeper', documents['docs/downloads.md'])
        for target, suffixes in release.TARGETS.items():
            for suffix in suffixes:
                self.assertIn(f'containerdesk-0.1.1-{target}{suffix}', documents['docs/downloads.md'])
        self.assertIn('v0.1.0', documents['docs/releases.md'])
        self.assertNotIn('v0.1.2', documents['docs/releases.md'])
        self.assertEqual(json.loads(documents['docs/_data/current_release.json'])['tag'], 'v0.1.1')

    def test_partial_release_does_not_overwrite_existing_pages(self):
        bad = fixture()
        bad['assets'].pop()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root/'docs').mkdir()
            (root/'docs/downloads.md').write_text('existing')
            with self.assertRaises(ValueError): release_site.write([bad], REPO, root)
            self.assertEqual((root/'docs/downloads.md').read_text(), 'existing')

    def test_empty_history_and_external_asset_url_are_rejected(self):
        with self.assertRaises(ValueError): release_site.render([], REPO)
        bad = fixture()
        bad['assets'][0]['browser_download_url'] = 'https://untrusted.invalid/installer'
        with self.assertRaises(ValueError): release_site.render([bad], REPO)

    def test_current_data_moves_only_to_published_release(self):
        older, draft = fixture(), {**fixture('0.1.2', 3), 'draft': True}
        before = release_site.render([draft, older], REPO)
        after = release_site.render([{**draft, 'draft': False}, older], REPO)
        self.assertEqual(json.loads(before['docs/_data/current_release.json'])['version'], '0.1.1')
        self.assertEqual(json.loads(after['docs/_data/current_release.json'])['version'], '0.1.2')


class PagesBuildTests(unittest.TestCase):
    site = dict(build_type='legacy', source={'branch': 'main', 'path': '/docs'}, html_url='https://alekpopovic.github.io/container-desk/')

    def test_explicit_post_and_wait_past_previous_build(self):
        old = dict(status='built', commit='a'*40)
        new = dict(status='built', commit='b'*40, url='https://api.github.com/build/2')
        with patch.object(release, 'api', side_effect=[self.site, {'status': 'queued'}, old, new]) as api, \
             patch.object(release, 'git'), patch.object(release_site.time, 'sleep'), \
             patch.object(release_site.subprocess, 'run', side_effect=[subprocess.CompletedProcess([], 1), subprocess.CompletedProcess([], 0)]):
            url, build = release_site.rebuild(REPO, 'b'*40)
            self.assertEqual(build, new)
            self.assertEqual(url, self.site['html_url'])
            self.assertIn(unittest.mock.call(f'repos/{REPO}/pages/builds', 'POST'), api.call_args_list)

    def test_wrong_pages_source_is_not_silently_changed(self):
        with patch.object(release, 'api', return_value={**self.site, 'build_type': 'workflow'}) as api:
            with self.assertRaisesRegex(ValueError, 'main /docs'): release_site.rebuild(REPO, 'b'*40)
            self.assertEqual(api.call_count, 1)

    def test_failed_current_pages_build_is_reported(self):
        failed = dict(status='errored', commit='b'*40, error={'message': 'Build failed'})
        with patch.object(release, 'api', side_effect=[self.site, {}, failed]), patch.object(release, 'git'), \
             patch.object(release_site.subprocess, 'run', return_value=subprocess.CompletedProcess([], 0)):
            with self.assertRaisesRegex(ValueError, 'Pages build failed'): release_site.rebuild(REPO, 'b'*40)

    def test_public_downloads_must_show_new_marker(self):
        from io import BytesIO
        with patch.object(release_site, 'urlopen', side_effect=[BytesIO(b'old content'), BytesIO(b'<!-- containerdesk-release:v0.1.1 -->')]), \
             patch.object(release_site.time, 'sleep'):
            release_site.verify_live(self.site['html_url'], 'v0.1.1')


if __name__ == '__main__': unittest.main()
