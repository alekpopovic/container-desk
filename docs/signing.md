# Optional package signing and notarization

Current evidence is **configuration-ready without credentials**. No Developer ID certificate or Apple account credentials have been supplied; notarization, signed-app runtime and internet-download Gatekeeper acceptance are **unverified**. Local package/runtime evidence in 053–054 remains separate. The user excluded a public-publishing workflow; none is installed. The optional script creates local output and submits only to Apple's notary service when explicitly invoked with owner-provided credentials. It never creates a GitHub release, tag or signing key.

## macOS preparation

Use an isolated, disposable native Mac with Xcode command-line tools, Python 3.12+, network access to Apple and the ordinary app archive, package metadata and complete verification report from the same successful native CI run. Check the source revision and checksums first. The script binds the archive and executable hashes to that report, requires a clean source revision and accepts only the current single-executable Tauri bundle layout. New frameworks/helpers require a signing review, not `codesign --deep` signing.

The owner supplies these values through a trusted CI job's secret environment, never committed files or command text:

| Secret | Owner-provided value |
|---|---|
| `APPLE_CERTIFICATE` | Base64 of the exported Developer ID Application identity and private key in a password-protected PKCS#12 file |
| `APPLE_CERTIFICATE_PASSWORD` | PKCS#12 export password |
| `APPLE_SIGNING_IDENTITY` | Exact `Developer ID Application: … (TEAMID)` identity |
| `APPLE_TEAM_ID` | Ten-character Apple team ID |
| `APPLE_ID` | Account email authorized for this team |
| `APPLE_PASSWORD` | App-specific password, not the account login password |

A future **separately authorized, trusted** job can bind `env: APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}` and each other name in the same way. Do not attach these secrets to the existing push/PR jobs, fork builds, unreviewed scripts, broad job environments or `pull_request_target`. No secret environment or signing workflow is activated by this change. Prefer a protected environment and a reviewed exact commit when such a job is separately introduced. Do not persist values through `GITHUB_ENV`, enable shell tracing or upload the temporary directory. Apple's command-line tools necessarily receive some credentials as process arguments; use an isolated runner with no other user workloads.

First run the read-only preflight:

```sh
python3 scripts/macos_signing.py
```

Exit 2 lists missing secret **names** or malformed field names; it creates no output/keychain and makes no network call. Exit 0 means only that credential fields have the expected shape. It cannot prove that a certificate/password is valid. No credential value is printed. Without credentials this is the expected, tested completion branch of prompt 055.

With explicit owner authorization and all values supplied in the environment, the opt-in invocation is:

```sh
python3 scripts/macos_signing.py --execute \
  --metadata dist-artifacts/containerdesk-0.1.0-aarch64-apple-darwin.json \
  --report test-results/verification.json \
  --output dist-artifacts/signed
```

Use the matching architecture/version metadata, never rename another build's report. The output directory must not exist. The script signs a private copy, leaving the verified unsigned input unchanged. It imports the identity into an owned temporary keychain, checks its exact Developer ID identity, enables hardened runtime with an empty entitlement dictionary and a secure timestamp, verifies the resource seal, identity/team and entitlements, then submits a ZIP to `notarytool`. It requires `Accepted` and an issue-free notary log, staples and validates the app ticket and runs `spctl`. It creates a DMG containing that stapled app and Applications link, signs the image, repeats notarization/ticket/Gatekeeper checks and archives the stapled app. Each native command has a deadline; Apple submissions get 20 minutes plus process shutdown time. A submission timeout is an unresolved Apple outcome, not a reason for automatic resubmission. Review its status with the owner before repeating.

The temporary PKCS#12 is deleted after import. All keychain consumers use the explicit temporary keychain; the script never replaces the user's default keychain or search list. Cleanup deletes the owned keychain after success, partial import, rejection, command timeout and ordinary interruption. Cleanup failure prevents success evidence. Forced machine termination cannot run cleanup: discard the ephemeral runner and its workspace. There is no support for retaining credentials on a shared/self-hosted runner. No private signing keys are generated.

`signing-verification.json` and new SHA256SUMS appear only after all checks **and cleanup** succeed. Failed output must not be distributed; there is no success proof. Even successful output retains `publiclyVerified: false`: install the exact final signed DMG through an actual downloaded/quarantined path on each supported Mac, verify Gatekeeper, Finder launch, native SSH/agent and remote Docker behavior, and attach that evidence before a separate public-release decision. Do not remove quarantine or disable Gatekeeper to satisfy this gate. Signing changes hashes, so unsigned-package runtime proof alone is insufficient.

## Linux integrity and optional owner signature

Unsigned deb/AppImage output always includes SHA256SUMS covering the actual packages and metadata. `sha256sum --check SHA256SUMS` establishes consistency with the manifest, not the identity of its author. For a future authorized distribution, the owner may sign this exact final manifest using an existing GPG signing key on their controlled machine:

```sh
gpg --local-user OWNER_VERIFIED_FINGERPRINT --armor --detach-sign SHA256SUMS
gpg --verify SHA256SUMS.asc SHA256SUMS
sha256sum --check SHA256SUMS
```

The user must authenticate the public-key fingerprint independently, then verify the manifest signature before trusting its hashes. A signature on one architecture/version's manifest covers only those exact files. Keep keys/passphrases outside the repository and ordinary CI. No key generation, import, upload, signature publication or automatic updater is implemented. Embedded AppImage signatures are not a substitute for explicit verification; this project uses an optional detached manifest signature so both formats share one policy. No Linux signature was produced in 055.

## Sources and validation limits

Reviewed 2026-09-29: [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/), [Tauri 2.12.0 signing implementation](https://github.com/tauri-apps/tauri/blob/tauri-v2.12.0/crates/tauri-bundler/src/bundle/macos/sign.rs), [Apple notarization requirements](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution), [Apple notarization troubleshooting](https://developer.apple.com/documentation/security/resolving-common-notarization-issues), [Apple custom notarization flow](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow), and [Tauri Linux signing](https://v2.tauri.app/distribute/sign/linux/). The project-specific post-packaging flow preserves the checked unsigned artifacts and independently checks Apple results; it does not inherit a bundler's success message as public-release evidence.

The Linux-executed failure tests check missing/malformed credentials, signature-result rejection and cleanup control flow with fake native commands. They are **not** execution of `security`, `codesign`, `notarytool`, `stapler` or signed application code. The full credentialed path needs actual owner credentials and native Mac verification before use for distribution.
