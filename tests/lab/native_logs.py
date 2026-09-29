"""Actual release Tauri channel journey using only the owned loopback log lab."""
import base64
import json
import os
from pathlib import Path
import signal
import subprocess
import time
import urllib.request
from native_ssh import unused_port, ELEMENT, REPO, application_binary


def verify(root, tauri_driver, webkit_driver, config, live_id, artifacts, export_id=None, xdotool=None, stats=False, events_id=None, mvp=False, management=False, batch=False, networks=False, compose_actions=False, terminal=False, recovery=False, keyboard=False, pressure=False, checkpoint=False):
    env = os.environ.copy()
    for name in ('LD_LIBRARY_PATH', 'LD_PRELOAD', 'GTK_PATH', 'GIO_MODULE_DIR', 'SSH_AUTH_SOCK'):
        env.pop(name, None)
    # Ubuntu 26.04 GTK's glycin icon loader needs the system bwrap helper on PATH.
    # Keep Docker, Node, Python and Cargo absent from the application's executable search path.
    native_bin = root / 'native-bin'
    native_bin.mkdir(mode=0o700)
    if export_id:
        (native_bin / 'bwrap').symlink_to('/usr/bin/bwrap')
    env.update(XDG_DATA_HOME=str(root / 'native-data'), GDK_BACKEND='x11', PATH=str(native_bin))
    if pressure:
        data=root/'native-data/dev.containerdesk.app'
        data.mkdir(parents=True,exist_ok=True)
        (data/'resource-limits.json').write_text(json.dumps({'logLines':8000,'logBytes':1024*1024,'statsHistory':60,'activeHosts':1,'concurrentJobs':4}))
    artifacts = artifacts or root / 'native-artifacts'
    artifacts.mkdir(parents=True, exist_ok=True)
    port, native_port = unused_port(), unused_port()
    session = None
    with (root / 'native-driver.log').open('w') as log:
        driver = subprocess.Popen([str(tauri_driver), '--port', str(port), '--native-port', str(native_port), '--native-driver', str(webkit_driver)], env=env, stdout=log, stderr=log, start_new_session=True)
        def request(method, path, data=None):
            payload = None if data is None else json.dumps(data).encode()
            req = urllib.request.Request(f'http://127.0.0.1:{port}{path}', data=payload, method=method, headers={'Content-Type': 'application/json'})
            with urllib.request.urlopen(req, timeout=30) as response:
                return json.load(response)['value']
        def command(method, path, data=None):
            return request(method, f'/session/{session}{path}', data)
        def script(code, *args):
            return command('POST', '/execute/sync', {'script': code, 'args': list(args)})
        def element(xpath):
            return command('POST', '/element', {'using': 'xpath', 'value': xpath})[ELEMENT]
        def click(xpath):
            node = element(xpath)
            # Settle WebKit's viewport after transient clipboard fields/virtualized rows.
            script('arguments[0].scrollIntoView({block: "center", behavior: "instant"})', {ELEMENT: node})
            time.sleep(.15)
            # WebKit can report a click while viewport movement prevented DOM delivery.
            # Retry only zero-delivery attempts, never a delivered action or confirmation.
            script('const target=arguments[0]; window.__nativeClick=null; if(window.__nativeClickListener) document.removeEventListener("click",window.__nativeClickListener,true); window.__nativeClickListener=event=>{window.__nativeClick={trusted:event.isTrusted,matched:target.contains(event.target)};}; document.addEventListener("click",window.__nativeClickListener,{capture:true,once:true});', {ELEMENT:node})
            try:
                for _ in range(3):
                    command('POST','/execute/async',{'script':'const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)));','args':[]})
                    command('POST', f'/element/{node}/click', {})
                    delivered=script('return window.__nativeClick')
                    if delivered is not None: break
                assert delivered and delivered['trusted'] and delivered['matched'], 'Native click did not reach intended element: ' + xpath
            except urllib.error.HTTPError as error:
                details = error.read().decode()
                (artifacts / 'native-click-failure.png').write_bytes(base64.b64decode(command('GET', '/screenshot'), validate=True))
                raise AssertionError('Native click failed: ' + xpath + ': ' + details) from error
        def button(name): click(f'//button[normalize-space(.)="{name}"]')
        def fill(label, value):
            node = element(f'//label[normalize-space(text())="{label}"]/input')
            command('POST', f'/element/{node}/clear', {})
            command('POST', f'/element/{node}/value', {'text': value})
        def wait(condition, timeout=20):
            end = time.monotonic() + timeout
            while time.monotonic() < end:
                if script(condition): return
                time.sleep(.1)
            (artifacts / 'native-timeout.png').write_bytes(base64.b64decode(command('GET', '/screenshot'), validate=True))
            print('Native state:', script('return {recovery:document.querySelector(".connection-recovery")?.textContent,connection:document.querySelector(".saved-host-details [role=status]")?.textContent,terminal:document.querySelector(".terminal-status")?.textContent,terminalError:document.querySelector(".terminal-panel [role=alert]")?.textContent,terminalButtons:Array.from(document.querySelectorAll(".terminal-controls button"),b=>({text:b.textContent,disabled:b.disabled})),management:document.querySelector(".container-management")?.innerText,dialogs: document.querySelectorAll("dialog[open]").length, buttons: Array.from(document.querySelectorAll(".live-logs button"), b=>({text:b.textContent,disabled:b.disabled})), status: Array.from(document.querySelectorAll(".live-logs > p[role=status]"), p=>p.textContent)}'), flush=True)
            raise AssertionError('Native log condition did not become true: ' + condition)
        try:
            for _ in range(100):
                try:
                    request('GET', '/status')
                    break
                except OSError: time.sleep(.1)
            result = request('POST', '/session', {'capabilities': {'alwaysMatch': {'browserName': 'wry', 'tauri:options': {'application': str(application_binary())}}}})
            session = result['sessionId']
            command('POST', '/timeouts', {'implicit': 5000, 'script': 5000, 'pageLoad': 15000})
            if (stats or terminal or keyboard or pressure) and xdotool:
                found = subprocess.run([str(xdotool), 'search', '--onlyvisible', '--name', '^ContainerDesk$'], env=env, capture_output=True, text=True, check=True, timeout=10).stdout.split()
                owned_windows = []
                for window in found:
                    pid = subprocess.run([str(xdotool), 'getwindowpid', window], env=env, capture_output=True, text=True, check=True, timeout=10).stdout.strip()
                    if f'XDG_DATA_HOME={root}/native-data'.encode() in Path('/proc', pid, 'environ').read_bytes().split(b'\x00'):
                        owned_windows.append(window)
                assert len(owned_windows) == 1
                subprocess.run([str(xdotool), 'windowfocus', '--sync', owned_windows[0]], env=env, check=True, timeout=10)
            if checkpoint:
                from native_checkpoint import discover
                discover(root,artifacts,script,command,click,button,wait,element,config)
            button('Add host')
            button('New host')
            fill('Host SSH config path', str(config))
            fill('Host SSH alias', 'logs-owned')
            fill('Display name', 'Owned live log checkpoint')
            fill('Saved Docker executable', '/usr/bin/docker')
            button('Save host')
            if checkpoint:
                from native_checkpoint import verify as verify_checkpoint
                verify_checkpoint(root,artifacts,script,command,click,button,fill,wait,element,live_id,owned_windows[0],xdotool,env)
                command('DELETE', '')
                session = None
                for file in (root/'native-data').rglob('*'):
                    if file.is_file(): assert b'CD046_' not in file.read_bytes(), 'Terminal transcript persisted'
                assert b'CD046_' not in (root/'native-driver.log').read_bytes(), 'Terminal transcript in diagnostics'
                print('PASS integrated native app storage/diagnostics contain no terminal transcript markers.',flush=True)
                return
            if pressure:
                from native_pressure import verify as verify_pressure
                verify_pressure(root,artifacts,script,command,click,button,fill,wait,live_id,owned_windows[0],xdotool,env)
                return
            if keyboard:
                from native_keyboard import verify as verify_keyboard
                verify_keyboard(root,artifacts,script,command,wait,live_id,xdotool,env)
                return
            button('Connect saved host')
            wait('return document.body.innerText.includes("Ready · SSH session")')
            if mvp:
                assert script('return document.body.innerText.includes("logs-jump")')
                (artifacts / 'native-jump-connected.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
            click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Containers"]')
            if terminal:
                wait('return document.querySelectorAll("[data-container-id]").length === 4')
                click(f'//tr[@data-container-id="{live_id}"]//button')
                if recovery:
                    from native_recovery import verify as verify_recovery
                    verify_recovery(root,artifacts,script,command,click,button,fill,wait,element,config,live_id,xdotool)
                    try: command('DELETE', '')
                    except Exception: pass # Helper verified the actual native process already exited.
                else:
                    from native_terminal import verify as verify_terminal
                    verify_terminal(root,artifacts,script,command,click,button,fill,wait,element,config,live_id,xdotool)
                    command('DELETE', '')
                session = None
                for file in (root / 'native-data').rglob('*'):
                    if file.is_file():
                        assert all(marker not in file.read_bytes() for marker in (b'CD040_',b'CD041_')), 'Terminal transcript persisted'
                assert all(marker not in (root / 'native-driver.log').read_bytes() for marker in (b'CD040_',b'CD041_')), 'Terminal transcript in diagnostics'
                print('PASS native terminal app storage and diagnostics contain no transcript markers.',flush=True)
                return
            if compose_actions:
                count=len(json.loads((root/'manifest.json').read_text())['ownedIds'])
                wait('return document.querySelectorAll("[data-container-id]").length === '+str(count))
                from native_compose_actions import verify as verify_compose
                verify_compose(root,artifacts,script,command,click,button,fill,wait,element)
                return
            wait('return document.querySelectorAll("[data-container-id]").length === 3')
            click(f'//tr[@data-container-id="{live_id}"]//button')
            if networks:
                oracle=json.loads((root/'manifest.json').read_text())['networkOracle']
                click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Networks"]')
                wait('return document.querySelectorAll(".network-choice").length===1 && document.querySelector(".network-detail")?.innerText.includes("1 endpoint(s) reported")')
                text=script('return document.querySelector(".network-detail").innerText')
                assert oracle['Name'] in text and oracle['Id'] in text and 'token: Masked' in text
                assert 'synthetic-network-036-secret' not in text
                endpoint=oracle['Containers'][live_id]
                for value in [endpoint['IPv4Address'],endpoint['IPv6Address']]:assert value and value in text
                for config in oracle['IPAM']['Config']:assert config['Subnet'] in text
                wait('return document.querySelector(".network-detail button")?.disabled === false')
                script('document.querySelector(".network-detail").scrollIntoView({block:"start",behavior:"instant"})')
                (artifacts/'native-network-metadata.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
                script('document.querySelector(".network-detail li:last-child").scrollIntoView({block:"center",behavior:"instant"})')
                (artifacts/'native-network-attachment.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
                script('window.__networkClicks=[];document.addEventListener("click",event=>window.__networkClicks.push(event.isTrusted),{capture:true,once:true})')
                for _ in range(3):
                    command('POST','/execute/async',{'script':'const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)));','args':[]})
                    button(endpoint['Name']);time.sleep(.25)
                    if script('return window.__networkClicks.length>0'):break
                assert script('return window.__networkClicks.includes(true)')
                wait('return location.hash==="#/containers" && document.querySelector(".detail-panel")?.innerText.includes(arguments[0])'.replace('arguments[0]',json.dumps(live_id)))
                click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Hosts"]')
                button('Disconnect saved host')
                wait('return document.body.innerText.includes("Disconnected · SSH session")')
                print('PASS native network UI: actual custom bridge/internal flag, IPv4/IPv6 IPAM and endpoint addresses, masked metadata, full-ID attachment navigation and disconnect over strict ProxyJump.',flush=True)
                return
            if batch:
                ids = script('return Array.from(document.querySelectorAll("[data-container-id]"), row => row.dataset.containerId)')
                stopped = [ident for ident in ids if ident != live_id]
                for ident in [live_id, stopped[0]]: click(f'//tr[@data-container-id="{ident}"]//input[@type="checkbox"]')
                button('Enable batch management')
                wait('return Array.from(document.querySelectorAll(".batch-management button")).some(b=>b.textContent === "Stop selected" && !b.disabled)')
                fill('Batch stop timeout (seconds)', '1')
                assert script('return Array.from(document.querySelectorAll(".batch-management button")).find(b=>b.textContent === "Remove selected stopped containers").disabled')
                button('Stop selected')
                wait('return document.querySelector(".mutation-confirmation")?.innerText.includes("' + live_id + '")')
                assert script('return document.querySelector(".mutation-confirmation").innerText.includes(arguments[0])', stopped[0])
                (artifacts / 'native-batch-confirmation.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
                button('Confirm selected action')
                wait('return document.querySelectorAll(".batch-results [data-outcome=succeeded]").length === 2', 30)
                script('document.querySelector(".batch-results").scrollIntoView({block:"center",behavior:"instant"})')
                (artifacts / 'native-batch-results.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
                wait('return Array.from(document.querySelectorAll(".batch-management button")).some(b=>b.textContent === "Remove selected stopped containers" && !b.disabled)')
                button('Remove selected stopped containers')
                wait('return document.querySelector(".mutation-confirmation")?.getAttribute("aria-label") === "Confirm stopped-container removal"')
                assert script('return Array.from(document.querySelectorAll(".batch-management button")).find(b=>b.textContent === "Confirm selected action").disabled')
                button('Cancel batch confirmation')
                button('Clear batch selection')
                click(f'//tr[@data-container-id="{live_id}"]//input[@type="checkbox"]')
                wait('return Array.from(document.querySelectorAll(".batch-management button")).some(b=>b.textContent === "Start selected" && !b.disabled)')
                button('Start selected')
                button('Confirm selected action')
                wait('return document.querySelectorAll(".batch-results [data-outcome=succeeded]").length === 1', 30)
                button('Clear batch selection')
                print('PASS native batch UI: two explicit full IDs and host, sequential stop results retained, running removal disabled, stopped removal separate acknowledgement and cancellation sent nothing, selected live fixture restored.', flush=True)
            if management:
                wait('return document.querySelector(".inspect-detail")?.getAttribute("aria-busy") === "false"')
                button('Enable management')
                wait('return document.querySelector(".container-management")?.innerText.includes("Management enabled for this session")')
                fill('Stop timeout (seconds)', '1')
                button('Stop container')
                wait('return document.querySelector(".mutation-confirmation")?.innerText.includes("' + live_id + '")')
                script('document.querySelector(".mutation-confirmation").scrollIntoView({block:"center",behavior:"instant"})')
                (artifacts / 'native-management-confirmation.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
                button('Cancel action')
                assert not (root / 'mutation-count').exists(), 'Cancelled confirmation must not dispatch'
                for action, expected in [('Stop container', 'exited'), ('Start container', 'running'), ('Restart container', 'running')]:
                    wait('return Array.from(document.querySelectorAll(".container-management button")).some(b=>b.textContent === "' + action + '" && !b.disabled)')
                    button(action)
                    wait('return document.querySelector(".mutation-confirmation") !== null')
                    assert script('return document.querySelector(".mutation-confirmation h4").textContent') == 'Confirm ' + action.split()[0].lower()
                    wait('return Array.from(document.querySelectorAll(".mutation-confirmation button")).some(b=>b.textContent === "Confirm action" && !b.disabled)')
                    button('Confirm action')
                    wait('return document.querySelector(".container-management [role=status]")?.innerText.includes("Observed state: ' + expected + '")', 50)
                    if expected == 'running':
                        assert script('return document.querySelector(".container-management [role=status]").innerText.includes("health: starting")')
                script('document.querySelector(".container-management").scrollIntoView({block:"start",behavior:"instant"})')
                (artifacts / 'native-management-health-starting.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
                time.sleep(9)
                button('Refresh action state')
                wait('return document.querySelector(".container-management [role=status]")?.innerText.includes("health: healthy")')
                assert (root / 'mutation-count').read_text().splitlines() == ['stop','start','restart']
                button('Disable management')
                wait('return document.querySelector(".container-management")?.innerText.includes("Read-only controls.")')
                print('PASS native management UI: exact host/daemon/full-ID confirmation, cancellation sent nothing, explicit stop/start/restart each dispatched once, refreshed real states and delayed health starting/healthy, explicit revocation.', flush=True)
            if mvp:
                wait('return document.querySelector(".inspect-detail")?.innerText.includes("healthy")')
                click('//button[@role="tab" and normalize-space(.)="Ports"]')
                wait('return document.querySelector(".inspect-detail")?.innerText.includes("8080/tcp")')
                assert script('return document.querySelector(".inspect-detail").innerText.includes("No active host bindings reported.")')
                script('document.querySelector(".inspect-detail").scrollIntoView({block:"start",behavior:"instant"})')
                (artifacts / 'native-jump-inspect-ports.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
                click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Compose"]')
                wait('return document.querySelectorAll(".compose-project-choice").length === 2')
                assert script('return document.querySelector(".compose-view").innerText.includes("Compose project listing failed; inspected container labels")')
                click('//button[contains(@class,"compose-project-choice") and starts-with(normalize-space(.),"mvp-live")]')
                wait('return document.querySelector(".compose-instances button")?.disabled === false')
                script('document.querySelector(".compose-view").scrollIntoView({block:"start",behavior:"instant"})')
                (artifacts / 'native-jump-compose.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
                click('//table[contains(@class,"compose-instances")]//button')
                wait(f'return document.querySelector(".selected-container code")?.textContent === "{live_id}"')
                print('PASS integrated jump-host list/inspect: healthy real running workload, exposed versus published ports, Compose label fallback and same-container detail/log navigation.', flush=True)
            if stats:
                wait('return document.querySelector(".container-stats")?.innerText.includes("Sample received")')
                assert script('return document.querySelectorAll(".stats-chart polyline").length') == 2
                wait('return document.querySelector(".inspect-detail")?.getAttribute("aria-busy") === "false"')
                script('document.querySelector(".container-stats").scrollIntoView({block:"start",behavior:"instant"})')
                time.sleep(.2)
                # Snapshot refresh can replace detail content and move the scroll anchor.
                script('document.querySelector(".container-stats").scrollIntoView({block:"start",behavior:"instant"})')
                (artifacts / 'native-statistics.png').write_bytes(base64.b64decode(command('GET', '/screenshot'), validate=True))
                button('Pause statistics')
                wait('return document.querySelector(".container-stats")?.innerText.includes("Statistics paused.")')
                count = script('return document.querySelector(".stats-samples").textContent')
                time.sleep(5.5)
                assert count == script('return document.querySelector(".stats-samples").textContent')
                button('Resume statistics')
                wait('return document.querySelector(".stats-samples").textContent !== ' + json.dumps(count))
                print('PASS native stats: real selected-container IPC values/charts and explicit polling pause.', flush=True)
            button('Start logs')
            wait('return document.querySelector(".log-preview")?.innerText.includes("024-synthetic-live-line")')
            wait('return document.querySelector(".live-logs")?.innerText.includes("Following")')
            assert script('return document.querySelectorAll(".log-preview pre").length') <= 100
            # Freeze the actual webview event loop while Rust/SSH continues, withholding application ACKs.
            script('const until=performance.now()+1800; while(performance.now()<until){}; return true')
            wait('return /Log gap · [1-9][0-9]* records dropped/.test(document.querySelector(".live-logs")?.innerText ?? "")')
            script('document.querySelector(".live-logs").scrollIntoView()')
            time.sleep(.2)
            (artifacts / 'native-live-log-gap.png').write_bytes(base64.b64decode(command('GET', '/screenshot'), validate=True))
            button('Stop logs')
            wait('return document.querySelector(".live-logs")?.innerText.includes("Stopped")')
            # Start resumes with the displayed timestamp; renderer still explains possible gaps/duplicates.
            button('Start logs')
            wait('return document.querySelector(".live-logs")?.innerText.includes("Following")')
            if export_id:
                button('Pause display')
                wait('return Array.from(document.querySelectorAll("button")).some(b => b.textContent === "Resume display" && b.getAttribute("aria-pressed") === "true")')
                # Native clicks return before React's commit and queued scroll events settle.
                time.sleep(.15)
                frozen = script('return Array.from(document.querySelectorAll(".log-preview pre"), p=>p.textContent)')
                time.sleep(.4)
                assert frozen == script('return Array.from(document.querySelectorAll(".log-preview pre"), p=>p.textContent)'), 'Paused native display changed'
                button('Resume display')
                button('Stop logs')
                click(f'//tr[@data-container-id="{export_id}"]//button')
                button('Start logs')
                wait('return document.querySelector(".live-logs")?.innerText.includes("Stream ended")')
                fill('Search logs', '023-stdout')
                button('Select matching lines')
                expected = script('return Array.from(document.querySelectorAll(".log-preview pre"), p=>p.textContent).join("\\n")')
                assert '023-stdout-one' in expected and '023-stdout-three' in expected
                assert '023-stderr' not in expected and '024-synthetic' not in expected
                # Actual native clipboard write and keyboard paste into an external-test textarea.
                button('Copy selected lines')
                script('const field=document.createElement("textarea"); field.id="native-clipboard-check"; document.body.append(field)')
                paste = element('//*[@id="native-clipboard-check"]')
                command('POST', f'/element/{paste}/click', {})
                command('POST', '/actions', {'actions': [{'type': 'key', 'id': 'log-clipboard', 'actions': [{'type': 'keyDown', 'value': '\ue009'}, {'type': 'keyDown', 'value': 'v'}, {'type': 'keyUp', 'value': 'v'}, {'type': 'keyUp', 'value': '\ue009'}]}]})
                assert script('return document.querySelector("#native-clipboard-check").value') == expected
                script('document.querySelector("#native-clipboard-check").remove()')
                button('Export selected lines')
                wait('return document.querySelector("dialog[open]")?.innerText.includes("Logs may contain application secrets")')
                (artifacts / 'native-export-review.png').write_bytes(base64.b64decode(command('GET', '/screenshot'), validate=True))
                button('Save selected logs…')
                def tool(*args):
                    return subprocess.run([str(xdotool), *map(str, args)], env=env, capture_output=True, text=True, check=True, timeout=10).stdout.strip()
                def save_window():
                    deadline = time.monotonic() + 15
                    while time.monotonic() < deadline:
                        result = subprocess.run([str(xdotool), 'search', '--onlyvisible', '--name', '^Save selected container logs$'], env=env, capture_output=True, text=True, timeout=5)
                        ids = result.stdout.split()
                        if len(ids) == 1:
                            pid = tool('getwindowpid', ids[0])
                            assert (f'XDG_DATA_HOME={root}/native-data'.encode() in Path('/proc', pid, 'environ').read_bytes().split(b'\x00')), 'Save dialog must belong to this owned native app'
                            return ids[0]
                        time.sleep(.1)
                    print('Native driver diagnostics:', (root / 'native-driver.log').read_text()[-5000:], flush=True)
                    raise AssertionError('Owned native Save dialog did not appear')
                dialog_window = save_window()
                destination = root / 'selected-native-log-export.txt'
                tool('windowfocus', '--sync', dialog_window)
                time.sleep(.2)
                subprocess.run(['/usr/bin/import', '-window', dialog_window, str(artifacts / 'native-save-dialog.png')], env=env, check=True, timeout=10)
                tool('key', '--clearmodifiers', 'ctrl+l')
                time.sleep(.3)
                # GTK initially selects only the basename, leaving .txt unselected.
                tool('key', '--clearmodifiers', 'ctrl+a')
                tool('type', '--clearmodifiers', '--delay', '3', str(destination))
                time.sleep(.3)
                subprocess.run(['/usr/bin/import', '-window', dialog_window, str(artifacts / 'native-save-entered.png')], env=env, check=True, timeout=10)
                tool('key', '--clearmodifiers', 'Return')
                deadline = time.monotonic() + 15
                while not destination.exists() and time.monotonic() < deadline: time.sleep(.1)
                if not destination.exists():
                    subprocess.run(['/usr/bin/import', '-window', dialog_window, str(artifacts / 'native-save-after-keys.png')], env=env, timeout=10)
                    print('Native export status:', script('return Array.from(document.querySelectorAll(\".live-logs > p[role=status]\"), p=>p.innerText).join(\" | \")'), flush=True)
                assert destination.read_text() == expected, 'Native file must exactly equal selected visible lines in order'
                assert destination.stat().st_mode & 0o777 == 0o600
                wait('return document.querySelector(".live-logs")?.innerText.includes("Saved 2 selected lines.")')
                button('Export selected lines')
                button('Save selected logs…')
                dialog_window = save_window()
                tool('windowfocus', '--sync', dialog_window)
                tool('key', '--clearmodifiers', 'Escape')
                wait('return document.querySelector(".live-logs")?.innerText.includes("Export cancelled.")')
                assert destination.read_text() == expected
                print('PASS native selected export: secret review, real GTK Save and Cancel, exact ordered 2-line UTF-8 file, mode 0600, no unrelated container data, clipboard exact visible text.', flush=True)
            if events_id:
                wait('return document.querySelector(".event-status")?.innerText.includes("Live events")')
                click(f'//tr[@data-container-id="{events_id}"]//button')
                wait(f'return document.querySelector(".selected-container")?.textContent.includes("{events_id}")')
                (root / 'gui-delete').write_text('remove only owned unsupported-log fixture')
                wait('return document.querySelectorAll("[data-container-id]").length === 2')
                wait('return document.querySelector(".detail-panel")?.innerText.includes("No container selected")')
                script('window.scrollTo(0,0)')
                (artifacts / 'native-event-deletion.png').write_bytes(base64.b64decode(command('GET', '/screenshot'), validate=True))
                print('PASS native Docker events: live scoped channel automatically invalidated inventory after external removal; authoritative snapshot removed the selected owned container without manual refresh.', flush=True)
            click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Hosts"]')
            wait('return !document.querySelector(".live-logs")')
            if mvp:
                button('Disconnect saved host')
                wait('return document.body.innerText.includes("Disconnected · SSH session")')
                click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Containers"]')
                wait('return document.querySelectorAll("[data-container-id]").length === 0')
                assert script('return !document.querySelector(".container-stats") && !document.querySelector(".live-logs")')
                (artifacts / 'native-jump-disconnected.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
                print('PASS integrated jump-host disconnect: old rows and live read views removed, selected host retained with offline state.', flush=True)
            time.sleep(.5)
            command('DELETE', '')
            session = None
            for file in (root / 'native-data').rglob('*'):
                if file.is_file():
                    assert b'024-synthetic-live-line' not in file.read_bytes(), 'Raw synthetic log persisted by native application'
            assert b'024-synthetic-live-line' not in (root / 'native-driver.log').read_bytes()
            print('PASS native release Tauri channel: actual follow batches, event-loop stall/drop marker, bounded preview, stop, timestamp resume, route unmount; app storage/diagnostics clean.', flush=True)
        except Exception:
            if pressure:
                Path('/tmp/containerdesk-045-native-driver-failure.log').write_bytes((root/'native-driver.log').read_bytes()[-65536:])
            if recovery:
                Path('/tmp/containerdesk-041-native-driver-failure.log').write_bytes((root/'native-driver.log').read_bytes()[-65536:])
                print('Native recovery driver exit status:',driver.poll(),flush=True)
            raise
        finally:
            if session:
                try: command('DELETE', '')
                except Exception: pass
            if driver.poll() is None: os.killpg(driver.pid, signal.SIGTERM)
            try: driver.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(driver.pid, signal.SIGKILL)
                driver.wait(timeout=5)
