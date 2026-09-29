#!/usr/bin/env python3
"""053 disposable Ubuntu 24.04 desktop, ordinary installed packages only."""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import shlex
import signal
import socket
import subprocess
import tempfile
import time

REPO = Path(__file__).resolve().parents[2]
IMAGE_SHA256 = '6a81c37564db9b1ee84e141922625e1d7c5b389b99bb3c572e0243607d5bb4d2'


def run(argv, **kw):
    return subprocess.run(argv, check=True, capture_output=True, text=True, timeout=kw.pop('timeout', 30), **kw)


def digest(path):
    with path.open('rb') as stream: return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ('qemu-root', 'image', 'packages', 'artifacts'): parser.add_argument('--'+key, type=Path, required=True)
    parser.add_argument('--format', choices=['deb','appimage'], default='deb')
    parser.add_argument('--tcg', action='store_true', help='Use a software-emulated guest when KVM is unavailable')
    args = parser.parse_args()
    assert digest(args.image) == IMAGE_SHA256, 'Unexpected Ubuntu image'
    args.artifacts.mkdir(parents=True, exist_ok=True)
    tools = args.qemu_root.resolve()
    env = os.environ.copy()
    for name in ('SSH_AUTH_SOCK', 'SSH_AGENT_PID', 'LD_PRELOAD'): env.pop(name, None)
    qenv = {**env, 'LD_LIBRARY_PATH': str(tools/'usr/lib/x86_64-linux-gnu')}
    result = {'baseImageSha256': IMAGE_SHA256, 'nativeArchitecture': 'x86_64', 'publicPublishing': False, 'format': args.format, 'guestAcceleration': 'tcg' if args.tcg else 'kvm'}
    with tempfile.TemporaryDirectory(prefix='containerdesk-053-vm-') as area:
        root = Path(area)
        vm = None
        try:
            for name in ('client', 'host'):
                run(['ssh-keygen', '-q', '-t', 'ed25519', '-N', '', '-f', str(root/name)])
            public = lambda name: ' '.join((root/(name+'.pub')).read_text().split()[:2])
            setup = '''#!/bin/sh
set -eu
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y --no-install-recommends openbox xfce4-panel xfdesktop4 xvfb xauth dbus-x11 xdotool imagemagick python3-pyatspi libfuse2t64 libgtk-3-bin desktop-file-utils fonts-dejavu-core libwebkit2gtk-4.1-0 libgtk-3-0t64 libxdo3
install -d -m 0755 /opt/containerdesk/packages /opt/containerdesk/results
chown lab:lab /opt/containerdesk/results
chmod 0755 /opt/containerdesk/smoke.py
touch /opt/containerdesk/ready
'''
            files = []
            for path, data, mode in [('/etc/ssh/ssh_host_ed25519_key', (root/'host').read_text(), '0600'),
                                     ('/etc/ssh/ssh_host_ed25519_key.pub', public('host')+'\n', '0644'),
                                     ('/opt/containerdesk/setup.sh', setup, '0700'),
                                     ('/opt/containerdesk/smoke.py', (REPO/'tests/lab/linux_package_smoke.py').read_text(), '0755')]:
                files.append({'path': path, 'permissions': mode, 'encoding': 'b64', 'content': base64.b64encode(data.encode()).decode()})
            cloud = {'users': [{'name': 'lab', 'shell': '/bin/bash', 'lock_passwd': True, 'ssh_authorized_keys': [public('client')], 'sudo': 'ALL=(ALL) NOPASSWD:ALL'}],
                     'ssh_deletekeys': False, 'ssh_genkeytypes': [], 'ssh_pwauth': False, 'write_files': files,
                     'runcmd': [['/bin/sh', '/opt/containerdesk/setup.sh']]}
            (root/'user-data').write_text('#cloud-config\n'+json.dumps(cloud)); (root/'user-data').chmod(0o600)
            (root/'meta-data').write_text('instance-id: containerdesk-053\nlocal-hostname: package-desktop\n')
            run([str(tools/'usr/bin/genisoimage'), '-quiet', '-output', str(root/'seed.iso'), '-volid', 'cidata', '-joliet', '-rock', str(root/'user-data'), str(root/'meta-data')])
            run([str(tools/'usr/bin/qemu-img'), 'create', '-f', 'qcow2', '-F', 'qcow2', '-b', str(args.image.resolve()), str(root/'disk.qcow2'), '12G'], env=qenv)
            with socket.socket() as sock:
                sock.bind(('127.0.0.1',0)); port = sock.getsockname()[1]
            (root/'known').write_text('owned-package-vm '+public('host')+'\n')
            (root/'config').write_text(f'Host package-vm\n HostName 127.0.0.1\n Port {port}\n User lab\n HostKeyAlias owned-package-vm\n UserKnownHostsFile {root/"known"}\n GlobalKnownHostsFile /dev/null\n IdentityFile {root/"client"}\n IdentitiesOnly yes\n IdentityAgent none\n BatchMode yes\n StrictHostKeyChecking yes\n UpdateHostKeys no\n ForwardAgent no\n ConnectTimeout 3\n')
            (root/'config').chmod(0o600)
            qemu = [str(tools/'usr/bin/qemu-system-x86_64'), '-accel', 'tcg' if args.tcg else 'kvm', '-cpu', 'max' if args.tcg else 'host', '-smp', '2', '-m', '3072', '-L', str(tools/'usr/share/qemu'), '-bios', str(tools/'usr/share/seabios/bios-256k.bin'), '-display', 'none', '-vga', 'none', '-global', 'virtio-net-pci.romfile=', '-monitor', 'none', '-serial', 'file:'+str(root/'serial.log'), '-no-reboot', '-drive', f'file={root/"disk.qcow2"},format=qcow2,if=virtio', '-drive', f'file={root/"seed.iso"},format=raw,media=cdrom,readonly=on', '-nic', f'user,model=virtio-net-pci,hostfwd=tcp:127.0.0.1:{port}-:22']
            with (root/'qemu.log').open('w') as log: vm = subprocess.Popen(qemu, env=qenv, stdout=log, stderr=log, start_new_session=True)
            ssh = ['ssh', '-F', str(root/'config'), '-T', '-n', '--', 'package-vm']
            def remote(command, timeout=30): return run([*ssh, command], env=env, timeout=timeout)
            setup_timeout = 1800 if args.tcg else 900
            deadline = time.monotonic()+setup_timeout
            while True:
                if vm.poll() is not None:
                    detail = (root/'qemu.log').read_text(errors='replace')[-2000:].replace(str(root), '<owned-vm>')
                    result['startupError'] = detail
                    raise RuntimeError('VM exited before desktop setup: '+detail)
                try:
                    remote('test -f /opt/containerdesk/ready', 5); break
                except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
                    if time.monotonic()>deadline: raise RuntimeError(f'Ubuntu desktop setup exceeded {setup_timeout} seconds') from None
                    time.sleep(2)
            print('Clean Ubuntu 24.04 desktop ready; waiting for checked CI packages.', flush=True)
            result['osRelease'] = remote('cat /etc/os-release').stdout
            result['kernel'] = remote('uname -r').stdout.strip()
            result['buildToolsAbsent'] = remote('for x in node npm cargo rustc docker; do if command -v "$x" >/dev/null; then exit 1; fi; done; echo verified').stdout.strip()=='verified'
            deadline = time.monotonic()+2400
            while not (args.packages/'SHA256SUMS').exists():
                if time.monotonic()>deadline: raise RuntimeError('No CI packages supplied within 2400 seconds')
                time.sleep(2)
            for line in (args.packages/'SHA256SUMS').read_text().splitlines():
                expected, name = line.split('  ',1)
                assert Path(name).name == name and digest(args.packages/name)==expected
            if args.format == 'deb':
                deb = next(args.packages.glob('*.deb'))
                result['debSha256'] = digest(deb)
                run(['scp','-F',str(root/'config'),str(deb),'package-vm:/tmp/owned-package.deb'],env=env,timeout=120)
                installation = remote('sudo env DEBIAN_FRONTEND=noninteractive apt-get install -y /tmp/owned-package.deb',600 if args.tcg else 300)
                (args.artifacts/'installation.txt').write_text(installation.stdout)
            else:
                appimage = next(args.packages.glob('*.AppImage'))
                result['appImageSha256'] = digest(appimage)
                run(['scp','-F',str(root/'config'),str(appimage),'package-vm:/tmp/owned-package.AppImage'],env=env,timeout=120)
                remote('sudo install -m 0755 /tmp/owned-package.AppImage /opt/containerdesk/packages/ContainerDesk.AppImage && sudo modprobe fuse && test -c /dev/fuse')
            result['runtimeVersions'] = remote("dpkg-query -W -f='${Package} ${Version}\\n' libwebkit2gtk-4.1-0 libgtk-3-0t64 libfuse2t64 openssh-client openbox xvfb").stdout
            session = 'env PATH=/usr/bin:/bin GDK_BACKEND=x11 LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 GTK_MODULES=gail:atk-bridge xvfb-run -a -s "-screen 0 1440x1100x24 -nolisten tcp" dbus-run-session -- python3 /opt/containerdesk/smoke.py'
            if args.format == 'appimage': session += ' --appimage /opt/containerdesk/packages/ContainerDesk.AppImage'
            try:
                completed = remote(session,360 if args.tcg else 180)
                (args.artifacts/'desktop-output.txt').write_text(completed.stdout)
            except subprocess.CalledProcessError as error:
                (args.artifacts/'desktop-error.txt').write_text(error.stdout+'\n'+error.stderr)
                raise
            finally:
                run(['scp','-r','-F',str(root/'config'),'package-vm:/opt/containerdesk/results/.',str(args.artifacts)],env=env,timeout=60)
            result['desktop'] = json.loads((args.artifacts/'desktop.json').read_text())
            assert result['desktop']['passed']
            result['passed'] = True
        finally:
            if vm:
                if vm.poll() is None: os.killpg(vm.pid,signal.SIGTERM)
                try: vm.wait(timeout=15)
                except subprocess.TimeoutExpired: os.killpg(vm.pid,signal.SIGKILL); vm.wait(timeout=5)
                result['vmReaped'] = True
            (args.artifacts/'vm.json').write_text(json.dumps(result,indent=2)+'\n')
    print('PASS clean Ubuntu VM package installation, desktop launch, SSH discovery and cleanup.',flush=True)


if __name__=='__main__': main()
