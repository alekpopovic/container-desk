---
title: "Effective SSH configuration"
section: "Hosts & SSH"
icon: "🔐"
---

# 🔐 Effective SSH configuration

After selecting a candidate or manual alias in Settings, choose **Resolve selected alias**. Only this explicit action invokes the selected OpenSSH executable. The explanation above the button notes that native `ssh -G` evaluates Match exec and can run local code. Trust the config and its includes; ProxyCommand is also trusted executable configuration. Resolution does not open an SSH session, but it is not side-effect free. [OpenSSH reference](https://man.openbsd.org/ssh#G).

Selection preserves the original alias, the displayed local config path and whether native default config policy applies. With native defaults, resolution passes `-G -- alias` and allows OpenSSH to read user/system defaults. With an explicitly selected or saved custom file, it passes `-G -F path -- alias`; OpenSSH then excludes its system-wide config. The distinction is carried in the typed selection, checked in Rust and shown in the UI. Paths containing spaces remain a single argv element. Future connections must retain this same alias/config policy and selected executable instead of connecting to the displayed HostName directly. [Meaning of -F](https://man.openbsd.org/ssh#F).

Rust validates the conservative alias grammar from [discovery](ssh-discovery.md) before any process creation, validates the chosen executable with the existing ownership/permissions checks, and launches it directly using Tokio argument arrays, null stdin and piped stdout/stderr. No local shell command is constructed. OpenSSH may itself run trusted Match exec code; the application does not reinterpret those directives.

Diagnostic capture has a 5-second overall deadline, 1 MiB stdout, 64 KiB stderr and 16 KiB line limit. Both pipes drain concurrently. Failure/oversized output/timeout are distinct typed errors; raw output is never returned, logged or persisted. The parser exposes only effective hostname, user, port, jump information, a ProxyCommand-present flag and original selection/executable references. Identity directives and ProxyCommand command contents are discarded. Missing, duplicate or invalid required effective values cannot produce success.

Each invocation owns a process group. Timeout/error cleanup signals that group only while the directly spawned leader remains unreaped, then kills/waits for the direct child. A drop guard covers interrupted execution and Tokio kill-on-drop remains enabled. Trusted configuration can create its own external side effects or detached processes; this is not a sandbox or transaction/rollback mechanism. The backend owner keeps the single-flight permit until completion/reaping even if the renderer leaves. The resolver now uses the shared [process runner](ssh-runner.md); its Job API supports cancellation for session owners. This Settings flow still retains its owner through the bounded operation and has no cancel button.

Demo mode denies resolution in Rust; mode changes and other native probes/discovery are excluded during resolution. UI edits/new selections clear effective values and late replies after route/mode changes are ignored. No connection, authentication, known_hosts update, key copying or mutation retry is added here.

Verification: [010 evidence](https://github.com/alekpopovic/container-desk/blob/main/codex/tracking/evidence/010.md). Native Linux tests compare fixture values to actual `/usr/bin/ssh -G`, including precedence, Include, jumps and a deliberately controlled Match exec marker. Browser fixtures test presentation separately. macOS and actual SSH sessions are still unverified.
