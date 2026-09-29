---
title: "Read-only image inventory"
section: "Containers & resources"
icon: "▦"
---

<!-- {% raw %} -->

# ▦ Read-only image inventory

Images are read from the selected native host/daemon/session. Listing uses a fixed `docker image ls --all --no-trunc --digests --format '{{json .}}'` command. Full `sha256:` IDs are the identity; repeated rows are merged while all distinct tags and repository digests remain available. Missing tags are explicitly shown as “No tags”. List size and creation fields are CLI-reported display strings; inspect separately exposes exact safe-integer bytes and the reported creation timestamp.

The dangling checkbox adds Docker's fixed `--filter dangling=true` option. It does not guess dangling state from a missing tag: the CLI filter selects untagged leaf images, while an all-images listing can contain intermediate images. The view has no pull, removal, prune or registry-credential workflow. Docker's [image listing](https://docs.docker.com/reference/cli/docker/image/ls/) and [inspect](https://docs.docker.com/reference/cli/docker/image/inspect/) documentation were checked on 2026-09-29.

Inspect accepts only a validated full image ID. The Rust projection exposes platform, creation/size, tags/digests, masked label names and exact container references. All label values are masked before IPC, including labels whose names look harmless. Config environment, commands, history and raw JSON are excluded. There is no sensitive-value reveal or image export in this increment; default diagnostics retain only existing fixed error codes, never raw inspect data.

Container references first use the bounded `ancestor=FULL_ID` filter, then inspect candidates in batches of at most 64 using a fixed narrow ID/image/name/state projection. Docker's [ancestor filter](https://docs.docker.com/reference/cli/docker/container/ls/) can also include descendants; only an exact immutable `.Image` match appears as a reference. Missing/inconsistent candidates fail the read instead of claiming a complete list. A reference opens the existing container details only when the same scope's current inventory contains that full ID; stale/missing inventory requires refresh.

Each read has a 30-second total deadline, a fresh daemon-binding probe, existing native host/global admission and the finite-read frontend scheduler. A response must still match the entire current scope and filter; late results from another host cannot replace an identically named tag. Successful snapshots remain visibly stale after a failed refresh. Leaving the route removes the view; no image data is persisted.

Bounds: 16 MiB per list and aggregated detail/reference batches; a separate candidate-ID response is capped at 325,000 bytes; 20,000 raw listing rows; 5,000 deduplicated images; 128 tags/digests per image; 4,096-byte text fields; 32-KiB listing/reference records. Detail JSON is at most 2 MiB with 256 labels, 64-KiB input label values, and 5,000 candidate container references. Label values are discarded, not copied to the wire projection. UI image and reference pages render at most 50 entries each. Capacity exhaustion is an error, not silent truncation.

Native acceptance uses the existing isolated Engine inside the disposable SSH target, without exposing Docker TCP or mounting the host Docker socket. The lab loads two zero-layer fixture images, gives one two tags and two metadata-only containers, and leaves the other genuinely untagged. Native direct/ProxyJump reads are compared with independent CLI inspect/filter results. Parser/browser cases additionally cover identical tags with different identities/scopes and late host-switch responses. See [034 evidence](https://github.com/alekpopovic/container-desk/blob/main/codex/tracking/evidence/034.md).

<!-- {% endraw %} -->
