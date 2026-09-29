"""042 native Linux desktop/agent/path acceptance, only within the supplied owned lab."""
import base64
import ctypes
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import time
import urllib.request
from native_ssh import unused_port, ELEMENT, REPO

XDOTOOL = Path('/tmp/containerdesk-025-tools/xdotool/usr/bin/xdotool')


def close_window(window):
    class Message(ctypes.Union):
        _fields_ = [('bytes', ctypes.c_char * 20), ('longs', ctypes.c_long * 5)]
    class Client(ctypes.Structure):
        _fields_ = [('type', ctypes.c_int), ('serial', ctypes.c_ulong), ('send_event', ctypes.c_int),
                    ('display', ctypes.c_void_p), ('window', ctypes.c_ulong), ('message_type', ctypes.c_ulong),
                    ('format', ctypes.c_int), ('data', Message)]
    class Event(ctypes.Union):
        _fields_ = [('client', Client), ('pad', ctypes.c_long * 24)]
    x = ctypes.CDLL('libX11.so.6')
    x.XOpenDisplay.restype = ctypes.c_void_p
    x.XInternAtom.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int]
    x.XInternAtom.restype = ctypes.c_ulong
    x.XSendEvent.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int, ctypes.c_long, ctypes.POINTER(Event)]
    x.XFlush.argtypes = [ctypes.c_void_p]
    x.XCloseDisplay.argtypes = [ctypes.c_void_p]
    display = x.XOpenDisplay(None)
    assert display
    try:
        event = Event()
        event.client = Client(33, 0, 1, display, int(window), x.XInternAtom(display, b'WM_PROTOCOLS', 0), 32, Message())
        event.client.data.longs[0] = x.XInternAtom(display, b'WM_DELETE_WINDOW', 0)
        assert x.XSendEvent(display, int(window), 0, 0, ctypes.byref(event))
        x.XFlush(display)
    finally:
        x.XCloseDisplay(display)


def verify(root, tauri_driver, webkit_driver, config, engine, artifacts=None, *_unused):
    artifacts = artifacts or root / 'launch-artifacts'
    artifacts.mkdir(parents=True, exist_ok=True)
    owned_home = root / 'Korisnik Željko 日本語'
    owned_home.mkdir(mode=0o700)
    config_dir = owned_home / '.ssh'
    config_dir.mkdir(mode=0o700)
    selected_config = config_dir / 'izabrani config'
    selected_config.write_bytes(config.read_bytes())
    selected_config.chmod(0o600)
    ssh = owned_home / 'OpenSSH klijent'
    ssh.symlink_to('/usr/bin/ssh')
    app = owned_home / 'ContainerDesk aplikacija'
    app.symlink_to(REPO / 'src-tauri/target/release/containerdesk')
    profile_marker = root / 'profile-executed'
    for name in ('.profile', '.bashrc', '.zprofile', '.zshrc'):
        (owned_home / name).write_text("/usr/bin/touch '" + str(profile_marker) + "'\n")
    stale = root / 'closed-agent.sock'
    with socket.socket(socket.AF_UNIX) as agent:
        agent.bind(str(stale))
    data = owned_home / 'Podaci aplikacije'
    # A deliberately minimal launch environment, not a modified copy of the developer shell.
    env = {'HOME': str(owned_home), 'XDG_DATA_HOME': str(data), 'PATH': '/nonexistent',
           'LANG': 'C.UTF-8', 'DISPLAY': os.environ['DISPLAY'], 'GDK_BACKEND': 'x11',
           'LIBGL_ALWAYS_SOFTWARE': '1', 'SSH_AUTH_SOCK': str(stale)}
    if 'XAUTHORITY' in os.environ:
        env['XAUTHORITY'] = os.environ['XAUTHORITY']

    def journey(environment, denied=False):
        port, native_port = unused_port(), unused_port()
        while port == native_port: native_port = unused_port()
        session = None
        with (root / ('denied-driver.log' if denied else 'launch-driver.log')).open('w') as log:
            driver = subprocess.Popen([str(tauri_driver), '--port', str(port), '--native-port', str(native_port), '--native-driver', str(webkit_driver)], env=environment, stdout=log, stderr=log, start_new_session=True)
            def request(method, path, value=None):
                req = urllib.request.Request(f'http://127.0.0.1:{port}{path}', data=None if value is None else json.dumps(value).encode(), method=method, headers={'Content-Type':'application/json'})
                with urllib.request.urlopen(req, timeout=35) as response: return json.load(response)['value']
            def command(method, path, value=None): return request(method, f'/session/{session}{path}', value)
            def script(code): return command('POST', '/execute/sync', {'script':code, 'args':[]})
            def wait(condition):
                end = time.monotonic()+25
                while time.monotonic()<end:
                    if script(condition): return
                    time.sleep(.1)
                raise AssertionError('Native launch condition failed: '+condition)
            def element(xpath): return command('POST','/element',{'using':'xpath','value':xpath})[ELEMENT]
            def click(xpath):
                node=element(xpath)
                command('POST','/execute/sync',{'script':'arguments[0].scrollIntoView({block:"center",behavior:"instant"})','args':[{ELEMENT:node}]})
                time.sleep(.2)
                command('POST',f'/element/{node}/click',{})
            def button(name):
                wait('return Array.from(document.querySelectorAll("button")).some(b=>b.textContent.trim()==='+json.dumps(name)+' && !b.disabled)')
                click('//button[normalize-space(.)='+json.dumps(name)+']')
            def fill(label,value):
                node=element('//label[normalize-space(text())='+json.dumps(label)+']/input')
                command('POST',f'/element/{node}/clear',{})
                command('POST',f'/element/{node}/value',{'text':value})
            def settings(): click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Settings"]')
            def screenshot(name): (artifacts/name).write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
            try:
                for _ in range(100):
                    try: request('GET','/status'); break
                    except OSError: time.sleep(.1)
                session=request('POST','/session',{'capabilities':{'alwaysMatch':{'browserName':'wry','tauri:options':{'application':str(app)}}}})['sessionId']
                command('POST','/timeouts',{'implicit':5000,'script':5000,'pageLoad':15000})
                if denied:
                    wait('return document.body.innerText.includes("Local settings are unavailable")')
                    settings()
                    assert script('return document.querySelector("#ssh-executable").disabled')
                    screenshot('native-denied-storage.png')
                    print('PASS native permission-denied settings lock: explicit storage notice, saving disabled, original retained, no fallback settings writes.',flush=True)
                    return
                settings()
                node=element('//*[@id="ssh-executable"]')
                command('POST',f'/element/{node}/value',{'text':str(ssh)})
                button('Validate and save SSH path')
                wait('return document.body.innerText.includes("SSH executable preference saved.")')
                button('Run diagnostics')
                wait('return document.body.innerText.includes("inherited SSH agent socket could not be reached")')
                assert script('return document.querySelector(".diagnostic-result").innerText').find(str(ssh))>=0
                screenshot('native-desktop-diagnostics.png')
                click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Hosts"]')
                for alias, success in [('direct-encrypted-unloaded',False),('direct-encrypted-stale-agent',False),('direct-encrypted-loaded',True),('via-jump-encrypted-loaded',True)]:
                    button('New host')
                    fill('Host SSH config path',str(selected_config))
                    fill('Host SSH alias',alias)
                    fill('Display name','Desktop '+alias)
                    fill('Saved Docker executable','/usr/bin/docker')
                    button('Save host')
                    button('Connect saved host')
                    expected='Ready · SSH session' if success else 'Connection error · SSH session'
                    wait('return document.querySelector(".saved-host-details")?.innerText.includes('+json.dumps(expected)+')')
                    details=script('return document.querySelector(".saved-host-details").innerText')
                    if success:
                        assert engine['engineId'] in details and engine['engineVersion'] in details
                    else: assert 'passphrase-protected key' in details and 'Native dependencies' in details
                    screenshot('native-'+alias+'.png')
                    button('Disconnect saved host')
                    wait('return document.querySelector(".saved-host-details")?.innerText.includes("Disconnected · SSH session")')
                    print('PASS native minimal desktop environment: '+alias+'; configured absolute SSH, Unicode config/home/data, real Engine identity when authenticated.',flush=True)
            finally:
                if session:
                    try: request('DELETE',f'/session/{session}')
                    except OSError: pass
                if driver.poll() is None:
                    os.killpg(driver.pid,signal.SIGTERM)
                    driver.wait(timeout=10)
    journey(env)
    saved=data/'dev.containerdesk.app/preferences/settings.json'
    preferences=json.loads(saved.read_text())
    assert preferences['sshExecutableOverride']==str(ssh) and len(preferences['hosts'])==4
    assert saved.stat().st_mode & 0o777 == 0o600
    assert saved.parent.stat().st_mode & 0o777 == 0o700
    assert not profile_marker.exists()
    # A real Freedesktop launcher invokes the same release app, without WebDriver or a shell.
    desktop=root/'dev.containerdesk.launch-check.desktop'
    desktop.write_text('[Desktop Entry]\nType=Application\nName=ContainerDesk Launch Check\nTerminal=false\nExec="'+str(app)+'"\n')
    subprocess.run(['/usr/bin/desktop-file-validate',str(desktop)],check=True,timeout=5)
    subprocess.run(['/usr/bin/gio','launch',str(desktop)],env=env,check=True,timeout=10)
    pid=None
    try:
        end=time.monotonic()+15
        while time.monotonic()<end:
            result=subprocess.run([str(XDOTOOL),'search','--onlyvisible','--name','^ContainerDesk$'],env=env,capture_output=True,text=True,timeout=3)
            owned=[]
            for window in result.stdout.split():
                candidate=subprocess.run([str(XDOTOOL),'getwindowpid',window],env=env,capture_output=True,text=True,check=True,timeout=3).stdout.strip()
                try:
                    environ=Path('/proc',candidate,'environ').read_bytes().split(b'\0')
                    if ('XDG_DATA_HOME='+str(data)).encode() in environ: owned.append((window,int(candidate),environ))
                except OSError: pass
            if owned: break
            time.sleep(.1)
        assert len(owned)==1
        window,pid,environ=owned[0]
        assert b'PATH=/nonexistent' in environ
        assert ('HOME='+str(owned_home)).encode() in environ
        time.sleep(2)
        subprocess.run(['/usr/bin/import','-window',window,str(artifacts/'native-desktop-launcher.png')],env=env,check=True,timeout=10)
        close_window(window)
        end=time.monotonic()+12
        while Path('/proc',str(pid)).exists() and time.monotonic()<end:
            stat=Path('/proc',str(pid),'stat').read_text()
            if stat[stat.rfind(')')+2:].split()[0]=='Z': break
            time.sleep(.1)
        else: assert not Path('/proc',str(pid)).exists(), 'Desktop-launched app did not exit'
        print('PASS actual gio desktop launcher: release window with minimal environment and quoted Unicode executable, normal window close.',flush=True)
    finally:
        if pid and Path('/proc',str(pid)).exists():
            stat=Path('/proc',str(pid),'stat').read_text()
            if stat[stat.rfind(')')+2:].split()[0]!='Z': os.kill(pid,signal.SIGTERM)
    denied=owned_home/'Read only data'
    denied_settings=denied/'dev.containerdesk.app/preferences'
    denied_settings.mkdir(parents=True,mode=0o700)
    denied_lock=denied_settings/'settings.lock'
    denied_lock.write_text('owned denied lock marker')
    denied_lock.chmod(0o400)
    try: journey({**env,'XDG_DATA_HOME':str(denied)},denied=True)
    finally: denied_lock.chmod(0o600)
    assert denied_lock.read_text()=='owned denied lock marker'
    assert not (denied_settings/'settings.json').exists() and not profile_marker.exists()
    assert all(b'BEGIN OPENSSH PRIVATE KEY' not in p.read_bytes() for p in data.rglob('*') if p.is_file())
    print('PASS native launch data: private file permissions, persisted absolute references, no profile scripts or private keys in app storage.',flush=True)
