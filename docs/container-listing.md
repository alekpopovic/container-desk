---
title: "Docker container listing adapter"
section: "Containers & resources"
icon: "▦"
---

<!-- {% raw %} -->

# ▦ Docker container listing adapter

Implemented in 019, based on the [Docker container ls command and formatting contract](https://docs.docker.com/reference/cli/docker/container/ls/). `docker/listing.rs` builds the registered read-only `docker ps --all --no-trunc --format '{{json .}}'` operation with the verified Docker endpoint/context and central POSIX quoting. The native adapter refreshes capability/daemon identity before dispatch and again before publishing. The complete read, including probes, has a 30-second deadline; stdout is bounded to 16 MiB and stderr to 16 KiB. Connection shutdown cancels owned jobs. The caller must additionally fence the host/session generation.

The parser accepts one object per line using serde_json, with optional final newline and CRLF support. A successful zero-byte response is an empty inventory. Blank records, banners, invalid JSON/types, missing/short/uppercase IDs and duplicate IDs reject the whole snapshot. A failed command never becomes empty success; errors include a static scoped code, not raw output. Unknown fields are ignored without being sent to the frontend. Bounds: 50,000 records, 256 KiB per record, 4 KiB per primary display field, 16 KiB ports display, 64 KiB input labels and 128 name aliases. Overflow returns an explicit error, not a silently shortened list.

Full IDs and the supplied verified scope remain stable keys. The first normalized CLI name is the display name, with the full ID as fallback. Missing image/state/status become `Unknown`/`unknown`/empty display text. Unknown future state strings remain text. CLI fields are kept in `ContainerSummary.cli` separately from precise inspect fields:

| CLI field | Normalized meaning |
|---|---|
| Names | Bounded list of display aliases; Unicode is preserved. No name authorizes an operation. |
| Ports | Original optional display string, including compressed ranges and IPv6. It is not converted to exact bindings. |
| CreatedAt / RunningFor | Optional display strings, not an invented precise creation timestamp. |
| Status | Preserved display text, including unhealthy/exited status. Exact health stays unknown until inspect. |
| Labels | Missing → unknown presence; empty → false; nonempty → true. Values are discarded before default IPC. |

Docker's Labels display is a comma-separated key=value string whose values may themselves contain commas or equals signs. It cannot safely establish an exact map or verified Compose metadata. This implementation normalizes presence and deliberately withholds all label values; precise redacted metadata belongs to the inspect adapter. Health, exact port bindings and Compose remain empty/unknown for CLI-only rows. Synthetic demo rows retain their separately defined exact fixture fields and `cli: null`. The Rust-generated TypeScript contract and runtime decoder cover the new display shape and bounds.

This prompt implements the real parser and native transport adapter; wiring it into saved-host resource sessions and the live container table is 020. The existing IPC still refuses unavailable live resource sessions. There is no demo fallback on an adapter failure.

## Real comparison and lab permissions

Run `python3 tests/lab/ssh_auth.py --engine --listing` with the pinned development tools. It first checks the real empty daemon from 018. It then loads an owned zero-layer image into the private target Engine and creates exactly two metadata-only containers (`listing-first`, `listing-second`) with no network. They are never started. Both direct and ProxyJump adapter results must contain exactly the provisioned full IDs and match a separate `docker ps --all --quiet --no-trunc` read. A deliberately wrong daemon ID must be rejected. The compiled Rust process has `PATH=/nonexistent`. The harness checks that each checkpoint filter actually selects exactly one test, preventing a zero-test false pass.

Even a zero-layer `docker load` uses a mount namespace. The initial ordinary-container attempt failed with `unshare: operation not permitted`; adding SYS_ADMIN alone reached an AppArmor mount denial. **Only `--listing` and only the disposable target** therefore receive SYS_ADMIN and `apparmor=unconfined`. Default seccomp, read-only root, internal network and no host Docker socket remain. The bastion and the 018-only lab do not receive these permissions; no host profile or global security setting is changed. This is development provisioning, never an application action or supported requirement on a user's server. The nested Engine still has private tmpfs data and no bridge/NAT. All owned containers, image data, network, agent and temporary keys are removed after the check; the outer SSH lab image cache remains.

Verified 2026-09-29 on native Ubuntu 26.04.1 x86_64 with Docker Engine/CLI 28.3.3 inside Alpine 3.22.5. Tests cover running/exited/unhealthy and Unicode through deterministic CLI-format fixtures; the real daemon comparison covers `created` containers and an empty inventory. Running nested workloads, native macOS and the live resource UI are not claimed here.

<!-- {% endraw %} -->
