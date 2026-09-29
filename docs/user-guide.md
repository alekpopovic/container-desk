# Using ContainerDesk

Install the package for your actual OS/CPU using [Linux](linux-packages.md) or [macOS](macos-packages.md) instructions and verify the manifest from the exact reviewed owner CI run. The owner source is [alekpopovic/container-desk](https://github.com/alekpopovic/container-desk). There is currently no approved public download or publishing workflow; CI packages are development output. Mac Developer ID signing/notarization and final platform acceptance remain separate gates. Follow [manual updates](updates.md) for hashes, settings backup and rollback; never disable host or OS trust checks to make an installer/connect test pass.

The application needs native OpenSSH and normal OS libraries. Node, Rust, Python, local Docker and jq are development/test tools, not app runtime prerequisites. The remote target needs Linux Docker Engine, a usable Docker CLI and POSIX-compatible noninteractive SSH command execution. Compose is optional for ordinary container views and required for verified project actions.

## Trusted SSH and first connection

Set up your own reviewed SSH config and existing key access outside the app. The [Serbian quick start](../README_SR.md#2-pripremi-pouzdan-ssh-pristup) includes direct/bastion/private alias examples. Use concrete short aliases, not wildcard expressions or options. Alias discovery reads bounded config/Include files without evaluating executable directives. **Connect saved host** or **Resolve selected alias** invokes OpenSSH and may evaluate trusted `Match exec`/`ProxyCommand` directives. Do not select an untrusted downloaded config.

Obtain host-key fingerprints independently, then verify/accept the direct host, bastion and final destination explicitly in your terminal. Keep normal known_hosts files and strict verification. Destination SSH options do not necessarily configure the bastion; check that alias independently. A working direct login does not prove that a bastion can forward to the private target. `ProxyJump` uses the local client to authenticate both hops; agent forwarding is unnecessary.

Use the desktop session's existing agent; `ssh-add -l` checks which identities it holds and `ssh-add /path/to/your/existing/key` unlocks a key in your terminal. The app collects neither key passphrases nor account passwords. Agent socket reachability does not prove a loaded key or access to a particular host. Finder/menu launch may inherit a different agent/PATH than a shell. Configure your OS session or a trusted `IdentityAgent` explicitly; see [desktop launch](desktop-launch.md). The app does not source shell profiles, run `ssh-add` or copy keys.

In the app:

1. **Settings → Run diagnostics** checks native OpenSSH and inherited agent reachability. **OpenSSH executable override** accepts a trusted absolute native executable path; this local setting differs from the remote Docker path.
2. **Hosts → New host** opens the editor. Leave **Host SSH config path** blank for the default config policy, or select a trusted absolute path. **Browse aliases → Use alias** lists concrete candidates; manual **Host SSH alias** entry handles aliases not discoverable from static Includes/Match blocks.
3. Set **Display name**, optional group/labels/favorite and the Docker fields described below. **Save host** stores references; it does not connect or grant management.
4. **Connect saved host** resolves the exact saved alias, verifies SSH trust/authentication and probes Docker. Inspect **Ready · SSH session**, the original alias/jump path and actual daemon identity before using resource tabs.
5. **Containers** shows the scoped inventory. Select a row for inspect, logs, stats and terminal controls. **Start logs** follows; **Stop logs** cancels the owned stream. **Hosts → Disconnect saved host** closes the selected connection. Only one saved host is actively connected at a time.

Settings also offers **Browse host candidates**, **Select alias**, **Resolve selected alias** and **Check SSH access** for diagnosis. These are separate from saving a host. The SSH-only check runs a bounded fixed marker; it is not proof of Docker permissions. Demo is explicit synthetic data with no SSH, not a fallback for a failed connection.

## Docker context, rootless and sudo

**Saved Docker executable** is a remote absolute binary path, or blank to use `docker` in the remote noninteractive PATH. It is not the local OpenSSH path. **Saved Docker context** names an already configured context on the SSH host; blank uses the effective remote user's existing selection. The app never runs `docker context use` or creates a context. An existing context may point to a different host: read the displayed Docker endpoint and daemon ID instead of assuming that the SSH host owns the daemon.

For rootless Docker, connect as the existing rootless owner and select their existing context/socket environment. For example, enter `rootless` only if that context actually exists for that user. The backend recognizes rootless from daemon security information rather than guessing from the socket path. It does not install rootless Docker or rewrite permissions.

**Use existing sudo -n Docker access** applies a fixed noninteractive sudo prefix. It requires an administrator-provided existing policy; no password prompt, policy installation or fallback to another privilege mode occurs. Sudo uses the administrator's Docker configuration and may reach a different daemon from the SSH user/rootless context. It does not enable management or terminal permission by itself. Capability probes show the actual endpoint/context and execution mode; changing the target/config/mode invalidates the old connection.

The server's login shell must accept POSIX quoting and fixed command forms. A fish/csh/restricted shell with incompatible behavior is unsupported; arrange an appropriate SSH account with the administrator. The app does not change login shells, wrap arbitrary scripts or retry through a different shell. Container terminal shell selection is a separate explicit command inside the selected container.

## Read-only views and actions

New/recovered connections start read-only. **Enable management** grants transient app permission for that host; each lifecycle action still requires its own bound confirmation. Check the alias/daemon, exact full IDs, operation and timeout. **Cancel action** cancels an unsubmitted confirmation; once dispatched, a Docker action is not rolled back or automatically replayed. Limited removal accepts selected stopped containers only, without force or volume removal. Images, volumes and networks remain read-only, including when management is enabled.

Read-only is enforced by Rust IPC authorization; it is not a reduction of the SSH/Docker account's server privileges. Refreshes can show stale/partial data or explicit loss gaps. A later read showing a running container does not prove whether a timed-out restart happened once, twice or not at all. Local activity is a bounded convenience history, not a tamper-proof server audit.

Compose discovery groups by labels/available CLI output. To act, select a project and provide **Remote project directory**, **Ordered remote config files (one absolute path per line)** and **Explicit project name**, acknowledge them and choose **Verify remote project**. Verify the displayed service instances against the actual deployment. Files must be absolute, ordered, readable to the SSH user and contain a resolvable configuration matching existing deployment hashes. Required interpolation values must already be available remotely. Config text and environment values never enter the renderer. Only existing-service start/stop/restart are allowed: no up, down, build, pull, recreate or deployment. Editing inputs, changing session or completing an action requires fresh verification. See [Compose actions](compose-actions.md) for limits and expiry.

For a selected running container, **Terminal → Enable management for terminal** (if required), **Enable terminal access**, **Open terminal**, then review and **Confirm terminal**. The confirmation includes user and selected shell. **Cancel terminal** leaves it unopened. Terminal access is write-capable; multiline paste requires its own review. Close the terminal, disable access or disconnect to end it. A missing shell produces a useful failure rather than an arbitrary fallback; the app never replays typed input after connection loss.

## Local data and explicit exports

Linux application data normally lives in `$XDG_DATA_HOME/dev.containerdesk.app` or `~/.local/share/dev.containerdesk.app`; macOS uses `~/Library/Application Support/dev.containerdesk.app`. `preferences/` stores schema-3 non-secret host/config/executable references, labels and theme. `activity/` stores bounded sanitized action outcomes. Files are private and locked against concurrent writers. Keys/passphrases, raw inspect/log contents and terminal transcripts are not preferences or default diagnostic data. Host names/paths can still be sensitive; keep backups private.

**Settings → Prepare support preview → Save reviewed report…** freezes a bounded sanitized report and opens a native save dialog only after review. Nothing is uploaded. Log export is separately user-selected and can contain secrets printed by your container; preview/review it before sharing. Inspect environment values are masked before default IPC; an explicit reveal is scoped to the selected session. [Support reports](support-reports.md), [settings](settings.md) and [backup/rollback](updates.md) describe exact retention and recovery.

## Troubleshooting

| Symptom | What to check |
|---|---|
| Unknown/changed host key | Verify the expected fingerprint independently, then repair trust deliberately in your terminal. Do not disable strict checking or discard known_hosts. Check the bastion and destination separately. |
| Jump authentication/forwarding failure | Test the configured bastion alias with the same key/agent, then the final alias. Confirm the bastion permits forwarding to that destination/port and can resolve/reach it. Do not enable agent forwarding as a workaround. |
| Shell works, desktop fails | Run app diagnostics; compare the inherited agent and native OpenSSH path. GUI launch does not source shell startup files. Use the existing desktop agent/IdentityAgent and a trusted absolute SSH executable if necessary. |
| Docker command not found | The remote noninteractive PATH may differ from an interactive login. Use **Saved Docker executable** with the real absolute path; installing local Docker does not fix the remote CLI. |
| Docker access denied / wrong containers | Confirm the remote user can run Docker, daemon is running, context exists and the displayed endpoint/daemon matches the intended target. Review rootless versus sudo mode; the app does not grant Docker group/sudo access. |
| Alias missing from Browse | Dynamic Match, wildcards, conditional/unreadable Includes may not yield concrete candidates. Inspect the discovery notes, then enter a valid concrete alias from the trusted config manually. |
| Logs unavailable or empty | Check the selected container's logging driver and application output. Some drivers/configurations do not expose logs through `docker logs`, including `none`. An empty successful read differs from an unsupported-driver error; the app does not change logging configuration. |
| Compose group visible but action refused | Labels alone are insufficient. Check exact remote paths/file order/project name, required interpolation, plugin support and deployment hash consistency. The app will not redeploy to repair drift. |
| Unknown mutation outcome after timeout/disconnection | The command may have run remotely. Read current state and consult your normal server history before deciding on another explicit action. Never treat Retry/reconnect as permission to repeat a mutation or terminal input. |
| Settings locked/recovered/unsupported | Quit other app instances. Keep damaged/newer files and private backups; follow the reported recovery notice and schema guidance. Do not edit schema numbers or overwrite a live profile. |

References reviewed for this guide: [OpenSSH configuration and ProxyJump](https://man.openbsd.org/ssh_config), [Docker rootless mode](https://docs.docker.com/engine/security/rootless/) and [Docker log visibility](https://docs.docker.com/engine/logging/). Application behavior above is cross-checked against the repository's current controls/backend and executed native evidence, not inferred from those external guides.
