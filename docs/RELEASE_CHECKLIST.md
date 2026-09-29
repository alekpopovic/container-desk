---
title: "Local release candidate checklist"
section: "Releases & platforms"
icon: "📦"
---

# 📦 Local release candidate checklist

Candidate: 0.1.0, source `ce33d6cd9bba5823642749a26d3e6d367a9bab73`, [native CI 36594720178](https://github.com/alekpopovic/container-desk/actions/runs/36594720178). Local directory: `dist-artifacts/release-candidate-0.1.0-ce33d6c/`. This checklist does not authorize publication.

## Required local gates

- [x] Review completed prompt evidence, known defects and [security/dependency findings](release-review.md).
- [x] Final local `npm run verify` passes all 13 checks after the last helper corrections.
- [x] All three native targets build and pass 13 standard checks; source is clean in original packaging metadata.
- [x] Native direct/private ProxyJump, encrypted agent, strict trust, inventory/inspect/logs/stats, exact management/Compose, PTY resize/Ctrl-C, recovery/no replay and owned cleanup pass on each target.
- [x] Ordinary deb/AppImage GUI and both Mac app/DMG GUI pass with acceleration/native attribution in [matrix](platform-matrix.md).
- [x] Independent manifest, byte-length, verifier digest, package source and installed/archive executable checks pass.
- [x] Final source contains no production app/lockfile change after the tested artifact commit.
- [x] Release notes state prerequisites, exact tested OS/architecture, signed/unsigned status and limits.
- [x] Local candidate contains only packages, original metadata/manifests, sanitized evidence and documentation; no private keys/credentials/raw logs or VM disks.

## Final smoke procedure

1. In the local candidate directory run `sha256sum --check SHA256SUMS`; on macOS use `shasum -a 256 --check SHA256SUMS`. Reject any missing/different file and any unexpected extra file.
2. Inspect `ARTIFACTS.json` and each `packages/<target>/*.json`: same clean source, version, native target and CI run; verify package bytes against original per-target manifests. Check all three standard and integration reports, two Mac package reports and final deb/AppImage reports against those identities.
3. Confirm all recorded native cleanup flags, no-replay event/state oracles and successful ordinary window/SSH behavior. The exact artifact set was already launched in 058; replaying those reports is a provenance recheck, not a new native runtime claim.
4. Run `python3 codex/scripts/track.py validate`, check the final tracker/evidence and review `git status --short`; preserve and separately report unrelated user changes.
5. Confirm no publisher/write permission, secrets, public tag/release or uploaded candidate was introduced. Keep local unsigned approval separate from the owner steps below.

## Optional owner distribution steps — not performed

- [ ] Supply the owner's real Apple signing/notarization configuration and execute the documented native signing flow, retaining cleanup proof.
- [ ] Independently test signed downloaded artifacts under normal Gatekeeper/quarantine behavior; do not disable protection or reuse unsigned evidence as that proof.
- [ ] Decide on Linux owner signatures and an approved public distribution channel.
- [ ] Authorize any public release separately. No workflow for public publishing is installed by this task.

Retest after any application/native dependency or packaging-content change. Documentation-only changes do not relabel an existing tested binary. Physical sleep/wake, Wayland, broad Mac VoiceOver and other OS versions remain optional future coverage with their own actual execution evidence.
