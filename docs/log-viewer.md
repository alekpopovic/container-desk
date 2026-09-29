# Log viewer and explicit export

The selected live container has a full-width log panel. It retains at most 20,000 sanitized records or 8 MiB of formatted UTF-8 text, including timestamp/channel/truncation markers. This is independent of the 024 backend queue. Oldest evictions are counted; Clear view resets the local view and selection without stopping the remote stream. Monotonic record IDs prevent cleared selections from matching new data.

The fixed-height viewer renders at most 32 rows with horizontal scrolling for long lines. Plain-text search is capped at 256 UTF-16 code units, performs literal case-insensitive matching and scans only the bounded buffer. It does not evaluate regular expressions. Timestamps can be hidden. Follow new lines scrolls to the latest records; selecting text holds the rendered snapshot and stops automatic scrolling. Pause display also freezes rendering while continuing ACKs, remote streaming and bounded retention. Stop logs cancels the remote subscription. A frozen display can hold another bounded 8 MiB snapshot; evicted live selections are removed. Resume shows the latest retained data with the accumulated loss markers.

## Untrusted text

A linear state parser discards ANSI CSI sequences, OSC hyperlinks/titles, DCS/SOS/PM/APC strings, C0/C1 controls except tab, bidi overrides/isolates/marks and Unicode line/paragraph separators. Unterminated control strings hide the rest of that physical record. Readable Unicode, including emoji joiners, is preserved. Rows display explicit control-removal, truncation and invalid-UTF-8 replacement markers. Text is rendered through ordinary React text nodes with no HTML, linkification or terminal emulator.

Copy selected lines uses exactly the formatted visible text, including the current timestamp setting and markers, joined with newline in buffer order. Clipboard reads are never part of the application. User selection can span filtering; the count and export preview describe the frozen selected set. Cleared or evicted records cannot be exported by the view.

## Native export

Export selected lines freezes the chosen current-buffer records and shows their source, count and a bounded preview. The action warns that logs can contain application secrets. Save selected logs opens the native file chooser only after that review. Cancel writes nothing. No automatic export/history exists.

The Rust command accepts a current scope, one full container ID, selected lines and the explicit secret acknowledgment, never a renderer-supplied path. It validates the full scope before opening the dialog and again after the user's choice. It enforces 1–20,000 lines, at most 8 MiB total, a bounded individual line and the same forbidden controls. A single export slot limits concurrent dialogs. Request Debug output omits the actual lines.

The path comes from the native chooser. Rust writes a randomly named application-owned temporary file beside the destination, mode 0600, flushes it and atomically renames it to the chosen file. Existing regular-file replacement follows the chooser's user confirmation. Symlink/nonregular targets are rejected; failed writes remove only a temporary file actually created by that operation. No path or text is added to application history or diagnostics. A filesystem sync failure after rename can leave the selected file present while reporting a failure; the app does not retry automatically.

The [official Tauri dialog plugin](https://v2.tauri.app/plugin/dialog/) is pinned to Rust package 2.8.0, compatible with the existing Tauri 2.12.0 toolchain. Default GTK3 supplies the Linux native chooser. On Ubuntu 26.04 the GTK glycin icon loader invokes the system `bwrap` helper; the native test therefore exposes only that helper on an otherwise isolated PATH. Docker/Node/Python/Cargo remain absent from the application executable search path. Rust-only use adds no npm package and grants no generic dialog or filesystem IPC permission to the renderer. Cargo.lock records the native dependencies, including rfd 0.16.0. The 3.0 alpha shown by registry search was not selected.

## Verification boundary

Parser/buffer tests cover hostile escapes, Unicode, literal oversized searches, combined byte/line bounds and ordered selection. Browser tests cover live follow with selected text, pause vs Stop, clear while streaming, clipboard and export payload selection. Native Linux tests must additionally verify the actual release/Tauri/GTK save flow and inspect its file bytes; a mocked save result is not sufficient native export evidence. Evidence 025 records that result and the exact environment. Native macOS, Wayland and package acceptance are separate gates.

The completed Linux acceptance used an owned Xvfb X11 display with software rendering (the surrounding desktop uses Wayland). `docs/verification/025-native/xvfb-harness.py` records the exact wrapper and temporary tool paths used. The dialog harness focuses the owned window, selects the entire filename including its extension, and uses real keyboard events. WebDriver clicks wait for viewport scrolling to settle. These are external test controls; no automation hook is added to the application.
