#!/usr/bin/env python3
"""Pinned dependency installation or fail-fast verification; never cleans source/build caches."""
import argparse
import json
import os
from pathlib import Path
import platform
import re
import signal
import subprocess
import sys
import time

REPO = Path(__file__).resolve().parents[1]
CARGO = ['--manifest-path', 'src-tauri/Cargo.toml', '--locked']


def tool_versions():
    package = json.loads((REPO/'package.json').read_text())
    rust = re.search(r'^channel\s*=\s*"([^"]+)"', (REPO/'rust-toolchain.toml').read_text(), re.M).group(1)
    expected = {'node': package['engines']['node'], 'npm': package['engines']['npm'], 'rustc': rust, 'cargo': rust}
    assert (REPO/'.nvmrc').read_text().strip() == expected['node'], 'Node pins differ'
    versions = {}
    for tool, wanted in expected.items():
        value = subprocess.run([tool, '--version'], cwd=REPO, check=True, capture_output=True, text=True, timeout=20).stdout.strip()
        actual = value.removeprefix('v') if tool in ('node', 'npm') else value.split()[1]
        if actual != wanted:
            raise RuntimeError(f'{tool} {actual}; required {wanted}. Select the pinned toolchain before verification.')
        versions[tool] = actual
    if sys.version_info < (3, 10):
        raise RuntimeError('Python 3.10+ is required')
    return versions


def execute(label, argv, timeout):
    print(f'RUN {label}: {" ".join(argv)}', flush=True)
    child = subprocess.Popen(argv, cwd=REPO, start_new_session=True)
    try:
        code = child.wait(timeout=timeout)
    except (subprocess.TimeoutExpired, KeyboardInterrupt) as error:
        try: os.killpg(child.pid, signal.SIGTERM)
        except ProcessLookupError: pass
        try: child.wait(timeout=10)
        except subprocess.TimeoutExpired:
            try: os.killpg(child.pid, signal.SIGKILL)
            except ProcessLookupError: pass
            child.wait(timeout=10)
        if isinstance(error, KeyboardInterrupt): raise
        print(f'FAIL {label}: exceeded {timeout}s', flush=True)
        return 124
    print(f'{"PASS" if code == 0 else "FAIL"} {label}: exit {code}', flush=True)
    return code


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--install', action='store_true', help='Only npm ci, pinned Playwright Chromium and cargo fetch --locked')
    parser.add_argument('--native-tools', type=Path, help='Opt into building and running the Linux 050 native desktop suite')
    parser.add_argument('--native-artifacts', type=Path, default=REPO/'test-results/native', help='Native output directory, used only with --native-tools')
    parser.add_argument('--report', type=Path, default=REPO/'test-results/verification.json')
    args = parser.parse_args()
    if args.install and args.native_tools: parser.error('--install is separate from verification')
    report = {'platform': platform.system(), 'architecture': platform.machine(), 'mode': 'install' if args.install else 'verify', 'checks': [], 'skipped': [], 'passed': False}
    code = 1
    try:
        report['toolchains'] = tool_versions()
        print('Pinned toolchains: ' + json.dumps(report['toolchains']), flush=True)
        if args.install:
            checks = [('locked frontend install', ['npm', 'ci'], 300),
                      ('pinned browser install', ['npm', 'exec', 'playwright', '--', 'install', 'chromium'], 300),
                      ('locked Rust fetch', ['cargo', 'fetch', *CARGO], 300)]
            report['skipped'] = ['Verification not requested: installation only']
        else:
            checks = [('frontend formatting', ['npm', 'run', 'format:check'], 60),
                      ('Rust formatting', ['cargo', 'fmt', '--manifest-path', 'src-tauri/Cargo.toml', '--check'], 60),
                      ('TypeScript', ['npm', 'run', 'typecheck'], 120),
                      ('frontend lint', ['npm', 'run', 'lint'], 60),
                      ('frontend unit tests', ['npm', 'test'], 120),
                      ('browser fixture tests', ['npm', 'run', 'test:ui'], 600),
                      ('frontend production build', ['npm', 'run', 'build'], 180),
                      ('Rust lint', ['cargo', 'clippy', *CARGO, '--all-targets', '--', '-D', 'warnings'], 1200),
                      ('Rust automation feature lint', ['cargo', 'clippy', *CARGO, '--all-targets', '--features', 'native-automation', '--', '-D', 'warnings'], 1200),
                      ('Rust tests', ['cargo', 'test', *CARGO], 1200),
                      ('native production build', ['npm', 'run', 'desktop:build'], 1200),
                      ('tracker tests', ['python3', '-m', 'unittest', 'discover', '-s', 'codex/tests', '-v'], 60),
                      ('tracker validation', ['python3', 'codex/scripts/track.py', 'validate'], 60)]
            report['skipped'] = ['Disposable VM integration: opt in separately with tests/lab/integration.py',
                                 'Other native OS/architecture runners, package installation and signing: separate platform gates']
            if args.native_tools:
                if platform.system() != 'Linux': raise RuntimeError('Selected desktop harness is Linux-only; macOS native runner remains pending')
                if os.environ.get('CARGO_TARGET_DIR'): raise RuntimeError('Unset CARGO_TARGET_DIR for the native desktop suite; its artifact paths are fixed')
                checks += [('native automation build', ['npm', 'run', 'desktop:build:automation'], 1200),
                           ('native desktop suite', ['python3', 'tests/lab/desktop.py', '--tools-dir', str(args.native_tools.resolve()), '--artifacts', str(args.native_artifacts.resolve())], 1200)]
            else:
                report['skipped'].append('Native desktop/SSH/Docker tests: not requested; use --native-tools PATH')
        report['checks'] = [{'name': name, 'command': command, 'status': 'not_run'} for name, command, _ in checks]
        for record, (name, command, timeout) in zip(report['checks'], checks):
            began = time.monotonic()
            record['status'] = 'running'
            try:
                code = execute(name, command, timeout)
            finally:
                record['seconds'] = round(time.monotonic()-began, 2)
            record.update(status='passed' if code == 0 else 'failed', exitCode=code, seconds=round(time.monotonic()-began, 2))
            if code: break
        report['passed'] = all(record['status'] == 'passed' for record in report['checks'])
        code = 0 if report['passed'] else max(1, code)
    except (OSError, RuntimeError, AssertionError, subprocess.SubprocessError) as error:
        report['error'] = str(error)
        print('FAIL verification: ' + str(error), file=sys.stderr, flush=True)
        code = 1
    except KeyboardInterrupt:
        report['error'] = 'Interrupted; owned active check terminated'
        code = 130
    finally:
        for record in report['checks']:
            if record['status'] == 'running':
                record.update(status='interrupted' if code == 130 else 'failed', exitCode=code)
        for skipped in report['skipped']: print('SKIP ' + skipped, flush=True)
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2) + '\n')
        print(('PASS' if report['passed'] else 'FAIL') + ' ' + report['mode'] + '; report: ' + str(args.report), flush=True)
    return code


if __name__ == '__main__':
    raise SystemExit(main())
