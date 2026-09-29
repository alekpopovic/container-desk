---
title: "ContainerDesk brand kit"
section: "Build & design"
icon: "🛠️"
---

# 🛠️ ContainerDesk brand kit

One container mark, one teal palette, one name. Use this kit for the desktop application, documentation, repository and project announcements.

![ContainerDesk banner](assets/brand/banner.svg)

## Download and assets

[📦 Download the complete brand kit](assets/brand/containerdesk-brand-kit.zip) · [Color tokens](assets/brand/tokens.json)

| Asset | Use |
|---|---|
| [Primary mark · SVG](assets/brand/mark.svg) · [PNG](assets/brand/mark-512.png) | App identity, avatar, favicon; square with rounded corners |
| [Monochrome mark · SVG](assets/brand/mark-mono.svg) | Single-color layouts; inherits `currentColor` when inlined |
| [Light wordmark · SVG](assets/brand/wordmark-light.svg) · [PNG](assets/brand/wordmark-light.png) | Light backgrounds |
| [Dark wordmark · SVG](assets/brand/wordmark-dark.svg) · [PNG](assets/brand/wordmark-dark.png) | Dark backgrounds |
| [Repository banner · SVG](assets/brand/banner.svg) · [PNG](assets/brand/banner.png) | README, documentation landing page, 1200 × 360 |
| [Social card · SVG](assets/brand/social-card.svg) · [PNG](assets/brand/social-card.png) | Link previews and announcements, 1200 × 630 |

## Name and voice

Write **ContainerDesk**, with capital C and D and no space. The short description is **“A native desktop for Docker over SSH.”** The headline is **“Your servers. Your SSH. One workspace.”** In Serbian: **“Tvoji serveri. Tvoj SSH. Jedno radno okruženje.”**

Use direct, practical language. Describe the real target, permission and result. Label previews and unverified behavior accurately. Do not imply affiliation with Docker, Apple, GitHub or OpenSSH. ContainerDesk remains a working product name, not a trademark-registration claim.

## Palette

| Token | Color | Role |
|---|---|---|
| Teal | `#0f766e` | Primary mark; white symbol |
| Deep teal | `#08695f` | Links and focus on light surfaces |
| Mint | `#6cdec6` | Links and accents on dark surfaces |
| Ink | `#192d38` | Light-theme text |
| Cloud | `#f5f7f8` | Light canvas |
| Night | `#101a22` | Dark canvas, banners |
| Snow | `#e7eef2` | Dark-theme text |
| Slate / mist | `#536775` / `#abbcc9` | Supporting text on light / dark surfaces |

Color never replaces a label. Do not use mint for small text on white. Preserve visible keyboard focus. The application and website share these existing semantic colors; the documentation shell does not change app status meanings.

## Typography and spacing

Use system sans-serif for interfaces and `ui-monospace` for commands and identifiers. Exported artwork uses DejaVu Sans. No remote font request is needed. UI spacing follows 4, 8, 12, 16, 24 and 32 px; normal controls use an 8 px radius.

Keep at least one quarter of the mark's width as clear space around standalone artwork. Use the mark at 24 px or larger in navigation; the favicon may be smaller. Keep proportions, container ribs, corner radius and colors intact. Do not rotate, stretch, add effects or put the light wordmark on a dark surface. Select the provided variant instead.

## Shared source and maintenance

`docs/assets/brand/mark.svg` is the shared web/UI source. It matches `src-tauri/icons/source.svg`, which generated the existing native installer icons. The app sidebar and favicon import the shared mark; both READMEs and the docs website use the same kit. Published v0.1.0 installers retain their original verified bytes.

Update the two matching SVG sources together if the mark changes, regenerate native icons with the pinned Tauri CLI, and verify native packaging separately. Export the SVG wordmarks/banners to same-sized PNG files. The ZIP contains distributable assets, tokens and a plain-text usage guide; it is a design deliverable, not an application build artifact.

The website's theme choice is stored locally under `containerdesk-docs-theme`; navigation and document content remain readable without JavaScript. There are no remote fonts, tracking scripts or analytics.
