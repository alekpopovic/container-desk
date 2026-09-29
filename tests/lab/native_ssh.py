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


def verify(root, tauri_driver, webkit_driver, config, engine, artifacts=None, inventory=False, inspect=False, compose=False):
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
            script('arguments[0].scrollIntoView({block:"center",behavior:"instant"})', {ELEMENT:node})
            time.sleep(.15)
            command("POST", f"/element/{node}/click", {})

        def tab(text):
            node = element(f'//button[@role="tab" and normalize-space(.)="{text}"]')
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
                add(alias, "Checkpoint " + alias, "/opt/fixture/docker-with-compose" if compose and alias=="direct-known" else "/usr/bin/docker")
                button("Connect saved host")
                text = wait_text("Ready · SSH session")
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
                        assert "Not configured" in text
                        copy_node = element('//button[@aria-label="Copy container id"]')
                        command("POST", f"/element/{copy_node}/click", {})
                        wait_inspect("Copied container id.")
                        expected_id = script('return document.querySelector(".selected-container code").textContent')
                        search = element('//label[normalize-space(text())="Search containers"]/input')
                        command("POST", f"/element/{search}/click", {})
                        command("POST", "/actions", {"actions": [{"type": "key", "id": "clipboard-check", "actions": [
                            {"type": "keyDown", "value": "\ue009"}, {"type": "keyDown", "value": "v"},
                            {"type": "keyUp", "value": "v"}, {"type": "keyUp", "value": "\ue009"}
                        ]}]})
                        command("DELETE", "/actions")
                        assert script('return document.querySelector("input[type=search]").value') == expected_id, "Native paste must contain the copied safe ID"
                        command("POST", f"/element/{search}/clear", {})
                        tab("Ports")
                        text = wait_inspect("Exposed container ports")
                        assert "8080/tcp" in text and "No active host bindings reported." in text
                        tab("Environment")
                        text = wait_inspect("Sensitive values masked")
                        assert "CHECKPOINT_TOKEN" in text
                        assert "synthetic-inspect-021-secret" not in text
                        assert "synthetic-label-021-secret" not in text
                        button("Reveal sensitive values")
                        text = wait_inspect("Sensitive values revealed")
                        assert "synthetic-inspect-021-secret" in text
                        tab("Labels")
                        text = inspect_text()
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
                        print(f"PASS native inspect {alias}: tabbed real fields, not-configured health, exposure versus bindings, clipboard ID readback, Rust redaction, reveal/hide", flush=True)
                    if compose:
                        node = element('//nav[@aria-label="Resources"]//a[normalize-space(.)="Compose"]')
                        command("POST", f"/element/{node}/click", {})
                        deadline = time.monotonic() + 20
                        while time.monotonic() < deadline:
                            if script('return document.querySelectorAll(".compose-project-choice").length') == 2: break
                            time.sleep(.1)
                        else: raise AssertionError('Native Compose projects did not load')
                        text = script('return document.querySelector(".compose-view").innerText')
                        assert ('Remote Compose plugin available.' if alias=='direct-known' else 'Remote Compose plugin absent;') in text
                        assert '/remote/nonexistent/compose.yml' in text and 'unverified' in text
                        assert 'web' in text
                        script('document.querySelector(".compose-view").scrollIntoView({block:"start",behavior:"instant"})')
                        (artifacts / f"native-compose-{alias}.png").write_bytes(base64.b64decode(command("GET", "/screenshot"), validate=True))
                        node = element('//button[contains(@class,"compose-project-choice") and starts-with(normalize-space(.),"checkpoint-b")]')
                        command("POST", f"/element/{node}/click", {})
                        deadline = time.monotonic() + 15
                        while time.monotonic() < deadline:
                            if script('return Array.from(document.querySelectorAll(".compose-instances button")).some(b=>b.textContent.trim()==="listing-second" && !b.disabled)'): break
                            time.sleep(.1)
                        else: raise AssertionError('Selected Compose instance did not become available in the authoritative inventory')
                        button('listing-second')
                        deadline = time.monotonic() + 15
                        while time.monotonic() < deadline:
                            if script('return document.querySelector(".detail-panel")?.innerText.includes("listing-second")'): break
                            time.sleep(.1)
                        else:
                            print('Compose navigation state:', script('return {hash:location.hash,selected:document.querySelector(".selected-container code")?.textContent,project:document.querySelector(".compose-project h3")?.textContent,buttons:Array.from(document.querySelectorAll(".compose-instances button"),b=>({name:b.textContent,disabled:b.disabled}))}'), flush=True)
                            (artifacts/'native-compose-navigation-timeout.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
                            raise AssertionError('Compose instance did not open existing container detail view')
                        assert script('return !!document.querySelector(".live-logs")')
                        print(f'PASS native Compose {alias}: two projects/same web service, plugin/fallback status, unverified missing remote path and shared container detail/log navigation.', flush=True)
                    node = element('//nav[@aria-label="Resources"]//a[normalize-space(.)="Hosts"]')
                    command("POST", f"/element/{node}/click", {})
                else:
                    node = element('//nav[@aria-label="Resources"]//a[normalize-space(.)="Containers"]')
                    command("POST", f"/element/{node}/click", {})
                    deadline = time.monotonic() + 20
                    while time.monotonic() < deadline:
                        if script('return document.body.innerText.includes("No containers in this snapshot") && !!document.querySelector("time")'): break
                        time.sleep(.1)
                    else: raise AssertionError('Native empty daemon did not show a successful empty snapshot')
                    assert script('return document.querySelectorAll("[data-container-id]").length') == 0
                    (artifacts / f"native-empty-{alias}.png").write_bytes(base64.b64decode(command("GET", "/screenshot"), validate=True))
                    print(f"PASS native empty daemon {alias}: successful snapshot timestamp, zero rows and explicit empty state.", flush=True)
                    node = element('//nav[@aria-label="Resources"]//a[normalize-space(.)="Hosts"]')
                    command("POST", f"/element/{node}/click", {})
                button("Disconnect saved host")
                wait_text("Disconnected · SSH session")
                assert engine["engineId"] not in details()
                print(f"PASS native UI {alias}: actual Engine identity/version, {route}, readonly, disconnect; PATH=/nonexistent", flush=True)
            for alias, expected in (("direct-unknown", "not trusted"), ("direct-changed", "changed"),
                                    ("via-jump-absent", "Authentication")):
                add(alias, "Checkpoint " + alias)
                button("Connect saved host")
                wait_text("Connection error · SSH session")
                assert expected.lower() in details().lower(), details()
                (artifacts / f"native-denied-{alias}.png").write_bytes(base64.b64decode(command("GET", "/screenshot"), validate=True))
                button("Disconnect saved host")
                wait_text("Disconnected · SSH session")
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
            wait_text("Disconnected · SSH session")
            assert time.monotonic() - started < 3, "cancel must not await the remote 30 second sleep"
            fill("Saved Docker executable", "/usr/bin/docker")
            fill("Display name", "Checkpoint reconnect")
            button("Save host changes")
            wait_text("Checkpoint reconnect")
            button("Connect saved host")
            wait_text("Ready · SSH session")
            assert engine["engineId"] in details()
            button("Disconnect saved host")
            wait_text("Disconnected · SSH session")
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
