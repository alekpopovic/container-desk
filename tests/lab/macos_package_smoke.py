#!/usr/bin/env python3
"""054 native macOS ordinary app/DMG inspection, Finder launch and SSH diagnostics."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import plistlib
import shutil
import signal
import stat
import subprocess
import tempfile
import time

REPO = Path(__file__).resolve().parents[2]


def run(argv, **kw):
    return subprocess.run(argv,check=True,capture_output=True,text=True,timeout=kw.pop('timeout',30),**kw)


def digest(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--packages',type=Path,default=REPO/'dist-artifacts')
    parser.add_argument('--artifacts',type=Path,default=REPO/'test-results/macos-package')
    args=parser.parse_args()
    assert platform.system()=='Darwin' and platform.machine() in ('arm64','x86_64')
    args.artifacts.mkdir(parents=True,exist_ok=True)
    app_data=Path.home()/'Library/Application Support/dev.containerdesk.app'
    assert not app_data.exists(), 'Refuse an existing application profile'
    metadata=json.loads(next(args.packages.glob('*.json')).read_text())
    for item in metadata['packages']:
        path=args.packages/item['file']
        assert path.name==item['file'] and digest(path)==item['sha256'] and path.stat().st_size==item['bytes']
    dmg=next(args.packages.glob('*.dmg'))
    result={'passed':False,'architecture':platform.machine(),'osVersion':run(['/usr/bin/sw_vers']).stdout,
            'dmgSha256':digest(dmg),'signing':'local_unsigned_or_linker_adhoc','notarization':'not_requested','gatekeeperApproved':False}
    with tempfile.TemporaryDirectory(prefix='cd054-',dir='/tmp') as area:
        root=Path(area).resolve()
        mount=root/'mounted';mount.mkdir()
        mounted=False
        agent=None
        installed=root/'Applications/ContainerDesk.app'
        try:
            run(['/usr/bin/hdiutil','verify',str(dmg)],timeout=120)
            run(['/usr/bin/hdiutil','attach','-readonly','-nobrowse','-mountpoint',str(mount),str(dmg)],timeout=60);mounted=True
            source=mount/'ContainerDesk.app'
            assert source.is_dir() and (mount/'Applications').is_symlink() and os.readlink(mount/'Applications')=='/Applications'
            result['dmgApplicationLink']=True
            run(['/usr/bin/ditto',str(source),str(installed)],timeout=60)
            run(['/usr/bin/hdiutil','detach',str(mount)],timeout=30);mounted=False
            info=plistlib.loads((installed/'Contents/Info.plist').read_bytes())
            assert info['CFBundleIdentifier']=='dev.containerdesk.app' and info['CFBundleShortVersionString']==metadata['version']
            assert info['LSMinimumSystemVersion']=='15.0'
            executable=installed/'Contents/MacOS'/info['CFBundleExecutable']
            assert os.access(executable,os.X_OK)
            arch=run(['/usr/bin/lipo','-archs',str(executable)]).stdout.strip()
            assert arch==platform.machine(), 'A cross-compiled or universal app is not this test target'
            assert digest(executable)==metadata['executableSha256BeforeBundle']
            icon=installed/'Contents/Resources'/info['CFBundleIconFile']
            if not icon.suffix:icon=icon.with_suffix('.icns')
            assert icon.is_file() and icon.read_bytes().startswith(b'icns')
            result.update(bundleIdentifier=info['CFBundleIdentifier'],version=info['CFBundleShortVersionString'],minimumSystemVersion=info['LSMinimumSystemVersion'],machOArchitecture=arch,installedExecutableSha256=digest(executable),iconSha256=digest(icon))
            assert not any(p.name in ('id_rsa','id_ed25519','known_hosts','authorized_keys') or p.suffix=='.p12' for p in installed.rglob('*'))
            signature=subprocess.run(['/usr/bin/codesign','-dv','--verbose=4',str(installed)],capture_output=True,text=True,timeout=15)
            result['signatureInspection']={'exit':signature.returncode,'details':signature.stderr}
            helper=root/'package-ui'
            run(['/usr/bin/swiftc',str(REPO/'tests/lab/macos_package_ui.swift'),'-o',str(helper)],timeout=120)
            # Finder receives the path as an AppleScript argv value, never interpolated code.
            finder='on run argv\nset appFile to POSIX file (item 1 of argv)\ntell application "Finder" to open appFile\nend run'
            run(['/usr/bin/osascript','-e',finder,str(installed)],timeout=30)
            run([str(helper),str(installed),str(args.artifacts/'finder.json'),'finder'],timeout=90)
            assert app_data.is_dir() and (app_data/'preferences').is_dir()
            assert stat.S_IMODE((app_data/'preferences').stat().st_mode)==0o700
            assert stat.S_IMODE((app_data/'preferences/settings.lock').stat().st_mode)==0o600
            result['nativeAppDataPath']=str(app_data)
            result['privatePreferencesPermissions']=True
            # Separate LaunchServices launch with an empty owned agent. No user agent/key is modified.
            agent_socket=root/'agent.sock'
            with (root/'agent.log').open('w') as log:
                agent=subprocess.Popen(['/usr/bin/ssh-agent','-D','-a',str(agent_socket)],stdout=log,stderr=log,start_new_session=True)
            deadline=time.monotonic()+10
            while not agent_socket.exists() and time.monotonic()<deadline:time.sleep(.1)
            assert agent_socket.is_socket() and agent.poll() is None
            run(['/usr/bin/open','-n','-a',str(installed),'--env','SSH_AUTH_SOCK='+str(agent_socket),'--env','PATH=/usr/bin:/bin'],timeout=30)
            run([str(helper),str(installed),str(args.artifacts/'diagnostics.json'),'diagnostics'],timeout=120)
            result['finder']=json.loads((args.artifacts/'finder.json').read_text())
            result['diagnostics']=json.loads((args.artifacts/'diagnostics.json').read_text())
            assert result['finder']['passed'] and result['diagnostics']['passed']
            run(['/usr/bin/open','-n','-a',str(installed)],timeout=30)
            run([str(helper),str(installed),str(args.artifacts/'support.json'),'support'],timeout=120)
            result['support']=json.loads((args.artifacts/'support.json').read_text())
            assert result['support']['passed'] and result['support']['nativeSupportSaveDialog']
            result['supportExportSha256']=digest(args.artifacts/'containerdesk-support.json')
            result['passed']=True
        except Exception as error:
            result['error']={'type':type(error).__name__,'message':str(error)}
            if isinstance(error,subprocess.CalledProcessError):
                (args.artifacts/'failure.txt').write_text(error.stdout+'\n'+error.stderr)
            raise
        finally:
            # The helper requests normal termination. On a failure, target only our unique installed path.
            rows=subprocess.run(['/bin/ps','-axo','pid=,command='],capture_output=True,text=True,timeout=10).stdout.splitlines()
            for row in rows:
                fields=row.strip().split(None,1)
                if len(fields)==2 and fields[1]==str(installed/'Contents/MacOS/containerdesk'):
                    try:os.kill(int(fields[0]),signal.SIGTERM)
                    except ProcessLookupError:pass
            if agent:
                if agent.poll() is None:os.killpg(agent.pid,signal.SIGTERM)
                agent.wait(timeout=10)
                result['ownedAgentReaped']=True
            if mounted:run(['/usr/bin/hdiutil','detach',str(mount)],timeout=30)
            if app_data.exists():shutil.rmtree(app_data)
            result['ownedProfileRemoved']=not app_data.exists()
            (args.artifacts/'package.json').write_text(json.dumps(result,indent=2)+'\n')
    print('PASS native macOS DMG contents, Finder launch, private app data, system OpenSSH, owned agent access and cleanup.')


if __name__=='__main__':main()
