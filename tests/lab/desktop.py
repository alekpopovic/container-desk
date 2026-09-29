#!/usr/bin/env python3
"""050 real Linux desktop suite; separate production refusal and instrumented GUI runs."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import select
import subprocess
import tempfile
from native_ssh import REPO, application_binary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tools-dir', type=Path, required=True)
    parser.add_argument('--artifacts', type=Path, required=True)
    parser.add_argument('--worker', choices=['guard', 'errors'])
    args = parser.parse_args()
    tools = args.tools_dir.resolve()
    artifacts = args.artifacts.resolve()
    artifacts.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='containerdesk-050-') as directory:
        root = Path(directory)
        if args.worker == 'guard':
            from production_guard import verify
            return verify(root, tools, artifacts)
        if args.worker == 'errors':
            subprocess.run(['python3', 'tests/lab/ssh_auth.py', '--engine', '--native-driver', str(tools/'bin/tauri-driver'),
                            '--webkit-driver', str(tools/'webkit/usr/bin/WebKitWebDriver'), '--native-artifacts', str(artifacts)],
                           cwd=REPO, check=True, timeout=300)
            return
        binary = application_binary()
        for journey in ['guard', 'errors']:
            read, write = os.pipe()
            with (root / ('xvfb-' + journey + '.log')).open('w') as log:
                display = subprocess.Popen([str(tools/'xvfb/usr/bin/Xvfb'), '-displayfd', str(write), '-screen', '0', '1440x1000x24', '-nolisten', 'tcp'],
                                           pass_fds=(write,), stdout=log, stderr=log)
                os.close(write)
                try:
                    assert select.select([read], [], [], 10)[0]
                    number = os.read(read, 32).decode().strip(); assert number.isdigit()
                    env = os.environ.copy()
                    for name in ['WAYLAND_DISPLAY','LD_LIBRARY_PATH','LD_PRELOAD','GTK_PATH','GIO_MODULE_DIR','DBUS_SESSION_BUS_ADDRESS','AT_SPI_BUS_ADDRESS']:
                        env.pop(name, None)
                    env.update(DISPLAY=':'+number, GDK_BACKEND='x11', LIBGL_ALWAYS_SOFTWARE='1',
                               XDG_DATA_HOME=str(root/journey/'data'), XDG_CONFIG_HOME=str(root/journey/'config'),
                               XDG_CACHE_HOME=str(root/journey/'cache'), XDG_RUNTIME_DIR=str(root/journey/'runtime'))
                    Path(env['XDG_RUNTIME_DIR']).mkdir(mode=0o700, parents=True)
                    subprocess.run(['/usr/bin/dbus-run-session', '--', 'python3', str(Path(__file__).resolve()), '--worker', journey,
                                    '--tools-dir', str(tools), '--artifacts', str(artifacts/journey)], cwd=REPO, env=env, check=True, timeout=360)
                finally:
                    os.close(read); display.terminate(); display.wait(timeout=10)
        subprocess.run(['python3', 'tests/lab/checkpoint.py', '--tools-dir', str(tools), '--artifacts', str(artifacts/'interaction')], cwd=REPO, check=True, timeout=450)
        subprocess.run(['python3', 'tests/lab/security.py', '--tools-dir', str(tools), '--artifacts', str(artifacts/'ipc')], cwd=REPO, check=True, timeout=150)
        result = {'linux': 'passed', 'macos': 'pending: no native macOS runner in this execution',
                  'nativeAutomationSha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
                  'productionSha256': hashlib.sha256((REPO/'src-tauri/target/release/containerdesk').read_bytes()).hexdigest(),
                  'journeys': ['production automation refusal', 'host selection and native connection errors', 'log cancel and bounded PTY', 'actual IPC authorization'],
                  'browserMocksUsed': False}
        (artifacts/'desktop.json').write_text(json.dumps(result, indent=2)+'\n')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
