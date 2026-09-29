"""050 production binary must ignore native-automation/inspector activation variables."""
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import time
import urllib.error
import urllib.request
from native_ssh import REPO, unused_port
from native_launch import close_window


def verify(root, tools, artifacts):
    artifacts.mkdir(parents=True, exist_ok=True)
    binary = REPO / 'src-tauri/target/release/containerdesk'
    xdotool = tools / 'xdotool/usr/bin/xdotool'
    ports = set()
    while len(ports) < 5:
        ports.add(unused_port())
    driver_port, native_port, embedded_port, inspector_port, inspector_http = ports
    activation = {'TAURI_WEBVIEW_AUTOMATION': 'true', 'TAURI_AUTOMATION': 'true',
                  'TAURI_WEBDRIVER_PORT': str(embedded_port),
                  'WEBKIT_INSPECTOR_SERVER': f'127.0.0.1:{inspector_port}',
                  'WEBKIT_INSPECTOR_HTTP_SERVER': f'127.0.0.1:{inspector_http}'}
    env = {**os.environ, **activation, 'PATH': '/nonexistent', 'XDG_DATA_HOME': str(root / 'guard-data')}
    with (root / 'guard-driver.log').open('w') as log:
        driver = subprocess.Popen([str(tools/'bin/tauri-driver'), '--port', str(driver_port), '--native-port', str(native_port),
                                   '--native-driver', str(tools/'webkit/usr/bin/WebKitWebDriver')],
                                  env=env, stdout=log, stderr=log, start_new_session=True)
        app_pid = None
        try:
            for _ in range(100):
                try:
                    with socket.create_connection(('127.0.0.1', driver_port), timeout=.1): break
                except OSError: time.sleep(.1)
            def attempt():
                body = json.dumps({'capabilities': {'alwaysMatch': {'browserName': 'wry', 'tauri:options': {'application': str(binary)}}}}).encode()
                req = urllib.request.Request(f'http://127.0.0.1:{driver_port}/session', data=body, headers={'Content-Type': 'application/json'})
                try:
                    with urllib.request.urlopen(req, timeout=20) as response:
                        return json.load(response).get('value', {})
                except urllib.error.HTTPError as error:
                    value = json.load(error).get('value', {})
                    return {'error': value.get('error', 'http_error')}
                except TimeoutError:
                    return {'error': 'client_deadline_20_seconds'}
            with concurrent.futures.ThreadPoolExecutor(max_workers=1) as pool:
                pending = pool.submit(attempt)
                deadline = time.monotonic() + 15
                window = None
                while time.monotonic() < deadline:
                    found = subprocess.run([str(xdotool), 'search', '--onlyvisible', '--name', '^ContainerDesk$'],
                                           env=env, capture_output=True, text=True, timeout=3).stdout.split()
                    for candidate in found:
                        pid = int(subprocess.run([str(xdotool), 'getwindowpid', candidate], env=env, check=True, capture_output=True, text=True, timeout=3).stdout)
                        if Path(f'/proc/{pid}/exe').resolve() == binary.resolve():
                            app_pid, window = pid, candidate
                            break
                    if window: break
                    assert driver.poll() is None
                    time.sleep(.1)
                assert window, 'Production binary did not open a real window under automation attempt'
                time.sleep(2)
                owned = {app_pid}
                while True:
                    previous = owned.copy()
                    for entry in Path('/proc').iterdir():
                        if not entry.name.isdigit(): continue
                        try:
                            rows = dict(line.split(':', 1) for line in (entry/'status').read_text().splitlines() if ':' in line)
                            if int(rows['PPid']) in owned: owned.add(int(entry.name))
                        except (OSError, ValueError, KeyError): pass
                    if owned == previous: break
                inspected = []
                socket_ids = set()
                for pid in owned:
                    try:
                        args = Path(f'/proc/{pid}/cmdline').read_bytes()
                        if b'WebKit' in args:
                            child_env = Path(f'/proc/{pid}/environ').read_bytes().split(b'\0')
                            assert not any(item.startswith(key.encode() + b'=') for item in child_env for key in activation)
                            inspected.append(pid)
                        for fd in Path(f'/proc/{pid}/fd').iterdir():
                            target = os.readlink(fd)
                            if target.startswith('socket:['): socket_ids.add(target[8:-1])
                    except (FileNotFoundError, ProcessLookupError): pass
                assert inspected, 'No actual WebKit child observed'
                listeners = []
                for network in ('tcp', 'tcp6'):
                    for line in Path('/proc/net', network).read_text().splitlines()[1:]:
                        columns = line.split()
                        if columns[3] == '0A' and columns[9] in socket_ids: listeners.append(columns[1])
                assert not listeners, 'Production app or WebKit child owns a TCP listener'
                outcome = pending.result(timeout=22)
                assert outcome.get('error') and 'sessionId' not in outcome, 'Production binary accepted a native WebDriver session'
                subprocess.run(['/usr/bin/import', '-window', window, str(artifacts/'production-window.png')], env=env, check=True, timeout=10)
                close_window(window)
                deadline = time.monotonic() + 10
                while Path(f'/proc/{app_pid}').exists() and time.monotonic() < deadline: time.sleep(.05)
                assert not Path(f'/proc/{app_pid}').exists()
                result = {'productionSha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
                          'realWindowOpened': True, 'forcedAutomationVariablesIgnored': sorted(activation),
                          'webKitChildrenWithSanitizedEnvironment': len(inspected), 'applicationTcpListeners': 0,
                          'webdriverRejected': outcome['error'], 'nativeWindowClosed': True}
                (artifacts/'production-guard.json').write_text(json.dumps(result, indent=2) + '\n')
                print('PASS 050 production: real window, forced automation variables removed before WebKit startup, zero app TCP listeners, no WebDriver session, normal native close.', flush=True)
        finally:
            if app_pid and Path(f'/proc/{app_pid}/exe').resolve() == binary.resolve():
                try: os.kill(app_pid, signal.SIGTERM)
                except ProcessLookupError: pass
            if driver.poll() is None: os.killpg(driver.pid, signal.SIGTERM)
            driver.wait(timeout=10)
