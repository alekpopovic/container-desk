---
title: "Explicit offline demo"
section: "Start here"
icon: "🧭"
---

# 🧭 Explicit offline demo

Choose **Open demo** in the sidebar. A fixed DEMO badge remains visible across all six routes and scrolling. Exit demo explicitly to return to the live workspace; live failures never substitute fixtures. Each transition replaces the session generation and clears policy grants. Demo mode is in memory and is not restored as a live connection on restart.

The Rust `ReadTransport` interface accepts a validated operation plan and returns the same scoped `ListContainersResponse` used by native IPC. `FixtureTransport` parses synthetic normalized JSONL (not Docker CLI output). Its four containers include healthy, exited, restarting and unhealthy states, IPv6 port publication and Compose project/service labels. Other fixture labels are discarded before IPC. Names are rendered as text.

The scenario selector covers successful empty results, permission errors, malformed JSON, oversized records, disconnects and timeouts. Error scenarios are deterministic simulations, not real network/time measurements. Limits are 16 MiB per snapshot, 256 KiB per record, 50,000 rows, 128 ports and 64 input labels per row. Four read slots and one native diagnostic slot prevent unbounded work; a mode change is rejected while those resources are occupied.

Native demo goes through the registered Rust handlers and capability grants. Browser preview has an explicitly selected in-memory provider with a Rust-generated JSON fixture (`npm run ipc:generate`). Runtime choice happens before requests, so native IPC failure cannot call the browser provider. Initial live mode contains no synthetic inventory. OpenSSH diagnostics/override probes are also denied by Rust in demo mode; disabled controls are supplementary.

When real SSH sessions/streams are implemented, mode switching must cancel and reap owned live resources before resetting policy. The current live provider reports feature_unavailable. The fixture parser must not be reused as an assertion that Docker CLI parsing or actual SSH operation works.

Verification: [008 evidence](https://github.com/alekpopovic/container-desk/blob/main/codex/tracking/evidence/008.md). Linux native build/startup passed; full demo interaction in the native window was not confirmed by the local input driver. The same serialized DTOs and permission paths are exercised in Tauri's Rust mock runtime, separately from browser component tests.
