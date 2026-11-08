# Direct Canvas text-card editing

## Why

[Issue #15](https://github.com/avacadorun-dev/writer-craft/issues/15) records the observed migration defect: text cards are writable only in a separate Markdown dialog. Original `CanvasTextCard` mounts an in-card textarea on double-click, focuses it, commits on same-window blur/Cmd-Enter and cancels on Escape.

## What changes

Make a real native multiline input the primary direct card editor. Newly created cards focus after durable creation. Keep source text/selection/clipboard/local Undo and IME inside the zoomed clipped card, with acknowledged debounced checkpoint/save, exact pending source and later draft preservation, actionable failure and guarded navigation.

## Scope

This corrects the existing workflow. GFM card rendering, full typed-card/connector/drawing parity and unrelated relationship research remain separately required. Do not drive or replace the running owner preview for this change. Native verification and independent UX acceptance remain mandatory before #15 is ready.
