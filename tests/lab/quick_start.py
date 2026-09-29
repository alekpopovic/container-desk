#!/usr/bin/env python3
"""057 documented user controls on a fresh native profile in the explicit 049 VM."""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import select
import signal
import socket
import subprocess
import tempfile
import time
import urllib.request
from native_ssh import ELEMENT, application_binary, unused_port


def worker(args):
    manifest = json.loads(args.manifest.read_text())
    original_config = Path(manifest['config'])
    if not original_config.parent.name.startswith('containerdesk-049-'):
        raise RuntimeError('Requires an explicitly owned integration VM manifest')
    with tempfile.TemporaryDirectory(prefix='cd057-') as folder:
        root = Path(folder)
        agent = subprocess.Popen(['/usr/bin/ssh-agent', '-D', '-a', str(root/'agent')], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        driver = None
        try:
            env = {**os.environ, 'SSH_AUTH_SOCK': str(root/'agent'), 'PATH': '/usr/bin:/bin', 'XDG_DATA_HOME': str(root/'data')}
            for _ in range(100):
                if (root/'agent').exists(): break
                time.sleep(.05)
            subprocess.run(['/usr/bin/ssh-add', str(original_config.parent/'client')], env=env, check=True, capture_output=True, timeout=10)
            config = root/'config'
            # Agent must authenticate: IdentityFile contains only the generated public key.
            config.write_text(original_config.read_text().replace('IdentityAgent none', 'IdentityAgent '+str(root/'agent'))
                .replace('IdentityFile '+str(original_config.parent/'client')+'\n', 'IdentityFile '+str(original_config.parent/'client.pub')+'\n'))
            config.chmod(0o600)
            trust_before = hashlib.sha256((original_config.parent/'known').read_bytes()).hexdigest()
            config_before = hashlib.sha256(config.read_bytes()).hexdigest()
            for alias in ['direct-owned', 'private-owned']:
                rows = subprocess.check_output(['/usr/bin/ssh', '-F', str(config), '-T', '-n', '--', alias,
                    "exec 'docker' 'ps' '--all' '--quiet'"], env=env, timeout=15).decode().splitlines()
                assert len(rows) == 4
            port, native = unused_port(), unused_port()
            while native == port: native = unused_port()
            with (root/'driver.log').open('w') as log:
                driver = subprocess.Popen([str(args.tools_dir/'bin/tauri-driver'), '--port', str(port), '--native-port', str(native),
                    '--native-driver', str(args.tools_dir/'webkit/usr/bin/WebKitWebDriver')], env=env, stdout=log, stderr=log, start_new_session=True)
                def request(method, path, value=None):
                    data = None if value is None else json.dumps(value).encode()
                    req = urllib.request.Request(f'http://127.0.0.1:{port}'+path, data=data, method=method, headers={'Content-Type':'application/json'})
                    with urllib.request.urlopen(req, timeout=35) as response: return json.load(response)['value']
                for _ in range(100):
                    try:
                        with socket.create_connection(('127.0.0.1',port),timeout=.1): break
                    except OSError: time.sleep(.1)
                created = request('POST','/session',{'capabilities':{'alwaysMatch':{'browserName':'wry','tauri:options':{'application':str(application_binary())}}}})
                session = created['sessionId']
                def command(method,path,value=None): return request(method,'/session/'+session+path,value)
                def script(code,*values): return command('POST','/execute/sync',{'script':code,'args':list(values)})
                def wait(code):
                    end=time.monotonic()+30
                    while time.monotonic()<end:
                        if script(code): return
                        time.sleep(.1)
                    raise RuntimeError('Native quick-start condition timed out: '+code)
                def click(xpath):
                    node=command('POST','/element',{'using':'xpath','value':xpath})[ELEMENT]
                    script('arguments[0].scrollIntoView({block:"center",behavior:"instant"})',{ELEMENT:node})
                    command('POST',f'/element/{node}/click',{})
                def button(name): click('//button[normalize-space(.)="'+name+'"]')
                def route(name): click('//nav[@aria-label="Resources"]//a[normalize-space(.)="'+name+'"]')
                def fill(label,value):
                    node=command('POST','/element',{'using':'xpath','value':'//label[normalize-space(text())="'+label+'"]/input'})[ELEMENT]
                    command('POST',f'/element/{node}/clear',{})
                    command('POST',f'/element/{node}/value',{'text':value})
                command('POST','/timeouts',{'implicit':5000,'script':5000,'pageLoad':15000})
                wait('return document.body.innerText.includes("No hosts added")')
                route('Settings');button('Run diagnostics')
                wait('return document.body.innerText.includes("OpenSSH is available.")')
                wait('return document.body.innerText.includes("The inherited SSH agent socket is reachable. Loaded keys were not checked; a selected host may use a different IdentityAgent.")')
                routes=[]
                for alias in ['direct-owned','private-owned']:
                    route('Hosts');button('New host')
                    fill('Host SSH config path',str(config));button('Browse aliases');button('Use '+alias)
                    fill('Display name','Quick start '+alias);fill('Saved Docker executable','/usr/bin/docker')
                    button('Save host')
                    wait('return document.querySelector(".saved-host-details")?.innerText.includes('+json.dumps(alias)+')')
                    button('Connect saved host')
                    wait('return document.querySelector(".saved-host-details")?.innerText.includes("Ready · SSH session")')
                    details=script('return document.querySelector(".saved-host-details").innerText')
                    assert 'unix:///var/run/docker.sock' in details
                    if alias=='private-owned': assert 'jump-owned' in details
                    route('Containers')
                    wait('return document.querySelectorAll("[data-container-id]").length===4')
                    click('//tr[@data-container-id="'+manifest['liveId']+'"]//button')
                    wait('return document.querySelector(".inspect-detail")?.getAttribute("aria-busy")==="false"')
                    assert not script('return document.body.innerText.includes("SEEDED_049_ENV")')
                    button('Start logs')
                    wait('return document.querySelector(".live-logs")?.innerText.includes("CD049_LOG")')
                    button('Stop logs')
                    button('Enable management');button('Stop container')
                    wait('return !!document.querySelector(".mutation-confirmation")')
                    assert manifest['liveId'] in script('return document.querySelector(".mutation-confirmation").innerText')
                    button('Cancel action');button('Disable management')
                    (args.artifacts/(alias+'.png')).write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
                    route('Hosts');button('Disconnect saved host')
                    wait('return !document.body.innerText.includes("Ready · SSH session")')
                    routes.append(alias)
                command('DELETE','')
                session=None
            settings=root/'data/dev.containerdesk.app/preferences/settings.json'
            saved=json.loads(settings.read_text())
            assert len(saved['hosts'])==2 and saved['schemaVersion']==3
            assert b'CD049_LOG' not in settings.read_bytes()
            assert hashlib.sha256(config.read_bytes()).hexdigest()==config_before
            assert hashlib.sha256((original_config.parent/'known').read_bytes()).hexdigest()==trust_before
            result={'passed':True,'freshProfile':True,'nativeAutomationSha256':hashlib.sha256(application_binary().read_bytes()).hexdigest(),
                'routes':routes,'realAgentAuthentication':True,'identityFilePublicOnly':True,'nativeDiagnostics':True,
                'browseSaveConnect':True,'realDockerRows':4,'maskedInspect':True,'realLogFollowAndStop':True,
                'managementConfirmationCancelled':True,'savedHosts':2,'trustAndConfigUnchanged':True,
                'browserMocksUsed':False,'scope':'Native opt-in test artifact, dedicated Linux VM, user controls; installer proof remains 053/054'}
        finally:
            if driver is not None:
                if driver.poll() is None: os.killpg(driver.pid,signal.SIGTERM)
                try: driver.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    os.killpg(driver.pid,signal.SIGKILL);driver.wait(timeout=5)
            agent.terminate();agent.wait(timeout=10)
    result['ownedAgentAndProfileRemoved']=not root.exists() and agent.poll() is not None
    (args.artifacts/'quick-start.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tools-dir',type=Path,required=True)
    parser.add_argument('--manifest',type=Path,required=True)
    parser.add_argument('--artifacts',type=Path,required=True)
    parser.add_argument('--worker',action='store_true')
    args=parser.parse_args()
    args.tools_dir=args.tools_dir.resolve();args.artifacts=args.artifacts.resolve()
    args.artifacts.mkdir(parents=True,exist_ok=True)
    if args.worker: return worker(args)
    with tempfile.TemporaryDirectory(prefix='cd057-display-') as folder:
        read,write=os.pipe()
        display=subprocess.Popen([str(args.tools_dir/'xvfb/usr/bin/Xvfb'),'-displayfd',str(write),'-screen','0','1440x1000x24','-nolisten','tcp'],pass_fds=(write,),stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        os.close(write)
        try:
            assert select.select([read],[],[],10)[0]
            number=os.read(read,32).decode().strip();assert number.isdigit()
            env=os.environ.copy()
            for key in ['WAYLAND_DISPLAY','LD_LIBRARY_PATH','LD_PRELOAD','GTK_PATH','GIO_MODULE_DIR','DBUS_SESSION_BUS_ADDRESS','AT_SPI_BUS_ADDRESS']:env.pop(key,None)
            env.update(DISPLAY=':'+number,GDK_BACKEND='x11',LIBGL_ALWAYS_SOFTWARE='1')
            for key in ['XDG_CONFIG_HOME','XDG_CACHE_HOME','XDG_RUNTIME_DIR']:
                path=Path(folder)/key;path.mkdir(mode=0o700);env[key]=str(path)
            subprocess.run(['/usr/bin/dbus-run-session','--','python3',str(Path(__file__).resolve()),'--worker',
                '--tools-dir',str(args.tools_dir),'--manifest',str(args.manifest.resolve()),'--artifacts',str(args.artifacts)],env=env,check=True,timeout=240)
        finally:
            os.close(read);display.terminate();display.wait(timeout=10)


if __name__=='__main__': main()
