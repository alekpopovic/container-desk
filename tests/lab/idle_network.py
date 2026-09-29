#!/usr/bin/env python3
"""Opt-in Linux ordinary-production idle network observation on a private desktop."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import select
import signal
import subprocess
import tempfile
import time
from native_launch import close_window


def worker(args):
    env = os.environ.copy()
    trace = args.artifacts/'idle-network.strace'
    with (args.artifacts/'idle-stderr.log').open('w') as log:
        process = subprocess.Popen(['/usr/bin/strace', '-f', '-qq', '-e', 'trace=network', '-o', str(trace),
            str(args.binary)], env=env, stdout=log, stderr=log, start_new_session=True)
        try:
            window = None
            until = time.monotonic()+25
            while time.monotonic() < until:
                found = subprocess.run([str(args.tools_dir/'xdotool/usr/bin/xdotool'), 'search', '--onlyvisible',
                    '--name', '^ContainerDesk$'], capture_output=True, text=True, timeout=3).stdout.split()
                for candidate in found:
                    pid = subprocess.check_output([str(args.tools_dir/'xdotool/usr/bin/xdotool'), 'getwindowpid', candidate], text=True, timeout=3).strip()
                    if Path(f'/proc/{pid}/exe').resolve() == args.binary:
                        window = candidate
                        break
                if window: break
                if process.poll() is not None: raise RuntimeError('Production app exited before showing a window')
                time.sleep(.1)
            if not window: raise RuntimeError('No ordinary native window observed')
            time.sleep(15)
            subprocess.run(['/usr/bin/import', '-window', window, str(args.artifacts/'idle-window.png')], check=True, timeout=10)
            close_window(window)
            if process.wait(timeout=15): raise RuntimeError('Traced app did not exit normally')
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGTERM)
                try: process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=5)
    text = trace.read_text()
    requests = [line for line in text.splitlines() if re.search(r'(?:connect|sendto|sendmsg|sendmmsg)\(', line)
                and re.search(r'AF_INET6?\b', line)]
    result = {'platform': 'Linux X11', 'productionSha256': hashlib.sha256(args.binary.read_bytes()).hexdigest(),
        'nativeWindow': True, 'normalClose': True, 'idleSecondsAfterWindow': 15,
        'tracedChildProcesses': True, 'internetConnectOrAddressedSendAttempts': len(requests),
        'passed': not requests, 'scope': 'Fresh-profile launch, 15 seconds idle and normal close; not all future interactions or OS-service traffic'}
    (args.artifacts/'idle-network.json').write_text(json.dumps(result, indent=2)+'\n')
    if requests: raise RuntimeError('Unexpected internet address in application network syscall; inspect owned trace')
    print(json.dumps(result, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--tools-dir', type=Path, required=True)
    parser.add_argument('--artifacts', type=Path, required=True)
    parser.add_argument('--worker', action='store_true')
    args = parser.parse_args()
    for key in ('binary', 'tools_dir', 'artifacts'): setattr(args, key, getattr(args, key).resolve())
    args.artifacts.mkdir(parents=True, exist_ok=True)
    if args.worker: return worker(args)
    with tempfile.TemporaryDirectory(prefix='cd-idle-') as folder:
        root = Path(folder)
        read, write = os.pipe()
        display = subprocess.Popen([str(args.tools_dir/'xvfb/usr/bin/Xvfb'), '-displayfd', str(write),
            '-screen', '0', '1280x900x24', '-nolisten', 'tcp'], pass_fds=(write,), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        os.close(write)
        try:
            if not select.select([read], [], [], 10)[0]: raise RuntimeError('Xvfb deadline')
            number = os.read(read, 32).decode().strip()
            if not number.isdigit(): raise RuntimeError('Invalid X display')
            env = os.environ.copy()
            for name in ('WAYLAND_DISPLAY', 'LD_LIBRARY_PATH', 'LD_PRELOAD', 'GTK_PATH', 'GIO_MODULE_DIR',
                         'DBUS_SESSION_BUS_ADDRESS', 'AT_SPI_BUS_ADDRESS'):
                env.pop(name, None)
            env.update(DISPLAY=':'+number, GDK_BACKEND='x11', LIBGL_ALWAYS_SOFTWARE='1')
            for key in ('XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'XDG_CACHE_HOME', 'XDG_RUNTIME_DIR'):
                path = root/key
                path.mkdir(mode=0o700)
                env[key] = str(path)
            subprocess.run(['/usr/bin/dbus-run-session', '--', 'python3', str(Path(__file__).resolve()),
                '--worker', '--binary', str(args.binary), '--tools-dir', str(args.tools_dir),
                '--artifacts', str(args.artifacts)], env=env, check=True, timeout=75)
        finally:
            os.close(read)
            display.terminate()
            display.wait(timeout=10)


if __name__ == '__main__': main()
