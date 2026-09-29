# Desktop launch and SSH setup

ContainerDesk runs the configured absolute OpenSSH executable (default `/usr/bin/ssh`) directly. It does not source `.profile`, `.bashrc`, `.zprofile` or `.zshrc`, or search for Docker, jq, Python, Node or Rust on the client. Installed packages still require the platform desktop libraries. Trusted SSH configuration may independently run its own `Match exec` or `ProxyCommand`; those helpers remain the user's responsibility and may have their own dependencies.

In Settings, use **Native dependencies** to validate an absolute SSH override and check the desktop process's inherited `SSH_AUTH_SOCK`. An unset socket does not mean authentication is impossible. A reachable socket does not prove a suitable key is loaded. OpenSSH can select a different agent with `IdentityAgent` in the chosen host configuration; the general diagnostic does not evaluate every host to discover that setting. No key identities or passphrases are requested by the diagnostic.

## Linux

1. Start ContainerDesk from the desktop application menu. Run Native dependencies; record the OpenSSH result and inherited agent status.
2. If an encrypted key is unavailable, load it with your normal desktop session agent using `ssh-add /absolute/path/to/key` in your terminal. Confirm direct and jump aliases there with the same executable/configuration and independently verified host fingerprints.
3. If terminal SSH works while the app fails, compare the desktop session's agent setup with the terminal's. An agent started only inside one terminal is not automatically inherited by an already running desktop application. Follow your desktop/distribution's session-agent setup, then restart the app. Do not expose the agent socket to other users.
4. A user-managed stable agent socket can instead be referenced by a host-scoped `IdentityAgent "/absolute/path/to/agent socket"` entry. Do not guess socket locations; use the agent your session actually owns. ContainerDesk does not change this configuration.
5. Select the saved host and connect explicitly. Authentication failure can mean a missing/locked key, an unreachable agent, server policy or a jump-host failure; the app does not infer that every failure is an encrypted-key problem. It never opens an invisible passphrase prompt.

## macOS — runtime verification pending

Use the normal login-session agent and Apple's `/usr/bin/ssh` when choosing Apple Keychain integration. Load/authorize the key in Terminal using your installed OS tools before opening ContainerDesk from Finder. Consult the local `man ssh-add` for the Keychain flag supported by that macOS version. ContainerDesk does not collect or store a passphrase.

This optional host-scoped fragment opts into Apple's Keychain behavior. Keep `IgnoreUnknown` before `UseKeychain` so a shared configuration remains parseable by Linux OpenSSH. Replace the example alias/path yourself; no configuration is installed by the app.

```sshconfig
IgnoreUnknown UseKeychain
Host example-alias
    IdentityFile "/absolute/path/to/key"
    AddKeysToAgent yes
    UseKeychain yes
```

`AddKeysToAgent` does not guarantee that a locked key can be used unattended. Verify the selected alias in Terminal first, close the app, and launch its `.app` from Finder rather than from a development shell. Check Native dependencies, connect directly and through an explicitly configured jump, then disconnect. Repeat with an unloaded encrypted key and an inaccessible agent; failures must stay bounded and must not prompt inside the app. These Finder/Keychain steps require a real macOS host and remain pending here.

## Paths and permissions

Spaces and Unicode characters are accepted in absolute config/executable references and app-data directories. Enter paths directly in app fields, without shell quotes. When writing SSH configuration yourself, quote values containing spaces. Native OpenSSH owns tilde/identity/include semantics; changing a process's HOME does not change the OS account's home-directory record or make every SSH `~` expansion use that variable. Choose an explicit config path when testing an alternate home.

Settings live in the platform app-data directory under `dev.containerdesk.app` (Linux: `$XDG_DATA_HOME/dev.containerdesk.app`, normally `~/.local/share/dev.containerdesk.app`; macOS uses the corresponding Application Support directory). The settings namespace is private (0700 directory, 0600 files), with atomic writes and a retained previous version. If settings storage is inaccessible while the OS webview can start, the app reports it and disables saving; it does not silently redirect to another directory or overwrite the original. Restore access for your user, close any second app instance, and reopen the app. The OS webview also needs writable runtime/cache storage: denying the entire app-data root can prevent the native window from starting, before app diagnostics are available. Restore that directory access in your OS. Do not run the app as root to work around this.

Native verification: [042 results](verification/042-native/RESULTS.md). Linux release startup through an actual `.desktop` entry and native agent/path tests are separate from future installed-package acceptance. No macOS execution, signing or notarization is claimed.

Sources: [OpenSSH configuration](https://man.openbsd.org/ssh_config.5), [Apple's Keychain and AddKeysToAgent guidance](https://developer.apple.com/library/archive/technotes/tn2449/_index.html), [Freedesktop launch argument rules](https://specifications.freedesktop.org/desktop-entry/latest-single/).
