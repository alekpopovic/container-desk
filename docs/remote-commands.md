---
title: "Remote command construction"
section: "Hosts & SSH"
icon: "🔐"
---

<!-- {% raw %} -->

# 🔐 Remote command construction

OpenSSH sends a command string to the remote login shell. Local argument arrays prevent local shell interpolation, but they do not preserve remote arguments by themselves. ContainerDesk supports a POSIX-compatible remote shell and encodes one remote command string, appended as exactly one SSH argv element. [OpenSSH command behavior](https://man.openbsd.org/ssh#DESCRIPTION).

`ssh::quoting` is the single encoder: wrap every argument in single quotes, and represent each embedded apostrophe by closing the quoted segment, escaping the apostrophe outside it, then opening a new segment. Empty values remain an empty argument; whitespace, newlines, dollar/backtick substitutions, glob characters and Go templates remain literal data. NUL is rejected because it cannot be passed in argv. Bounds are 16 KiB per input argument, 128 tokens and 128 KiB for the final encoded command. [POSIX quoting](https://pubs.opengroup.org/onlinepubs/9799919799/utilities/V3_chap02.html#tag_19_02_02), [GNU single-quote explanation](https://www.gnu.org/s/bash/manual/html_node/Single-Quotes.html).

`docker::prepare` accepts an immutable validated registry CommandPlan and DockerCommandConfig, never a label, display name or pasted script. It preserves the category, response kind and deadline. The configuration constructor accepts an optional absolute remote Docker executable path, optional named context and explicit sudo choice. Paths must be at most 4,096 bytes, start with one slash, have a filename, contain no control characters or dot/dot-dot components; spaces and apostrophes are valid and encoded. This is syntax validation, not proof that a binary exists remotely. Without an override the fixed token docker uses the remote noninteractive PATH. ContainerDesk does not source profiles or start an interactive/login shell to discover the binary; OpenSSH's normal remote shell startup behavior remains the user's environment.

Context names use the conservative 1–256 ASCII letter/digit/dot/underscore/dash subset, starting with a letter/digit. A selected context is inserted as Docker's global --context option before every operation. Optional sudo is exactly sudo -n -- before the chosen Docker token, requiring an existing remote noninteractive policy. No credential input, sudo configuration edit or fallback is implemented. [Docker global options](https://docs.docker.com/reference/cli/docker/#options).

The registry validates full lowercase 64-hex container IDs; image IDs accept the full 64-hex form with an optional sha256: prefix. It rejects names, tags, shortened IDs, option-like identifiers and shell text. Numeric bounds remain operation-specific. Operand separators are fixed tokens, and {{json .}} remains one unchanged argument. A typed image-inspect read plan is prepared for later resource handlers; it does not expose a new IPC read view. Images remain read-only.

PreparedCommand cannot be deserialized from IPC or constructed from arbitrary shell text; Debug output omits command contents. Its structured SSH helper retains the original selected alias/config policy and rejects terminal plans, which require the separate future PTY path. Preparation itself is not authorization: the eventual dispatcher must consume a scoped authorization, revalidate at dispatch and never replay a mutation. Some prepared lifecycle plans have deadlines above the current snapshot runner's 30-second cap; their future executor must implement the operation-specific bounded policy before use. No mutation execution is added here.

Verification: [012 evidence](https://github.com/alekpopovic/container-desk/blob/main/codex/tracking/evidence/012.md). An explicitly inert local /bin/sh -c harness stands in for the remote POSIX shell and captures NUL-separated argv. This local shell is used only in tests; production SSH is still spawned directly. Hostile strings round-trip byte-for-byte without creating the injected marker. A fixed capture fixture behind an absolute path containing spaces/apostrophes verifies real command plans, contexts, image/container IDs and templates. No real SSH server or Docker daemon is exercised by these harnesses. Later capability/lab gates must reject incompatible remote shells and prove actual daemon behavior.

Prompt 016 adds [capability-bound configuration](docker-capabilities.md), which pins the resolved default endpoint with `--host` or a named context with `--context`. Global target/privilege options are constructed in one place for probe, read, write and terminal plans. Resource dispatch must verify the fresh daemon identity and authorization before using that backend-owned binding.

<!-- {% endraw %} -->
