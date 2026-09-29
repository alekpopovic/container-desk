#!/usr/bin/env python3
"""Real logs through an owned loopback sshd. Never mount the host Docker socket into a target.
Only generated, labelled workload IDs are admitted by the temporary SSH command gate.
No host inventory/list/mutation command is admitted. No user SSH/Docker configuration is used.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import pwd
import signal
import socket
import subprocess
import tempfile
import time
import threading
from ssh_auth import BASE
REPO = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sshd-root', type=Path, required=True, help='Extracted Ubuntu openssh-server package root; no system install')
    parser.add_argument('--stream', action='store_true')
    parser.add_argument('--reads', action='store_true', help='Slow native reads and disconnect cancellation; requires --stream')
    parser.add_argument('--events', action='store_true', help='Owned event lifecycle/reconnect checks; requires --stream')
    parser.add_argument('--stats', action='store_true', help='Check stats with the same owned workloads; requires --stream')
    parser.add_argument('--native-driver', type=Path)
    parser.add_argument('--webkit-driver', type=Path)
    parser.add_argument('--native-artifacts', type=Path)
    parser.add_argument('--export-xdotool', type=Path)
    parser.add_argument('--focus-xdotool', type=Path, help='Focus only the owned test window on an isolated X11 display')
    args = parser.parse_args()
    if bool(args.native_driver) != bool(args.webkit_driver) or (args.native_driver and not args.stream):
        parser.error('Native driver flags require --stream and both driver paths')
    if args.reads and (not args.stream or args.stats or args.events): parser.error('--reads requires --stream and excludes --stats/--events')
    if args.events and (not args.stream or args.stats): parser.error('--events requires --stream and excludes --stats')
    if args.stats and not args.stream: parser.error('--stats requires --stream')
    with tempfile.TemporaryDirectory(prefix='containerdesk-logs-lab-') as directory:
        root = Path(directory)
        config = root / 'docker-config'
        config.mkdir(mode=0o700)
        env = os.environ.copy()
        for key in list(env):
            if key.startswith('DOCKER_') or key in ('SSH_AUTH_SOCK', 'SSH_ASKPASS'):
                env.pop(key, None)
        docker = ['/usr/bin/docker', '--config', str(config), '--host', 'unix:///var/run/docker.sock']
        owned = []
        removed = set()
        server = None
        watcher = None
        finished = threading.Event()
        try:
            sequence = "printf '023-stdout-one\\n'; sleep 0.1; printf '023-stderr-two\\n' >&2; sleep 0.1; printf '023-stdout-three\\n'; printf '\\377023-invalid\\n'; printf '023-synthetic-log-private\\n'; awk 'BEGIN { for (i=0; i<300000; i++) printf \"z\"; printf \"\\n\" }' >&2"
            for driver in (('json-file', 'none', 'live') if args.stream else ('json-file', 'none')):
                name = root.name + '-' + driver
                workload = sequence if driver != 'live' else "while true; do awk 'BEGIN { for (i=0; i<5000; i++) print \"024-synthetic-live-line\" }'; sleep 0.1; done"
                created = subprocess.run(docker + ['create', '--name', name, '--label', 'dev.containerdesk.lab=023', '--network', 'none', '--read-only', '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges', '--pids-limit', '32', '--memory', '32m', '--log-driver', 'json-file' if driver == 'live' else driver] + (['--log-opt', 'max-size=2m', '--log-opt', 'max-file=1'] if driver == 'live' else []) + [BASE, '/bin/sh', '-c', workload], env=env, check=True, capture_output=True, text=True, timeout=30)
                ident = created.stdout.strip()
                assert len(ident) == 64 and all(c in '0123456789abcdef' for c in ident)
                owned.append(ident)
                subprocess.run(docker + ['start', ident], env=env, check=True, stdout=subprocess.DEVNULL, timeout=10)
                if driver == 'live':
                    continue
                waited = subprocess.run(docker + ['wait', ident], env=env, check=True, capture_output=True, text=True, timeout=15)
                assert waited.stdout.strip() == '0', 'Owned log workload must exit successfully'
            oracle = subprocess.run(docker + ['logs', '--timestamps', '--tail', '100', '--', owned[0]], env=env, check=True, capture_output=True, timeout=10)
            print('Native CLI oracle: stdout/stderr records', len(oracle.stdout.splitlines()), len(oracle.stderr.splitlines()), 'largest stderr line bytes', max(map(len, oracle.stderr.splitlines()), default=0), flush=True)
            (root / 'oracle.stdout').write_bytes(oracle.stdout)
            (root / 'oracle.stderr').write_bytes(oracle.stderr)
            for key in ('host', 'client'):
                subprocess.run(['/usr/bin/ssh-keygen', '-q', '-t', 'ed25519', '-N', '', '-f', str(root / key)], check=True)
            (root / 'authorized_keys').write_text('restrict ' + (root / 'client.pub').read_text())
            (root / 'authorized_keys').chmod(0o600)
            with socket.socket() as sock:
                sock.bind(('127.0.0.1', 0))
                port = sock.getsockname()[1]
            user = pwd.getpwuid(os.getuid()).pw_name
            # This fixture-only gate prevents tests from reading unrelated host resources or changing state.
            gate = REPO / 'tests/lab/logs_gate.py'
            server_config = root / 'sshd_config'
            server_config.write_text(f'ListenAddress 127.0.0.1\nPort {port}\nHostKey {root}/host\nPidFile {root}/pid\nAuthorizedKeysFile {root}/authorized_keys\nStrictModes no\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\nAllowUsers {user}\nAllowTcpForwarding no\nPermitTTY no\nForceCommand /usr/bin/python3 {gate} --config {config} --control {root} --owned {" ".join(owned)}\nSshdSessionPath {args.sshd_root}/usr/lib/openssh/sshd-session\nSshdAuthPath {args.sshd_root}/usr/lib/openssh/sshd-auth\n')
            public = (root / 'host.pub').read_text().split()
            known = root / 'known_hosts'
            known.write_text(f'[127.0.0.1]:{port} {public[0]} {public[1]}\n')
            ssh_config = root / 'ssh_config'
            ssh_config.write_text(f'Host logs-owned\n HostName 127.0.0.1\n User {user}\n Port {port}\n IdentityFile {root}/client\n IdentitiesOnly yes\n IdentityAgent none\n UserKnownHostsFile {known}\n StrictHostKeyChecking yes\n')
            before = hashlib.sha256(known.read_bytes()).hexdigest()
            with (root / 'server.log').open('w') as log:
                server = subprocess.Popen([str(args.sshd_root / 'usr/sbin/sshd'), '-D', '-e', '-f', str(server_config)], env={'PATH': '/usr/bin:/bin', 'LANG': 'C'}, stdout=log, stderr=log, start_new_session=True)
                time.sleep(.3)
                if server.poll() is not None:
                    raise RuntimeError('Owned loopback SSH server did not start')
                preflight = subprocess.run(['/usr/bin/ssh', '-F', str(ssh_config), '-o', 'BatchMode=yes', 'logs-owned', "exec 'printf' 'containerdesk-access-ok'"], env=env, capture_output=True, text=True, timeout=10)
                if preflight.returncode or preflight.stdout != 'containerdesk-access-ok':
                    print('Owned marker preflight failed:', preflight.returncode, preflight.stderr)
                    print((root / 'server.log').read_text())
                    raise RuntimeError('Owned SSH gate did not execute the marker')
                manifest = root / 'manifest.json'
                manifest.write_text(json.dumps({'config': str(ssh_config), 'containerId': owned[0], 'unsupportedId': owned[1], 'oracleStdout': str(root / 'oracle.stdout'), 'oracleStderr': str(root / 'oracle.stderr'), 'liveId': owned[-1] if args.stream else None}))
                def cut_network():
                    while not finished.wait(.05):
                        if args.events:
                            for marker, operation, target in [('event-start', 'start', owned[0]), ('event-delete', 'rm', owned[0]), ('gui-delete', 'rm', owned[1])]:
                                if (root / marker).exists() and not (root / (marker + '-done')).exists():
                                    subprocess.run(docker + [operation, '--', target], env=env, check=True, stdout=subprocess.DEVNULL, timeout=10)
                                    if operation == 'rm': removed.add(target)
                                    (root / (marker + '-done')).write_text('owned lifecycle completed')

                        if args.stats and (root / 'stats-dispatch').exists() and not (root / 'stats-removed').exists():
                            target = (root / 'stats-dispatch').read_text()
                            assert target == owned[-1], 'Removal controller only admits the owned live fixture'
                            subprocess.run(docker + ['rm', '-f', '--', target], env=env, check=True, stdout=subprocess.DEVNULL, timeout=10)
                            removed.add(target)
                            (root / 'stats-removed').write_text('removed owned fixture')
                        if (root / 'cut-network').exists():
                            # Only descendants of our exact temporary sshd; no user SSH process is targeted.
                            children = {}
                            for item in Path('/proc').iterdir():
                                if item.name.isdigit():
                                    try:
                                        status = (item / 'status').read_text()
                                        parent = int(next(line.split()[1] for line in status.splitlines() if line.startswith('PPid:')))
                                        children.setdefault(parent, []).append(int(item.name))
                                    except (OSError, StopIteration, ValueError): pass
                            targets = []
                            def visit(pid):
                                for child in children.get(pid, []): visit(child)
                                targets.append(pid)
                            visit(server.pid)
                            for pid in targets:
                                try: os.kill(pid, signal.SIGKILL)
                                except ProcessLookupError: pass
                            return
                if args.stream:
                    watcher = threading.Thread(target=cut_network, daemon=True)
                    watcher.start()
                if args.stream and args.native_driver:
                    from native_logs import verify
                    verify(root, args.native_driver, args.webkit_driver, ssh_config, owned[-1], args.native_artifacts, owned[0] if args.export_xdotool else None, args.export_xdotool or args.focus_xdotool, args.stats, owned[1] if args.events else None)
                checkpoint = 'checkpoint028_owned_reads' if args.reads else 'checkpoint027_owned_events' if args.events else 'checkpoint026_owned_stats' if args.stats else 'checkpoint024_owned_stream' if args.stream else 'checkpoint023_owned_logs'
                result = subprocess.run(['cargo', 'test', '--manifest-path', str(REPO / 'src-tauri/Cargo.toml'), '--locked', '--no-run', '--message-format=json'], env=env, capture_output=True, text=True, check=True, timeout=180)
                executables = [json.loads(line)['executable'] for line in result.stdout.splitlines() if line.startswith('{') and json.loads(line).get('reason') == 'compiler-artifact' and json.loads(line).get('executable') and json.loads(line).get('target', {}).get('name') == 'containerdesk_lib' and json.loads(line).get('profile', {}).get('test') is True]
                assert len(executables) == 1
                test_env = {**env, 'PATH': '/nonexistent', 'CONTAINERDESK_LOG_LAB_MANIFEST': str(manifest)}
                listed = subprocess.run([executables[0], checkpoint, '--ignored', '--list'], env=test_env, capture_output=True, text=True, check=True, timeout=10)
                assert sum(line.endswith(': test') for line in listed.stdout.splitlines()) == 1
                checked = subprocess.run([executables[0], checkpoint, '--ignored', '--nocapture'], env=test_env, capture_output=True, text=True, timeout=60)
                assert all(marker not in checked.stdout + checked.stderr for marker in ('023-synthetic-log-private', '024-synthetic-live-line')), 'Synthetic raw log entered application diagnostics'
                print(checked.stdout, end='')
                if checked.returncode:
                    print(checked.stderr)
                    raise RuntimeError('Native log checkpoint failed')
            assert hashlib.sha256(known.read_bytes()).hexdigest() == before
            print('PASS: owned slow native reads obey host limits and disconnect cancellation; strict trust unchanged.' if args.reads else 'PASS: owned Docker events, cancellation, gap/replay and authoritative deleted-container snapshot; strict client trust unchanged.' if args.events else 'PASS: stats running/stopped/disappeared/concurrent/disconnect checks; strict client trust unchanged.' if args.stats else 'PASS: native streaming/cancellation/network-loss checkpoint; strict client trust unchanged.' if args.stream else 'PASS: real exited fixture logs match independent native Docker CLI; unsupported driver, UTF-8/line bounds; no raw synthetic log in app diagnostics; strict client trust unchanged.')
        finally:
            finished.set()
            if watcher: watcher.join(timeout=2)
            if server is not None:
                if server.poll() is None:
                    os.killpg(server.pid, signal.SIGTERM)
                server.wait(timeout=10)
            for ident in reversed(owned):
                if ident in removed: continue
                subprocess.run(docker + ['rm', '-f', '--', ident], env=env, check=True, stdout=subprocess.DEVNULL, timeout=10)
            print('Cleaned only owned log containers, loopback sshd and temporary keys/data.')


if __name__ == '__main__':
    main()
