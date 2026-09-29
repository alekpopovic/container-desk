#!/usr/bin/env python3
"""Opt-in 049: real native Rust/OpenSSH -> disposable QEMU VM -> dedicated Docker.

No host Docker invocation/socket, privileged container, host network changes, user keys,
or production SSH configuration. Requires Linux/KVM and an explicitly supplied image.
"""
import argparse
import base64
import hashlib
import json
import os
import platform
import secrets
from pathlib import Path
import shlex
import signal
import socket
import subprocess
import tempfile
import time

REPO = Path(__file__).resolve().parents[2]
IMAGE_SHA512 = 'd0ddf1faae4d44aee3ad6621f166bd414c2f99b6974fb455408612d59cbb31a5390ca259800ac0c6b60505493880ed6de8beb986f6d0883b26c8e84ce75a266c'
ARM_IMAGE_SHA512 = 'a53620902f99b1fd4591125d348e4385f036d590607f4fa90d6f85fea0248007ec763cc81d025f347ede5bd746b3726cf5eddd461b6c94c35376f71cb7558317'
PRIVATE = ('127.49.0.2', 22222)


def run(argv, **kwargs):
    return subprocess.run(argv, check=True, text=True, capture_output=True,
                          timeout=kwargs.pop('timeout', 30), **kwargs)


def closed(address):
    try:
        with socket.create_connection(address, timeout=1):
            return False
    except (ConnectionRefusedError, TimeoutError):
        return True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--qemu-root', type=Path, required=True, help='Extracted distribution packages; usr/bin and usr/share')
    parser.add_argument('--image', type=Path, required=True, help='Official pinned Alpine 3.22.4 BIOS/cloud-init qcow2')
    parser.add_argument('--artifacts', type=Path, required=True)
    parser.add_argument('--tcg', action='store_true', help='Use software CPU emulation for the remote VM when KVM is unavailable')
    parser.add_argument('--encrypted-agent', action='store_true', help='058 require encrypted generated key and owned native agent')
    parser.add_argument('--provision-only', action='store_true', help='Check native VM/SSH prerequisites only; not backend acceptance')
    parser.add_argument('--quick-start-tools', type=Path, help='Opt into the 057 native GUI quick-start before backend checks')
    args = parser.parse_args()
    if args.encrypted_agent and args.quick_start_tools:
        parser.error('Encrypted-agent matrix and the 057 walkthrough use separate owned agents; run each separately')
    args.artifacts.mkdir(parents=True, exist_ok=True)
    tools = args.qemu_root.resolve()
    image = args.image.resolve()
    mac = platform.system() == 'Darwin'
    arm = mac and platform.machine() == 'arm64'
    image_hash = ARM_IMAGE_SHA512 if arm else IMAGE_SHA512
    assert hashlib.sha512(image.read_bytes()).hexdigest() == image_hash, 'Unexpected base image'
    assert closed(PRIVATE), 'Private endpoint conflicts with an existing local listener'
    env = os.environ.copy()
    for key in ('SSH_AUTH_SOCK', 'SSH_AGENT_PID', 'SSH_ASKPASS', 'LD_PRELOAD'):
        env.pop(key, None)
    qenv = env if mac else {**env, 'LD_LIBRARY_PATH': str(tools / 'usr/lib/x86_64-linux-gnu')}
    bins = tools/'bin' if mac else tools/'usr/bin'
    qemu = bins/('qemu-system-aarch64' if arm else 'qemu-system-x86_64')
    result = {'schema': 2, 'appVersion': json.loads((REPO/'package.json').read_text())['version'],
              'sourceCommit': run(['git','rev-parse','HEAD'],cwd=REPO).stdout.strip(),
              'guestAcceleration': 'tcg' if mac or args.tcg else 'kvm', 'baseImageSha512': image_hash, 'clientOs': platform.platform(), 'clientArchitecture': platform.machine(),
              'encryptedAgentRequested': args.encrypted_agent,
              'qemu': run([str(qemu), '--version'], env=qenv).stdout.splitlines()[0],
              'clientSsh': run(['/usr/bin/ssh', '-V']).stderr.strip(),
              'hostDockerUsed': False, 'privilegedContainers': False,
              'privateAddress': list(PRIVATE), 'privateDirectTcpClosedBefore': True}
    with tempfile.TemporaryDirectory(prefix='containerdesk-049-', dir='/tmp') as area:
        root = Path(area)
        vm = None
        agent = None
        unrelated = None
        client = None
        observed = set()
        vm_identity = None
        try:
            passphrase = secrets.token_urlsafe(24) if args.encrypted_agent else ''
            for name in ('client', 'direct', 'jump', 'private', 'wrong'):
                try:
                    run(['/usr/bin/ssh-keygen', '-q', '-t', 'ed25519', '-N', passphrase if name == 'client' else '', '-f', str(root / name)])
                except subprocess.SubprocessError:
                    raise RuntimeError('Owned test key generation failed; credential-bearing argv withheld') from None
            if args.encrypted_agent:
                agent = subprocess.Popen(['/usr/bin/ssh-agent', '-D', '-a', str(root/'agent')], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                for _ in range(100):
                    if (root/'agent').exists(): break
                    time.sleep(.05)
                env['SSH_AUTH_SOCK'] = str(root/'agent')
                # Only an ephemeral lab key. The passphrase never enters reports or app env.
                (root/'passphrase').write_text(passphrase)
                (root/'passphrase').chmod(0o600)
                (root/'askpass').write_text('#!/bin/sh\nexec /bin/cat '+shlex.quote(str(root/'passphrase'))+'\n')
                (root/'askpass').chmod(0o700)
                denied_key = subprocess.run(['/usr/bin/ssh-keygen', '-y', '-P', '', '-f', str(root/'client')], capture_output=True, timeout=10)
                assert denied_key.returncode != 0, 'Generated identity must actually be encrypted'
                empty = subprocess.run(['/usr/bin/ssh-add', '-l'], env=env, capture_output=True, timeout=5)
                assert empty.returncode == 1, 'Owned agent must start empty'
            public = lambda name: ' '.join((root / (name + '.pub')).read_text().split()[:2])
            files = []

            def guest_file(path, content, permissions='0600'):
                files.append({'path': path, 'permissions': permissions, 'encoding': 'b64',
                              'content': base64.b64encode(content.encode()).decode()})

            guest_file('/opt/containerdesk/authorized_keys', public('client') + '\n', '0644')
            for role, address, port in [('direct', '0.0.0.0', 22220), ('jump', '0.0.0.0', 22221), ('private', *PRIVATE)]:
                guest_file(f'/opt/containerdesk/{role}-key', (root / role).read_text())
                guest_file(f'/opt/containerdesk/sshd-{role}.conf',
                           f'Port {port}\nListenAddress {address}\nHostKey /opt/containerdesk/{role}-key\n'
                           f'PidFile /run/containerdesk-{role}.pid\nAuthorizedKeysFile /opt/containerdesk/authorized_keys\n'
                           'AllowUsers lab root\nPermitRootLogin prohibit-password\nPasswordAuthentication no\n'
                           'KbdInteractiveAuthentication no\nAllowAgentForwarding no\nX11Forwarding no\n'
                           + ('AllowTcpForwarding local\nPermitOpen 127.49.0.2:22222\n' if role == 'jump' else 'AllowTcpForwarding no\n'))
            guest_file('/opt/containerdesk/cut.py', '''import os, pwd, signal
from pathlib import Path
uid = pwd.getpwnam("lab").pw_uid
for entry in Path("/proc").iterdir():
    if not entry.name.isdigit(): continue
    try:
        fields = dict(line.split(":", 1) for line in (entry / "status").read_text().splitlines() if ":" in line)
        if int(fields["Uid"].split()[0]) == uid and fields["Name"].strip().startswith("sshd"):
            os.kill(int(entry.name), signal.SIGKILL)
    except (ProcessLookupError, FileNotFoundError): pass
''', '0700')
            guest_file('/opt/containerdesk/lost-response-docker', """#!/bin/sh
/usr/bin/docker "$@"
status=$?
case " $* " in
  *" stop "*) if [ "$status" -eq 0 ]; then kill -HUP "$PPID"; sleep 5; exit 99; fi ;;
esac
exit "$status"
""", '0755')
            guest_file('/opt/containerdesk/setup.sh', (REPO / 'tests/lab/integration_guest.sh').read_text(), '0700')
            # Apostrophe and space exercise the application's central remote POSIX quoting.
            compose = {'services': {name: {'image': 'alpine@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce',
                        'command': ['sleep', '3600'], 'network_mode': 'none', 'read_only': True,
                        'cap_drop': ['ALL'], 'labels': {'dev.containerdesk.lab': '049'}} for name in ('web', 'worker')}}
            guest_file("/opt/owned project's/compose.yml", json.dumps(compose), '0644')
            cloud = {'users': ['default'], 'ssh_pwauth': False, 'write_files': files,
                     'runcmd': [['/bin/sh', '/opt/containerdesk/setup.sh']]}
            seed = root/'seed'
            seed.mkdir(mode=0o700)
            (seed / 'user-data').write_text('#cloud-config\n' + json.dumps(cloud))
            (seed / 'user-data').chmod(0o600)
            (seed / 'meta-data').write_text('instance-id: containerdesk-049\nlocal-hostname: owned049\n')
            if mac:
                run(['/usr/bin/hdiutil', 'makehybrid', '-o', str(root/'seed.iso'), '-iso', '-joliet', '-default-volume-name', 'cidata', str(seed)])
            else:
                run([str(bins/'genisoimage'), '-quiet', '-output', str(root/'seed.iso'), '-volid', 'cidata',
                     '-joliet', '-rock', str(seed/'user-data'), str(seed/'meta-data')])
            run([str(bins / 'qemu-img'), 'create', '-f', 'qcow2', '-F', 'qcow2', '-b', str(image), str(root / 'disk.qcow2'), '8G'], env=qenv)
            # Hold both ephemeral ports until immediately before QEMU binds them; a race fails closed.
            sockets = [socket.socket(), socket.socket()]
            for sock in sockets:
                sock.bind(('127.0.0.1', 0))
            direct_port, jump_port = [sock.getsockname()[1] for sock in sockets]
            for sock in sockets:
                sock.close()
            (root / 'known').write_text(''.join(f'{role}-key {public(role)}\n' for role in ('direct', 'jump', 'private')))
            for role in ('direct', 'jump', 'private'):
                (root / ('bad-' + role)).write_text(''.join(f'{item}-key {public("wrong" if item == role else item)}\n' for item in ('direct', 'jump', 'private')))
            config = root / 'config'
            sections = []
            for alias, role, port, trust, jump in [
                    ('direct-owned', 'direct', direct_port, 'known', None),
                    ('jump-owned', 'jump', jump_port, 'known', None),
                    ('private-owned', 'private', PRIVATE[1], 'known', 'jump-owned'),
                    ('direct-bad', 'direct', direct_port, 'bad-direct', None),
                    ('private-bad', 'private', PRIVATE[1], 'bad-private', 'jump-owned'),
                    ('jump-bad', 'jump', jump_port, 'bad-jump', None),
                    ('via-bad-jump', 'private', PRIVATE[1], 'known', 'jump-bad'),
                    ('direct-unknown', 'direct', direct_port, 'empty-known', None)]:
                sections.append(f'Host {alias}\n HostName {PRIVATE[0] if role == "private" else "127.0.0.1"}\n Port {port}\n'
                                f' HostKeyAlias {role}-key\n UserKnownHostsFile {root / trust}\n'
                                + (f' ProxyJump {jump}\n' if jump else ''))
            (root/'empty-known').write_text('')
            identity_file = root/('client.pub' if args.encrypted_agent else 'client')
            identity_agent = str(root/'agent') if args.encrypted_agent else 'none'
            config.write_text(''.join(sections) + f'Host *\n User lab\n IdentityFile {identity_file}\n IdentitiesOnly yes\n'
                              f' IdentityAgent {identity_agent}\n BatchMode yes\n StrictHostKeyChecking yes\n UpdateHostKeys no\n'
                              ' ForwardAgent no\n ConnectTimeout 3\n GlobalKnownHostsFile /dev/null\n')
            config.chmod(0o600)
            watched = [config, root / 'known', root/'empty-known'] + list(root.glob('bad-*'))
            hashes = {p: hashlib.sha256(p.read_bytes()).hexdigest() for p in watched}
            machine = (['-machine', 'virt', '-cpu', 'cortex-a72', '-bios', str(tools/'share/qemu/edk2-aarch64-code.fd')] if arm else [])
            acceleration = ['-accel', 'tcg,thread=multi'] if mac or args.tcg else ['-enable-kvm', '-cpu', 'host']
            if args.tcg and not mac: machine = ['-cpu', 'max']
            if mac and not arm: machine = ['-cpu', 'max']
            firmware = [] if mac else ['-L', str(tools/'usr/share/qemu'), '-bios', str(tools/'usr/share/seabios/bios-256k.bin')]
            command = [str(qemu), *acceleration, *machine, '-smp', '2', '-m', '2048', *firmware,
                       '-display', 'none', '-vga', 'none', '-global', 'virtio-net-pci.romfile=', '-monitor', 'none', '-serial', 'file:' + str(root / 'serial.log'),
                       '-no-reboot', '-drive', f'file={root / "disk.qcow2"},format=qcow2,if=virtio',
                       '-drive', f'file={root / "seed.iso"},format=raw,readonly=on'+(',if=virtio' if arm else ',media=cdrom'),
                       '-nic', f'user,model=virtio-net-pci,hostfwd=tcp:127.0.0.1:{direct_port}-:22220,hostfwd=tcp:127.0.0.1:{jump_port}-:22221']
            with (root / 'qemu.log').open('w') as log:
                vm = subprocess.Popen(command, env=qenv, stdout=log, stderr=log, start_new_session=True)
            vm_identity = identity(vm.pid)

            def remote(command, **kwargs):
                return run(['/usr/bin/ssh', '-F', str(config), '-T', '-n', '-l', 'root', '--', 'direct-owned', command], env=env, **kwargs)

            if args.encrypted_agent:
                # First prove an actual SSH auth refusal with the empty owned agent.
                auth_deadline = time.monotonic()+900
                while True:
                    failed = subprocess.run(['/usr/bin/ssh', '-F', str(config), '-T', '-n', '--', 'direct-owned', 'true'], env=env, capture_output=True, timeout=8)
                    if b'Permission denied' in failed.stderr: break
                    assert vm.poll() is None, 'VM exited during encrypted-agent preflight: '+(root/'qemu.log').read_text()[-2000:]
                    if time.monotonic()>auth_deadline: raise RuntimeError('VM SSH did not reach empty-agent authentication check')
                    time.sleep(1)
                assert failed.returncode == 255 and not failed.stdout
                run(['/usr/bin/ssh-add', str(root/'client')], env={**env, 'SSH_ASKPASS':str(root/'askpass'), 'SSH_ASKPASS_REQUIRE':'force', 'DISPLAY':':0'})
                result['encryptedKeyRejectsEmptyPassphrase'] = True
                result['emptyAgentAuthenticationRejected'] = True
                result['encryptedKeyLoadedIntoNativeAgent'] = True
            deadline = time.monotonic() + (900 if mac or args.tcg else 240)
            while True:
                assert vm.poll() is None, 'VM exited: ' + (root / 'qemu.log').read_text()[-2000:]
                try:
                    remote('test -f /opt/containerdesk/ready', timeout=5)
                    break
                except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
                    if time.monotonic() >= deadline:
                        # Seed contains private keys: never dump cloud-init/user-data or serial logs.
                        raise RuntimeError('Guest setup exceeded its native platform deadline; private logs removed on cleanup') from None
                    time.sleep(1)
            if args.provision_only:
                result.update(provisionOnly=True, backendAcceptance='not_run')
                (args.artifacts/'provision.json').write_text(json.dumps(result,indent=2)+'\n')
                print(json.dumps(result,indent=2),flush=True)
                return
            unrelated = subprocess.Popen(['/usr/bin/ssh', '-F', str(config), '-M', '-N', '-S', str(root/'user-master'), '-l', 'root', '--', 'direct-owned'], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
            for _ in range(100):
                if (root/'user-master').exists(): break
                time.sleep(.05)
            assert unrelated.poll() is None and (root/'user-master').exists()
            print('Dedicated VM Docker ready; checking private transport and native backend.', flush=True)
            assert closed(PRIVATE)
            denied = subprocess.run(['/usr/bin/ssh', '-F', str(config), '-T', '-n', '-o', 'ProxyJump=none', '--',
                                     'private-owned', "exec 'printf' 'must-not-run'"], env=env, capture_output=True, timeout=6)
            assert denied.returncode != 0 and not denied.stdout
            reached = run(['/usr/bin/ssh', '-F', str(config), '-T', '-n', '--', 'private-owned',
                           "exec 'printf' 'private-through-bastion'"], env=env)
            assert reached.stdout == 'private-through-bastion'
            result.update(privateDirectTcpClosedDuring=True, privateWithoutJumpRejected=True, privateViaJumpVerified=True)
            oracle = json.loads(remote("docker inspect owned-live untouched owned049-web-1 owned049-worker-1").stdout)
            versions = json.loads(remote("docker version --format '{{json .}}'").stdout)
            result['guest'] = {'alpine': remote('cat /etc/alpine-release').stdout.strip(),
                               'kernel': remote('uname -r').stdout.strip(),
                               'dockerServer': versions['Server']['Version'], 'dockerClient': versions['Client']['Version'],
                               'compose': remote('docker compose version --short').stdout.strip(),
                               'ssh': remote('ssh -V').stderr.strip()}
            manifest = root / 'manifest.json'
            manifest.write_text(json.dumps({'config': str(config), 'ownedIds': [row['Id'] for row in oracle],
                                           'liveId': oracle[0]['Id'], 'composeIds': [row['Id'] for row in oracle[2:]],
                                           'configuration': {'projectName': 'owned049', 'workingDirectory': "/opt/owned project's",
                                                             'configFiles': ["/opt/owned project's/compose.yml"]}}))
            if args.quick_start_tools:
                gui = run(['python3', str(REPO/'tests/lab/quick_start.py'), '--tools-dir', str(args.quick_start_tools.resolve()),
                    '--manifest', str(manifest), '--artifacts', str(args.artifacts.resolve()/'quick-start')], env=env, timeout=260)
                print(gui.stdout, flush=True)
                unchanged = json.loads(remote('docker inspect owned-live untouched owned049-web-1 owned049-worker-1').stdout)
                assert all(after['State']['StartedAt'] == before['State']['StartedAt'] for before, after in zip(oracle, unchanged))
                result['quickStartNoMutation'] = True
            build = run(['cargo', 'test', '--manifest-path', str(REPO / 'src-tauri/Cargo.toml'), '--locked', '--lib', '--no-run', '--message-format=json'], env=env, timeout=180)
            records = [json.loads(line) for line in build.stdout.splitlines() if line.startswith('{')]
            executable = next(item['executable'] for item in records if item.get('reason') == 'compiler-artifact' and item.get('profile', {}).get('test') and item.get('executable'))
            result['nativeTestExecutableSha256'] = hashlib.sha256(Path(executable).read_bytes()).hexdigest()
            listed = run([executable, 'checkpoint049_disposable_integration', '--ignored', '--list'], env=env).stdout
            assert sum(line.endswith(': test') for line in listed.splitlines()) == 1, 'Expected exactly one native test'
            since = int(remote('date +%s').stdout.strip()) + 1
            time.sleep(1.2)
            test_env = {**env, 'PATH': '/nonexistent', 'CONTAINERDESK_INTEGRATION_MANIFEST': str(manifest)}
            with (root / 'native.log').open('w') as log:
                client = subprocess.Popen([executable, 'checkpoint049_disposable_integration', '--ignored', '--nocapture'],
                                          env=test_env, stdout=log, stderr=log, start_new_session=True)
                deadline = time.monotonic() + 180
                from native_pressure import process_sample
                while client.poll() is None:
                    for alias in ('direct-owned', 'private-owned'):
                        marker = root / ('cut-' + alias)
                        done = root / ('cut-' + alias + '-done')
                        if marker.exists() and not done.exists():
                            remote("exec 'python3' '/opt/containerdesk/cut.py'")
                            done.write_text('owned guest lab SSH sessions cut')
                    if mac:
                        observed.update(ssh_identities(config, exclude=unrelated.pid))
                    else:
                        observed.update((pid, int(started)) for pid, started in process_sample(client.pid)['sshIdentities'])
                    if time.monotonic() > deadline:
                        raise RuntimeError('Native integration deadline exceeded')
                    time.sleep(.05)
            # Test emits only explicit PASS summaries; retain these, never arbitrary remote diagnostics.
            log = (root / 'native.log').read_text()
            result['nativeExit'] = client.returncode
            result['checks'] = [line for line in log.splitlines() if line.startswith(('PASS 049:', 'PASS 058:'))]
            if client.returncode:
                print(log[-8000:])  # Rust's redacted types; no raw shell/inspect transcript.
                raise RuntimeError('Native integration assertions failed')
            assert len(result['checks']) == 8, 'Missing native acceptance checks'
            until = int(remote('date +%s').stdout.strip()) + 1
            events = [json.loads(line) for line in remote(f"docker events --since {since} --until {until} --filter type=container --format '{{{{json .}}}}'").stdout.splitlines()]
            counts = {}
            for i, row in enumerate(oracle):
                counts[row['Name'].lstrip('/')] = {action: sum(event['Actor']['ID'] == row['Id'] and event['Action'] == action for event in events) for action in ('start', 'die')}
                assert all(value == (0 if i == 1 else 3 if i == 0 else 2) for value in counts[row['Name'].lstrip('/')].values()), 'Unexpected action count or replay'
            result['independentDockerEventCounts'] = counts
            after = json.loads(remote('docker inspect owned-live untouched owned049-web-1 owned049-worker-1').stdout)
            assert all(row['State']['Running'] for row in after)
            assert after[1]['State']['StartedAt'] == oracle[1]['State']['StartedAt']
            assert all(after[i]['State']['StartedAt'] != oracle[i]['State']['StartedAt'] for i in (0, 2, 3))
            assert hashes == {p: hashlib.sha256(p.read_bytes()).hexdigest() for p in watched}
            result.update(independentDockerStateVerified=True, untouchedContainerUnchanged=True, trustAndConfigUnchanged=True)
            # Remove guest resources using only explicit IDs before destroying the owned VM disk.
            remote('docker rm -f -- ' + ' '.join(shlex.quote(row['Id']) for row in oracle))
            assert not remote('docker ps --all --quiet').stdout.strip()
            result['guestContainersRemoved'] = len(oracle)
            assert unrelated.poll() is None, 'Application terminated unrelated SSH master'
            run(['/usr/bin/ssh', '-F', str(config), '-S', str(root/'user-master'), '-O', 'check', '-l', 'root', '--', 'direct-owned'], env=env)
            result['unrelatedSshMasterPreserved'] = True
            assert observed and all(identity(pid) != (pid, started) for pid, started in observed), 'Owned SSH processes survived backend shutdown'
            result['observedSshIdentitiesReaped'] = len(observed)
        finally:
            for pid, started in observed:
                if identity(pid) == (pid, started):
                    try: os.kill(pid, signal.SIGTERM)
                    except ProcessLookupError: pass
            for child in (unrelated, client, vm):
                if child is not None:
                    if child.poll() is None:
                        try: os.killpg(child.pid, signal.SIGTERM)
                        except ProcessLookupError: pass
                        try:
                            child.wait(timeout=10)
                        except subprocess.TimeoutExpired:
                            try: os.killpg(child.pid, signal.SIGKILL)
                            except ProcessLookupError: pass
                    child.wait(timeout=10)
            if agent is not None:
                agent.terminate(); agent.wait(timeout=10)
                result['ownedAgentReaped'] = agent.poll() is not None
            result['vmReaped'] = vm_identity is not None and identity(vm_identity[0]) != vm_identity
    result['temporaryKeysSeedAndDiskRemoved'] = not root.exists()
    assert result['vmReaped'] and result['temporaryKeysSeedAndDiskRemoved']
    (args.artifacts / 'integration.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))


def ssh_identities(config, exclude):
    rows = run(['/bin/ps', '-axo', 'pid=,lstart=,command=']).stdout.splitlines()
    result=set()
    for row in rows:
        fields=row.split(None,6)
        if len(fields)==7 and str(config) in fields[6] and re_ssh(fields[6]):
            pid=int(fields[0])
            if pid != exclude: result.add((pid,' '.join(fields[1:6])))
    return result


def re_ssh(command):
    return command.startswith(('/usr/bin/ssh ', 'ssh '))


def identity(pid):
    if platform.system() == 'Darwin':
        value=subprocess.run(['/bin/ps','-p',str(pid),'-o','lstart='],capture_output=True,text=True,timeout=5)
        return (pid,' '.join(value.stdout.split())) if value.returncode==0 and value.stdout.strip() else None
    try:
        stat = Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()
        return pid, int(stat[19])
    except (FileNotFoundError, ProcessLookupError):
        return None


if __name__ == '__main__':
    main()
