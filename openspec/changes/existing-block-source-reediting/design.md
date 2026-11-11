# Design — direct existing-block editing

## Gesture routing stays in the desktop adapter

The pinned CraftEditor revision `ec0c7e6d` already turns a primary press on an opaque atom into one guarded `select_node` on the shared session, and it exposes the measured selected selection through `canvas_selection_rect`. It deliberately has no double-click or context-menu callback. This change therefore routes the author's gesture in `crates/studio-desktop` and reuses `BlockDialog`; it does not fork an editor, add a second document authority or repin an unqualified engine revision.

The canvas still reports its single native response. After `show_canvas_with_sources` returns, the adapter reads `response.double_clicked()` and the session's whole-object selection. Only an atom kind that the source dialog already supports is opened. Normal text and word-double-click selection never reach that branch because their selection is not a `node_selection`.

## Exact-object safety

- Double-click: the engine has already selected the atom under the pointer before the adapter reads the response, so the dialog target is the clicked object, not a stale selection. The dialog captures the current revision and a sealed selection bookmark through the existing constructor.
- Context menu: the command is offered only when an editable whole-object selection exists and, for a pointer-opened menu, the right-click position lies inside that object's measured `canvas_selection_rect`. A right-click over prose therefore never edits a previously selected block from elsewhere in the document. A right-click over a different, not yet selected object also offers no block command; the author selects it first with a normal click. This is deliberate: without an engine hit-test the alternative would silently act on the wrong object.
- Keyboard: `F2` acts on the current editable selection of the focused manuscript. The existing `Shift+F10` menu path exposes the same item. A code block caret selection is accepted by `editable_target`, matching the existing overflow command.
- Read-only mode, an open dialog and active IME composition suppress the gesture, so a transient surface cannot open a competing modal.

## Guarded editing and history

- `BlockDialog` remains the only editing surface. Its revision check, selection bookmark restore and `restore_target` refusal are unchanged.
- Typed source lives in the dialog until Apply. Cancel, Escape, a stale revision, a failed save or a failed render keeps the dialog and its typed text in place; canonical content and history are untouched.
- A successful Apply uses the existing granular node/command path: one history entry, stable block identity, retained unknown attributes, preserved surrounding prose and rendered output. Undo/Redo and save/reopen follow the existing session semantics.
- The permanent toolbar is unchanged; **More formatting → Edit block** stays as the fallback. No new block-command row or extra toolbar button is added.

## Deliberate limits

- No engine repin, storage migration, transfer-version change or new dependency.
- A right-click does not itself change the selection (see above); the menu never targets an object other than the current measured selection.
- Compiled native double-click/right-click/F2 behavior, eight locales and independent review remain outstanding evidence for issue #7.
