# Volume metadata and mount relationships

The selected live daemon has a read-only Volumes route with searchable, paged list/detail views. A volume is identified by its validated name within the complete host/daemon/session scope. Hexadecimal names are displayed as reported; the application does not infer whether a volume was created anonymously from its name alone. Missing driver, scope, creation time and mountpoint remain unknown/not reported, including external-driver responses.

Native Rust authorizes and admits each read through the existing host/global read limits, verifies the daemon binding, then dispatches fixed metadata commands through the owned OpenSSH connection:

- `docker volume ls --format <fixed name/driver/scope JSON template>`
- `docker volume inspect -- NAME`
- `docker ps --all --quiet --no-trunc --filter volume=NAME`
- `docker inspect --type container --format <fixed identity/state/mounts JSON template> -- IDS`

The name permits ASCII alphanumerics followed by alphanumerics, underscore, dot or dash, up to 255 bytes. IDs must be full 64-character hexadecimal container identities. The existing central POSIX encoder quotes every remote argument. No renderer-provided command or template is accepted. Reference candidates are rechecked against actual `Mounts` entries with `Type=volume` and the exact selected name. Bind mounts and other volume names are excluded. The view shows container identity, destination, reported state and read-only/read-write/unknown mount access; links use the existing scoped container detail selection.

All label and driver-option **values** are masked before IPC. Unknown plugin status fields and container mount source paths are omitted. A reported volume mountpoint is display-only text. No code opens, copies, exports or follows that path. No volume contents browser, deletion or prune operation is registered or presented. Metadata is transient and is not written to preferences or activity history.

A reference scan is an observation, not a transaction. If a candidate container disappears while the fixed inspect batch runs, the parser preserves valid surviving records and reports an **incomplete** snapshot with unresolved IDs. The next explicit refresh can reconcile it. A completed scan with zero references says only that none were observed in that snapshot, and explicitly states that this is not proof deletion is safe. Other failures retain an explicitly stale list or show a detail error. Old host/daemon/session results are discarded; stale reference links are disabled.

Bounds: 30-second overall read deadline; 16 MiB list or aggregate detail/reference stdout; 2 MiB individual detail/mount batch; 64 KiB diagnostic stderr per process; 5,000 volume names, candidate IDs and mount references; 64 IDs per inspect batch; 128 mounts per container; 256 labels/options; 4 KiB fields/keys and 64 KiB raw values. Oversized responses fail explicitly. List and reference rendering use pages of 50. Only these idempotent reads use the existing bounded read scheduler; no new mutation or filesystem access path is introduced.

The owned private-Engine lab `python3 tests/lab/ssh_auth.py --engine --listing --inventory --volumes` provisions named, anonymous and unused local volumes, read-only/read-write mounts on metadata-only containers, and one exactly labelled disposable container removed between enumeration and inspect. Its fixture-only Docker executable admits metadata/probe argv and records those commands; a controller checks that no returned mountpoint becomes a command argument. The removal is test fault injection, not an application command. Native UI verification additionally uses the production executable and real Docker CLI. External-plugin and missing-mountpoint variations are parser/browser fixtures; no external storage plugin or remote data contents are accessed.

Docker references: [volume list](https://docs.docker.com/reference/cli/docker/volume/ls/), [volume inspect](https://docs.docker.com/reference/cli/docker/volume/inspect/), [container inspect](https://docs.docker.com/reference/cli/docker/container/inspect/), consulted 2026-09-29. Native execution evidence is recorded separately from fixture evidence in prompt 035.
