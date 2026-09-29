#!/usr/bin/env python3
"""Package a checked ordinary native executable; never sign or publish a release."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import tarfile

from verify import REPO, execute, tool_versions


def sha256(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest() if hasattr(hashlib, 'file_digest') else _digest(stream)


def _digest(stream):
    digest = hashlib.sha256()
    for block in iter(lambda: stream.read(1024 * 1024), b''): digest.update(block)
    return digest.hexdigest()


def checked_report(path):
    report = json.loads(path.read_text())
    required = {'frontend formatting', 'Rust formatting', 'TypeScript', 'frontend lint',
                'frontend unit tests', 'browser fixture tests', 'frontend production build',
                'Rust lint', 'Rust automation feature lint', 'Rust tests',
                'native production build', 'tracker tests', 'tracker validation'}
    passed = {r['name'] for r in report.get('checks', []) if r.get('status') == 'passed' and r.get('exitCode') == 0}
    if report.get('mode') != 'verify' or report.get('passed') is not True or not required <= passed:
        raise ValueError('All required verification checks must pass before packaging')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--report', type=Path, default=REPO/'test-results/verification.json')
    parser.add_argument('--output', type=Path, default=REPO/'dist-artifacts')
    args = parser.parse_args()
    report = checked_report(args.report)
    tools = tool_versions()
    version = json.loads((REPO/'package.json').read_text())['version']
    if not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?', version):
        raise ValueError('Version is not safe for artifact names')
    system, arch = platform.system(), platform.machine()
    targets = {('Linux', 'x86_64'): ('x86_64-unknown-linux-gnu', 'deb,appimage'),
               ('Darwin', 'arm64'): ('aarch64-apple-darwin', 'app'),
               ('Darwin', 'x86_64'): ('x86_64-apple-darwin', 'app')}
    target, bundles = targets[(system, arch)]
    if system == 'Linux':
        release = platform.freedesktop_os_release()
        if release.get('ID') != 'ubuntu' or release.get('VERSION_ID') != '24.04':
            raise ValueError('Linux distribution packages require the Ubuntu 24.04 baseline')
    root = Path(os.environ.get('CARGO_TARGET_DIR', str(REPO/'src-tauri/target'))).resolve()
    binary = root/'release/containerdesk'
    if not binary.is_file(): raise ValueError('Ordinary release executable is missing')
    # The ordinary build must be the artifact recorded by the required verifier.
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=REPO, text=True, timeout=10).strip()
    recorded = report.get('productionBuild', {})
    if recorded.get('sourceCommit') != revision or recorded.get('sha256') != sha256(binary):
        raise ValueError('Production build differs from verification; run checks again')
    # Fresh CI jobs have no bundles. Refuse stale intermediate output.
    bundle_root = root/'release/bundle'
    if bundle_root.exists() and any(bundle_root.iterdir()): raise ValueError('Bundle output must be empty; choose a fresh build directory')
    if args.output.is_symlink() or (args.output.exists() and any(args.output.iterdir())):
        raise ValueError('Artifact output must be a new or empty directory')
    if any(key.startswith(('APPLE_', 'TAURI_SIGNING_')) and value for key, value in os.environ.items()):
        raise ValueError('Unsigned CI packaging must not receive signing credentials')
    before = sha256(binary)
    code = execute('unsigned native bundle', ['npm', 'exec', 'tauri', '--', 'bundle', '--ci', '--no-sign',
                   '--bundles', bundles, '--config', '{"bundle":{"active":true}}'], 600)
    if code: return code
    formats = [('deb/*.deb', '.deb'), ('appimage/*.AppImage', '.AppImage')] if system == 'Linux' else [('macos/*.app', '.app.tar.gz')]
    selected = []
    for pattern, suffix in formats:
        found = sorted(bundle_root.glob(pattern))
        if len(found) != 1: raise ValueError(f'Expected one {pattern} package; found {len(found)}')
        selected.append((found[0], suffix))
    args.output.mkdir(parents=True, exist_ok=True)
    stem = f'containerdesk-{version}-{target}'
    packages = []
    for source, suffix in selected:
        package = args.output/(stem+suffix)
        if suffix == '.app.tar.gz':
            with tarfile.open(package, 'w:gz', dereference=False) as archive:
                archive.add(source, arcname=source.name)
        else:
            shutil.copy2(source, package)
            if suffix == '.AppImage': package.chmod(0o755)
        packages.append(package)
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=REPO, text=True, timeout=10).strip()
    dirty = bool(subprocess.check_output(['git', 'status', '--porcelain', '--untracked-files=no'], cwd=REPO, timeout=10))
    metadata = {'schemaVersion': 2, 'version': version, 'sourceCommit': revision, 'sourceDirty': dirty,
                'target': target, 'os': platform.platform(), 'toolchains': tools,
                'runId': os.environ.get('GITHUB_RUN_ID'), 'runAttempt': os.environ.get('GITHUB_RUN_ATTEMPT'),
                'event': os.environ.get('GITHUB_EVENT_NAME', 'local'),
                'verificationReportSha256': sha256(args.report),
                'packages': [{'file': p.name, 'bytes': p.stat().st_size, 'sha256': sha256(p)} for p in packages],
                'executableSha256BeforeBundle': before, 'executableSha256AfterBundle': sha256(binary),
                'signing': 'disabled', 'notarization': 'not_requested', 'runtimeAcceptance': 'not_run',
                'releaseApproved': False,
                'locks': {p: sha256(REPO/p) for p in ['package-lock.json', 'src-tauri/Cargo.lock']}}
    meta = args.output/(stem+'.json')
    meta.write_text(json.dumps(metadata, indent=2)+'\n')
    (args.output/'SHA256SUMS').write_text(''.join(f'{sha256(p)}  {p.name}\n' for p in [*packages, meta]))
    if os.environ.get('GITHUB_OUTPUT'):
        with open(os.environ['GITHUB_OUTPUT'], 'a') as output: output.write('version='+version+'\n')
    print(json.dumps(metadata, indent=2))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
