#!/usr/bin/env python3
"""Disposable native SSH acceptance. Requires local Docker, cargo and OpenSSH; no user keys.
Only labelled app-owned containers and an internal network are created and removed.
No ports are published; the local host reaches only the owned bridge addresses.
"""
import argparse
import hashlib
import ipaddress
import io
import tarfile
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time

REPO = Path(__file__).resolve().parents[2]
IMAGE = "containerdesk-ssh-lab:020"
BASE = "alpine@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce"


def run(args, **kwargs):
    return subprocess.run(args, check=True, text=True, timeout=kwargs.pop("timeout", 30), **kwargs)


def native_test(executable, name, env):
    listed = run([executable, name, "--ignored", "--list"], env=env, capture_output=True).stdout
    assert sum(line.endswith(": test") for line in listed.splitlines()) == 1, "Expected exactly one native checkpoint test"
    run([executable, name, "--ignored", "--nocapture"], env=env, timeout=60)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", action="store_true", help="Run a real isolated empty Docker Engine in the SSH target")
    parser.add_argument("--listing", action="store_true", help="Seed two metadata-only containers in the private Engine for listing checks")
    parser.add_argument("--inspect", action="store_true", help="Check native inspect and secret redaction; requires --inventory")
    parser.add_argument("--inventory", action="store_true", help="Check live resource sessions and cancellation; requires --listing")
    parser.add_argument("--native-artifacts", type=Path, help="Explicit directory for this run native screenshots")
    parser.add_argument("--native-driver", type=Path, help="Optional external tauri-driver executable for the real native UI journey")
    parser.add_argument("--webkit-driver", type=Path, help="WebKitWebDriver executable, required with --native-driver")
    args = parser.parse_args()
    if args.inspect and not args.inventory:
        parser.error("--inspect requires --inventory")
    if args.inventory and not args.listing:
        parser.error("--inventory requires --listing")
    if args.listing and not args.engine:
        parser.error("--listing requires --engine")
    if args.native_driver and (not args.engine or not args.webkit_driver):
        parser.error("--native-driver requires --engine and --webkit-driver")
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
                   *(["--cap-add", "SYS_ADMIN", "--security-opt", "apparmor=unconfined"] if args.listing and role == "target" else []),
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
            engine = {}
            if args.engine:
                # Separate Engine: private filesystem/socket, no host socket or --privileged flag.
                dc("exec", "-d", names[-1], "sh", "-c",
                   "exec dockerd --host unix:///run/docker.sock --data-root /tmp/engine-data "
                   "--exec-root /tmp/engine-exec --pidfile /tmp/engine.pid --storage-driver vfs "
                   "--iptables=false --ip6tables=false --bridge=none --ip-forward=false "
                   "--ip-masq=false --userland-proxy=false > /tmp/engine.log 2>&1")
                for attempt in range(100):
                    probe = subprocess.run(docker + ["exec", names[-1], "docker", "info", "--format", "{{.ServerVersion}}"],
                                           env=env, capture_output=True, text=True, timeout=5)
                    if probe.returncode == 0:
                        engine = {"engineVersion": probe.stdout.strip(), "engineId": dc("exec", names[-1], "docker", "info", "--format", "{{.ID}}", capture_output=True).stdout.strip()}
                        print("Real isolated Docker Engine ready: " + probe.stdout.strip(), flush=True)
                        break
                    if attempt == 99:
                        dc("exec", names[-1], "cat", "/tmp/engine.log")
                        raise RuntimeError("isolated Docker Engine did not start")
                    time.sleep(0.1)
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
            manifest.write_text(json.dumps({"config": str(config), "cases": cases, "askpassMarker": str(marker), "realEngine": args.engine, **engine}))
            test_env = {**env, "CONTAINERDESK_SSH_LAB_MANIFEST": str(manifest), "SSH_ASKPASS": str(askpass),
                        "SSH_ASKPASS_REQUIRE": "force", "DISPLAY": "lab:0"}
            run(["cargo", "test", "--manifest-path", str(REPO / "src-tauri/Cargo.toml"), "--locked", "disposable_lab", "--", "--ignored", "--nocapture"], env=test_env, timeout=180)
            if args.engine:
                build = run(["cargo", "test", "--manifest-path", str(REPO / "src-tauri/Cargo.toml"), "--locked", "--lib", "--no-run", "--message-format=json"],
                            env=env, capture_output=True, timeout=180)
                artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith("{")]
                executable = next(item["executable"] for item in artifacts
                                  if item.get("reason") == "compiler-artifact" and item.get("profile", {}).get("test") and item.get("executable"))
                native_test(executable, "checkpoint018_real_engine", {**test_env, "PATH": "/nonexistent"})
            if args.listing:
                # A zero-layer fixture image needs no download or workload execution. Docker load
                # still needs a mount namespace: --listing alone gives the owned target SYS_ADMIN
                # and an AppArmor exception. Default seccomp and all host-resource boundaries stay.
                # These containers are metadata only; running/exited states use parser fixtures.
                image_config = json.dumps({"architecture": "amd64", "os": "linux", "rootfs": {"type": "layers", "diff_ids": []},
                                           "config": {"Cmd": ["/bin/true"]}}).encode()
                config_name = hashlib.sha256(image_config).hexdigest() + ".json"
                archive = io.BytesIO()
                with tarfile.open(fileobj=archive, mode="w") as tar:
                    for name, data in ((config_name, image_config), ("manifest.json", json.dumps([
                        {"Config": config_name, "RepoTags": ["containerdesk-empty:019"], "Layers": []}]).encode())):
                        info = tarfile.TarInfo(name)
                        info.size = len(data)
                        tar.addfile(info, io.BytesIO(data))
                subprocess.run(docker + ["exec", "-i", names[-1], "docker", "load"], input=archive.getvalue(),
                               env=env, check=True, timeout=30, stdout=subprocess.DEVNULL)
                expected = []
                for name in ("listing-first", "listing-second"):
                    expected.append(dc("exec", names[-1], "docker", "create", "--network", "none", "--name", name,
                                       "--label", "dev.containerdesk.fixture=019", "--label", "test.value=comma,equals=next",
                                       *(["--env", "CHECKPOINT_TOKEN=synthetic-inspect-021-secret", "--label", "innocent=synthetic-label-021-secret", "--expose", "8080/tcp", "--expose", "53/udp"] if args.inspect else []),
                                       "containerdesk-empty:019", capture_output=True).stdout.strip())
                current = json.loads(manifest.read_text())
                current["expectedContainerIds"] = expected
                manifest.write_text(json.dumps(current))
                native_test(executable, "checkpoint019_real_listing", {**test_env, "PATH": "/nonexistent"})
            if args.inventory:
                native_test(executable, "checkpoint020_live_inventory", {**test_env, "PATH": "/nonexistent"})
            if args.inspect:
                native_test(executable, "checkpoint021_live_inspect", {**test_env, "PATH": "/nonexistent"})
            if args.native_driver:
                from native_ssh import verify
                verify(root, args.native_driver.resolve(), args.webkit_driver.resolve(), config, engine, args.native_artifacts, args.inventory, args.inspect)
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
