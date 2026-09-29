# Versions, manual updates and rollback

ContainerDesk 0.1.0 uses **manual installer updates**. It does not check for updates, download installers in the background or install them automatically. There is no updater plugin, placeholder service endpoint, telemetry client or hosted runtime asset. The production webview's network policy permits only Tauri IPC. A user-selected SSH connection still makes its expected SSH traffic; system WebKit/desktop services are outside the app's telemetry claim.

## One application version

`package.json` is authoritative. `src-tauri/tauri.conf.json` reads `../package.json`; the frontend displays the native `app_version` result. Cargo's application version and the two root npm-lock values are aligned by the script below. Support reports use Cargo's aligned version. Dependency versions/checksums are left unchanged.

```sh
python3 scripts/version.py                  # refuse any disagreement
python3 scripts/version.py --set 0.1.1      # example only: edit a reviewed release branch
npm run verify
```

The initial installer scheme accepts numeric `MAJOR.MINOR.PATCH`, with no `v` prefix, leading zeroes or prerelease suffix. Candidate status belongs in evidence, not a new advertised release. No version was bumped for 056. The script validates the starting state, prepares all edits before writing and restores original files on ordinary write failure. Forced termination can interrupt multiple-file changes; inspect the Git diff and rerun the check before building. It never tags, commits or publishes. `npm run verify`, dependency installation and CI packaging now fail during preflight on a version mismatch. Review and commit the version and lockfiles together, then build native installers from that exact commit.

## Obtain and verify a package

There is currently **no approved public download** and no public-publishing workflow. The owner-controlled source is [alekpopovic/container-desk](https://github.com/alekpopovic/container-desk); development packages are attached to successful [native CI runs](https://github.com/alekpopovic/container-desk/actions/workflows/ci.yml), retained for 14 days. Select a reviewed exact source commit and the artifact for your CPU/OS. Do not use a similarly named repository or infer approval from a green build. A future approved public release must be linked here with its final manifest, platform evidence and signatures; no substitute endpoint is shipped now.

After extracting a trusted CI artifact, verify the complete manifest in that directory:

```sh
sha256sum --check SHA256SUMS          # Linux
shasum -a 256 --check SHA256SUMS      # macOS
```

Check package metadata's `sourceCommit`, `sourceDirty: false`, `target`, version, toolchains and verification-report hash against the selected run. Hashes on their own do not authenticate a download's author. Signed distribution has additional requirements in [signing](signing.md), including a verified owner key for any detached Linux manifest signature. Current unsigned/ad-hoc Mac packages are local development output, not approved internet installers.

## Preserve settings before replacement

1. Disconnect hosts, close terminals and quit every ContainerDesk process. A pending mutation may have an unknown remote outcome; read/reconcile it before trying another action. Installing/rolling back the desktop app does not reverse a Docker operation.
2. Locate the actual application-data folder. Linux normally uses `$XDG_DATA_HOME/dev.containerdesk.app`, falling back to `~/.local/share/dev.containerdesk.app`; macOS uses `~/Library/Application Support/dev.containerdesk.app`. Preserve any custom data-directory choice.
3. Make a separate, dated private copy of the entire `preferences` directory **outside the live app folder**, preserving permissions (directories 0700, files 0600). Label it with app version, settings `schemaVersion` and source commit if known. Verify the copied `settings.json` hash against the original. Save the previously trusted installer and its manifest too. On a first run there may be no settings file yet.
4. Keep that copy until the new version has been checked and rollback is no longer needed. Host names, paths and labels are not credentials but can be sensitive; do not upload the backup into an issue. The optional `activity` directory contains sanitized local history and has its own schema; preserve it separately if wanted. The application never backs up your SSH keys/config or `known_hosts`; manage those independently using your existing secure process.
5. Install the exact reviewed replacement. For deb, use the package manager; for AppImage, replace the chosen file and retain its executable bit; for Mac, replace the app only after quitting. See [Linux packages](linux-packages.md) and [Mac packages](macos-packages.md). Reopen, check displayed version, saved hosts/theme/SSH selection, diagnostics and a read-only connection before enabling management.

## Schema and rollback limits

Current preferences use schema 3. The app reads/migrates schema 1 and 2; migration retains the exact old primary as `preferences/settings.previous.json`. Every subsequent successful preference write rotates that **single** previous file. It is a recovery buffer, **not a permanent pre-upgrade backup**. Unknown newer schema numbers are left untouched and settings writes disabled; the app must not silently replace them with defaults. Corrupt-file recovery retains the damaged original before trying a compatible previous file.

To roll back, quit all instances, retain a private copy of the post-upgrade files, reinstall the previously verified compatible app, then restore its matching pre-upgrade `preferences` copy while the app is closed. Restore the whole compatible set rather than mixing current and previous files from different versions; keep ownership and 0700/0600 modes and avoid symlinks. Locks do not authorize restoring into a running process. Use the older app's own schema documentation; changing `schemaVersion` by hand is not a migration. There is **no reverse migration**, no guarantee that an arbitrary old app can read schema 3, and no restoration of remote Docker state or live terminals. Reopening an older backup with the current app migrates it again.

The 056 test uses a committed schema-2 fixture and the real atomic file adapter: migrate to 3, rotate the previous file with another write, prove the independent copy remains byte-exact, restore it while closed, and reopen/migrate it again. It does not claim execution of a nonexistent older release binary.

## Future updater gate

An updater remains deferred. Enabling one requires a separately reviewed implementation, real owner-controlled hosting/credentials, signed metadata/artifacts, a pinned trusted verification key and key rotation policy, an exact supported-platform acceptance run, and user-visible update/rollback behavior. Developer ID notarization alone is not an updater signature. No fake endpoint, self-generated owner signing key or automatic-install path is present.
