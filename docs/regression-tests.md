---
title: "Frontend and parser regressions — 048"
section: "Reviews & evidence"
icon: "🧪"
---

# 🧪 Frontend and parser regressions — 048

Normal tests use fixed data and mock IPC. They do not discover a user's SSH aliases, contact production hosts or require Docker. Local subprocess tests exercise quoting/resolution/cancellation only against owned temporary inputs. Native network/pressure tests remain opt-in ignored Rust tests with explicit manifests and bounded lab commands.

Use the pinned Node/npm and Rust toolchains from [toolchains](toolchains.md):

```sh
npm run check
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run test:ui
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
```

`npm run check` includes types, lint, format and Node tests. Playwright starts its own loopback Vite fixture server at port 1431; it has two workers, zero retries, and six fixed viewport/theme projects. Routine screenshots use Playwright’s ignored per-test output directory. The full-suite run exposed an old accessibility test writing into sealed 044 evidence; that destination was corrected and only this run’s regenerated historical images were restored from HEAD. Native desktop execution and remote server operations require separate `tests/lab` commands and evidence. Rebuilding or viewing a browser fixture is not native verification.

| Desired behavior | Regression entry points |
|---|---|
| Identical names/IDs on different hosts cannot share replies or selection | `client.test.ts`, `container-cache.test.ts`, images/volumes/networks public-adapter tests; component stale-host cases |
| Late success/error cannot replace new selection/session/daemon state; stale empty data stays distinguishable | `client.test.ts`, `container-cache.test.ts`, `read-scheduler.test.ts`, `connection.spec.ts`, inspect scope/reveal tests |
| Trust/authentication/timeout errors offer specific guidance and wait for explicit retry | New `connection-errors.spec.ts` uses real `ConnectionPanel` through deterministic mock IPC; advances a fake clock by 60 seconds and requires exactly one start until the user retries |
| Log control strings are inert, retained bytes/lines are bounded and drops remain visible | `log-viewer.test.ts`, `live-logs.test.ts`, `logs.test.ts`, `logs.spec.ts`; independent UTF-8 byte accounting and cancellation/ACK bounds |
| Mutation uncertainty never becomes success or automatic replay | `management.test.ts`, `mutation-reads.test.ts`, `management.spec.ts`, batch/Compose partial/unknown tests and Rust policy/activity tests |
| Old prepared confirmation cannot appear/dispatch after replacing its session | New held-confirmation case in `management.spec.ts`; host-keyed production panel and public prepare adapter, with no elapsed-time race or server |
| Null/missing Docker data does not become fabricated start time, health, PID limit or published port | New observed-created Rust parser test and public `inspectContainer` test; [documented actual remote CLI fixture](https://github.com/alekpopovic/container-desk/blob/main/tests/fixtures/docker/README.md) → Rust serde fixture → TypeScript adapter |
| Expired/cancelled confirmations and inert hostile text remain safe | Existing management/Compose/terminal dialog and accessibility cases; default inspect/support secret projection checks |

The new fixtures are development/test inputs only. No automation command, network dependency or mock transport was added to the production app. Rust's generated fixture drift check ties the wire sample to the actual parser; tests assert the intended distinction between absent and zero rather than snapshotting markup or mirroring parser expressions.

## Controlled fault demonstration

For 048 only, a Python `try/finally` harness temporarily removed the production inspect parser's `0001-01-01` start-time normalization and ran only `observed_created_container_preserves_nulls`. The new regression failed with exit 101: `Some("0001-01-01T00:00:00Z")` instead of `None`. The harness restored the parser byte-for-byte (SHA-256 `a458338611e7b5d4ca9d30a9e90261fa95180260c6646cdabb37fc327da9112a`) before the full Rust suite ran successfully. No faulty production code remains in the diff. [Sanitized fault evidence](verification/048/controlled-fault.txt).

Final 048 verification: **87 Node**, **151 Rust** (28 explicit opt-in tests ignored), and **348 Playwright** checks across all six projects passed. The remote fixture corroboration was an additional bounded actual SSH/CLI observation; it is not included in those unit/mock counts. See [048 evidence](https://github.com/alekpopovic/container-desk/blob/main/codex/tracking/evidence/048.md) for exact commands and limits.
