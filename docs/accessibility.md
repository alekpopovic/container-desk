---
title: "Keyboard and accessible inspection"
section: "Start here"
icon: "🧭"
---

# 🧭 Keyboard and accessible inspection

Use Ctrl on Linux or Cmd on macOS with these keys. macOS bindings are implemented but await native macOS verification.

| Keys | Action |
|---|---|
| F | Focus the current resource search; from Hosts/Settings, open Containers search. |
| R | Refresh the current container, Compose, image, volume or network inventory when its existing Refresh button is enabled. |
| Shift + H | Open Hosts and focus the first visible saved host, or New host if none exists. Press Enter to select; connecting remains a separate action. |
| Shift + L | Open Containers and focus its Logs tab; if no container console exists, focus container search. Press Enter to open the tab. |

These shortcuts do not handle input while focus is inside terminal controls or an open confirmation. Terminal focus can be left with the existing Ctrl+Shift+Escape shortcut. Screen readers may reserve commands in browse mode; their normal focus mode remains available. No shortcut grants management, starts a stream, reopens a terminal or dispatches a mutation.

Tab/Shift+Tab traverse controls. Native modal confirmations start on Cancel, contain the Tab cycle and accept Escape before dispatch. Returning focus remembers the invoker before asynchronous preparation disables it. Existing logs/support dialogs use the same Tab boundary handling. A native operating-system file chooser manages its own focus.

Container, Compose and resource tables expose captions/headers; selected states and connection/error states include text, not only color. Connection/action status is announced with status/alert roles. Raw streaming logs and terminal content are not added to an application-wide live region. Logs remain text, with labeled selection checkboxes and a keyboard-focusable scroll region.

Inventories above 200 rows offer **Use paged table**, rendering at most 24 rows per page. This gives keyboard and screen-reader users explicit Previous/Next controls without relying on scroll virtualization. Search remains available for a known name or full ID. Scrollable tables/logs accept focus for keyboard scrolling. Font sizes and virtual row metrics follow the root font size; the table width also scales, with horizontal scrolling contained inside the table. At 200% text, use the details panel for full values and keyboard scrolling for additional columns.

Light/dark colors retain the tested 4.5:1 text and 3:1 focus contrast targets. Reduced-motion preferences disable CSS animation, transitions and smooth scrolling. Fixed viewport dimensions are not treated as evidence for native OS scaling: browser text-size checks and Linux WebKitGTK text-size checks are recorded separately.

The review follows the [WAI modal dialog pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/), [WCAG text contrast guidance](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html) and [200% text resizing guidance](https://www.w3.org/WAI/WCAG22/Understanding/resize-text.html). This targeted audit is not a conformance certification or a substitute for macOS VoiceOver/user testing.
