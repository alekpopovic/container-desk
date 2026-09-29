"""043 actual release IPC, native GTK export, and clearing only local troubleshooting data."""
import hashlib
import json
from pathlib import Path
import subprocess
import time
from native_launch import XDOTOOL


def verify(root,artifacts,env,data,config_dir,engine,script,command,click,button,wait,settings,screenshot):
    def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()
    watched=[p for p in config_dir.iterdir() if p.is_file()] + [root/'config',root/'plain',root/'encrypted',root/'known']
    before={p:digest(p) for p in watched}
    button('Desktop direct-encrypted-unloaded · direct-encrypted-unloaded')
    button('Connect saved host')
    wait('return document.querySelector(".saved-host-details")?.innerText.includes("Connection error · SSH session")')
    settings()
    button('Prepare support preview')
    wait('return !!document.querySelector(".support-preview")')
    frozen=script('return document.querySelector(".support-preview").value')
    value=json.loads(frozen)
    assert value['connection']['error']=={'stage':'authenticate','code':'authentication_failed'}
    assert value['activity'][0]['errors']==['operation_timed_out']
    assert value['connection']['host']=='host-1'
    assert value['activity'][0]['durationMsBucket']==300
    for secret in ['SYNTHETIC_043_SECRET',str(root),'direct-encrypted-unloaded',engine['engineId'],'a'*64,'h_'+'2'*32]:
        assert secret not in frozen, 'Source data leaked to support preview'
    screenshot('native-support-preview.png')
    def tool(*args): return subprocess.run([str(XDOTOOL),*map(str,args)],env=env,check=True,capture_output=True,text=True,timeout=10).stdout.strip()
    def save_window():
        end=time.monotonic()+15
        while time.monotonic()<end:
            found=subprocess.run([str(XDOTOOL),'search','--onlyvisible','--name','^Save reviewed support report$'],env=env,capture_output=True,text=True,timeout=5).stdout.split()
            if len(found)==1:
                pid=tool('getwindowpid',found[0])
                assert ('XDG_DATA_HOME='+str(data)).encode() in Path('/proc',pid,'environ').read_bytes().split(b'\0')
                return found[0]
            time.sleep(.1)
        raise AssertionError('Owned support save dialog did not appear')
    button('Save reviewed report…')
    window=save_window(); tool('windowfocus','--sync',window); tool('key','--clearmodifiers','Escape')
    wait('return document.querySelector(".support-panel")?.textContent.includes("Save cancelled; no report written.")')
    assert script('return document.querySelector(".support-preview").value')==frozen
    destination=root/'reviewed-support.json'
    assert not destination.exists()
    button('Save reviewed report…')
    window=save_window(); tool('windowfocus','--sync',window)
    subprocess.run(['/usr/bin/import','-window',window,str(artifacts/'native-support-save-dialog.png')],env=env,check=True,timeout=10)
    tool('key','--clearmodifiers','ctrl+l');time.sleep(.2)
    tool('key','--clearmodifiers','ctrl+a');tool('type','--clearmodifiers','--delay','3',str(destination));time.sleep(.2)
    tool('key','--clearmodifiers','Return')
    end=time.monotonic()+15
    while not destination.exists() and time.monotonic()<end:time.sleep(.1)
    assert destination.read_text()==frozen
    assert destination.stat().st_mode & 0o777 == 0o600
    wait('return document.querySelector(".support-panel")?.textContent.includes("Reviewed support report saved.")')
    assert script('return document.querySelector(".support-preview")===null')
    # Store only the already reviewed, pseudonymous output as an acceptance artifact.
    (artifacts/'reviewed-support.json').write_text(frozen)
    button('Prepare support preview')
    wait('return !!document.querySelector(".support-preview")')
    button('Clear local troubleshooting data…')
    wait('return !!document.querySelector("dialog[open]")')
    button('Cancel')
    assert len(json.loads((data/'dev.containerdesk.app/activity/history.json').read_text())['records'])==1
    button('Clear local troubleshooting data…')
    screenshot('native-support-clear-confirmation.png')
    button('Clear local history and preview')
    wait('return document.querySelector(".support-panel")?.textContent.includes("Local activity and prepared report cleared.")')
    assert script('return document.querySelector(".support-preview")===null')
    assert json.loads((data/'dev.containerdesk.app/activity/history.json').read_text())['records']==[]
    assert before=={p:digest(p) for p in watched}
    assert destination.read_text()==frozen
    button('Prepare support preview')
    wait('return !!document.querySelector(".support-preview")')
    assert json.loads(script('return document.querySelector(".support-preview").value'))['activity']==[]
    screenshot('native-support-cleared.png')
    print('PASS native support: actual authentication stage/error, pseudonymous frozen preview, GTK Cancel/Save, exact 0600 JSON, clear Cancel/confirm, empty persisted history, all owned SSH config/key/trust hashes and exported file unchanged.',flush=True)
