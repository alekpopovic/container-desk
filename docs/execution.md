---
title: "Execution authorization"
section: "Reviews & evidence"
icon: "🧪"
---

# 🧪 Execution authorization

On 2026-09-28 the user requested: “odradi sve promptove do kraja”. This supersedes the earlier per-turn stop-after-one instruction for this run. Execute prompts sequentially through the tracker, with independent acceptance checks, evidence, commit and push for each prompt. Continue only after the prerequisite is actually complete. Never bypass a missing native platform or disposable SSH lab gate. Preserve immutable prompt text/hashes and unrelated work.

Standing commit/push authorization remains in AGENTS.md. Push only to the current configured upstream, without force. A failed push or essential environment check must be reported accurately.

On 2026-09-29, after the public-publisher capability and its approval blocker were explained, the user instructed: “ok odravi sve promtove do kraja bez toga”. Continue all remaining prompts sequentially **without adding the manual workflow for public publishing**. Resolve 052 by this explicit scope change; keep the original prompt immutable. CI artifacts and local package/signing-readiness work remain in scope. Do not publish a release or install an equivalent publisher indirectly. Missing native evidence still remains a real gate.

On 2026-09-29, after completion of the 60 prompts, the user explicitly requested: “ok sad napravili rilis i gurni sve instalacio fajlove na github da moze da se instaliraju” (create the release and upload all installation files to GitHub). This separately authorizes manual publication of the already verified 0.1.0 installers, their checksums/metadata and a release tag. It supersedes the earlier no-publication limit for this release only; it does not request a publisher workflow, automatic updates, signing credentials or re-enabling GitHub Actions. Preserve the owner's `c2464b8` commit disabling Actions and publish the exact existing tested package bytes with unsigned/notarization limits visible.

## Manual release follow-up authorization

After completing all original prompts and publishing v0.1.0, the owner explicitly requested a manually entered release version, reuse of the existing CI, all installers and the complete commit range since the previous release, followed by updates to the docs Releases/Downloads pages and a new Pages build. This authorizes the separate [manual release workflow](release-workflow.md). It supersedes the earlier omission of a public-publisher workflow for this follow-up; original prompt hashes and historical evidence remain unchanged.
