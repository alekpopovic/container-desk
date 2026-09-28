# Workspace layout and design tokens

Implemented in prompt 003. `WorkspaceShell` provides host groups, a persistent host heading, resource navigation and a table/detail area. Routes use local URL hashes: `#/containers`, `#/compose`, `#/images`, `#/volumes`, `#/networks`, `#/settings`. Unknown hashes fall back to Containers. Links use `aria-current="page"`; normal browser Back navigation works without a routing dependency.

The production application has no configured host and displays explicit unavailable/empty states. The Add host button is disabled with an adjacent explanation. Resource tables show their column headings and “No live data”, never invented workloads or misleading success. Settings offers System/Light/Dark theme selection for the current window only; persistence remains future work. Host groups are presentation filters over the currently empty inventory, not saved configuration.

## Tokens and layout

`src/styles.css` owns semantic light/dark CSS variables for canvas, panel/sidebar surfaces, foreground, muted text, borders, accent, focus and offline/loading/error colors. Tailwind's inline theme exposes the semantic canvas/surface/foreground/muted/accent/font tokens. System appearance follows the OS; explicit light or dark selection overrides it. Status labels accompany colored indicators so meaning does not depend on color alone.

Typography uses locally available system sans-serif fonts, with monospace for SSH aliases/endpoints. Body text is 14px, headings 22–24px, supporting labels 10–13px. The spacing scale is 4/8/12/16/24/32px, with 8px panel radii and a visible 3px focus outline. The ContainerDesk teal wordmark and outlined panels maintain continuity with the scaffold without introducing remote assets or fonts.

The initial native content size is 1280×800; the 640×480 minimum remains unchanged. Above 1050px, the inventory and 248px detail panel are side by side. At narrower widths, details stack under the inventory, navigation wraps as necessary and text remains breakable. At 720px and below, the host header and details use a single-column layout. Vertical scrolling is intentional for taller content; horizontal page overflow is tested against.

The detail panel repeats host name, SSH alias and actual-endpoint display fields, including explicit unset values. Presentation-only `DisplayHost`/`WorkspaceState` props are not backend domain models or authorization. There are no mutation buttons or action dialogs in this increment; future action dialogs must preserve the exact host/daemon/session identity required by the architecture.

## Keyboard and accessibility

A first-focus “Skip to content” button focuses the main landmark without changing the resource route. Host group controls are native buttons with pressed state; resource links support Tab/Enter; theme radios support native arrow-key navigation. Every route has a visible heading and landmark labels. Loading sets `aria-busy`; errors use an alert and all provided text renders as React text, including markup-like error content.

Playwright checks actual CSS token contrast: normal/muted text against canvas/surface/sidebar and semantic status pairs at least 4.5:1; focus against those surfaces at least 3:1. These are targeted automated checks, not a complete accessibility certification or screen-reader audit.

## Verification boundaries

`tests/ui/fixture.html` is an isolated development-only entry for synthetic loading/offline/error states and long/untrusted host text. Production imports neither the fixture nor its host values; the distribution was inspected to confirm their absence. It does not implement the separate offline development feature from prompt 008.

`npm run test:ui` runs 18 checks over light/dark at 1280×800, 800×700 and 640×480. It starts its own loopback Vite server at port 1431 with no reuse of another server. Install the pinned Playwright browser if needed with `npm exec playwright -- install chromium`. Browser screenshots and fixture checks are distinct from the real Linux native captures in [003 evidence](../codex/tracking/evidence/003.md).
