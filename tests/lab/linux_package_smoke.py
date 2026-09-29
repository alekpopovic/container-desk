#!/usr/bin/env python3
"""Run only inside the owned disposable desktop VM; ordinary installed executable."""
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import time
import pyatspi

OUT = Path('/opt/containerdesk/results')


def run(argv, **kw):
    return subprocess.run(argv,check=True,capture_output=True,text=True,timeout=kw.pop('timeout',10),**kw)


def nodes(root, depth=0):
    if depth>30: return
    yield root
    try:
        for index in range(min(root.childCount,2000)):
            child=root.getChildAtIndex(index)
            if child: yield from nodes(child,depth+1)
    except Exception: return


def find(name, timeout=20):
    deadline=time.monotonic()+timeout
    while time.monotonic()<deadline:
        for item in nodes(pyatspi.Registry.getDesktop(0)):
            try:
                if item.name==name: return item
                try:
                    text=item.queryText()
                    if text.getText(0,text.characterCount)==name:return item
                except Exception:pass
            except Exception: pass
        time.sleep(.2)
    raise AssertionError('Accessible element not found: '+name)


def click(name):
    item=find(name)
    action=item.queryAction()
    assert action.nActions>0 and action.doAction(0), 'Native action failed: '+name


def main():
    assert os.geteuid()!=0 and Path.home()==Path('/home/lab'), 'Owned VM user only'
    result={'passed':False,'launch':'gtk-launch installed desktop entry','automationFeature':False}
    owned=[]
    app_pid=None
    folder=Path.home()/'.ssh'; folder.mkdir(mode=0o700,exist_ok=True)
    config=folder/'config'
    assert not config.exists(), 'Refuse to overwrite existing SSH configuration'
    config.write_text('Include package-hosts\nHost package-direct\n HostName 127.0.0.1\n User owned\n Port 65000\n')
    include=folder/'package-hosts'
    assert not include.exists()
    include.write_text('Host package-via-jump\n HostName 127.0.0.2\n ProxyJump package-direct\n')
    config.chmod(0o600); include.chmod(0o600)
    before={p:hashlib.sha256(p.read_bytes()).hexdigest() for p in (config,include)}
    try:
        for argv in (['openbox'],['xfce4-panel'],['xfdesktop']):
            log=(OUT/(argv[0]+'.log')).open('w')
            owned.append(subprocess.Popen(argv,stdout=log,stderr=log,start_new_session=True));log.close()
        time.sleep(2)
        desktop=next(Path('/usr/share/applications').glob('*containerdesk*.desktop'),None)
        if desktop is None: desktop=next(Path('/usr/share/applications').glob('*ContainerDesk*.desktop'))
        run(['desktop-file-validate',str(desktop)])
        result['desktopEntry']=desktop.read_text()
        icon=Path('/usr/share/icons/hicolor/512x512/apps/containerdesk.png')
        assert icon.is_file() and icon.read_bytes().startswith(b'\x89PNG\r\n\x1a\n')
        result['installedIconSha256']=hashlib.sha256(icon.read_bytes()).hexdigest()
        result['installedBinarySha256']=hashlib.sha256(Path('/usr/bin/containerdesk').read_bytes()).hexdigest()
        assert os.access('/usr/bin/containerdesk',os.X_OK)
        run(['gdbus','call','--session','--dest','org.a11y.Bus','--object-path','/org/a11y/bus','--method','org.freedesktop.DBus.Properties.Set','org.a11y.Status','ScreenReaderEnabled','<true>'])
        with (OUT/'application.log').open('w') as app_log:
            subprocess.run(['gtk-launch',desktop.stem],check=True,stdout=app_log,stderr=app_log,timeout=10)
        deadline=time.monotonic()+30
        while time.monotonic()<deadline:
            search=subprocess.run(['xdotool','search','--onlyvisible','--name','^ContainerDesk$'],capture_output=True,text=True,timeout=5)
            if search.returncode==0:
                window=search.stdout.split()[0]; app_pid=int(run(['xdotool','getwindowpid',window]).stdout);break
            time.sleep(.2)
        assert app_pid and Path(f'/proc/{app_pid}/exe').resolve()==Path('/usr/bin/containerdesk')
        result['realWindowOpened']=True
        click('Settings')
        click('Browse host candidates')
        find('Select package-direct');find('Select package-via-jump')
        result['defaultSshConfigAndIncludeDiscovered']=True
        click('Select package-via-jump')
        click('Resolve selected alias')
        find('/usr/bin/ssh');find('127.0.0.2')
        result['nativeOpenSshResolution']=True
        run(['import','-window',window,str(OUT/'installed-discovery.png')])
        result['sshFilesUnchanged']=all(hashlib.sha256(p.read_bytes()).hexdigest()==v for p,v in before.items())
        assert result['sshFilesUnchanged']
        run(['xdotool','windowactivate','--sync',window,'key','--clearmodifiers','alt+F4'])
        deadline=time.monotonic()+15
        while Path(f'/proc/{app_pid}').exists() and time.monotonic()<deadline:time.sleep(.1)
        assert not Path(f'/proc/{app_pid}').exists(), 'Application did not close normally'
        result['normalWindowClose']=True
        result['passed']=True
    except Exception as error:
        result['error']={'type':type(error).__name__,'message':str(error)}
        raise
    finally:
        if not result['passed']:
            labels=[]
            for item in nodes(pyatspi.Registry.getDesktop(0)):
                try:
                    if item.name:labels.append({'role':item.getRoleName(),'name':item.name})
                except Exception:pass
            (OUT/'failure-accessibility.json').write_text(json.dumps(labels,indent=2)+'\n')
            subprocess.run(['import','-window','root',str(OUT/'failure-screen.png')],timeout=10,capture_output=True)
        if app_pid and Path(f'/proc/{app_pid}/exe').resolve()==Path('/usr/bin/containerdesk'):
            try: os.kill(app_pid,signal.SIGTERM)
            except ProcessLookupError:pass
        for child in reversed(owned):
            if child.poll() is None:os.killpg(child.pid,signal.SIGTERM)
            try:child.wait(timeout=5)
            except subprocess.TimeoutExpired:os.killpg(child.pid,signal.SIGKILL);child.wait(timeout=5)
        (OUT/'desktop.json').write_text(json.dumps(result,indent=2)+'\n')
    print('PASS installed ordinary package: desktop entry, SSH config/include discovery, native ssh resolution, unchanged trust/config and graceful close.')


if __name__=='__main__': main()
