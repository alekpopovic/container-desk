---
title: "Container detail panels"
section: "Containers & resources"
icon: "▦"
---

# ▦ Container detail panels

The inspected container has six tabs: Overview, Ports, Mounts, Networks, Labels and Environment. Arrow keys, Home and End move and select tabs; Tab enters the selected panel. All data is rendered as text, with wrapping for long paths and no automatic links or HTML interpretation.

Overview shows running state and health state separately, plus exit code and the Engine's OOMKilled value when provided. Rust reports healthcheck configuration as true for CMD/CMD-SHELL, false for absent configuration in a known Config object or an explicit NONE test, and unknown for missing Config or unrecognized/empty tests. Probe commands/output remain excluded from IPC. A configured healthcheck with no current status is not presented as healthy.

Ports separates Config.ExposedPorts from active NetworkSettings.Ports host bindings. Exposure does not imply a published host port. Every IPv4/IPv6 binding is retained, IPv6 is bracketed and protocols remain visible. Requested HostConfig bindings are not presented as active before Engine reports them. Exposed ports retain the same 128-entry and protocol/port-number limits as other normalized port metadata.

Copy actions are limited to explicit container/image/network IDs, network addresses and mount source/destination fields. Environment/label values have no copy action, including after reveal. The application only writes plain text from an explicit click and reports success after the write promise resolves. The [WebKit Async Clipboard API](https://webkit.org/blog/10855/async-clipboard-api/) requires a secure context and user gesture. The app does not read the clipboard, install a clipboard plugin or broaden Tauri permissions. If copying is unavailable, it displays a manual-selection message instead of claiming success.

Reveal/hide behavior remains session- and selection-scoped as documented in `container-inspect.md`. Changing tabs does not request new data or broaden access. Refresh, selection/session/route changes clear revealed data. Label and environment values continue to be masked by Rust before ordinary IPC responses.
