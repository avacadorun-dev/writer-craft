# Native project media previews

## Why

Issue #10 requires original `.wstudio` project media to work in the native board. Byte-preserving import alone does not provide the original PDF/image viewing workflow.

## What changes

Adopt the separately qualified MIT CraftMediaPreview immutable-source and egui texture providers. Resolve original project-scoped media aliases on a dedicated bounded worker, display actual image/PDF pixels in the board and open an explicit native page viewer. Keep original metadata and bytes unchanged after every success, unsupported codec, cancellation or failure.

## Scope

This is a complete bounded PDF/image integration slice, not completion of #10. Animated image playback, audio/video, source segments, full PDF fidelity/controls and complete original Canvas workflows remain required alpha.3 work. Public CI uses synthetic media only. No WebView or subprocess runtime is introduced.
