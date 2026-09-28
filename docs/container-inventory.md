# Container inventory

The live table is bound only after an explicitly connected saved host reaches Ready. Rust checks the current host, daemon, session ID and generation before and after each read, in addition to the policy registration. The listing adapter checks the actual Docker binding around the command. A changed binding revokes the native session. Management and terminal permissions remain disabled.

The connection owner alone closes its master. Reads and the idle health watcher borrow clonable command capabilities, without holding the owner's mutex across remote work. Closing the owner signals shutdown and cancels/reaps outstanding reads. Four nonqueued backend read permits bound concurrent calls.

The frontend keeps at most three in-memory snapshots, keyed by saved HostId and daemon ID. Selection uses full container ID within that key. Every successful result has a timestamp; refresh, failures and reuse across session generations visibly mark old data stale. Request serials and full scope checks discard late responses. Host/mode changes fence the render before effect cleanup, and Demo/Live transitions clear the cache namespace. No container snapshots are persisted.

Name, state, health, image, ports and age sort independently; name/image/full-ID search combines with a state filter. More than 200 matching rows use a 56-pixel virtual row window capped at 24 rendered rows, including overscan. A synthetic 1,000-row browser check measured 26.1–34.6 ms from search input to filtered result across six viewport/theme configurations. This measures browser fixture responsiveness, not remote latency or native throughput. Native Linux was separately exercised with two actual Docker containers through direct and ProxyJump SSH.

CLI display fields retain the limitations described in `container-listing.md`: health remains unknown until precise data is available, age is a CLI display string with creation-time sorting when parseable, and list label values are not exposed. Refresh is explicit; polling, inspect details and streams belong to subsequent prompts.

## Verification reproduction

Use the pinned toolchain from the repository and the disposable lab prerequisites from `checkpoints/018-ssh.md`. The lab's optional populated-list mode grants SYS_ADMIN and an AppArmor exception only to its owned target for Docker load; it never mounts the host Docker socket into that target.

```sh
npm run test:ui -- tests/ui/workspace.spec.ts tests/ui/containers.spec.ts
python3 tests/lab/ssh_auth.py --engine --listing --inventory \
  --native-driver /tmp/containerdesk-018-tools/bin/tauri-driver \
  --webkit-driver /tmp/containerdesk-018-tools/webkit/usr/bin/WebKitWebDriver \
  --native-artifacts docs/verification/020-native
```

The last command runs actual compiled Rust resource reads, a marker-observed hanging read followed by cancellation, and controlled identity drift through fixed test wrappers. It then drives the actual release UI with WebKitWebDriver. Wrappers inject faults only in the disposable lab; they are not remote application dependencies. Artifact directories are explicit so later runs cannot overwrite historical checkpoint screenshots.
