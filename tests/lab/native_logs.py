"""Actual release Tauri channel journey using only the owned loopback log lab."""
import base64
import json
import os
from pathlib import Path
import signal
import subprocess
import time
import urllib.request
from native_ssh import unused_port, ELEMENT, REPO


def verify(root, tauri_driver, webkit_driver, config, live_id, artifacts, export_id=None, xdotool=None, stats=False, events_id=None):
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
            command('POST', f'/element/{node}/click', {})
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
            print('Native state:', script('return {dialogs: document.querySelectorAll("dialog[open]").length, buttons: Array.from(document.querySelectorAll(".live-logs button"), b=>({text:b.textContent,disabled:b.disabled})), status: Array.from(document.querySelectorAll(".live-logs > p[role=status]"), p=>p.textContent)}'), flush=True)
            raise AssertionError('Native log condition did not become true: ' + condition)
        try:
            for _ in range(100):
                try:
                    request('GET', '/status')
                    break
                except OSError: time.sleep(.1)
            result = request('POST', '/session', {'capabilities': {'alwaysMatch': {'browserName': 'wry', 'tauri:options': {'application': str(REPO / 'src-tauri/target/release/containerdesk')}}}})
            session = result['sessionId']
            command('POST', '/timeouts', {'implicit': 5000, 'script': 5000, 'pageLoad': 15000})
            if stats and xdotool:
                found = subprocess.run([str(xdotool), 'search', '--onlyvisible', '--name', '^ContainerDesk$'], env=env, capture_output=True, text=True, check=True, timeout=10).stdout.split()
                owned_windows = []
                for window in found:
                    pid = subprocess.run([str(xdotool), 'getwindowpid', window], env=env, capture_output=True, text=True, check=True, timeout=10).stdout.strip()
                    if f'XDG_DATA_HOME={root}/native-data'.encode() in Path('/proc', pid, 'environ').read_bytes().split(b'\x00'):
                        owned_windows.append(window)
                assert len(owned_windows) == 1
                subprocess.run([str(xdotool), 'windowfocus', '--sync', owned_windows[0]], env=env, check=True, timeout=10)
            button('Add host')
            button('New host')
            fill('Host SSH config path', str(config))
            fill('Host SSH alias', 'logs-owned')
            fill('Display name', 'Owned live log checkpoint')
            fill('Saved Docker executable', '/usr/bin/docker')
            button('Save host')
            button('Connect saved host')
            wait('return document.body.innerText.includes("Ready · Read-only session")')
            click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Containers"]')
            wait('return document.querySelectorAll("[data-container-id]").length === 3')
            click(f'//tr[@data-container-id="{live_id}"]//button')
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
            time.sleep(.5)
            command('DELETE', '')
            session = None
            for file in (root / 'native-data').rglob('*'):
                if file.is_file():
                    assert b'024-synthetic-live-line' not in file.read_bytes(), 'Raw synthetic log persisted by native application'
            assert b'024-synthetic-live-line' not in (root / 'native-driver.log').read_bytes()
            print('PASS native release Tauri channel: actual follow batches, event-loop stall/drop marker, bounded preview, stop, timestamp resume, route unmount; app storage/diagnostics clean.', flush=True)
        finally:
            if session:
                try: command('DELETE', '')
                except Exception: pass
            if driver.poll() is None: os.killpg(driver.pid, signal.SIGTERM)
            try: driver.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(driver.pid, signal.SIGKILL)
                driver.wait(timeout=5)
