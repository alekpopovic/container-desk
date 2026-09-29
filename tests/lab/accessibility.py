#!/usr/bin/env python3
"""Owned X11 + isolated D-Bus/Orca wrapper for the 044 release keyboard journey."""
import argparse
import os
from pathlib import Path
import select
import subprocess
import tempfile
import time


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tools-dir', type=Path, required=True, help='Extracted tools containing xvfb, openssh, xdotool, webkit and bin/tauri-driver')
    parser.add_argument('--artifacts', type=Path, required=True)
    parser.add_argument('--owned-session', action='store_true', help=argparse.SUPPRESS)
    args=parser.parse_args()
    tools=args.tools_dir.resolve(); artifacts=args.artifacts.resolve()
    if args.owned_session:
        assert os.environ.get('CONTAINERDESK_ORCA_LOG') and os.environ.get('DBUS_SESSION_BUS_ADDRESS')
        env=os.environ.copy()
        with open(Path(env['CONTAINERDESK_ORCA_LOG']).with_name('orca-console.log'),'w') as output:
            orca=subprocess.Popen(['/usr/bin/orca','--debug-file',env['CONTAINERDESK_ORCA_LOG']],env=env,stdout=output,stderr=output)
            try:
                time.sleep(3)
                assert orca.poll() is None, 'Orca could not start on the owned session'
                result=subprocess.run(['python3','tests/lab/logs.py','--sshd-root',str(tools/'openssh'),'--stream','--jump','--keyboard','--native-driver',str(tools/'bin/tauri-driver'),'--webkit-driver',str(tools/'webkit/usr/bin/WebKitWebDriver'),'--focus-xdotool',str(tools/'xdotool/usr/bin/xdotool'),'--native-artifacts',str(artifacts)],env=env,timeout=180)
                return result.returncode
            finally:
                if orca.poll() is None: orca.terminate(); orca.wait(timeout=10)
    with tempfile.TemporaryDirectory(prefix='containerdesk-044-a11y-') as area:
        root=Path(area); read,write=os.pipe()
        with (root/'xvfb.log').open('w') as log:
            server=subprocess.Popen([str(tools/'xvfb/usr/bin/Xvfb'),'-displayfd',str(write),'-screen','0','1440x1000x24','-nolisten','tcp'],pass_fds=(write,),stdout=log,stderr=log)
            os.close(write)
            try:
                assert select.select([read],[],[],10)[0], 'Xvfb startup timed out'
                display=os.read(read,32).decode().strip(); assert display.isdigit()
                env=os.environ.copy()
                for key in ['WAYLAND_DISPLAY','LD_LIBRARY_PATH','LD_PRELOAD','GTK_PATH','GIO_MODULE_DIR','DBUS_SESSION_BUS_ADDRESS','AT_SPI_BUS_ADDRESS']:
                    env.pop(key,None)
                env.update(DISPLAY=':'+display,GDK_BACKEND='x11',LIBGL_ALWAYS_SOFTWARE='1',GSETTINGS_BACKEND='memory',XDG_DATA_HOME=area+'/data',XDG_CONFIG_HOME=area+'/config',XDG_CACHE_HOME=area+'/cache',XDG_RUNTIME_DIR=area+'/runtime',CONTAINERDESK_ORCA_LOG=area+'/orca-debug.log',GTK_A11Y='1')
                Path(env['XDG_RUNTIME_DIR']).mkdir(mode=0o700)
                return subprocess.run(['/usr/bin/dbus-run-session','--','python3',str(Path(__file__).resolve()),'--tools-dir',str(tools),'--artifacts',str(artifacts),'--owned-session'],env=env,timeout=210).returncode
            finally:
                os.close(read);server.terminate();server.wait(timeout=10)

if __name__=='__main__': raise SystemExit(main())
