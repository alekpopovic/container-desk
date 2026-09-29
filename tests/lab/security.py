#!/usr/bin/env python3
"""047 actual release CSP/navigation/IPC checks; owned loopback page, no remote host operations."""
import argparse
import base64
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import select
import signal
import subprocess
import tempfile
import threading
import time
import urllib.request
from native_ssh import REPO, ELEMENT, unused_port, application_binary
from native_launch import close_window


def run(root,tools,artifacts,baseline):
    artifacts.mkdir(parents=True,exist_ok=True)
    env=os.environ.copy()
    for key in ['LD_LIBRARY_PATH','LD_PRELOAD','GTK_PATH','GIO_MODULE_DIR','WEBKIT_INSPECTOR_SERVER','WEBKIT_INSPECTOR_HTTP_SERVER','TAURI_AUTOMATION','TAURI_WEBVIEW_AUTOMATION','SSH_AUTH_SOCK']:env.pop(key,None)
    env.update(XDG_DATA_HOME=str(root/'data'),PATH='/nonexistent',GDK_BACKEND='x11',LIBGL_ALWAYS_SOFTWARE='1')
    binary=REPO/'src-tauri/target/release/containerdesk'
    xdotool=tools/'xdotool/usr/bin/xdotool'
    # Default binary, with no WebDriver/automation environment. Inspect actual TCP listeners.
    with (root/'standalone.log').open('w') as log:
        app=subprocess.Popen([str(binary)],env=env,stdout=log,stderr=log)
        try:
            deadline=time.monotonic()+15;window=None
            while time.monotonic()<deadline:
                found=subprocess.run([str(xdotool),'search','--onlyvisible','--pid',str(app.pid)],env=env,capture_output=True,text=True,timeout=5).stdout.split()
                if found:window=found[0];break
                assert app.poll() is None;time.sleep(.1)
            assert window,'Standalone app failed to open'
            time.sleep(1)
            owned={app.pid}
            while True:
                previous=owned.copy()
                for entry in Path('/proc').iterdir():
                    if not entry.name.isdigit():continue
                    try:
                        ppid=int(next(row.split()[1] for row in (entry/'status').read_text().splitlines() if row.startswith('PPid:')))
                        if ppid in owned:owned.add(int(entry.name))
                    except (OSError,StopIteration,ValueError):pass
                if owned==previous:break
            listeners=set()
            for path in ['/proc/net/tcp','/proc/net/tcp6']:
                for row in Path(path).read_text().splitlines()[1:]:
                    columns=row.split()
                    if columns[3]=='0A':listeners.add(columns[9])
            matched=[]
            for pid in owned:
                for fd in Path('/proc',str(pid),'fd').iterdir():
                    try:
                        target=os.readlink(fd)
                        if target.startswith('socket:[') and target[8:-1] in listeners:matched.append((pid,fd.name))
                    except OSError:pass
            assert not matched,'Normal release app owns a TCP listening endpoint'
            close_window(window);assert app.wait(timeout=15)==0
        finally:
            if app.poll() is None:app.terminate();app.wait(timeout=10)
    requested=[]
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            requested.append(self.path);self.send_response(200);self.send_header('Content-Type','text/html');self.end_headers();self.wfile.write(b'<!doctype html><title>Owned 047 remote UI probe</title><h1>OWNED_047_REMOTE_UI</h1>')
        def log_message(self,*args):pass
    server=ThreadingHTTPServer(('127.0.0.1',0),Handler);thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
    origin=f'http://127.0.0.1:{server.server_port}'
    port,native_port=unused_port(),unused_port()
    while port==native_port:native_port=unused_port()
    session=None
    with (root/'driver.log').open('w') as log:
        driver=subprocess.Popen([str(tools/'bin/tauri-driver'),'--port',str(port),'--native-port',str(native_port),'--native-driver',str(tools/'webkit/usr/bin/WebKitWebDriver')],env=env,stdout=log,stderr=log,start_new_session=True)
        def request(method,path,data=None):
            req=urllib.request.Request(f'http://127.0.0.1:{port}{path}',data=None if data is None else json.dumps(data).encode(),method=method,headers={'Content-Type':'application/json'})
            try:
                with urllib.request.urlopen(req,timeout=25) as response:return json.load(response)['value']
            except urllib.error.HTTPError as error:
                raise AssertionError('Native WebDriver '+method+' '+path+': '+error.read().decode()) from error
        def command(method,path,data=None):return request(method,f'/session/{session}{path}',data)
        def script(code,*args):return command('POST','/execute/sync',{'script':code,'args':list(args)})
        def wait(code):
            deadline=time.monotonic()+15
            while time.monotonic()<deadline:
                if script(code):return
                time.sleep(.1)
            raise AssertionError('Native security condition failed: '+code)
        def invoke(name,args):return command('POST','/execute/async',{'script':'const done=arguments[arguments.length-1];window.__TAURI_INTERNALS__.invoke(arguments[0],arguments[1]).then(value=>done({ok:true,value}),error=>done({ok:false,error}));','args':[name,args]})
        def click(xpath):
            node=command('POST','/element',{'using':'xpath','value':xpath})[ELEMENT]
            script('arguments[0].scrollIntoView({block:"center",behavior:"instant"})',{ELEMENT:node});time.sleep(.15)
            command('POST',f'/element/{node}/click',{})
        def fill(label,value):
            node=command('POST','/element',{'using':'xpath','value':f'//label[normalize-space(text())="{label}"]/input'})[ELEMENT]
            command('POST',f'/element/{node}/clear',{});command('POST',f'/element/{node}/value',{'text':value})
        try:
            for _ in range(100):
                try:request('GET','/status');break
                except OSError:time.sleep(.1)
            session=request('POST','/session',{'capabilities':{'alwaysMatch':{'browserName':'wry','tauri:options':{'application':str(application_binary())}}}})['sessionId']
            command('POST','/timeouts',{'implicit':5000,'script':10000,'pageLoad':15000})
            wait('return document.querySelectorAll("nav[aria-label=Resources] a").length===7')
            original=script('return location.href')
            if baseline:
                script('location.href=arguments[0]',origin+'/navigate')
                wait('return document.body.textContent.includes("OWNED_047_REMOTE_UI")')
                assert '/navigate' in requested
                (artifacts/'baseline.json').write_text(json.dumps({'baselineBinary':'046','standaloneTcpListeners':0,'remoteUiLoaded':True,'ownedHttpRequests':requested},indent=2)+'\n')
                print('REPRODUCED: 046 release main window loaded the owned external UI URL despite restricted subresource CSP.',flush=True)
                return
            results={}
            # Actual IPC validation, bypassing frontend controls. Nothing connects to an SSH host.
            for alias in ['-oProxyCommand=touch','host name','host;touch','host\nnext','$(id)','x'*257]:
                value=invoke('select_ssh_alias',{'request':{'alias':alias,'configPath':str(root/'owned-config')}})
                assert not value['ok'] and value['error']['code']=='invalid_alias';results['invalidAliases']=results.get('invalidAliases',0)+1
            value=invoke('select_ssh_alias',{'request':{'alias':'owned','configPath':'relative/config'}})
            assert not value['ok'];results['relativeConfigRejected']=True
            scope={'selection':{'hostId':'h_'+'1'*32,'selectionGeneration':1},'sessionId':'s_'+'2'*32,'sessionGeneration':1,'daemonId':'synthetic-daemon'}
            for name,payload in [('set_management',{'scope':scope,'enabled':True}),('prepare_confirmation',{'scope':scope,'operation':{'category':'mutation','spec':{'containerIds':['a'*64],'operation':'stop','timeoutSeconds':1}}}),('inspect_container',{'scope':scope,'containerId':'a'*64,'revealSensitive':False})]:
                value=invoke(name,{'request':payload});assert not value['ok'];results[name]='denied'
            for name in ['plugin:shell|execute','plugin:fs|read_text_file','plugin:webview|create_webview_window']:
                assert not invoke(name,{})['ok'];results[name]='denied'
            assert not invoke('set_management',{'request':{'scope':{**scope,'sessionId':'../../host'},'enabled':True}})['ok']
            results['invalidSessionIdRejected']=True
            secret='SEEDED_047_PRIVATE';config=root/'owned-config';config.write_text('# '+secret+'\n')
            hostile='<img src=x onerror=alert(1)> '+secret
            click('//button[normalize-space(.)="Add host"]');click('//button[normalize-space(.)="New host"]')
            fill('Host SSH config path',str(config));fill('Host SSH alias','security-owned');fill('Display name',hostile)
            click('//button[normalize-space(.)="Save host"]')
            wait('return document.querySelector(".saved-host-details")?.textContent.includes("security-owned")')
            assert script('return document.body.textContent.includes(arguments[0])',hostile)
            assert script('return document.querySelectorAll("img[onerror],script[src=x]").length===0')
            preview=invoke('prepare_support_report',{});assert preview['ok'];report=preview['value']['report']
            assert all(value not in report for value in [secret,hostile,'security-owned',str(root)])
            (artifacts/'reviewed-support.json').write_text(report+'\n')
            results['hostileHostTextInert']=True;results['seededSecretsAbsentFromDefaultReport']=True
            # Local app routes remain usable. Owned external navigation/popups/forms/frames are refused.
            click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Settings"]')
            wait('return location.hash==="#/settings"')
            expected=script('return location.href')
            script('window.__probe=0;const node=document.createElement("script");node.textContent="window.__probe=1";document.head.append(node);node.remove()')
            assert script('return window.__probe===0');results['inlineScriptBlocked']=True
            script('const frame=document.createElement("iframe");frame.src=arguments[0]+"/frame";document.body.append(frame);const form=document.createElement("form");form.action=arguments[0]+"/form";document.body.append(form);form.submit();window.open(arguments[0]+"/popup","_blank");location.href=arguments[0]+"/navigate";',origin)
            time.sleep(1)
            assert script('return location.href')==expected and not requested
            assert len(command('GET','/window/handles'))==1
            for url in ['data:text/html,OWNED_047_DATA','file:///etc/hosts','tauri://localhost/../../etc/passwd']:
                script('location.href=arguments[0]',url);time.sleep(.2);assert script('return location.href')==expected
            results.update(externalNavigationBlocked=True,popupsBlocked=True,formsAndFramesBlocked=True,nonAppSchemesAndPathsBlocked=True,ownedHttpRequests=0,standaloneTcpListeners=0,originalOrigin=original)
            (artifacts/'native-security.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
            (artifacts/'native-security.json').write_text(json.dumps(results,indent=2)+'\n')
            print('PASS native automation build: exact local UI survives external/data/file/path navigation and popup/form/frame probes; zero owned HTTP requests; inline script blocked; actual IPC rejects malformed aliases/IDs and unauthorized commands; hostile host text inert; default report omits seeded secrets; standalone binary and descendants have no TCP listener.',flush=True)
        finally:
            if session:
                try:command('DELETE','')
                except Exception:pass
            os.killpg(driver.pid,signal.SIGTERM);driver.wait(timeout=10)
            server.shutdown();server.server_close();thread.join(timeout=5)


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--tools-dir',type=Path,required=True);parser.add_argument('--artifacts',type=Path,required=True);parser.add_argument('--worker',action='store_true');parser.add_argument('--baseline',action='store_true');args=parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='containerdesk-047-') as directory:
        root=Path(directory)
        if args.worker:return run(root,args.tools_dir.resolve(),args.artifacts.resolve(),args.baseline)
        read,write=os.pipe()
        with (root/'xvfb.log').open('w') as log:
            display=subprocess.Popen([str(args.tools_dir/'xvfb/usr/bin/Xvfb'),'-displayfd',str(write),'-screen','0','1440x1000x24','-nolisten','tcp'],pass_fds=(write,),stdout=log,stderr=log);os.close(write)
            try:
                assert select.select([read],[],[],10)[0];number=os.read(read,32).decode().strip();assert number.isdigit()
                env=os.environ.copy()
                for key in ['WAYLAND_DISPLAY','LD_LIBRARY_PATH','LD_PRELOAD','GTK_PATH','GIO_MODULE_DIR','DBUS_SESSION_BUS_ADDRESS','AT_SPI_BUS_ADDRESS']:env.pop(key,None)
                env.update(DISPLAY=':'+number,GDK_BACKEND='x11',LIBGL_ALWAYS_SOFTWARE='1',XDG_DATA_HOME=str(root/'desktop-data'),XDG_CONFIG_HOME=str(root/'config'),XDG_CACHE_HOME=str(root/'cache'),XDG_RUNTIME_DIR=str(root/'runtime'));Path(env['XDG_RUNTIME_DIR']).mkdir(mode=0o700)
                command=['/usr/bin/dbus-run-session','--','python3',str(Path(__file__).resolve()),'--worker','--tools-dir',str(args.tools_dir.resolve()),'--artifacts',str(args.artifacts.resolve())]
                if args.baseline:command.append('--baseline')
                completed=subprocess.run(command,env=env,timeout=120);raise SystemExit(completed.returncode)
            finally:os.close(read);display.terminate();display.wait(timeout=10)

if __name__=='__main__':main()
