---
title: "IPC contract"
section: "Build & design"
icon: "🛠️"
---

# 🛠️ IPC contract

Rust `src-tauri/src/domain.rs` owns wire DTOs. `ts-rs` 12.0.1 is a development-only derive dependency. `npm run ipc:generate` writes the committed TypeScript declarations and Rust-serialized JSON fixture. Ordinary `npm run rust:test` compares both files without rewriting them; `npm run check` type-checks the renderer and consumes that same JSON through the IPC adapter. Generated TypeScript is excluded from formatting so checks remain deterministic. Generator documentation: https://docs.rs/ts-rs/12.0.1/ts_rs/.

Wire fields use camelCase, enum values snake_case, optional fields explicit null. IDs are string newtypes: host `h_` plus 32 lowercase hex digits, session `s_` plus 32, subscription `sub_` plus 32, and full Docker container IDs 64 lowercase hex digits. These are opaque application identifiers, never SSH aliases or shell arguments. Rust validates untrusted requests before lookup. Generations are nonzero u32 values, exactly representable in JavaScript; future issuers must use checked increments and replace an exhausted session instead of wrapping. TypeScript string aliases are serialization types, not authorization.

`HostSelection` binds host ID to a renderer selection epoch. `SessionScope` additionally binds a backend-issued session ID/generation and daemon identity. Connection replies echo the selection; inventory requests, replies, rows and cancellation carry the full scope. The renderer adapter accepts a current-selection/current-session getter and rejects late success **and error** results. Views must use these adapters and discard `stale_session` results instead of replacing the new host's state. Backend lookup rejects mismatches independently of frontend behavior.

`app_version`, `list_hosts`, `connect_host`, `list_containers`, `cancel_subscription`, `get_preferences`, `set_theme`, `dependency_diagnostics` and `set_ssh_executable` are registered and allowed to the local main window. The dedicated set_ssh_executable command accepts a user-chosen OpenSSH path and performs the bounded validation described in native-dependencies.md. There is no arbitrary command/script execution API. The private JS `call` helper is not exported. Capability flags describe availability; they are not grants and cannot replace future Rust policy checks.

In 004 the saved-host list is empty, valid connections return `host_not_found`, missing sessions return `session_not_found`, and there is no process launcher. A known test session returns `feature_unavailable` for inventory and `subscription_not_found` for cancellation. These are deliberate increment boundaries, not successful transport stubs. Storage, session creation and actual subscription cancellation arrive in later prompts. Session-map writes have no production entry yet; later transport must enforce the architecture's three-host and bounded-job limits.

Errors contain a stable code, fixed safe message and optional validated scope. They have no arbitrary diagnostic payload. The renderer uses a complete code/message table (checked against the generated union) and never displays untyped bridge text. No requests are retried. Container detail contains environment names and a masking flag, with no unrestricted inspect/environment value field; parsers added later must retain this boundary.

Tauri mock-runtime command tests exercise the registered Rust handlers and serialization, not native windows, transport or production ACL behavior. The native build validates registered permission generation; platform runtime acceptance remains separate. Invalid JSON shape/type can be rejected by Tauri before a command is called; domain validation errors inside a correctly shaped request use `AppError`.

Prompt 007 adds inspect/log read commands and internal registered confirmation/mutation/terminal handlers. The latter have no granted window capability yet. Runtime authorization, fixed argv registry and one-use confirmations are documented in [operation policy](operation-policy.md). Session scope ownership is now centralized in PolicyEngine.
