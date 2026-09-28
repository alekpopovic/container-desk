#!/usr/bin/env python3
"""Synthetic Docker API for probe parsing through the real remote CLI. NOT an Engine.
Only /_ping, /version and /info exist. No host socket, workloads or secrets are exposed.
"""
import http.server
import json
import os
import socket
import socketserver
import subprocess
import threading
import time
from pathlib import Path

class Server(socketserver.ThreadingMixIn, socketserver.UnixStreamServer):
    daemon_threads = True

class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def do_HEAD(self):
        self.do_GET()

    def do_GET(self):
        path = self.path.rsplit("/", 1)[-1]
        if path == "_ping":
            data = b"OK"
        elif path == "version":
            data = json.dumps({"Version": "29.0.0-fixture", "ApiVersion": "1.51", "MinAPIVersion": "1.24", "Os": self.server.os_type, "Arch": "amd64", "GitCommit": "fixture", "GoVersion": "go1.24.0", "KernelVersion": "fixture"}).encode()
        elif path == "info":
            data = json.dumps({"ID": "containerdesk-api-fixture", "OSType": self.server.os_type, "SecurityOptions": ["name=rootless"] if self.server.rootless else [], "ServerVersion": "29.0.0-fixture", "Name": "synthetic", "Driver": "vfs", "NCPU": 1, "MemTotal": 1048576}).encode()
        else:
            self.send_error(404)
            return
        self.send_response(200)
        self.send_header("API-Version", "1.51")
        self.send_header("OSType", self.server.os_type)
        self.send_header("Content-Type", "application/json" if path != "_ping" else "text/plain")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(data)

for name, os_type, rootless in [("rootless", "linux", True), ("windows", "windows", False)]:
    path = f"/tmp/probe-{name}.sock"
    server = Server(path, Handler)
    server.os_type = os_type
    server.rootless = rootless
    os.chmod(path, 0o666)
    threading.Thread(target=server.serve_forever, daemon=True).start()
denied = socket.socket(socket.AF_UNIX)
denied.bind("/tmp/probe-denied.sock")
denied.listen(1)
os.chmod("/tmp/probe-denied.sock", 0o600)  # root-owned; SSH lab user cannot access it
config = Path("/tmp/probe-docker-config")
config.mkdir(mode=0o755)
subprocess.run(["/usr/bin/docker", "--config", str(config), "context", "create", "rootless", "--docker", "host=unix:///tmp/probe-rootless.sock"], check=True, stdout=subprocess.DEVNULL)
Path("/tmp/probe-ready").touch()
while True:
    time.sleep(60)
