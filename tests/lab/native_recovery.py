"""041 actual Linux connection reset, master loss and owned native exit audit."""
import ctypes
import base64
import json
import os
from pathlib import Path
import signal
import subprocess
import time


def verify(root,artifacts,script,command,click,button,fill,wait,element,config,live_id,xdotool):
    native_button=button
    def button(name):
        wait('return Array.from(document.querySelectorAll("button")).some(b=>b.textContent.trim()==='+json.dumps(name)+' && !b.disabled)',40)
        native_button(name)
    def processes():
        found={}
        for path in Path('/proc').iterdir():
            if not path.name.isdigit(): continue
            try:
                if f'XDG_DATA_HOME={root}/native-data'.encode() not in (path/'environ').read_bytes().split(b'\0'): continue
                stat=(path/'stat').read_text();fields=stat[stat.rfind(')')+2:].split()
                found[int(path.name)]={'start':fields[19],'name':(path/'comm').read_text().strip(),'args':(path/'cmdline').read_bytes()}
            except (OSError,ValueError,IndexError): pass
        return found
    def vanished(old):
        deadline=time.monotonic()+12
        while time.monotonic()<deadline:
            remaining=[]
            for pid,entry in old.items():
                try:
                    stat=Path('/proc',str(pid),'stat').read_text();fields=stat[stat.rfind(')')+2:].split()
                    if fields[19]==entry['start']: remaining.append((pid,fields[0]))
                except FileNotFoundError: pass
            if not remaining: return
            time.sleep(.1)
        raise AssertionError('Owned pre-interruption processes were not reaped (PID/state): '+str(remaining))
    def ssh(): return {pid:entry for pid,entry in processes().items() if entry['name']=='ssh'}
    def ready():
        wait('return document.querySelectorAll("[data-container-id]").length===4 && document.querySelector(".saved-host-details [role=status]")?.textContent.includes("Ready")',50)
        wait('return !!document.querySelector(".container-console")',20)
    def hold_stats():
        (root/'stats-held').unlink(missing_ok=True)
        (root/'hold-stats').write_text('hold only this owned stats command')
        deadline=time.monotonic()+15
        while not (root/'stats-held').exists():
            assert time.monotonic()<deadline, 'No actual stats command entered owned delay gate'
            time.sleep(.05)
        assert (root/'stats-held').read_text()==live_id
    def connected_terminal():
        if script('return Array.from(document.querySelectorAll("button")).some(b=>b.textContent==="Stop logs" && !b.disabled)'):
            button('Stop logs')
            wait('return Array.from(document.querySelectorAll("button")).some(b=>b.textContent==="Start logs" && !b.disabled)')
        script('const target=Array.from(document.querySelectorAll("button[role=tab]")).find(b=>b.textContent==="Terminal");target.focus();window.__nativeTerminalTab=null;target.addEventListener("click",e=>window.__nativeTerminalTab=e.isTrusted,{once:true})')
        subprocess.run([str(xdotool),'key','--clearmodifiers','Return'],check=True,capture_output=True,timeout=10)
        wait('return window.__nativeTerminalTab===true && !!document.querySelector(".terminal-panel")')
        button('Enable management for terminal')
        button('Enable terminal access')
        button('Open terminal')
        wait('return !!document.querySelector(".mutation-confirmation")',40)
        button('Confirm terminal')
        wait('return document.querySelector(".terminal-status")?.textContent.includes("Connected")',40)
        wait('return document.querySelector(".xterm-accessibility-tree")?.textContent.includes("$")')
    # Actual SSH log stream and native stats are active when only the owned server's
    # connection handlers are killed. Its listener remains available for a fresh attempt.
    button('Start logs')
    wait('return document.querySelector(".live-logs")?.innerText.includes("Following")')
    wait('return document.querySelector(".container-stats")?.textContent.includes("Sample received")',30)
    hold_stats()
    old=ssh();assert old
    (root/'interrupt-network').write_text('only owned sshd connections')
    deadline=time.monotonic()+5
    while not (root/'interrupt-network-done').exists():
        assert time.monotonic()<deadline
        time.sleep(.05)
    (root/'hold-stats').unlink()
    wait('return document.querySelector(".connection-recovery")?.textContent.includes("restored read-only")',50)
    ready();vanished(old)
    assert (root/'interrupt-network-done').exists()
    assert not (root/'pty-execs.jsonl').exists()
    button('Start logs')
    wait('return document.querySelector(".live-logs")?.innerText.includes("Following")')
    assert 'Log gap' in script('return document.querySelector(".live-logs").innerText')
    script('document.querySelector(".live-logs").scrollIntoView({block:"start",behavior:"instant"})')
    (artifacts/'native-recovered-logs.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
    print('PASS native connection interruption during logs/stats: old SSH processes gone, selected host recovered read-only, fresh rows, logs restarted explicitly with visible gap.',flush=True)
    # A separate actual local master fault occurs while a terminal and stats are active.
    connected_terminal()
    script('document.querySelector(".xterm-helper-textarea").focus()')
    subprocess.run([str(xdotool),'type','--clearmodifiers','--delay','25','--',"printf 'CD041_%s\\n' OWNED\n"],check=True,capture_output=True,timeout=15)
    wait('return document.querySelector(".xterm-accessibility-tree")?.textContent.includes("CD041_OWNED")')
    hold_stats()
    old=ssh()
    masters={pid:entry for pid,entry in old.items() if b'ControlMaster=yes' in entry['args'] or b'[mux]' in entry['args']}
    assert len(masters)==1, 'Exactly one explicitly owned native master is required'
    pid,entry=next(iter(masters.items()))
    assert processes()[pid]['start']==entry['start']
    os.kill(pid,signal.SIGKILL)
    wait('return !document.querySelector(".terminal-panel")')
    vanished(old)
    (root/'hold-stats').unlink()
    ready()
    assert len((root/'pty-execs.jsonl').read_text().splitlines())==1, 'Lost terminal must never reopen'
    wait('return document.querySelector(".container-management")?.textContent.includes("Enable management")')
    print('PASS native master fault during terminal/stats: old owner processes gone, generation fenced, read-only recovery and no terminal replay.',flush=True)
    connected_terminal()
    hold_stats()
    old=processes()
    assert any(entry['name']=='containerdesk' for entry in old.values())
    assert len((root/'pty-execs.jsonl').read_text().splitlines())==2
    script('document.querySelector(".terminal-panel").scrollIntoView({block:"start",behavior:"instant"})')
    (artifacts/'native-before-graceful-exit.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
    started=time.monotonic()
    app_pids={pid:entry for pid,entry in old.items() if entry['name']=='containerdesk'}
    assert len(app_pids)==1
    found=subprocess.run([str(xdotool),'search','--onlyvisible','--pid',str(next(iter(app_pids))),'--name','^ContainerDesk$'],check=True,capture_output=True,text=True,timeout=10).stdout.split()
    assert len(found)==1
    # A real WM_DELETE_WINDOW message follows the native window-manager close path.
    class ClientMessage(ctypes.Structure):
        _fields_=[('type',ctypes.c_int),('serial',ctypes.c_ulong),('send_event',ctypes.c_int),('display',ctypes.c_void_p),('window',ctypes.c_ulong),('message_type',ctypes.c_ulong),('format',ctypes.c_int),('data',ctypes.c_long*5)]
    class XEvent(ctypes.Union):
        _fields_=[('client',ClientMessage),('padding',ctypes.c_long*24)]
    x11=ctypes.CDLL('libX11.so.6')
    x11.XOpenDisplay.argtypes=[ctypes.c_char_p];x11.XOpenDisplay.restype=ctypes.c_void_p
    x11.XInternAtom.argtypes=[ctypes.c_void_p,ctypes.c_char_p,ctypes.c_int];x11.XInternAtom.restype=ctypes.c_ulong
    x11.XSendEvent.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_long,ctypes.POINTER(XEvent)]
    x11.XFlush.argtypes=[ctypes.c_void_p];x11.XCloseDisplay.argtypes=[ctypes.c_void_p]
    display=x11.XOpenDisplay(None);assert display
    try:
        event=XEvent();event.client.type=33;event.client.display=display;event.client.window=int(found[0]);event.client.message_type=x11.XInternAtom(display,b'WM_PROTOCOLS',0);event.client.format=32;event.client.data[0]=x11.XInternAtom(display,b'WM_DELETE_WINDOW',0)
        assert x11.XSendEvent(display,int(found[0]),0,0,ctypes.byref(event))
        x11.XFlush(display)
    finally: x11.XCloseDisplay(display)
    # Session deletion may terminate a driver-owned app; first prove the app exited itself.
    app_pids={pid:entry for pid,entry in old.items() if entry['name']=='containerdesk'}
    deadline=time.monotonic()+12
    while True:
        alive=[]
        for pid,entry in app_pids.items():
            try:
                stat=Path('/proc',str(pid),'stat').read_text();fields=stat[stat.rfind(')')+2:].split()
                if fields[19]==entry['start'] and fields[0]!='Z': alive.append(pid)
            except FileNotFoundError: pass
        if not alive: break
        assert time.monotonic()<deadline, 'Native application did not exit within cleanup deadline'
        time.sleep(.1)
    try: command('DELETE','')
    except Exception: pass
    # WebKit driver descendants can remain until session teardown; inspect app + its SSH owners.
    vanished({pid:entry for pid,entry in old.items() if entry['name'] in ('containerdesk','ssh')})
    (root/'hold-stats').unlink()
    elapsed=time.monotonic()-started
    assert elapsed<12
    assert not (root/'mutation-count').exists()
    (artifacts/'process-audit.json').write_text(json.dumps({'initialNativeStatsSample':True,'statsInFlightAtFaults':3,'networkResetOldSshReaped':True,'masterLossOldSshReaped':True,'terminalExecCount':2,'gracefulExitSeconds':round(elapsed,3),'appAndSshProcessesReaped':True,'mutationDispatches':0},indent=2)+'\n')
    print('PASS actual native window exit: active terminal/stats/events closed, app and owned SSH processes gone in '+str(round(elapsed,3))+' seconds, no dispatched mutations.',flush=True)
