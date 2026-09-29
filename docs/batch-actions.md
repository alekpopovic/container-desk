---
title: "Explicit container batches"
section: "Management & terminal"
icon: "⚡"
---

# ⚡ Explicit container batches

Use the checkboxes beside container names to select 1–20 full IDs on the connected host. Selection belongs to the entire host/daemon/session scope and is cleared on scope changes. Filtering does not implicitly select anything. Selected rows remain available in the batch panel when an inventory refresh removes a target, so its individual result is still visible.

Management needs the existing explicit session opt-in. Start, stop and restart each require a one-use confirmation listing the host, daemon and every full ID. The backend checks a fresh inventory before issuing the intent. Execution admits one batch per host, at most three globally, and sends exactly one target at a time. Each target gets a fresh bounded daemon/state preflight; each dispatched command keeps the existing SSH child ownership, output limits and deadline. There is no mutation retry.

Each result records its full ID, outcome, whether dispatch was attempted and an optional fixed error code. Success, disappearance and command permission failures remain distinct. Individual failure does not conceal later results. Unknown transport outcomes or invalidated sessions stop subsequent dispatches. Aggregate partial/unknown outcomes and the ordered individual results survive restart in bounded local activity history.

Cancel pending actions sets an exact scope/intent cancellation flag. It does not interrupt an already dispatched command: that target is allowed to finish or reach its existing deadline, and its outcome remains visible. The backend checks the flag between targets and just before dispatch. Leaving the batch view requests the same cancellation; application/network loss retains conservative unknown/not-dispatched history. A dispatch race may already have passed the cancellation check; the UI therefore describes cancellation as a request and distinguishes dispatched targets. Progress polling reads local activity at most once per second, without overlapping requests; late progress cannot replace the final response.

Removal has its own confirmation and acknowledgement. Only `created` or `exited` containers qualify. The backend checks that condition during confirmation and immediately before dispatch using a narrow structured inspect projection. The fixed command is `docker rm -- FULL_ID`; it contains neither force nor volume removal, matching the documented [Docker rm options](https://docs.docker.com/reference/cli/docker/container/rm/) (checked 2026-09-29). Docker can still refuse a target that changes state after the final read. Such a refusal is recorded individually, with no retry. Images, networks and volumes remain read-only.

Native acceptance uses temporary labelled containers and two owned loopback SSH servers with strict verification on both hops. The lab injects a permission denial only for a designated owned target; that proves the response path, not Docker authorization-plugin behavior. Native evidence and browser fixture evidence are separated in [033](https://github.com/alekpopovic/container-desk/blob/main/codex/tracking/evidence/033.md).
