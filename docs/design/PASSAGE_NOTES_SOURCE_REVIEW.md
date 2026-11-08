# Persistent passage notes — independent source UX review

Review checkpoint: application `fdef468` with core `9ee65a0`, the new generic text-decoration contract and the pending source-integration fixes. Reviewer: independent of the passage-note implementation; the reviewer's earlier media work is excluded from this assessment.

Disposition: **changes required before author-pilot acceptance**. This is a source review, not a compiled screenshot, native interaction test, accessibility approval or `full-editor-parity` acceptance. Parent application hooks were not wired at the reviewed checkpoint. No owner application, manuscript, clipboard or preview was opened or modified for this review.

## Contract and evidence

Read `persistent-text-annotations` PA-01–PA-06, `EDITOR_UX_PARITY.md`, `UX_ACCEPTANCE.md`, Balanced/adaptive desktop guidance and the owner feedback log. The source reference retains a visible mark independently of selection, preserves unresolved notes, separates metadata from manuscript content, and reveals an explicitly selected note's exact passage.

Inspected the application note controller, inspector and dialogs; immutable note-preview/core guards; generic decoration painting and activation; the original `DocumentAnnotationsExtension`, manuscript editor click/reveal integration, annotation inspector and annotation end-to-end scenarios. Also read the pinned egui 0.36.2 `TextEdit`, `Undoer`, `TextBuffer::insert_text_at` and `ScrollArea::show_rows` implementations. No Cargo command or local build was run.

The list/detail structure, separate bounded scroll regions, explicit resolve/reopen/reattach actions, exact-anchor core commands and pure proof-backed decoration projection fit the intended architecture. They do not by themselves establish a safe daily workflow.

## Required P1 corrections

### NSR-01 — Preserve dirty note work across navigation and closing

`NotesState::clear` resets all state; `prepare` clears drafts and the composer on document changes. The checkpoint even tests that a nonempty private draft disappears. Existing-note edits live only in `drafts` until Save, while a new composer body also has no recovery owner.

An author can type a substantial note, switch documents or close the app, and lose that work without a decision. Keeping another document's draft out of the current panel is necessary; deleting it is not necessary.

Before clearing or closing, retain drafts under their owning document/note identities or complete a Save/Discard/Cancel transition. Preserve the captured composer target. Parent hooks must include note modals in global modal/shortcut gating and route focused-note Save correctly: the existing manuscript-only Cmd+S must not imply that an unsaved note was saved. Do not let a global “All changes saved” state conceal dirty note work.

Regression: edit note A, switch to document B, cancel the transition or return to A, then close/reopen according to the chosen explicit save/recovery policy. The complete draft remains available without appearing in B. Test composer cancellation, failed save and app close as well.

### NSR-02 — Give every note editor and composer an explicit identity

The detail `TextEdit` and composer use implicit automatic widget IDs. The same detail slot displays different note bodies, and the same modal slot is reused for successive composers. Conditional conflict controls can also change automatic IDs for the same note.

Pinned egui loads `TextEditState` by widget ID, including cursor and Undo state. Its Undo handler can replace the current buffer with an older string from that state. Reusing the slot can therefore mix caret/Undo state between note A and note B or between successive new-note drafts.

Bind detail fields to stable document/note identities. Bind a composer to a unique captured-composition instance. Explicitly handle canonical reload/discard so old editor history cannot restore the discarded draft into a different entity.

Regression: type in A, open B, press Cmd+Z and verify that A's text never enters B. Repeat with two new composers and with a conflict banner appearing before the field. Verify cursor/selection and IME ownership remain with the correct field.

### NSR-03 — Do not silently cut authored note input at 10,000 characters

Both fields set `char_limit(10_000)`. The pinned egui input path applies that limit to paste and IME insertion by taking a prefix; it supplies no visible overflow result here. The core already validates the note length safely.

A long paste can lose its suffix before the author presses Save. This repeats the source field's hard limit without the requested native preservation/error behavior.

Keep the complete local draft, show a localized length/validation state and refuse an invalid save explicitly. The bounded viewport must not become a content truncation mechanism.

Regression: paste and compose over the limit with Cyrillic, emoji and CJK text. The full draft remains recoverable; canonical metadata does not change until a valid explicit save.

### NSR-04 — Never activate an unverified old anchor

A failed decoration preview sets `invalid` and suppresses painting, but retains the old `preview`. The inspector only shows a warning: rows and Show passage can still call `activate_note` using the old attached state. `selection_for` checks the literal at those coordinates, not the missing live-replay authority. A repeated quote can still match at an obsolete position.

Gate passage activation and validity-dependent actions on a verified current document/preview context. Keep the original quote and note reachable as unverified/unresolved, with an explicit reload/recovery route. On document changes or metadata-read failure, never display or activate another document's cached notes; preserve drafts separately from the visible snapshot.

Regression: load a recovered or unprovable changed tree containing repeated quotes, force preview failure, and click the old note. It must not select a different occurrence. Also switch between documents with identical template JSON while snapshot preparation fails; no previous document's note becomes visible or active.

### NSR-05 — Bound the composer vertically and keep its actions reachable

The shared modal constrains width only. The selected quote is a multiline label, the note field grows with its multiline content, and the footer follows them in normal flow. A long valid note or a quote containing many newlines can make the modal taller than the minimum supported window and push Add/Cancel offscreen.

Use a bounded scrolling body, a controlled quote preview and a reachable footer outside that scrolling content. Preserve the complete editable draft. Apply the same reachability check to long inspector details and their primary Save action.

Regression: at 1000×680, use a long multiline quote and a valid note with hundreds of lines, then a preserved over-limit draft. Header, close, Add/Cancel and validation remain reachable; Escape returns focus to the meaningful source surface.

## P2 corrections and integration checks

### NSR-06 — Use the virtualized list's actual row pitch

`show_rows(ui, 44.0, …)` uses a 44-point row plus `item_spacing.y`; the current theme spacing is eight points. The manual reveal calculation uses `index * 44`, so its offset drifts by eight points per row. A later marked passage can open the inspector without revealing its selected row.

Use the same measured pitch for virtualization and reveal, or deliberately remove the inter-row spacing. Keep keyboard focus in the list while navigating; explicit Show passage may transfer focus. Test the first, hundredth and last notes, status reordering and list resize without a wheel-only requirement.

### NSR-07 — Preserve native selection gestures when opening notes

The generic decoration loop treats every unmodified clicked response as activation; the host's `activate_note` always expands the selection to the whole note and focuses the manuscript. Parent hooks must distinguish normal text interaction from an explicit reveal.

The original first highlight click opens/reveals its note. Repeated clicks on an already active mark, double/triple-click word/paragraph selection, drag and modified selection must still work; they must not repeatedly replace the author's intended caret/range with the complete quote. Keep Show passage as an explicit reveal operation. Test typing after a second click inside a long marked passage and ensure only the intended insertion is changed.

The keyboard context menu is positioned from the whole canvas origin rather than the visible caret. After a long scroll, that origin can be far outside the viewport. Anchor Shift+F10 actions to the visible selection/caret and verify Escape/focus return.

### NSR-08 — Keep refresh/conflict handling recoverable and identifiable

`invalidate` removes the snapshot that `prepare` uses to recognize a known local canonical advance. Parent save hooks must rebase unchanged authored note fields from a verified acknowledgement before invalidation; otherwise an unrelated local save can appear as an external note conflict.

Test two dirty note drafts, a manuscript autosave, and a lifecycle change on one note. The other draft must stay saveable under the correct guard. A genuine external change must retain the local draft.

Reload current note currently replaces the local draft immediately. Make that discard explicit or preserve a recovery copy. The delete confirmation should identify the target with its quote/note excerpt; “the manuscript stays unchanged” alone does not let the author verify which note will be removed.

### NSR-09 — Retain the source inspector's useful row information

The original list exposes the quote and note text, with an attention indicator for detached/needs-attention entries. The new row shows only a shortened note body and a Resolved prefix. Several “Revise this” notes become indistinguishable, and unresolved notes cannot be recognized until individually opened.

Keep a compact quote/context line and a clear attachment/attention state, while preserving the selected row and the existing resolve/reopen lifecycle. This is existing source information, not a request for a new notes product or an AI feature.

## Retest and acceptance boundary

The 22 directly referenced note message IDs have catalog rows in all eight language columns at this checkpoint; conditional lifecycle labels also need the normal catalog checks. Actual wrapping, focus, selection contrast and native field behavior still require a compiled review in Light/Dark, minimum/common windows, long German/French text, Russian and CJK/IME input.

Before acceptance, verify create → move caret → visible persistent mark → reopen → edit note → resolve/reopen → detach → explicit reattach → delete/cancel, including repeated/Unicode quotes, tables/lists/blockquote passages, live edits and Undo. Include failed saves, metadata conflicts, lost preview authority, dirty navigation, keyboard-only access, long lists and overlapping marks. Parent hook/source fixes and test passes must be recorded against exact commits; a source review cannot turn the native UX gate green.

This review covers author passage notes only. Dictionary-link journeys, the remaining full-editor inventory and general viewport anchoring keep their separate parity/acceptance requirements.
