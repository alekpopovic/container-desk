#!/usr/bin/env python3
"""058 CI-only preparation/execution of an owned native SSH/Docker VM lab."""
import argparse
import hashlib
import os
from pathlib import Path
import platform
import subprocess
import sys
import tempfile
import time

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0,str(REPO/'tests/lab'))
from integration import IMAGE_SHA512, ARM_IMAGE_SHA512


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--provision-only',action='store_true')
    args=parser.parse_args()
    mac=platform.system()=='Darwin'
    arm=mac and platform.machine()=='arm64'
    if platform.machine() not in ('arm64','x86_64'): raise RuntimeError('Unsupported native client architecture')
    if mac:
        subprocess.run(['brew','install','qemu'],check=True,timeout=900,env={**os.environ,'HOMEBREW_NO_AUTO_UPDATE':'1'})
        tools=Path(subprocess.check_output(['brew','--prefix','qemu'],text=True,timeout=15).strip())
    else:
        tools=Path('/')
        if not Path('/usr/bin/qemu-system-x86_64').is_file():
            raise RuntimeError('Install qemu-system-x86, qemu-utils, seabios and genisoimage in the owned Linux CI runner')
    name='nocloud_alpine-3.22.4-'+('aarch64-uefi' if arm else 'x86_64-bios')+'-cloudinit-r0.qcow2'
    expected=ARM_IMAGE_SHA512 if arm else IMAGE_SHA512
    artifacts=REPO/'test-results'/('native-vm-provision' if args.provision_only else 'native-vm')
    artifacts.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='cd058-image-',dir='/tmp') as folder:
        image=Path(folder)/name
        # Only an idempotent pinned-image download is retried; app actions never are.
        for attempt in range(3):
            try:
                subprocess.run(['/usr/bin/curl','--fail','--location','--silent','--show-error','--max-time','120',
                    '--output',str(image),'https://dl-cdn.alpinelinux.org/alpine/v3.22/releases/cloud/'+name],check=True,timeout=130)
                break
            except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
                if attempt == 2: raise RuntimeError('Pinned guest image download failed after three bounded attempts') from None
                image.unlink(missing_ok=True)
                print('Retrying the pinned guest image download after a transient network failure.',flush=True)
                time.sleep(3)
        with image.open('rb') as stream: actual=hashlib.file_digest(stream,'sha512').hexdigest()
        if actual != expected: raise RuntimeError('Official pinned VM image hash mismatch')
        command=['python3',str(REPO/'tests/lab/integration.py'),'--qemu-root',str(tools),'--image',str(image),
                 '--encrypted-agent','--artifacts',str(artifacts)]
        if args.provision_only: command.append('--provision-only')
        if not mac and not os.access('/dev/kvm',os.R_OK|os.W_OK): command.append('--tcg')
        subprocess.run(command,cwd=REPO,check=True,timeout=2400)


if __name__=='__main__': main()
