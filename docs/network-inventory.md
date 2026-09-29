---
title: "Network inventory and attachments"
section: "Containers & resources"
icon: "▦"
---

# ▦ Network inventory and attachments

The live Networks route lists full network identities and reported names, drivers and scopes. Details show internal/IPv6 flags, creation metadata, IPAM driver, subnet/range/gateway/auxiliary addresses, masked labels/options, and reported container endpoints. Missing optional fields are unknown/not reported. Malformed optional metadata becomes unknown with a visible incomplete-metadata notice; invalid required identity, duplicates and bounds violations fail the read.

Rust admits only `docker network ls --no-trunc --format <fixed JSON projection>` and `docker network inspect -- ID`, through the existing read policy, per-host/global limits, owned native OpenSSH session and verified daemon binding. Local IDs accept full 64-character lowercase hex; 25-character lowercase alphanumeric IDs support reported Swarm-style identities without enabling Swarm administration. Renderer input cannot supply names as targets, commands or templates. Central remote POSIX quoting remains authoritative.

IPAM and endpoint addresses are parsed as IPv4/IPv6 with optional bounded CIDR prefixes. A wrong address family or malformed address is unknown, not a link or executable string. Network names, option/label keys and endpoint text are React text nodes. Every network/IPAM option and label value is masked before IPC; unknown raw fields are discarded. No network metadata is persisted.

Attachment records retain the inspect map's endpoint key even when it is not a container ID, has no name/address or is no longer present in the current container inventory. Only a valid full container ID present in the same current host/daemon/session inventory enables navigation to the existing container details. Otherwise the endpoint stays visible with a deleted/stale/non-container explanation. A missing `Containers` map is distinguished from an explicitly empty map. Counts reflect the reported snapshot, not a guarantee of current membership. Explicit refresh reconciles reads and stale links are disabled.

Bounds: 30-second overall operation, 16 MiB stdout, 64 KiB diagnostic stderr; 5,000 networks/endpoints; 32 KiB list records; 4 KiB strings/keys; 256 label/option entries with 64 KiB raw values; 128 IPAM configs with up to 256 auxiliary addresses each. Oversized responses fail. Lists and attachments render 50 per page. Reads use the existing bounded idempotent scheduler; no network create/connect/disconnect/delete/prune capability is exposed.

Native verification uses `tests/lab/logs.py --stream --jump --networks --sshd-root <extracted OpenSSH root>`. It creates one disposable internal dual-stack bridge and one running labelled workload on it; existing owned exited workloads remain isolated on none. The fixture-only SSH gate filters network listing to the exact owned network and admits inspection only of that ID, without granting app mutation. Independent Docker inspect and actual backend/UI results compare the nonzero attachment count, full container ID, IPv4/IPv6 addresses and IPAM. Cleanup removes only its owned workloads/network/SSH processes and temporary keys. Host/none, external drivers, unknown endpoints and malformed optional fields are parser/browser fixtures, not claimed as live external-plugin or Swarm proof.

Docker CLI references consulted 2026-09-29: [network list and full IDs](https://docs.docker.com/reference/cli/docker/network/ls/), [network inspection](https://docs.docker.com/reference/cli/docker/network/inspect/). Prompt 036 evidence records the executed native Linux configuration and platform limits.
