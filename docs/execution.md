# Execution authorization

On 2026-09-28 the user requested: “odradi sve promptove do kraja”. This supersedes the earlier per-turn stop-after-one instruction for this run. Execute prompts sequentially through the tracker, with independent acceptance checks, evidence, commit and push for each prompt. Continue only after the prerequisite is actually complete. Never bypass a missing native platform or disposable SSH lab gate. Preserve immutable prompt text/hashes and unrelated work.

Standing commit/push authorization remains in AGENTS.md. Push only to the current configured upstream, without force. A failed push or essential environment check must be reported accurately.

On 2026-09-29, after the public-publisher capability and its approval blocker were explained, the user instructed: “ok odravi sve promtove do kraja bez toga”. Continue all remaining prompts sequentially **without adding the manual workflow for public publishing**. Resolve 052 by this explicit scope change; keep the original prompt immutable. CI artifacts and local package/signing-readiness work remain in scope. Do not publish a release or install an equivalent publisher indirectly. Missing native evidence still remains a real gate.
