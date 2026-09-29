#!/usr/bin/env python3
"""Refresh docs from published GitHub releases and explicitly rebuild branch-based Pages."""
import argparse
import json
import os
import subprocess
from pathlib import Path
import time
from urllib.request import Request, urlopen
from urllib.error import URLError

import release

ROOT = Path(__file__).resolve().parents[1]
FILES = ('docs/downloads.md', 'docs/releases.md', 'docs/_data/current_release.json')


def published(releases):
    return sorted((r for r in releases if not r['draft'] and r.get('published_at')),
                  key=lambda r: (r['published_at'], r['id']), reverse=True)


def source_data(releases, repo):
    public = published(releases)
    if not public:
        raise ValueError('No published releases; nothing to advertise')
    latest = public[0]
    version = release.version_number(latest['tag_name'])
    expected = {f'containerdesk-{version}-{target}{suffix}'
                for target, suffixes in release.TARGETS.items() for suffix in suffixes}
    expected |= {'SHA256SUMS', 'release-provenance.json'}
    assets = {a['name']: a for a in latest['assets'] if a.get('state') == 'uploaded'}
    if not expected <= set(assets):
        raise ValueError('Published release is missing installers or provenance; leave current downloads intact')
    prefix = f'https://github.com/{repo}/releases/download/{latest["tag_name"]}/'
    for name in expected:
        if assets[name]['browser_download_url'] != prefix + name or assets[name]['size'] <= 0:
            raise ValueError('Release asset URL or size is invalid')
    return public, latest, version, assets


def render(releases, repo):
    public, latest, version, assets = source_data(releases, repo)
    tag = latest['tag_name']
    url = f'https://github.com/{repo}/releases/tag/{tag}'
    kind = 'Pre-release' if latest['prerelease'] else 'Release'
    lines = ['---', 'title: "Downloads"', 'section: "Releases & platforms"', 'icon: "📦"', '---', '',
             f'# 📦 Download ContainerDesk {tag}', '', f'**{kind} · {latest["published_at"][:10]} · Unsigned packages**', '',
             'Choose your operating system and processor. Compare the installer SHA-256 with the matching row in SHA256SUMS before installing.', '',
             '> macOS packages are not Developer ID signed or notarized. Gatekeeper may block a downloaded app. '
             'See [signing limitations](signing.md) and [platform evidence](platform-matrix.md).', '',
             '| Platform | Installers |', '|---|---|']
    for target, suffixes in release.TARGETS.items():
        label = {'x86_64-unknown-linux-gnu': '🐧 Linux x86_64', 'aarch64-apple-darwin': '🍎 macOS Apple Silicon',
                 'x86_64-apple-darwin': '🍎 macOS Intel'}[target]
        links = [f'[{suffix.removeprefix(".")}]({assets[f"containerdesk-{version}-{target}{suffix}"]["browser_download_url"]})' for suffix in suffixes]
        lines.append(f'| {label} | {" · ".join(links)} |')
    lines += ['', f'[SHA256SUMS]({assets["SHA256SUMS"]["browser_download_url"]}) · '
              f'[Build provenance]({assets["release-provenance.json"]["browser_download_url"]}) · '
              f'[All assets and release notes]({url})', '',
              '## Install and update', '',
              '[User guide](user-guide.md) · [Linux packages](linux-packages.md) · [macOS packages](macos-packages.md) · '
              '[Manual updates and rollback](updates.md)', '',
              '## Release history', '', '[All published releases](releases.md) · [How manual releases work](release-workflow.md)', '',
              f'<!-- containerdesk-release:{tag} -->', '']
    history = ['---', 'title: "Releases"', 'section: "Releases & platforms"', 'icon: "📦"', '---', '',
               '# 📦 Releases', '', '[Download the current release](downloads.md) · [Manual release workflow](release-workflow.md)', '',
               'Published releases are listed newest first, including pre-releases. Drafts never appear here. '
               'Each new manual release includes all commits since the preceding published release in its notes and attached `COMMITS.md`.', '',
               '| Version | Published (UTC) | Status | Notes and assets |', '|---|---|---|---|']
    for item in public:
        release.version_number(item['tag_name'])
        item_url = f'https://github.com/{repo}/releases/tag/{item["tag_name"]}'
        history.append(f'| [{item["tag_name"]}]({item_url}) | {item["published_at"][:10]} | '
                       f'{"Pre-release" if item["prerelease"] else "Release"} | [Commits, notes and installers]({item_url}) |')
    history += ['', 'The original [v0.1.0 publication receipt](releases/v0.1.0.md) and native evidence remain historical records. '
                'Unsigned packages do not establish Apple signing/notarization readiness.', '']
    current = dict(version=version, tag=tag, url=url, prerelease=latest['prerelease'], published_at=latest['published_at'])
    return {FILES[0]: '\n'.join(lines), FILES[1]: '\n'.join(history), FILES[2]: json.dumps(current, indent=2) + '\n'}


def write(releases, repo, root=ROOT):
    documents = render(releases, repo)  # Validate everything before changing any page.
    for name, text in documents.items():
        (root/name).parent.mkdir(parents=True, exist_ok=True)
        (root/name).write_text(text)


def rebuild(repo, commit):
    endpoint = f'repos/{repo}/pages'
    site = release.api(endpoint)
    if site['build_type'] != 'legacy' or site['source'] != {'branch': 'main', 'path': '/docs'}:
        raise ValueError('GitHub Pages must use Deploy from a branch: main /docs')
    # GITHUB_TOKEN pushes do not start another push workflow or Pages build.
    release.api(endpoint + '/builds', 'POST')
    deadline = time.monotonic() + 600
    while time.monotonic() < deadline:
        build = release.api(endpoint + '/builds/latest')
        built_commit = build.get('commit')
        if built_commit and build.get('status') in ('built', 'errored'):
            release.sha(built_commit)
            release.git('fetch', 'origin', 'main')
            # A concurrent ordinary docs commit is fine if it includes our update.
            ancestry = subprocess.run(['git', 'merge-base', '--is-ancestor', commit, built_commit],
                                      cwd=ROOT, capture_output=True, timeout=30)
            if ancestry.returncode not in (0, 1):
                raise ValueError('Cannot verify Pages build ancestry')
            if ancestry.returncode == 0:
                if build['status'] != 'built':
                    raise ValueError('GitHub Pages build failed: ' + str(build.get('error')))
                return site['html_url'], build
        time.sleep(10)
    raise TimeoutError('GitHub Pages did not finish this docs revision within ten minutes')


def verify_live(url, tag):
    marker = f'<!-- containerdesk-release:{tag} -->'
    for attempt in range(12):
        try:
            request = Request(url + 'downloads.html?release=' + tag, headers={'Cache-Control': 'no-cache'})
            with urlopen(request, timeout=20) as response:
                if marker in response.read().decode('utf-8'):
                    return
        except (URLError, TimeoutError):
            pass
        if attempt != 11:
            time.sleep(5)
    raise TimeoutError('Pages built, but updated Downloads content is not yet visible over HTTPS')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', choices=['render', 'publish'])
    args = parser.parse_args()
    repo = 'alekpopovic/container-desk'
    if args.operation == 'publish':
        repo, _ = release.context()
        if release.git('status', '--porcelain'):
            raise ValueError('Website update requires a clean checkout')
        release.git('fetch', 'origin', 'main')
        release.git('checkout', '--detach', 'refs/remotes/origin/main')
    releases = list(release.pages(f'repos/{repo}/releases'))
    write(releases, repo)
    if args.operation == 'publish':
        changed = set(release.git('diff', '--name-only').splitlines())
        changed.update(release.git('ls-files', '--others', '--exclude-standard').splitlines())
        if not changed <= set(FILES):
            raise ValueError('Unexpected worktree changes during docs update')
        if changed:
            tag = published(releases)[0]['tag_name']
            release.git('add', '--', *FILES)
            release.git('-c', 'user.name=github-actions[bot]', '-c', 'user.email=41898282+github-actions[bot]@users.noreply.github.com',
                        'commit', '-m', f'docs: update downloads and releases for {tag}')
            release.git('push', 'origin', 'HEAD:refs/heads/main')
        commit = release.git('rev-parse', 'HEAD')
        url, build = rebuild(repo, commit)
        tag = published(releases)[0]['tag_name']
        verify_live(url, tag)
        print(json.dumps({'site': url, 'docsCommit': commit, 'build': build['url'], 'version': tag}))
        with open(os.environ['GITHUB_STEP_SUMMARY'], 'a') as output:
            output.write(f'Updated [Downloads]({url}downloads.html) and [Releases]({url}releases.html); Pages build succeeded for `{commit}`.\n')


if __name__ == '__main__':
    main()
