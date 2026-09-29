"""046 one production native session: discovery, reads, lifecycle, Compose, PTY, cleanup."""
import base64
import json
from pathlib import Path
import subprocess
import time
from native_pressure import process_sample


def discover(root,artifacts,script,command,click,button,wait,element,config):
    click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Settings"]')
    node=element('//*[@id="ssh-config-path"]')
    command('POST',f'/element/{node}/clear',{})
    command('POST',f'/element/{node}/value',{'text':str(config)})
    button('Browse host candidates')
    wait('return document.querySelectorAll(".ssh-candidates button").length===3')
    button('Select logs-owned')
    button('Resolve selected alias')
    wait('return document.querySelector("[aria-label=\\"Effective SSH configuration\\"]")?.textContent.includes("logs-jump")')
    text=script('return document.querySelector("[aria-label=\\"Effective SSH configuration\\"]").textContent')
    assert 'logs-owned' in text and '127.0.0.1' in text
    script('document.querySelector("[aria-label=\\"Effective SSH configuration\\"]").scrollIntoView({block:"center",behavior:"instant"})')
    (artifacts/'native-discovery.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
    print('PASS native discovery: three literal aliases from owned config; explicitly selected/resolved destination retains ProxyJump.',flush=True)


def verify(root,artifacts,script,command,click,button,fill,wait,element,live_id,window,xdotool,env):
    native_button=button
    def button(name):
        wait('return Array.from(document.querySelectorAll("button")).some(b=>b.textContent.trim()==='+json.dumps(name)+' && !b.disabled)',40)
        native_button(name)
    app_pid=int(subprocess.run([str(xdotool),'getwindowpid',window],env=env,check=True,capture_output=True,text=True).stdout)
    observed=set();stages={}
    def sample(stage):
        state=process_sample(app_pid);identities=state.pop('sshIdentities');observed.update(identities);stages[stage]=state
        return identities
    def present(identities):
        result=[]
        for pid,start in identities:
            try:
                if Path('/proc',str(pid),'stat').read_text().rsplit(') ',1)[1].split()[19]==start:result.append(pid)
            except OSError:pass
        return result
    def stream_identities(identities,marker):
        result=[]
        for identity in identities:
            try:
                if marker in Path('/proc',str(identity[0]),'cmdline').read_bytes():result.append(identity)
            except OSError:pass
        assert result,'Expected actual owned SSH stream process'
        return result
    def reaped(identities):
        deadline=time.monotonic()+10
        while present(identities) and time.monotonic()<deadline:time.sleep(.1)
        assert not present(identities),'Owned SSH stream survived closure'
    sample('beforeConnection');assert stages['beforeConnection']['sshChildren']==0
    button('Connect saved host')
    wait('return document.body.innerText.includes("Ready · SSH session")')
    assert script('return document.body.innerText.includes("logs-jump")')
    daemon=script('return Array.from(document.querySelectorAll(".saved-host-details dt")).find(e=>e.textContent==="Daemon identity")?.nextElementSibling?.textContent')
    assert daemon and daemon!='Not verified'
    assert not script('return document.body.innerText.includes("Management unavailable")')
    click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Containers"]')
    count=len(json.loads((root/'manifest.json').read_text())['ownedIds'])
    wait('return document.querySelectorAll("[data-container-id]").length==='+str(count))
    click(f'//tr[@data-container-id="{live_id}"]//button')
    wait('return document.querySelector(".inspect-detail")?.getAttribute("aria-busy")==="false"')
    sample('inventory')
    button('Start logs')
    wait('return document.querySelector(".live-logs")?.innerText.includes("Following")')
    wait('return document.querySelectorAll("[data-log-key]").length>0')
    logs=stream_identities(sample('followingLogs'),b"'logs' '--follow'")
    script('document.querySelector(".live-logs").scrollIntoView({block:"center",behavior:"instant"})')
    (artifacts/'native-logs.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
    button('Stop logs');reaped(logs);sample('stoppedLogs')
    button('Enable management')
    fill('Stop timeout (seconds)','1')
    button('Stop container')
    wait('return !!document.querySelector(".mutation-confirmation")')
    identity=script('return document.querySelector(".mutation-confirmation").innerText')
    assert live_id in identity and 'logs-owned' in identity and daemon in identity
    button('Cancel action');assert not (root/'mutation-count').exists()
    for action,state in [('Stop container','exited'),('Start container','running')]:
        button(action);wait('return !!document.querySelector(".mutation-confirmation")');button('Confirm action')
        wait('return document.querySelector(".container-management [role=status]")?.innerText.includes("Observed state: '+state+'")',50)
    assert (root/'mutation-count').read_text().splitlines()==['stop','start']
    script('document.querySelector(".container-management").scrollIntoView({block:"center",behavior:"instant"})')
    (artifacts/'native-management.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
    button('Disable management')
    wait('return document.querySelector(".container-management")?.innerText.includes("Read-only controls.")')
    sample('afterLifecycle')
    from native_compose_actions import verify as compose
    compose(root,artifacts,script,command,click,button,fill,wait,element,keep_connection=True)
    sample('afterCompose')
    click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Containers"]')
    wait('return document.querySelectorAll("[data-container-id]").length==='+str(count))
    click(f'//tr[@data-container-id="{live_id}"]//button')
    click('//button[@role="tab" and normalize-space(.)="Terminal"]')
    button('Enable management for terminal');button('Enable terminal access');button('Open terminal')
    wait('return !!document.querySelector(".mutation-confirmation")')
    identity=script('return document.querySelector(".mutation-confirmation").innerText')
    assert live_id in identity and 'logs-owned' in identity and '1000:1000' in identity and daemon in identity
    button('Cancel terminal');assert not (root/'pty-execs.jsonl').exists()
    button('Open terminal');button('Confirm terminal')
    wait('return document.querySelector(".xterm-accessibility-tree")?.textContent.includes("$")',40)
    def keys(text):
        script('document.querySelector(".xterm-helper-textarea").focus()')
        subprocess.run([str(xdotool),'type','--clearmodifiers','--delay','25','--',text],env=env,check=True,capture_output=True,timeout=20)
    keys("printf 'CD046_%s\\n' READY\n")
    wait('return document.querySelector(".xterm-accessibility-tree")?.textContent.includes("CD046_READY")')
    rendered=script('const row=document.querySelector(".xterm-rows"); const css=getComputedStyle(row); return {color:css.color,font:css.fontFamily,fontSize:css.fontSize}')
    assert rendered['color']=='rgb(227, 237, 242)' and 'monospace' in rendered['font'], 'Native CSP blocked terminal renderer styles'
    assert script('window.__inlineProbe=0; const node=document.createElement("script");node.textContent="window.__inlineProbe=1";document.head.append(node);node.remove();return window.__inlineProbe===0'), 'Native CSP must still reject inline scripts'
    terminal=stream_identities(sample('terminalConnected'),b"'exec' '--interactive'")
    script('document.querySelector(".terminal-panel").scrollIntoView({block:"center",behavior:"instant"})')
    (artifacts/'native-terminal.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
    keys("while :; do printf 'CD046_%s\\n' FLOW; sleep 0.05; done\n")
    wait('return document.querySelector(".xterm-accessibility-tree")?.textContent.includes("CD046_FLOW")')
    click('//button[@role="tab" and normalize-space(.)="Logs"]')
    reaped(terminal);sample('terminalTabClosed')
    assert len((root/'pty-execs.jsonl').read_text().splitlines())==1,'Terminal replayed on tab close'
    click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Hosts"]')
    button('Disconnect saved host');wait('return document.body.innerText.includes("Disconnected · SSH session")')
    reaped(observed);sample('disconnected')
    assert stages['disconnected']['sshChildren']==0 and stages['disconnected']['zombies']==0
    (artifacts/'native-cleanup.json').write_text(json.dumps({'platform':'Linux x86_64, actual release WebKitGTK','ownedContainerCount':count,'singleSavedAlias':'logs-owned','jumpAlias':'logs-jump','cancelledActionsDispatched':0,'containerActions':['stop','start'],'composeGuiActions':['restart'],'terminalGuiDispatches':1,'terminalRenderedStyles':rendered,'inlineScriptBlocked':True,'logStreamIdentitiesReaped':len(logs),'terminalStreamIdentitiesReaped':len(terminal),'observedSshIdentitiesReaped':len(observed),'stages':stages},indent=2)+'\n')
    print('PASS integrated native session: same saved host/daemon/full container identity; actual logs and reaped stream; cancelled then single stop/start; verified Compose restart; cancelled then non-root PTY; terminal tab closure under output; all observed owned SSH identities reaped after disconnect.',flush=True)
