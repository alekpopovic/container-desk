#!/usr/bin/env python3
"""Explicit manual release: reuse native CI; publish only a complete verified asset set."""
import argparse
import hashlib
import html
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

from version import ROOT, FILES, check, set_version

TARGETS = {
    'x86_64-unknown-linux-gnu': ('.deb', '.AppImage'),
    'aarch64-apple-darwin': ('.app.tar.gz', '.dmg'),
    'x86_64-apple-darwin': ('.app.tar.gz', '.dmg'),
}


def command(argv, root=ROOT, **kwargs):
    try:
        return subprocess.run(argv, cwd=root, text=True, capture_output=True,
                              check=True, timeout=180, **kwargs).stdout.strip()
    except subprocess.CalledProcessError as error:
        # Preserve actionable API/protected-branch errors; credentials stay in GH_TOKEN.
        print((error.stderr or error.stdout or str(error))[-8000:], file=sys.stderr)
        raise


def git(*args):
    return command(['git', *args], root=ROOT)


def api(endpoint, method='GET', data=None):
    argv = ['gh', 'api', endpoint, '--method', method]
    if data is not None:
        argv += ['--input', '-']
    result = command(argv, input=json.dumps(data) if data is not None else None)
    return json.loads(result) if result else None


def pages(endpoint):
    # Paginate explicitly: GitHub's latest-release endpoint excludes pre-releases.
    for number in range(1, 1001):
        batch = api(f'{endpoint}?per_page=100&page={number}')
        items = batch.get('artifacts', []) if isinstance(batch, dict) else batch
        yield from items
        if len(items) < 100:
            return
    raise ValueError('Pagination limit exceeded; refusing incomplete release history')


def version_number(value):
    if not re.fullmatch(r'v?(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)', value):
        raise ValueError('Use MAJOR.MINOR.PATCH (optional v), without leading zeroes or suffixes')
    return value.removeprefix('v')


def sha(value):
    if not re.fullmatch(r'[0-9a-f]{40}', value):
        raise ValueError('Expected an exact 40-character source commit')
    return value


def context():
    repo, run = os.environ['GITHUB_REPOSITORY'], os.environ['GITHUB_RUN_ID']
    if repo != 'alekpopovic/container-desk' or not re.fullmatch(r'[0-9]+', run):
        raise ValueError('Release runs only in the owner repository')
    if os.environ.get('GITHUB_EVENT_NAME') != 'workflow_dispatch' or os.environ.get('GITHUB_REF') != 'refs/heads/main':
        raise ValueError('Release requires manual dispatch from main')
    return repo, run


def previous_release(releases):
    public = [r for r in releases if not r['draft'] and r.get('published_at')]
    return max(public, key=lambda r: (r['published_at'], r['id'])) if public else None


def prepare(version, repo, run, selected):
    version = version_number(version)
    selected = sha(selected)
    tag = 'v' + version
    releases = list(pages(f'repos/{repo}/releases'))
    if any(r['tag_name'] == tag for r in releases):
        raise ValueError('Release already exists; re-run a failed publish job to resume its draft')
    previous = previous_release(releases)
    git('fetch', 'origin', 'main', '--tags')
    if git('tag', '--list', tag):
        raise ValueError('Tag already exists; it will never be overwritten')
    latest = git('rev-parse', 'refs/remotes/origin/main')
    if latest != selected:
        # A full retry may resume only this run's exact version-only commit.
        marker = f'ContainerDesk-Release-Run: {run}'
        changed = set(git('diff-tree', '--no-commit-id', '--name-only', '-r', latest).splitlines())
        if (git('show', '-s', '--format=%P', latest) != selected
                or marker not in git('show', '-s', '--format=%B', latest).splitlines()
                or not changed or not changed <= set(FILES)):
            raise ValueError('main advanced since dispatch; start a new run from current main')
    git('checkout', '--detach', latest)
    current = check(ROOT)
    if latest != selected and current != version:
        raise ValueError('Retry version differs from this run\'s prepared commit')
    if tuple(map(int, version.split('.'))) < tuple(map(int, current.split('.'))):
        raise ValueError('Version must not move backwards')
    previous_tag, previous_sha = '', ''
    if previous:
        previous_tag = previous['tag_name']
        old = version_number(previous_tag)
        if tuple(map(int, version.split('.'))) <= tuple(map(int, old.split('.'))):
            raise ValueError('Version must be newer than the last published release')
        previous_sha = sha(git('rev-parse', f'refs/tags/{previous_tag}^{{commit}}'))
        git('merge-base', '--is-ancestor', previous_sha, latest)
    if git('status', '--porcelain'):
        raise ValueError('Release preparation needs a clean checkout')
    if current != version:
        set_version(version, ROOT)
        changed = set(git('diff', '--name-only').splitlines())
        if not changed <= set(FILES):
            raise ValueError('Unexpected files changed during the version update')
        git('add', '--', *FILES)
        git('-c', 'user.name=github-actions[bot]', '-c', 'user.email=41898282+github-actions[bot]@users.noreply.github.com',
            'commit', '-m', f'release: prepare v{version}', '-m', f'ContainerDesk-Release-Run: {run}')
        # A normal fast-forward push refuses concurrent main updates; never force.
        git('push', 'origin', 'HEAD:refs/heads/main')
    source = sha(git('rev-parse', 'HEAD'))
    return dict(version=version, source_sha=source, previous_tag=previous_tag, previous_sha=previous_sha)


def digest(path):
    with path.open('rb') as stream:
        result = hashlib.sha256()
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            result.update(block)
        return result.hexdigest()


def validate_artifact(folder, version, target, source, run):
    stem = f'containerdesk-{version}-{target}'
    expected = {stem + suffix for suffix in TARGETS[target]}
    expected |= {stem + '.json', 'SHA256SUMS'}
    actual = {p.name for p in folder.iterdir()}
    if actual != expected or any(p.is_symlink() or not p.is_file() for p in folder.iterdir()):
        raise ValueError('Artifact has missing, unexpected or non-regular files')
    metadata = json.loads((folder/(stem + '.json')).read_text())
    for key, value in [('version', version), ('target', target), ('sourceCommit', source), ('runId', run)]:
        if metadata.get(key) != value:
            raise ValueError(f'Artifact {key} does not match this release run')
    if metadata.get('sourceDirty') is not False or metadata.get('schemaVersion') != 2:
        raise ValueError('Artifact must describe a clean, verified source')
    if metadata.get('signing') != 'disabled' or metadata.get('notarization') != 'not_requested':
        raise ValueError('Unexpected signing metadata in unsigned release')
    packages = metadata.get('packages', [])
    if len(packages) != 2 or {p['file'] for p in packages} != expected - {stem + '.json', 'SHA256SUMS'}:
        raise ValueError('Incomplete native package metadata')
    for entry in packages:
        path = folder/entry['file']
        if entry['bytes'] <= 0 or path.stat().st_size != entry['bytes'] or digest(path) != entry['sha256']:
            raise ValueError('Installer size or checksum mismatch')
    expected_sums = {p.name: digest(p) for p in folder.iterdir() if p.name != 'SHA256SUMS'}
    lines = (folder/'SHA256SUMS').read_text().splitlines()
    parsed = {}
    for line in lines:
        match = re.fullmatch(r'([0-9a-f]{64})  ([A-Za-z0-9_.-]+)', line)
        if not match or match[2] in parsed:
            raise ValueError('Invalid or duplicate checksum entry')
        parsed[match[2]] = match[1]
    if parsed != expected_sums:
        raise ValueError('Artifact manifest mismatch')
    return metadata


def commit_notes(repo, previous_tag, previous_sha, source):
    sha(source)
    if previous_sha:
        sha(previous_sha)
        version_number(previous_tag)
        if git('rev-parse', f'refs/tags/{previous_tag}^{{commit}}') != previous_sha:
            raise ValueError('Previous release tag moved during the build')
        git('merge-base', '--is-ancestor', previous_sha, source)
    revision = f'{previous_sha}..{source}' if previous_sha else source
    lines = git('log', '--reverse', '--topo-order', '--format=%H%x00%s', revision).splitlines()
    notes = ['# Commits since ' + (previous_tag or 'the beginning of the repository'), '']
    for line in lines:
        commit, title = line.split('\0', 1)
        sha(commit)
        title = ''.join(c if c.isprintable() else ' ' for c in title)
        title = re.sub(r'([\\`*_\[\]{}!|])', r'\\\1', html.escape(title))
        notes.append(f'- [`{commit[:12]}`](https://github.com/{repo}/commit/{commit}) {title}')
    if previous_sha:
        notes += ['', f'[Full comparison](https://github.com/{repo}/compare/{previous_sha}...{source})']
    return '\n'.join(notes) + '\n', len(lines)


def owned_draft(release, marker, source, prerelease):
    if (not release.get('draft') or marker not in release.get('body', '').splitlines()
            or release.get('target_commitish') != source or release.get('prerelease') != prerelease):
        raise ValueError('Refusing to modify a published release or another run\'s draft')


def verify_assets(assets, expected, complete=False):
    seen = set()
    for asset in assets:
        name = asset['name']
        if (name in seen or name not in expected or asset.get('state') != 'uploaded'
                or asset.get('size') != expected[name].stat().st_size
                or asset.get('digest') != 'sha256:' + digest(expected[name])):
            raise ValueError('Remote draft asset differs from the verified local asset set')
        seen.add(name)
    if complete and seen != set(expected):
        raise ValueError('Release asset set is incomplete')
    return seen


def publish(version, repo, run, source, previous_tag, previous_sha, prerelease):
    version = version_number(version)
    sha(source)
    if git('rev-parse', 'HEAD') != source or check(ROOT) != version or git('status', '--porcelain'):
        raise ValueError('Publisher must check out the exact clean release source/version')
    # The reusable CI call is a required job dependency; obtain only this run's artifacts.
    artifacts = list(pages(f'repos/{repo}/actions/runs/{run}/artifacts'))
    tag = 'v' + version
    marker = f'<!-- containerdesk-release:{run}:{source} -->'
    with tempfile.TemporaryDirectory(prefix='containerdesk-release-') as temp:
        temp = Path(temp)
        output = temp/'publish'
        output.mkdir()
        metadatas = []
        for target in TARGETS:
            prefix = f'containerdesk-{version}-{target}-{source}-'
            candidates = [a for a in artifacts if not a['expired'] and a['name'].startswith(prefix)
                          and a['name'][len(prefix):].isdecimal()]
            if not candidates:
                raise ValueError(f'Missing CI package artifact for {target}')
            selected = max(candidates, key=lambda a: int(a['name'][len(prefix):]))
            folder = temp/target
            command(['gh', 'run', 'download', run, '--repo', repo, '--name', selected['name'], '--dir', str(folder)])
            metadata = validate_artifact(folder, version, target, source, run)
            if metadatas and metadata['locks'] != metadatas[0]['locks']:
                raise ValueError('Native builds have different source lockfiles')
            metadatas.append(metadata)
            for path in folder.iterdir():
                if path.name != 'SHA256SUMS':
                    shutil.copy2(path, output/path.name)
        for name, value in metadatas[0]['locks'].items():
            if name not in ('package-lock.json', 'src-tauri/Cargo.lock') or digest(ROOT/name) != value:
                raise ValueError('Build lockfile differs from tagged source')
        if set(metadatas[0]['locks']) != {'package-lock.json', 'src-tauri/Cargo.lock'}:
            raise ValueError('Missing lockfile provenance')
        notes, count = commit_notes(repo, previous_tag, previous_sha, source)
        (output/'COMMITS.md').write_text(notes)
        provenance = dict(schemaVersion=1, version=version, sourceCommit=source, previousTag=previous_tag,
                          previousCommit=previous_sha, commitCount=count,
                          runUrl=f'https://github.com/{repo}/actions/runs/{run}',
                          nativeTargets=list(TARGETS), nativeCI='passed', signing='disabled',
                          notarization='not_requested',
                          runtimeScope='Existing CI: native SSH/Docker matrix and macOS package smoke; not new Linux installed-GUI evidence')
        (output/'release-provenance.json').write_text(json.dumps(provenance, indent=2) + '\n')
        (output/'SHA256SUMS').write_text(''.join(f'{digest(p)}  {p.name}\n' for p in sorted(output.iterdir())))
        body = (f'ContainerDesk {tag}\n\nUnsigned packages for Linux x86_64 and macOS Apple Silicon/Intel. '
                'Mac packages are not Developer ID signed or notarized; Gatekeeper may block downloads.\n\n'
                f'Source: `{source}`. [Native CI]({provenance["runUrl"]}). '
                'All six installers, three build metadata files, provenance, SHA256SUMS and COMMITS.md are attached. '
                'Compare each installer checksum with SHA256SUMS before installing.\n\n')
        body += notes if len(notes.encode()) < 100000 else f'All {count} commits are listed in the attached COMMITS.md.\n'
        body += '\n' + marker
        releases = list(pages(f'repos/{repo}/releases'))
        previous = previous_release(releases)
        if (previous['tag_name'] if previous else '') != previous_tag:
            raise ValueError('A different release was published during the build; restart to include the correct range')
        existing = next((r for r in releases if r['tag_name'] == tag), None)
        if existing:
            owned_draft(existing, marker, source, prerelease)
        else:
            if git('ls-remote', '--tags', 'origin', f'refs/tags/{tag}'):
                raise ValueError('Tag was created during the build; refusing to overwrite it')
            existing = api(f'repos/{repo}/releases', 'POST', dict(tag_name=tag, target_commitish=source,
                           name=f'ContainerDesk {tag}', body=body, draft=True, prerelease=prerelease))
        endpoint = f'repos/{repo}/releases/{existing["id"]}'
        expected = {p.name: p for p in output.iterdir()}
        present = verify_assets(list(pages(endpoint + '/assets')), expected)
        for name, path in sorted(expected.items()):
            if name not in present:
                command(['gh', 'release', 'upload', tag, str(path), '--repo', repo])
        verify_assets(list(pages(endpoint + '/assets')), expected, complete=True)
        owned_draft(api(endpoint), marker, source, prerelease)
        if git('ls-remote', '--tags', 'origin', f'refs/tags/{tag}'):
            git('fetch', 'origin', f'refs/tags/{tag}:refs/tags/{tag}')
            if git('rev-parse', f'refs/tags/{tag}^{{commit}}') != source:
                raise ValueError('Tag changed before publication')
        release = api(endpoint, 'PATCH', {'draft': False, 'make_latest': 'false' if prerelease else 'true'})
        if release['draft']:
            raise ValueError('Release is still a draft')
        verify_assets(list(pages(endpoint + '/assets')), expected, complete=True)
        git('fetch', 'origin', f'refs/tags/{tag}:refs/tags/{tag}')
        if git('rev-parse', f'refs/tags/{tag}^{{commit}}') != source:
            raise ValueError('Published tag does not identify the verified source')
        print(release['html_url'])
        if os.environ.get('GITHUB_STEP_SUMMARY'):
            with open(os.environ['GITHUB_STEP_SUMMARY'], 'a') as stream:
                stream.write(f'Published [{tag}]({release["html_url"]}) with {len(expected)} assets and {count} commits.\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', choices=['prepare', 'publish'])
    args = parser.parse_args()
    repo, run = context()
    if args.operation == 'prepare':
        result = prepare(os.environ['RELEASE_VERSION'], repo, run, os.environ['GITHUB_SHA'])
        with open(os.environ['GITHUB_OUTPUT'], 'a') as output:
            for key, value in result.items():
                output.write(f'{key}={value}\n')
        print(json.dumps(result))
    else:
        choice = os.environ['RELEASE_PRERELEASE']
        if choice not in ('true', 'false'):
            raise ValueError('Invalid prerelease choice')
        publish(os.environ['RELEASE_VERSION'], repo, run, os.environ['RELEASE_SOURCE_SHA'],
                os.environ['RELEASE_PREVIOUS_TAG'], os.environ['RELEASE_PREVIOUS_SHA'], choice == 'true')


if __name__ == '__main__':
    main()
