---
title: "Documentation website"
section: "Build & design"
icon: "🛠️"
---

# 🛠️ Documentation website

The Markdown files in `docs/` are the source for the public [ContainerDesk documentation](https://alekpopovic.github.io/container-desk/). The same files remain readable on GitHub.

## Publishing

Repository **Settings → Pages → Build and deployment** uses **Deploy from a branch**, branch **main**, folder **/docs**. GitHub builds the checked-in Markdown with its supported Jekyll renderer and deploys it through its managed Pages service. No custom site-build workflow is required. After a manual app release, its site-update job commits Downloads/Releases and explicitly requests a branch-based Pages rebuild.

The existing native application CI configuration is at `.github/workflows/ci.yml`. It was restored there by owner commit `5636d34`; this documentation change leaves that workflow intact. Pages deployment is separate from native application testing and release publication. A successful docs deployment is not native execution evidence.

`_config.yml` sets the project URL and `/container-desk` base path. `README.md` has `permalink: /` and is the site home. All documents use the shared `_layouts/default.html`, local CSS/JavaScript and the [brand kit](branding.md). `404.html` provides recovery links. This setup follows [GitHub's branch publishing instructions](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site).

## Edit a guide

1. Keep technical claims, code examples and evidence accurate. Original prompt files and task hashes in `codex/` are immutable history.
2. Use front matter with `title`, `section` and `icon`, then one H1 with the corresponding icon. Wrap a page containing Docker Go templates or Actions expressions in Liquid raw/endraw tags inside HTML comments, after front matter. GitHub hides those comments and Jekyll 3.10 preserves the examples literally.
3. Add the document's path to the appropriate group in `_data/navigation.yml` and link it from the [documentation index](README.md). Search filters these guide titles locally.
4. Use relative Markdown links within `docs/`. `jekyll-relative-links` converts them for the website. Link repository files outside `docs/` through their canonical GitHub URL. Do not embed machine-specific paths.
5. Historical checkpoint/result pages carry `historical: true`. Style and navigation do not turn past verification into a new platform claim.

## Local preview

From the repository root, with Ruby and Bundler installed:

```sh
BUNDLE_GEMFILE=docs/Gemfile bundle install
BUNDLE_GEMFILE=docs/Gemfile bundle exec jekyll serve --source docs --destination /tmp/containerdesk-docs-site --baseurl /container-desk
```

Open `http://127.0.0.1:4000/container-desk/`. The checked-in lockfile pins the local renderer and relative-links plugin; the versions match the [GitHub Pages dependency list](https://pages.github.com/versions/). GitHub controls its hosted Ruby runtime and built-in plugin set.

Keep generated `_site`, `.jekyll-cache`, `.bundle` and local gems out of commits. Before pushing, build the site, run `python3 scripts/docs/check_links.py /tmp/containerdesk-docs-site`, check images, and inspect wide and narrow screens, both themes, navigation, search and keyboard access. After deployment, verify the live site and representative nested documents.

After starting the local site, run the browser smoke checks with the project's pinned Node and installed Playwright browser:

```sh
DOCS_URL=http://127.0.0.1:4000/container-desk/ node scripts/docs/check_browser.mjs
```

This checks guide discovery, themes, mobile navigation, literal code examples, layout bounds and no-JavaScript access. Screenshots go to ignored `test-results/docs/`. Set `DOCS_URL` to the public site to verify a deployment.

## Release-driven updates

[Manual release](release-workflow.md) generates `downloads.md`, `releases.md` and `_data/current_release.json` from actually published releases. The shared header reads the current tag from that data file. Edit the templates in `scripts/release_site.py` rather than hand-editing the three generated files. A failed publication never updates Downloads; a later docs/Pages failure is visible as a failed workflow job and can be rerun independently.
