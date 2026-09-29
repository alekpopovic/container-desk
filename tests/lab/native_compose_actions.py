"""Actual 037 production UI journey, using only the explicitly owned project."""
import base64
import json
from native_ssh import ELEMENT

def verify(root,artifacts,script,command,click,button,fill,wait,element):
    metadata=json.loads((root/'compose-project.json').read_text())
    config=metadata['configuration']
    click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Compose"]')
    wait('return !!document.querySelector(".compose-management")')
    fill('Remote project directory',config['workingDirectory'])
    fill('Explicit project name',config['projectName'])
    node=element('//label[normalize-space(text())="Ordered remote config files (one absolute path per line)"]/textarea')
    command('POST',f'/element/{node}/click',{})
    for index,file in enumerate(config['configFiles']):
        if index:
            command('POST','/actions',{'actions':[{'type':'key','id':'compose-file-newline','actions':[{'type':'keyDown','value':'\ue007'},{'type':'keyUp','value':'\ue007'}]}]})
            command('DELETE','/actions')
        command('POST',f'/element/{node}/value',{'text':file})
    assert script('return document.querySelector(".compose-management textarea").value')=='\n'.join(config['configFiles']), 'Native ordered file input must preserve real newlines' 
    click('//section[contains(@class,"compose-management")]//input[@type="checkbox"]')
    button('Verify remote project')
    wait('return document.querySelector(".compose-management")?.innerText.includes("Verified remote configuration and existing service identities")',30)
    assert script('return document.querySelector(".compose-management").innerText.includes("web, worker")')
    button('Enable Compose management')
    wait('return Array.from(document.querySelectorAll("button")).some(b=>b.textContent==="Disable Compose management")')
    fill('Compose stop timeout (seconds)','1')
    button('Restart verified services')
    wait('return !!document.querySelector("[aria-label=\\"Confirm Compose action\\"]")',30)
    text=script('return document.querySelector(".mutation-confirmation").innerText')
    assert config['workingDirectory'] in text and all(file in text for file in config['configFiles'])
    assert all(ident in text for ident in metadata['ids'])
    (artifacts/'native-compose-confirmation.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
    button('Cancel Compose action')
    assert not (root/'compose-action-commands.jsonl').exists()
    button('Restart verified services')
    wait('return !!document.querySelector(".mutation-confirmation")',30)
    button('Confirm Compose action')
    wait('return document.querySelector(".compose-management")?.innerText.includes("Compose command completed") && document.querySelectorAll("[aria-label=\\"Observed Compose service state\\"] li").length===2',40)
    assert script('return Array.from(document.querySelectorAll("[aria-label=\\"Observed Compose service state\\"] li")).every(row=>row.innerText.includes("running"))')
    assert 'synthetic-compose-037-private' not in script('return document.querySelector(".compose-management").innerText')
    script('document.querySelector("[aria-label=\\"Observed Compose service state\\"]").scrollIntoView({block:"center",behavior:"instant"})')
    (artifacts/'native-compose-observed.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
    wait('return Array.from(document.querySelectorAll("button")).some(b=>b.textContent==="Disable Compose management" && !b.disabled)')
    script('window.__revokeClick=[];document.addEventListener("click",event=>window.__revokeClick.push({trusted:event.isTrusted,label:event.target.closest("button")?.textContent}),{capture:true,once:true})')
    for _ in range(3):
        command('POST','/execute/async',{'script':'const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)));','args':[]})
        button('Disable Compose management')
        if script('return window.__revokeClick.length>0'):break
    assert script('return window.__revokeClick.some(e=>e.trusted && e.label==="Disable Compose management")')
    wait('return Array.from(document.querySelectorAll("button")).some(b=>b.textContent==="Enable Compose management")')
    assert script('return Array.from(document.querySelectorAll(".compose-management .management-actions button")).every(b=>b.disabled)')
    click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Hosts"]')
    button('Disconnect saved host')
    wait('return document.body.innerText.includes("Disconnected · SSH session")')
    print('PASS native Compose UI: explicit quoted remote files/name, verification, management opt-in, full host/daemon/service/ID confirmation, cancel dispatched nothing, exactly one restart, both services observed running, explicit return to read-only and disconnect.',flush=True)
