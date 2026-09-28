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


def verify(root, tauri_driver, webkit_driver, config, engine):
    env = os.environ.copy()
    for key in ("LD_LIBRARY_PATH", "LD_PRELOAD", "GTK_PATH", "GIO_MODULE_DIR", "SSH_AUTH_SOCK"):
        env.pop(key, None)
    env.update(XDG_DATA_HOME=str(root / "native-data"), GDK_BACKEND="x11", PATH="/nonexistent")
    port, native_port = unused_port(), unused_port()
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
                artifact = REPO / "docs/checkpoints" / f"018-native-{alias}.png"
                artifact.parent.mkdir(exist_ok=True)
                artifact.write_bytes(base64.b64decode(screenshot, validate=True))
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
