#!/usr/bin/env python3
"""Disposable native SSH acceptance. Requires local Docker, cargo and OpenSSH; no user keys.
Only labelled app-owned containers and an internal network are created and removed.
No ports are published; the local host reaches only the owned bridge addresses.
"""
import hashlib
import ipaddress
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time

REPO = Path(__file__).resolve().parents[2]
IMAGE = "containerdesk-ssh-lab:016"
BASE = "alpine@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce"


def run(args, **kwargs):
    return subprocess.run(args, check=True, text=True, timeout=kwargs.pop("timeout", 30), **kwargs)


def main():
    with tempfile.TemporaryDirectory(prefix="containerdesk-auth-lab-") as directory:
        root = Path(directory)
        docker_config = root / "docker-client"
        docker_config.mkdir()
        env = os.environ.copy()
        for key in ("DOCKER_CONTEXT", "DOCKER_HOST", "DOCKER_TLS", "DOCKER_TLS_VERIFY", "DOCKER_CERT_PATH"):
            env.pop(key, None)
        docker = ["docker", "--config", str(docker_config), "--host", "unix:///var/run/docker.sock"]

        def dc(*args, **kwargs):
            return run(docker + list(args), env=env, **kwargs)

        dc("build", "--build-arg", f"LAB_BASE_IMAGE={BASE}", "--label", "dev.containerdesk.lab=013",
           "-t", IMAGE, "-f", str(REPO / "tests/lab/ssh-auth.Dockerfile"), str(REPO / "tests/lab"), timeout=180)
        names = []
        network = "containerdesk-auth-" + root.name.rsplit("-", 1)[-1]
        agent = None
        network_created = False
        try:
            for name in ("plain", "encrypted", "forced-failure", "jump-host", "target-host", "wrong-host"):
                run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "disposable-lab-only" if name == "encrypted" else "", "-f", str(root / name)], stdout=subprocess.DEVNULL)
            authorized = root / "authorized_keys"
            authorized.write_text((root / "plain.pub").read_text() + (root / "encrypted.pub").read_text() + 'command="exit 7" ' + (root / "forced-failure.pub").read_text())
            authorized.chmod(0o644)
            # Docker's default address pool can be exhausted by unrelated existing networks.
            # Select a small unused subnet without modifying/removing any of those networks.
            networks = dc("network", "ls", "-q", capture_output=True).stdout.split()
            occupied = []
            for network_id in networks:
                entries = json.loads(dc("network", "inspect", network_id, "--format", "{{json .IPAM.Config}}", capture_output=True).stdout)
                occupied.extend(ipaddress.ip_network(entry["Subnet"]) for entry in entries or [] if entry.get("Subnet"))
            routes = json.loads(run(["ip", "-json", "route", "show", "table", "all"], capture_output=True).stdout)
            occupied.extend(ipaddress.ip_network(route["dst"], strict=False) for route in routes if route.get("dst") not in (None, "default", "0.0.0.0/0"))
            subnet = next((ipaddress.ip_network(f"10.203.{n}.0/28") for n in range(256)
                           if not any(ipaddress.ip_network(f"10.203.{n}.0/28").overlaps(existing) for existing in occupied if existing.version == 4)), None)
            if subnet is None:
                raise RuntimeError("No unused small lab subnet found; unrelated networks preserved")
            dc("network", "create", "--internal", "--subnet", str(subnet), "--label", "dev.containerdesk.lab=013", network, stdout=subprocess.DEVNULL)
            network_created = True
            ports = {}
            addresses = {}
            for role in ("jump", "target"):
                config = root / f"sshd-{role}.conf"
                config.write_text("Port 22\nHostKey /lab/host_key\nAuthorizedKeysFile /lab/authorized_keys\n"
                                  "PidFile /tmp/sshd.pid\nPermitRootLogin no\nAllowUsers lab\n"
                                  "PasswordAuthentication yes\nPermitEmptyPasswords no\nKbdInteractiveAuthentication no\n"
                                  "AllowTcpForwarding yes\nAllowAgentForwarding yes\nLogLevel ERROR\n")
                name = network + "-" + role
                dc("run", "-d", "--name", name, "--label", "dev.containerdesk.lab=013", "--network", network,
                   "--network-alias", role, "--read-only", "--tmpfs", "/run", "--tmpfs", "/tmp",
                   "--mount", f"type=bind,source={config},target=/lab/sshd_config,readonly",
                   "--mount", f"type=bind,source={root / (role + '-host')},target=/lab/host_key,readonly",
                   "--mount", f"type=bind,source={authorized},target=/lab/authorized_keys,readonly", IMAGE, stdout=subprocess.DEVNULL)
                names.append(name)
                ports[role] = 22
                info = json.loads(dc("inspect", name, "--format", "{{json .NetworkSettings.Networks}}", capture_output=True).stdout)
                addresses[role] = info[network]["IPAddress"]
                for attempt in range(50):
                    try:
                        with socket.create_connection((addresses[role], ports[role]), timeout=0.3) as connection:
                            assert connection.recv(256).startswith(b"SSH-2.0-")
                        break
                    except (OSError, AssertionError):
                        if attempt == 49:
                            raise RuntimeError("owned SSH server did not start") from None
                        time.sleep(0.1)
            # Actual remote Docker CLI, but a deliberately synthetic read-only API fixture.
            # This validates CLI formats/classification; it does not prove a running Docker Engine.
            dc("exec", "-d", names[-1], "python3", "/opt/fixture/docker_probe_fixture.py")
            for attempt in range(100):
                probe = subprocess.run(docker + ["exec", names[-1], "test", "-f", "/tmp/probe-ready"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=5)
                if probe.returncode == 0:
                    break
                if attempt == 99:
                    raise RuntimeError("synthetic Docker API fixture did not start")
                time.sleep(0.05)
            agent_socket = root / "agent.sock"
            agent = subprocess.Popen(["ssh-agent", "-D", "-a", str(agent_socket)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            for _ in range(50):
                if agent_socket.exists():
                    break
                time.sleep(0.05)
            setup_askpass = root / "load-test-key"
            setup_askpass.write_text("#!/bin/sh\nprintf '%s\\n' disposable-lab-only\n")
            setup_askpass.chmod(0o700)
            agent_env = {**env, "SSH_AUTH_SOCK": str(agent_socket), "SSH_ASKPASS": str(setup_askpass), "SSH_ASKPASS_REQUIRE": "force", "DISPLAY": "lab:0"}
            run(["ssh-add", str(root / "encrypted")], env=agent_env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            public = lambda name: " ".join((root / (name + ".pub")).read_text().split()[:2])
            known = {
                "known": f"jump-key {public('jump-host')}\ntarget-key {public('target-host')}\n",
                "unknown-target": f"jump-key {public('jump-host')}\n",
                "unknown-jump": f"target-key {public('target-host')}\n",
                "changed-target": f"jump-key {public('jump-host')}\ntarget-key {public('wrong-host')}\n",
                "changed-jump": f"jump-key {public('wrong-host')}\ntarget-key {public('target-host')}\n",
            }
            for name, content in known.items():
                (root / name).write_text(content)
            cases = []
            sections = []

            def host(alias, role, trust="known", identity="plain", loaded=False, jump=None, private=False):
                sections.append(f"Host {alias}\n HostName {'target' if private else addresses[role]}\n Port {22 if private else ports[role]}\n"
                                f" User lab\n HostKeyAlias {role}-key\n UserKnownHostsFile {root / trust}\n GlobalKnownHostsFile {root / 'empty'}\n"
                                f" IdentityFile {root / identity}\n IdentitiesOnly yes\n IdentityAgent {agent_socket if loaded else 'none'}\n"
                                + (f" ProxyJump {jump}\n" if jump else ""))

            def case(alias, expected, **kwargs):
                host(alias, "target", **kwargs)
                cases.append({"alias": alias, "expected": expected})

            (root / "empty").write_text("")
            case("direct-known", "verified")
            case("direct-command-failed", "remote_command_failed", identity="forced-failure")
            case("direct-unknown", "unknown_host_key", trust="unknown-target")
            case("direct-changed", "changed_host_key", trust="changed-target")
            case("direct-absent", "authentication_failed", identity="absent")
            case("direct-encrypted-unloaded", "authentication_failed", identity="encrypted")
            case("direct-encrypted-loaded", "verified", identity="encrypted", loaded=True)
            for alias, trust, identity, loaded in (
                ("jump-known", "known", "plain", False), ("jump-unknown", "unknown-jump", "plain", False),
                ("jump-changed", "changed-jump", "plain", False), ("jump-absent", "known", "absent", False),
                ("jump-encrypted-unloaded", "known", "encrypted", False), ("jump-encrypted-loaded", "known", "encrypted", True),
            ):
                host(alias, "jump", trust=trust, identity=identity, loaded=loaded)
            case("via-known", "verified", jump="jump-known", private=True)
            case("via-target-unknown", "unknown_host_key", trust="unknown-target", jump="jump-known", private=True)
            case("via-target-changed", "changed_host_key", trust="changed-target", jump="jump-known", private=True)
            for jump, expected in (("jump-unknown", "unknown_host_key"), ("jump-changed", "changed_host_key"),
                                   ("jump-absent", "authentication_failed"), ("jump-encrypted-unloaded", "authentication_failed"),
                                   ("jump-encrypted-loaded", "verified")):
                case("via-" + jump, expected, jump=jump, private=True)
            config = root / "config"
            # Deliberately permissive source policy: app's strict overlay must win on both hops.
            config.write_text("Host *\n BatchMode no\n StrictHostKeyChecking no\n UpdateHostKeys yes\n ForwardAgent yes\n" + "".join(sections))
            config.chmod(0o600)
            watched = [config] + [root / name for name in known] + [root / "empty"]
            before = {p: hashlib.sha256(p.read_bytes()).hexdigest() for p in watched}
            askpass = root / "unexpected-askpass"
            marker = root / "askpass-was-called"
            askpass.write_text(f"#!/bin/sh\ntouch '{marker}'\nexit 1\n")
            askpass.chmod(0o700)
            manifest = root / "manifest.json"
            manifest.write_text(json.dumps({"config": str(config), "cases": cases, "askpassMarker": str(marker)}))
            test_env = {**env, "CONTAINERDESK_SSH_LAB_MANIFEST": str(manifest), "SSH_ASKPASS": str(askpass),
                        "SSH_ASKPASS_REQUIRE": "force", "DISPLAY": "lab:0"}
            run(["cargo", "test", "--manifest-path", str(REPO / "src-tauri/Cargo.toml"), "--locked", "disposable_lab", "--", "--ignored", "--nocapture"], env=test_env, timeout=180)
            assert not marker.exists()
            assert before == {p: hashlib.sha256(p.read_bytes()).hexdigest() for p in watched}
            print(f"PASS: {len(cases)} native SSH cases; config/trust hashes unchanged; no askpass invoked", flush=True)
        finally:
            if agent is not None:
                agent.terminate()
                agent.wait(timeout=5)
            for name in reversed(names):
                dc("rm", "-f", name, stdout=subprocess.DEVNULL)
            if network_created:
                dc("network", "rm", network, stdout=subprocess.DEVNULL)
            print("Cleaned owned containers, network, agent and temporary key directory.", flush=True)


if __name__ == "__main__":
    main()
