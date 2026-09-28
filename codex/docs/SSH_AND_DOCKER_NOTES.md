# SSH and Docker implementation notes

## The existing command path

The user already uses an SSH alias through a jump host. The first application adapter should preserve that successful path:

```bash
ssh app-server 'docker ps -a --no-trunc --format "{{json .}}"'
```

This is a terminal reference command, not a instruction to construct a local shell string inside Rust. Native spawning uses the SSH executable, arguments, and a separately encoded remote command. The app parses JSON itself, so neither local nor remote jq is required.

## Diagnostic reference commands

Run only against a host the user selected. These snippets assume the default configured remote Docker endpoint and direct Docker permissions; a configured remote context or sudo mode changes the fixed builder accordingly.

```bash
ssh -G app-server
ssh app-server 'docker version --format "{{json .}}"'
ssh app-server 'docker ps -a --no-trunc --format "{{json .}}"'
ssh app-server 'docker stats --no-stream --no-trunc --format "{{json .}}"'
ssh app-server 'docker compose ls --all --format json'
```

`ssh -G` resolves configuration and can execute `Match exec`; do not use it automatically on untrusted downloaded config files. Config discovery should not execute arbitrary entries.

| Operation | Output contract / caveat |
|---|---|
| `docker ps --format` JSON template | JSON lines, one record per container |
| `docker container inspect ID` | JSON array; potentially contains secrets |
| `docker logs --tail N --timestamps ID` | Plain text; stdout and stderr may both be valid logs |
| `docker logs --follow ...` | Long-lived child with cancellation and bounded buffers |
| `docker stats --no-stream --format` | Snapshot values including human unit strings |
| `docker events --format` | JSON event stream; gaps require snapshot reconciliation |
| `docker compose ls --format json` | Project discovery; metadata is not automatically an executable project config |

Do not pass placeholder IDs literally. Choose IDs returned from the selected current daemon and revalidate before mutating it.

## OpenSSH integration traps to address in code

- Config parsing is only host candidate discovery; effective precedence belongs to OpenSSH.
- Jump-host options/credentials may differ from target options. Test both hops unattended.
- GUI applications do not inherit all interactive shell environment settings. Prefer a known SSH path and visible agent diagnostics.
- Long Unix control-socket paths differ in tolerance across Linux/macOS; use a short private app directory.
- Reusing an existing user control master is not ownership. Never close it on app exit.
- An SSH command timeout does not undo a Docker mutation already executed remotely.
- The Docker CLI may use a context pointing to another endpoint. Show and consistently preserve actual daemon identity.
- Mask environment/labels in Rust before default responses leave the backend; masking only UI text is insufficient.

## Optional future API adapter

Docker officially supports SSH access to a remote daemon. A later adapter may route Engine API traffic through an OpenSSH-backed transport for better typed streams. That is an architectural extension, not a required rewrite of the CLI MVP. It must retain existing alias/jump/agent semantics and explicitly negotiate API versions. Exposing port 2375 is not needed.
