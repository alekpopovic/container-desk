#!/usr/bin/env python3
"""Run the 045 release pressure journey on a fresh owned X11 display."""
import argparse
import os
from pathlib import Path
import select
import subprocess
import tempfile
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--tools-dir',type=Path,required=True)
parser.add_argument('--artifacts',type=Path,required=True)
args=parser.parse_args();tools=args.tools_dir.resolve()
with tempfile.TemporaryDirectory(prefix='containerdesk-045-display-') as area:
    read,write=os.pipe()
    with open(Path(area)/'xvfb.log','w') as log:
        server=subprocess.Popen([str(tools/'xvfb/usr/bin/Xvfb'),'-displayfd',str(write),'-screen','0','1440x1000x24','-nolisten','tcp'],pass_fds=(write,),stdout=log,stderr=log)
        os.close(write)
        try:
            assert select.select([read],[],[],10)[0]
            display=os.read(read,32).decode().strip();assert display.isdigit()
            env=os.environ.copy()
            for key in ['WAYLAND_DISPLAY','LD_LIBRARY_PATH','LD_PRELOAD','GTK_PATH','GIO_MODULE_DIR','DBUS_SESSION_BUS_ADDRESS','AT_SPI_BUS_ADDRESS']:env.pop(key,None)
            env.update(DISPLAY=':'+display,GDK_BACKEND='x11',LIBGL_ALWAYS_SOFTWARE='1',XDG_DATA_HOME=area+'/data',XDG_CONFIG_HOME=area+'/config',XDG_CACHE_HOME=area+'/cache',XDG_RUNTIME_DIR=area+'/runtime')
            Path(env['XDG_RUNTIME_DIR']).mkdir(mode=0o700)
            result=subprocess.run(['/usr/bin/dbus-run-session','--','python3','tests/lab/logs.py','--sshd-root',str(tools/'openssh'),'--stream','--jump','--pressure','--native-driver',str(tools/'bin/tauri-driver'),'--webkit-driver',str(tools/'webkit/usr/bin/WebKitWebDriver'),'--focus-xdotool',str(tools/'xdotool/usr/bin/xdotool'),'--native-artifacts',str(args.artifacts.resolve())],env=env,timeout=300)
            raise SystemExit(result.returncode)
        finally:
            os.close(read);server.terminate();server.wait(timeout=10)
