"""040 actual release webview -> native PTY -> owned Docker, never mocked IPC."""
import base64
import json
import time
import subprocess
from native_ssh import ELEMENT


def verify(root, artifacts, script, command, click, button, fill, wait, element, config, live_id, xdotool):
    native_button=button
    def button(name):
        wait('return Array.from(document.querySelectorAll("button")).some(b=>b.textContent.trim()==='+json.dumps(name)+' && !b.disabled)',40)
        native_button(name)
    def text_contains(value):
        wait('return document.querySelector(".xterm-accessibility-tree")?.textContent.includes('+json.dumps(value)+')')
    def keys(text):
        script('document.querySelector(".xterm-helper-textarea").focus()')
        # Use real X11 key events with normal typing cadence; WebKit's zero-delay W3C
        # action batches lost printable key events in this native driver version.
        assert xdotool
        subprocess.run([str(xdotool),'type','--clearmodifiers','--delay','25','--',text],check=True,timeout=20,capture_output=True)
    def paste(text):
        script('const group=document.createElement("div");group.id="owned-native-clipboard";group.style="position:fixed;top:0;left:0;z-index:9999;background:white";const field=document.createElement("textarea");field.value=arguments[0];const button=document.createElement("button");button.id="owned-native-copy";button.textContent="Copy owned fixture";button.onclick=()=>{field.focus();field.select();window.__ownedCopied=document.execCommand("copy")};group.append(field,button);document.body.append(group)',text)
        click('//*[@id="owned-native-copy"]')
        assert script('return window.__ownedCopied')
        script('document.querySelector("#owned-native-clipboard").remove();document.querySelector(".xterm-helper-textarea").focus()')
        script('window.__ownedPaste=null;document.addEventListener("paste",e=>window.__ownedPaste={trusted:e.isTrusted,target:e.target.className,length:e.clipboardData?.getData("text/plain").length},{capture:true,once:true})')
        command('POST','/actions',{'actions':[{'type':'key','id':'terminal-paste','actions':[{'type':'keyDown','value':'\ue009'},{'type':'keyDown','value':'\ue008'},{'type':'keyDown','value':'v'},{'type':'keyUp','value':'v'},{'type':'keyUp','value':'\ue008'},{'type':'keyUp','value':'\ue009'}]}]})
        command('DELETE','/actions')
        try:
            wait('return !!document.querySelector(".terminal-paste")')
        except Exception:
            print('Native paste metadata:',script('return {paste:window.__ownedPaste,focus:document.activeElement?.className}'),flush=True)
            raise
        assert script('return window.__ownedPaste?.trusted && window.__ownedPaste.length===arguments[0]',len(text))
    def open_terminal(cancel_first=False, alias="logs-owned"):
        click('//button[@role="tab" and normalize-space(.)="Terminal"]')
        wait('return Array.from(document.querySelectorAll("button")).some(b=>b.textContent==="Enable management for terminal" && !b.disabled)')
        button('Enable management for terminal')
        button('Enable terminal access')
        button('Open terminal')
        wait('return !!document.querySelector("[aria-label=\\"Confirm terminal access\\"]")')
        identity=script('return document.querySelector(".mutation-confirmation").innerText')
        assert live_id in identity and alias in identity and '1000:1000' in identity
        if cancel_first:
            button('Cancel terminal')
            assert not (root/'pty-execs.jsonl').exists(), 'Cancelled intent dispatched a terminal'
            button('Open terminal')
            wait('return !!document.querySelector(".mutation-confirmation")')
        button('Confirm terminal')
        wait('return document.querySelector(".terminal-status")?.textContent.includes("Connected")',40)
        # Local SSH starts before remote exec/shell initialization finishes. Wait for this
        # owned Alpine shell's actual prompt before typing; never replay an early command.
        text_contains('$')
    open_terminal(True)
    keys("printf 'CD040_%s\\n' READY\n")
    text_contains('CD040_READY')
    print('PASS native terminal keyboard and real shell output.',flush=True)
    before=script('return document.querySelector(".terminal-size").textContent')
    command('POST','/window/rect',{'width':1100,'height':900})
    wait('return document.querySelector(".terminal-size").textContent!=='+json.dumps(before))
    columns,rows=map(int,script('return document.querySelector(".terminal-size").textContent').split('×'))
    keys('stty size\n')
    text_contains(f'{rows} {columns}')
    payload="printf 'CD040_%s\\n' PASTE_ONE\nprintf 'CD040_%s\\n' PASTE_TWO\n"
    paste(payload)
    assert not script('return document.querySelector(".xterm-accessibility-tree").textContent.includes("CD040_PASTE_ONE")')
    button('Cancel paste')
    assert not script('return document.querySelector(".xterm-accessibility-tree").textContent.includes("CD040_PASTE_ONE")')
    paste(payload)
    button('Send multiline paste')
    text_contains('CD040_PASTE_ONE')
    text_contains('CD040_PASTE_TWO')
    script('document.querySelector(".terminal-panel").scrollIntoView({block:"start",behavior:"instant"})')
    (artifacts/'native-terminal-connected.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
    keys("while :; do printf 'CD040_%s\\n' FLOW; sleep 0.05; done\n")
    text_contains('CD040_FLOW')
    click('//button[@role="tab" and normalize-space(.)="Logs"]')
    wait('return !document.querySelector(".terminal-panel")')
    # A route switch unmounts the terminal; a separately selected saved host gets a fresh scope.
    click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Hosts"]')
    button('Disconnect saved host')
    wait('return document.body.innerText.includes("Disconnected · SSH session")')
    button('New host')
    fill('Host SSH config path',str(config))
    fill('Host SSH alias','logs-second')
    fill('Display name','Owned terminal second host')
    fill('Saved Docker executable','/usr/bin/docker')
    button('Save host')
    button('Connect saved host')
    wait('return document.body.innerText.includes("Ready · SSH session")')
    click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Containers"]')
    wait('return document.querySelectorAll("[data-container-id]").length===4')
    click(f'//tr[@data-container-id="{live_id}"]//button')
    open_terminal(alias="logs-second")
    assert script('return document.querySelector(".terminal-identity").textContent.includes("Owned terminal second host")')
    assert not script('return document.querySelector(".xterm-accessibility-tree").textContent.includes("CD040_FLOW")')
    keys("printf 'CD040_%s\\n' SECOND\n")
    text_contains('CD040_SECOND')
    button('Disable terminal access')
    wait('return document.querySelector(".terminal-status")?.textContent.match(/revoked|not permitted/)')
    (artifacts/'native-terminal-revoked.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
    time.sleep(.25)
    assert len((root/'pty-execs.jsonl').read_text().splitlines())==2, 'No terminal reconnect or replay'
    click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Hosts"]')
    button('Disconnect saved host')
    wait('return document.body.innerText.includes("Disconnected · SSH session")')
    print('PASS native terminal UI: explicit cancel/open, non-root real shell output, fit/stty resize, actual native multiline clipboard cancel/send, tab closure under output, two saved host scopes, permission revocation, no reconnect.',flush=True)
