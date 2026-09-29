---
title: "Manual release workflow"
section: "Releases & platforms"
icon: "📦"
---

# 📦 Publish a release

Open **[Actions → Manual release](https://github.com/alekpopovic/container-desk/actions/workflows/release.yml) → Run workflow**, select **main**, enter a version such as `0.1.1` (or `v0.1.1`) and choose whether it is a pre-release. Click **Run workflow** once. The version must be newer than the preceding published release.

This explicitly invoked workflow uses the existing **[Native checks and unsigned packages](ci.md)** workflow. It runs the same Linux and macOS matrix, verification, packaging, native SSH/Docker tests, Mac package acceptance and desktop probes. It does not maintain a separate build matrix.

## What happens

1. **Prepare.** Freeze the selected `main` commit. Align `package.json`, `package-lock.json`, `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock` with your version, and push a version-only commit to `main` when needed. Tauri already reads the package version. A normal push refuses a concurrent update; no force push is used.
2. **Check and package.** Call `.github/workflows/ci.yml` using that exact prepared commit. All required jobs must pass before publication. The reusable workflow retains its read-only token; it receives no publisher credentials.
3. **Publish.** Download only this workflow run's native package artifacts. Verify all target/version/source/run metadata, source lockfile hashes, installer sizes and SHA-256 values. Create a draft, upload the complete set, check GitHub's stored asset digests, then publish. A failed upload leaves an unpublished draft.
4. **Update the website.** Generate [Downloads](downloads.md), [Releases](releases.md) and `docs/_data/current_release.json` from actually published releases. Push only these docs changes on top of current `main`.
5. **Rebuild Pages.** Explicitly request a GitHub Pages build for `main/docs`, wait for the build to succeed and check that the public Downloads page advertises the new version. A token-authored push alone is not treated as proof that Pages ran.

A completed run links to the release and documentation in its job summaries. Repository rules must permit the normal version/docs pushes by `GITHUB_TOKEN`. The workflow does not bypass protected branches; a rejected push stops the corresponding job with its actual error.

## Included downloads

| Platform | Installation files |
|---|---|
| Linux x86_64 | `.deb` and `.AppImage` |
| macOS Apple Silicon | `.dmg` and `.app.tar.gz` |
| macOS Intel | `.dmg` and `.app.tar.gz` |

Every release includes these **six installation files**, three original native-build metadata JSON files, `release-provenance.json`, `COMMITS.md` and one consolidated `SHA256SUMS` — **12 assets**. GitHub additionally provides its normal source archives. The manifest covers every attached file except itself.

Packages remain unsigned. macOS Developer ID signing, notarization and downloaded Gatekeeper acceptance are not supplied by this workflow. Pre-release is selected by default; unchecking it changes the release classification, not signing status. Existing native CI gates do not establish new Linux installed-GUI acceptance for every release.

## Which commits are included

The starting point is the **most recently published non-draft release, including pre-releases**, rather than GitHub's stable-only “latest” endpoint. Its tag must be an ancestor of the selected source. The ending point is the exact versioned commit passed to CI and tagged for publication.

The notes enumerate every commit in `previous-release-commit..release-commit`, including merged branch commits, merge commits and the version bump. They include commit links and a comparison link. The same full list is attached as `COMMITS.md`; unusually large histories use that complete attachment instead of truncating the list to fit GitHub's release-body limit. Without a previous release, the range includes all history reachable from the selected source.

Commits pushed after the source snapshot belong to the next release. The subsequent docs update is also after the release tag, so it appears in the next range. Existing tags and published releases are never overwritten.

## Failure and retry

- **Main changed before preparation:** start a new run from current `main`. A full retry may reuse only its own recorded version-only commit. The version commit remains if a later build fails.
- **A CI job failed:** use **Re-run failed jobs**. Attempt-suffixed artifacts allow retries without overwriting earlier artifacts. Publication selects the latest successful package artifact for each target within the same run.
- **Upload failed:** re-run the failed publish job. It resumes only a draft bearing this run's exact source/run marker and identical asset hashes. It never clobbers another draft or an existing release. If publication already succeeded but its response was lost, inspect the public release before doing anything else.
- **Docs push or Pages failed after publication:** the release remains public; re-run the failed website job. It fetches current `main`, regenerates the pages and explicitly requests Pages again. This does not rebuild installers or republish the release.

The workflow runs only on manual dispatch from this repository's `main`. Publication does not run for PRs, tags or ordinary pushes. It uses the built-in `GITHUB_TOKEN`; no PAT, signing secrets, paid service or new environment approval is required by this implementation. Only preparation/publishing/site-update jobs receive write permissions. Site updates additionally receive `pages: write`.

## Verification boundary

Local tests cover isolated Git version commits/retries, commit ranges including merges, hostile version strings, pre-release history selection, package integrity and missing assets, draft ownership, website generation and Pages rebuild handling. Workflow syntax is validated with actionlint. These checks do not claim that a new version has been built or publicly released; the first manually dispatched version gets its own real CI/publication evidence.

[GitHub: workflow inputs and token-triggered events](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow) · [GitHub: request a Pages build](https://docs.github.com/en/rest/pages/pages#request-a-github-pages-build)
