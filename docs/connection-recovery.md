# Connection recovery and exit

The application keeps its existing native OpenSSH transport. Its private multiplexed master is checked every 500 ms, with a three-second control-command deadline; OpenSSH keepalives use 15 seconds and two unanswered messages. Direct fallback has no master, so it verifies the fixed access marker every five seconds with a five-second process deadline. Local admission pressure defers that probe. Health checks do not extend the application's idle lease. Strict host verification and the selected trusted SSH configuration apply to every connection; no password prompt, key copying or agent forwarding is introduced.

A failed health check broadcasts cancellation to the connection's channels immediately. The session worker publishes a new generation with a connection-loss diagnosis, removes the old daemon binding and closes only its own transport. Old scope/input requests fail, even if a late operation returns. A failed transport probe is distinguished from a positively different Docker daemon/context: network failure does not masquerade as daemon drift. Actual daemon drift retains the existing stale-session rejection and explicit reselection workflow.

The always-mounted saved-host controller polls the authoritative connection snapshot and also reconciles on focus, visibility/online events and a large timer gap. These browser events are hints, not proof of OS suspension or restored connectivity. Automatic reconnect is armed only for the currently selected saved host after it was observed ready in this application lifetime. Merely opening the app or saving a host does not connect it.

Reconnect delays are one, two and four seconds, with at most three attempts. Hidden windows do not spend attempts. Authentication/trust/configuration failures stop automatic retries; manual disconnect, metadata changes, workspace/host changes and unmount clear recovery ownership. A failed native admission consumes its attempt and cannot create an unbounded loop. Briefly flapping connections retain their spent budget; a minute of stable readiness or an explicit user action resets it. Late connection results may close only their exact returned token.

A new ready session starts read-only and loads new snapshots. Logs start explicitly and display a gap on every new observation; no continuous history is implied across a new connection. Terminal tabs are disposed and remain closed. Mutations and terminal input are never replayed. SSH setup may evaluate the selected trusted config's executable directives again, just like an explicit reconnect.

## Exit ownership

The first Tauri ExitRequested event prevents immediate exit, fences new backend operations synchronously and starts asynchronous cleanup. Repeated exit requests remain prevented while cleanup runs. A final Exit event also invokes the same bounded cleanup as a backstop for runtime paths that skip ExitRequested. Terminals, sessions, log/event owners and the shared diagnostic/process runner are cancelled together, with a ten-second overall deadline. The runner's global stop signal cancels both finite commands and streams; their owners kill only their own process groups, reap their direct children and release capacity after cleanup. Other runner instances remain untouched.

After cleanup completes or its deadline expires, the app requests exit once more. Existing private-runtime lease, socket inode and conservative stale-socket recovery rules remain in force. Arbitrary SSH processes, user-owned masters, known_hosts and config files are never cleaned. The deadline bounds application shutdown; an uninterruptible OS process cannot be guaranteed to finish by an application deadline, and killing a local SSH client cannot undo remote commands or detached workloads.

The native test uses connection resets and master termination in an explicitly owned Linux loopback SSH/Docker lab. It is not a physical laptop suspend test or macOS evidence. Results and exact commands belong in the prompt 041 evidence.

API references: [Tauri RunEvent](https://docs.rs/tauri/2.12.0/tauri/enum.RunEvent.html), [ExitRequestApi](https://docs.rs/tauri/2.12.0/tauri/struct.ExitRequestApi.html).
