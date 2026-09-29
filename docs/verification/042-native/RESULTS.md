# 042 native Linux desktop launch

2026-09-29, Ubuntu 26.04.1 x86_64, UID 1000, WebKitGTK 2.52.6, Tauri 2.12.0. Release executable SHA-256: `81f419830d8569c40b56b8ee133e3fa705b92344768f762aaa8acd290797a463`.

`python3 /tmp/containerdesk-042-xvfb.py` allocated an owned Xvfb display (1440×1000, no TCP), selected X11/software rendering, and ran:

```sh
python3 tests/lab/ssh_auth.py --engine --launch \
  --native-driver /tmp/containerdesk-025-tools/bin/tauri-driver \
  --webkit-driver /tmp/containerdesk-025-tools/webkit/usr/bin/WebKitWebDriver \
  --native-artifacts docs/verification/042-native
```

Final exit 0. The launcher journey additionally uses `/tmp/containerdesk-025-tools/xdotool/usr/bin/xdotool`, `/usr/bin/gio`, `/usr/bin/desktop-file-validate` and `/usr/bin/import` as development drivers only. Node 24.21.0/npm 11.19.0 were first on the harness PATH. The application itself receives a new minimal environment: PATH=/nonexistent, an owned Unicode HOME and XDG_DATA_HOME, a closed owned SSH_AUTH_SOCK, locale and display variables only.

Observed:

- Seven actual SSH integration tests passed, including 16 authentication cases, both-hop trust/agent failures, owned multiplex lifetime and Docker probe classification. The Docker API classification fixtures within that suite remain synthetic.
- A separate native Rust checkpoint passed against the actual isolated Docker Engine 28.3.3 through direct and ProxyJump routes, with PATH=/nonexistent (1.59 seconds).
- Actual release webview saved and used a Unicode/space absolute OpenSSH override, a config inside `Korisnik Željko 日本語/.ssh/izabrani config`, and a Unicode/space app-data directory. Settings persisted and reloaded with 0700 namespace / 0600 file permissions.
- Native diagnostics reported the inherited closed socket as inaccessible. An unloaded encrypted key and an explicit stale IdentityAgent failed promptly with the conditional passphrase/desktop-agent guidance. Loaded encrypted keys succeeded directly and through a jump using the selected config's separate Unicode/space IdentityAgent socket. The real Engine identity/version appeared in the native UI.
- An actual validated `.desktop` entry, with a quoted Unicode executable reference, was launched through `gio launch`. Its release window appeared, loaded the four saved hosts and closed through WM_DELETE_WINDOW. The app process environment was checked: PATH=/nonexistent and the owned Unicode HOME. This was separate from WebDriver's direct launch.
- A settings lock file with mode 0400 caused an actual permission denial for UID 1000. The release UI showed the storage notice and disabled saving; the original marker remained and no replacement settings file was written.
- No owned profile-script marker was created. No private-key block appeared in app storage. Askpass was never invoked; source config/known-host hashes remained unchanged. The harness cleaned its containers, private network, agent, keys and temporary data. No production SSH alias was used.

Representative screenshots were visually reviewed for diagnostics, desktop startup, authentication failure/success and storage denial. Assertions use actual native IPC/OpenSSH and filesystem results, not screenshots alone.

An earlier run denied the entire app-data root. WebKit could not create its native window and the driver timed out before app diagnostics; it was not counted as a passing permission case. OS webview runtime storage must be writable. The final test isolates a real settings-file permission error while keeping WebKit runtime storage available, and the documented OS prerequisite remains explicit.

macOS Finder/Keychain: **PENDING — no macOS machine available**. Wayland, packaged installation and supported baseline distribution checks remain pending their native package/platform gates. Linux desktop launch here does not establish those results.
