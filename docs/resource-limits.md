# Resource limits and pressure checks

The app reads an optional `resource-limits.json` from its application data directory at startup. On Linux this is normally `$XDG_DATA_HOME/dev.containerdesk.app` or `~/.local/share/dev.containerdesk.app`; on macOS it is `~/Library/Application Support/dev.containerdesk.app`. The file contains only numeric resource limits. Restart to apply changes. Settings shows the effective values and warns if an invalid/unreadable configuration was ignored. No file is created or rewritten automatically.

```json
{
  "logLines": 8000,
  "logBytes": 1048576,
  "statsHistory": 60,
  "activeHosts": 1,
  "concurrentJobs": 4
}
```

| Setting | Default | Allowed range | Scope |
|---|---:|---:|---|
| `logLines` | 20000 | 1000–20000 | Each backend retained log queue/snapshot and frontend log buffer |
| `logBytes` | 8388608 | 262144–8388608 | Each such buffer; the frontend counts its displayed UTF-8 text including annotations |
| `statsHistory` | 360 | 60–360 | Points per container; at most 12 container histories |
| `activeHosts` | 1 | 0–1 | Live host sessions; zero refuses new live connections. Demo remains available. |
| `concurrentJobs` | 4 | 2–4 | Native SSH jobs per transport runner and the backend read admission pool |

The implemented app has one selected active connection. Host switching cancels the previous owner; this task does not introduce background multi-host sessions. The architecture's original three-host value was a proposed upper target, not evidence that this app supports three simultaneous connections. The runtime configuration can reduce current limits; it cannot expand product scope or override authorization.

Missing fields use defaults. The parser rejects unknown fields, fractions/out-of-range values, files larger than 4096 bytes, symlinks and non-regular files. It never blocks on a FIFO. On rejection the complete safe default set is used with a visible notice, rather than partially applying a malformed file. Configuration is frozen at launch, including the UI's validated numeric copy, so shrinking a live queue cannot race an active export.

Limits apply per bounded owner, not as a promise that process memory equals `logBytes`. Rust queues, one outstanding IPC batch, the frontend retained buffer, a paused display, renderer structures and allocators have separate costs. The log record ceiling remains 256 KiB; records/bytes discarded by retention remain visible. Event queues retain at most 512 hints plus 1024 deduplication keys and deliver at most 64 events per ACK. Log IPC batches remain at most 128 lines/256 KiB plus framing. Read responses remain capped at 16 MiB (listing) and 2 MiB (inspect); oversize responses fail explicitly. No silent partial inventory is returned.

A runner keeps its configured permit count until child cleanup/reaping completes. Its shutdown wait acquires that actual capacity, including reduced configurations. Two concurrent jobs can be consumed by logs and events; additional reads then report resource pressure rather than adding unbounded pending jobs. Native control-master/ProxyJump processes are separate from the job count. The pressure harness includes descendants plus background SSH processes matching runtime directories whose lease file is held by the exact app PID; it never adopts a different app instance or user-owned master.

Reproduce the checks with the pinned Node version:

```sh
node --expose-gc tests/bench/log-pressure.ts
cargo test --manifest-path src-tauri/Cargo.toml --locked pressure_ -- --ignored --nocapture
python3 tests/lab/pressure.py --tools-dir /path/to/extracted-native-tools --artifacts /tmp/containerdesk-pressure-results
```

The native harness creates disposable Docker containers, keys, SSH servers, an actual strict ProxyJump and an isolated X11/D-Bus session. Its gate supplies **synthetic** 1000-container summaries, a roughly 1 MiB inspect response and high-rate events over real SSH. Logs come from an actual owned Docker container. These synthetic inventories are not a claim that 1000 real Docker containers were run. The harness samples the actual release app and descendants for 120 seconds, reports RSS/PSS (including leased background masters), frame intervals, filter roundtrip latency, DOM row counts and retained log bounds, then requires cancellation/disconnect to return owned SSH counts to baseline with no zombies and every observed SSH PID/start-time identity reaped.

The short Node benchmark explicitly invokes GC to measure retained data, not whole-app steady-state memory. Rust parser/queue figures use unoptimized native test builds. Native app RSS measurements include shared pages more than once; PSS is reported separately when readable. The plateau gate compares median RSS in the last 30 seconds to seconds 30–60 and allows 32 MiB growth; raw samples remain available so the claim can be assessed beyond a binary pass.

The pressure review found that every successful inventory timestamp used to remount the inspect panel. A sustained event stream therefore reset the selected tab and repeatedly fetched large inspect payloads. The panel now stays mounted for the same full host/session/container identity. A newer inventory marks its details as an earlier snapshot; **Refresh details** makes one explicit new read. Inventory refreshes still mask revealed values and invalidate pending reveal responses. Changing host, session or container still discards the complete detail state. This keeps the existing virtualization and ACK batching rather than adding another unmeasured rendering layer.
