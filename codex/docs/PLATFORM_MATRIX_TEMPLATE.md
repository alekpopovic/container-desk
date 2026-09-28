# Native acceptance matrix — copy to docs/platform-matrix.md

Initial entries are **NOT RUN**. Filling this template is part of implementation, not proof supplied by the prompt package.

| Check | Linux x86_64 | macOS arm64 | macOS x86_64 |
|---|---|---|---|
| Exact OS version / app commit | NOT RUN | NOT RUN | NOT RUN |
| Native package install and GUI launch | NOT RUN | NOT RUN | NOT RUN |
| OpenSSH path and minimal GUI environment | NOT RUN | NOT RUN | NOT RUN |
| Direct connection | NOT RUN | NOT RUN | NOT RUN |
| ProxyJump via private test target | NOT RUN | NOT RUN | NOT RUN |
| Encrypted identity through OS agent | NOT RUN | NOT RUN | NOT RUN |
| Unknown/changed host key handling | NOT RUN | NOT RUN | NOT RUN |
| Container list / inspect | NOT RUN | NOT RUN | NOT RUN |
| Logs follow / cancel / output bounds | NOT RUN | NOT RUN | NOT RUN |
| Stats / reconnect / stale snapshots | NOT RUN | NOT RUN | NOT RUN |
| Start / stop / restart / unknown outcome | NOT RUN | NOT RUN | NOT RUN |
| Compose paths and service actions | NOT RUN | NOT RUN | NOT RUN |
| Terminal input / resize / Ctrl-C / cleanup | NOT RUN | NOT RUN | NOT RUN |
| Sleep/wake or network interruption | NOT RUN | NOT RUN | NOT RUN |
| Unrelated SSH master remains running | NOT RUN | NOT RUN | NOT RUN |
| Redacted diagnostics export | NOT RUN | NOT RUN | NOT RUN |
| Package SHA-256 and evidence link | NOT RUN | NOT RUN | NOT RUN |
| Public signing/notarization | Separate distribution choice | UNVERIFIED | UNVERIFIED |

Record exact commands, logs and screenshots elsewhere and link each completed cell to evidence. A cross-compiled package may have a valid build result while runtime remains NOT RUN. Missing notarization must not be mislabeled as failed local functionality or as successful public distribution.
