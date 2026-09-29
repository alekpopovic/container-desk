---
title: "Selected-container resource statistics"
section: "Containers & resources"
icon: "▦"
---

<!-- {% raw %} -->

# ▦ Selected-container resource statistics

A live selected container has CPU, memory, network I/O, block I/O and PID measurements. No stats command runs against a guessed alias, an unselected saved host or the demo provider. The existing native OpenSSH session and pinned Docker identity/configuration are reused.

## Sampling and resource bounds

The narrow `container_stats` command takes a current session scope and one validated full container ID. It dispatches `docker stats --no-stream --no-trunc --format '{{json .}}' -- ID`, with the centralized POSIX quoting and normal read-only policy. A backend guard permits at most one statistics request per host (across containers and generations), within the existing four shared read slots. It rejects excess requests immediately and releases ownership on all exits. A whole sample has a 30-second deadline; each capture is at most 16 KiB stdout and 16 KiB diagnostic stderr.

A fixed inspect template reads only running state and start time before and after stats. This is necessary: an actual local Docker 29.8.1 experiment returned zero-valued JSON for an explicitly specified stopped container even without `--all`. Stopped containers, missing containers and changed start times become gaps. A narrow exact-ID missing-container diagnostic is recognized; unknown command failures remain static errors. Identity is checked before and after the sample; context/daemon drift revokes the old session. This is a bounded observation, not an atomic Docker transaction.

Default polling waits five seconds after completion. The panel can select 5, 10, 30 or 60 seconds. An already scheduled interval can run once with its previous delay; subsequent intervals use the selected value. Only the selected container is sampled. An inactive (hidden or unfocused) window and explicit Pause stop scheduling; a pending read may finish but its result is ignored after the pause transition. A new request cannot overlap that pending request. Disconnect closes and reaps session-owned processes; route/container changes stop scheduling and fence late results. Panel interval/pause preferences are transient and reset on selecting another container.

History is in memory, keyed by host ID, daemon, session generations and full container ID. It retains 360 normalized points for each of at most 12 recently used identities, evicting the least recently updated identity. Returning to a container and pauses insert observation gaps; unavailable individual measurements break their chart lines. Nothing is interpolated as zero. Raw strings exist only in the latest sample's expandable view, not history/persistence/default diagnostics. No export is added by this prompt.

## Measurement interpretation

CPU is the Docker-reported percentage and may exceed 100. Linux memory follows the CLI's cache-subtracted semantics; it is not reinterpreted as raw API memory. Network and block I/O are cumulative totals, not rates. PID counts include kernel threads. These meanings follow the [official Docker stats reference](https://docs.docker.com/reference/cli/docker/container/stats/), checked on 2026-09-29.

Decimal units (kB/MB/GB...) use powers of 1000; binary units (KiB/MiB/GiB...) use powers of 1024. The parser keeps the original bounded strings and returns nullable finite normalized values. Unknown units, placeholders, negative/non-finite/malformed values and numbers beyond the JavaScript safe numeric range remain unknown. A zero/unknown memory limit is unavailable, not a measured zero. Wrong IDs, duplicate JSON records, invalid field types, control characters and overlong fields fail closed. Byte figures/charts inherit CLI rounding and are approximate; numeric text uses suitable binary units to keep small nonzero totals visible. CPU safety validation does not clamp at 100%.

## Verification boundary

Parser/IPC tests cover unit conversion, CPU above 100, unknown values, hostile inputs, bounded identity history and overlapping/late polling. A browser fixture with a controlled clock/focus exercises actual panel effects for inactivity, resume, interval change, stopped/disappeared gaps and disconnect. This is explicitly simulated lifecycle evidence, not native window evidence.

The owned loopback SSH/Docker lab additionally checks real running/stopped output, concurrent request rejection, container disappearance between state/stats reads and cancellation on disconnect. The actual release Tauri/WebKit app renders those values and charts and pauses/resumes sampling on an owned Xvfb X11 display. Native macOS/Wayland and packaged execution remain separate platform gates. Evidence is in `codex/tracking/evidence/026.md`.

<!-- {% endraw %} -->
