"""Native Linux WebDriver journey against the owned live SSH lab, with no IPC mocks.
The external drivers are development tools, never dependencies of the packaged app.
"""
import base64
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import time
import urllib.error
import urllib.request

REPO = Path(__file__).resolve().parents[2]
ELEMENT = "element-6066-11e4-a52e-4f735466cecf"


def unused_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def verify(root, tauri_driver, webkit_driver, config, engine, artifacts=None, inventory=False, inspect=False):
    env = os.environ.copy()
    for key in ("LD_LIBRARY_PATH", "LD_PRELOAD", "GTK_PATH", "GIO_MODULE_DIR", "SSH_AUTH_SOCK"):
        env.pop(key, None)
    env.update(XDG_DATA_HOME=str(root / "native-data"), GDK_BACKEND="x11", PATH="/nonexistent")
    port, native_port = unused_port(), unused_port()
    while native_port == port:
        native_port = unused_port()
    artifacts = artifacts or root / "native-artifacts"
    artifacts.mkdir(parents=True, exist_ok=True)
    session = None
    with (root / "native-driver.log").open("w") as log:
        driver = subprocess.Popen([str(tauri_driver), "--port", str(port), "--native-port", str(native_port),
                                   "--native-driver", str(webkit_driver)], env=env, stdout=log, stderr=log, start_new_session=True)

        def request(method, path, data=None):
            body = None if data is None else json.dumps(data).encode()
            req = urllib.request.Request(f"http://127.0.0.1:{port}{path}", data=body, method=method,
                                         headers={"Content-Type": "application/json"})
            try:
                with urllib.request.urlopen(req, timeout=30) as response:
                    return json.load(response)["value"]
            except urllib.error.HTTPError as error:
                raise RuntimeError(error.read().decode()[:4096]) from None

        def command(method, path, data=None):
            return request(method, f"/session/{session}{path}", data)

        def script(code, *args):
            return command("POST", "/execute/sync", {"script": code, "args": list(args)})

        def element(xpath):
            return command("POST", "/element", {"using": "xpath", "value": xpath})[ELEMENT]

        def button(text):
            node = element(f'//button[normalize-space(.)="{text}"]')
            command("POST", f"/element/{node}/click", {})

        def fill(label, value):
            node = element(f'//label[normalize-space(text())="{label}"]/input')
            command("POST", f"/element/{node}/clear", {})
            command("POST", f"/element/{node}/value", {"text": value})

        def details():
            return script('return document.querySelector("[aria-label=\\"Selected saved host\\"]")?.innerText ?? ""')

        def wait_text(expected, timeout=20):
            deadline = time.monotonic() + timeout
            while time.monotonic() < deadline:
                value = details()
                if expected in value:
                    return value
                time.sleep(0.1)
            raise AssertionError(f"Native details never showed {expected!r}: {value}")

        def add(alias, name, docker="/usr/bin/docker"):
            button("New host")
            fill("Host SSH config path", str(config))
            fill("Host SSH alias", alias)
            fill("Display name", name)
            fill("Saved Docker executable", docker)
            button("Save host")
            wait_text(name)

        try:
            for attempt in range(100):
                if driver.poll() is not None:
                    raise RuntimeError("native driver exited before startup")
                try:
                    with socket.create_connection(("127.0.0.1", port), timeout=0.1):
                        break
                except OSError:
                    if attempt == 99:
                        raise
                    time.sleep(0.1)
            result = request("POST", "/session", {"capabilities": {"alwaysMatch": {
                "browserName": "wry", "tauri:options": {"application": str(REPO / "src-tauri/target/release/containerdesk")},
            }}})
            session = result["sessionId"]
            command("POST", "/timeouts", {"implicit": 5000, "script": 5000, "pageLoad": 15000})
            button("Add host")
            for alias, route in (("direct-known", "Direct"), ("via-known", "jump-known")):
                add(alias, "Checkpoint " + alias)
                button("Connect saved host")
                text = wait_text("Ready · Read-only session")
                for expected in (route, engine["engineVersion"], engine["engineId"], "unix:///var/run/docker.sock"):
                    assert expected in text, f"Missing {expected!r} in native details"
                script('document.querySelector("[aria-label=\\"Selected saved host\\"]").scrollIntoView()')
                screenshot = command("GET", "/screenshot")
                artifact = artifacts / f"native-{alias}.png"
                artifact.parent.mkdir(exist_ok=True)
                artifact.write_bytes(base64.b64decode(screenshot, validate=True))
                if inventory:
                    node = element('//nav[@aria-label="Resources"]//a[normalize-space(.)="Containers"]')
                    command("POST", f"/element/{node}/click", {})
                    deadline = time.monotonic() + 20
                    while time.monotonic() < deadline:
                        if script('return document.querySelectorAll("[data-container-id]").length') == 2:
                            break
                        time.sleep(.1)
                    else:
                        raise AssertionError("Native live container table did not load two rows")
                    button("listing-first")
                    assert "Selected container" in script('return document.querySelector(".detail-panel").innerText')
                    previous_read = script('return document.querySelector("time").dateTime')
                    button("Refresh containers")
                    deadline = time.monotonic() + 15
                    while time.monotonic() < deadline:
                        if script('return document.querySelector("time").dateTime') != previous_read:
                            break
                        time.sleep(.1)
                    else:
                        raise AssertionError("Native refresh did not record a new successful timestamp")
                    assert "listing-first" in script('return document.querySelector(".detail-panel").innerText')
                    (artifacts / f"native-list-{alias}.png").write_bytes(base64.b64decode(command("GET", "/screenshot"), validate=True))
                    print(f"PASS native live table {alias}: two real containers, row selection and refresh", flush=True)
                    if inspect:
                        def inspect_text():
                            return script('return document.querySelector(".inspect-detail")?.innerText ?? ""')
                        def wait_inspect(expected):
                            deadline = time.monotonic() + 15
                            while time.monotonic() < deadline:
                                text = inspect_text()
                                if expected in text:
                                    return text
                                time.sleep(.1)
                            raise AssertionError("Native inspect did not reach expected state")
                        text = wait_inspect("Sensitive values masked")
                        assert "CHECKPOINT_TOKEN" in text
                        assert "synthetic-inspect-021-secret" not in text
                        assert "synthetic-label-021-secret" not in text
                        button("Reveal sensitive values")
                        text = wait_inspect("Sensitive values revealed")
                        assert "synthetic-inspect-021-secret" in text
                        assert "synthetic-label-021-secret" in text
                        button("Hide sensitive values")
                        text = wait_inspect("Sensitive values masked")
                        assert "synthetic-inspect-021-secret" not in text
                        assert "synthetic-label-021-secret" not in text
                        # Cover multiple host-inventory poll ticks and wait for actual native paint.
                        stable_until = time.monotonic() + 2.2
                        while time.monotonic() < stable_until:
                            text = inspect_text()
                            assert "Sensitive values masked" in text, "Unchanged inventory must not reload details"
                            assert "synthetic-inspect-021-secret" not in text
                            time.sleep(.1)
                        script("Array.from(document.querySelectorAll('button')).find(b => b.textContent.trim() === 'Reveal sensitive values').scrollIntoView({block: 'center'})")
                        command("POST", "/execute/async", {"script": "const done = arguments[arguments.length - 1]; requestAnimationFrame(() => requestAnimationFrame(() => done(true)));", "args": []})
                        (artifacts / f"native-inspect-{alias}.png").write_bytes(base64.b64decode(command("GET", "/screenshot"), validate=True))
                        print(f"PASS native inspect {alias}: real fields, Rust default redaction, explicit reveal and hide", flush=True)
                    node = element('//nav[@aria-label="Resources"]//a[normalize-space(.)="Hosts"]')
                    command("POST", f"/element/{node}/click", {})
                button("Disconnect saved host")
                wait_text("Disconnected · Read-only session")
                assert engine["engineId"] not in details()
                print(f"PASS native UI {alias}: actual Engine identity/version, {route}, readonly, disconnect; PATH=/nonexistent", flush=True)
            for alias, expected in (("direct-unknown", "not trusted"), ("direct-changed", "changed"),
                                    ("via-jump-absent", "Authentication")):
                add(alias, "Checkpoint " + alias)
                button("Connect saved host")
                wait_text("Connection error · Read-only session")
                assert expected.lower() in details().lower(), details()
                button("Disconnect saved host")
                wait_text("Disconnected · Read-only session")
                print(f"PASS native UI {alias}: bounded diagnostic, explicit disconnect", flush=True)
            button("Checkpoint via-known · via-known")
            fill("Display name", "Checkpoint cancel")
            fill("Saved Docker executable", "/opt/fixture/docker-hang")
            button("Save host changes")
            wait_text("Checkpoint cancel")
            button("Connect saved host")
            wait_text("Checking remote capabilities")
            started = time.monotonic()
            button("Disconnect saved host")
            wait_text("Disconnected · Read-only session")
            assert time.monotonic() - started < 3, "cancel must not await the remote 30 second sleep"
            fill("Saved Docker executable", "/usr/bin/docker")
            fill("Display name", "Checkpoint reconnect")
            button("Save host changes")
            wait_text("Checkpoint reconnect")
            button("Connect saved host")
            wait_text("Ready · Read-only session")
            assert engine["engineId"] in details()
            button("Disconnect saved host")
            wait_text("Disconnected · Read-only session")
            print("PASS native UI cancellation and explicit reconnect: old probe cannot populate new session", flush=True)
        finally:
            if session:
                try:
                    request("DELETE", f"/session/{session}")
                except (OSError, RuntimeError):
                    pass
            if driver.poll() is None:
                os.killpg(driver.pid, signal.SIGTERM)
                driver.wait(timeout=10)

    if inspect:
        for path in [root / "native-driver.log", *(root / "native-data").rglob("*")]:
            if path.is_file():
                data = path.read_bytes()
                assert b"synthetic-inspect-021-secret" not in data, "Environment value leaked to app logs/storage"
                assert b"synthetic-label-021-secret" not in data, "Label value leaked to app logs/storage"
        print("PASS native logs and persisted app files contain no synthetic inspect values", flush=True)
