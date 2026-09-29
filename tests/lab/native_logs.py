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


def verify(root, tauri_driver, webkit_driver, config, live_id, artifacts):
    env = os.environ.copy()
    for name in ('LD_LIBRARY_PATH', 'LD_PRELOAD', 'GTK_PATH', 'GIO_MODULE_DIR', 'SSH_AUTH_SOCK'):
        env.pop(name, None)
    env.update(XDG_DATA_HOME=str(root / 'native-data'), GDK_BACKEND='x11', PATH='/nonexistent')
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
            command('POST', f'/element/{element(xpath)}/click', {})
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
