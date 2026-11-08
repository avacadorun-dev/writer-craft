# Progressive media readiness source review

Status: implemented in source; hosted compilation, focused tests and native acceptance pending. Tracked by issues #7 and #4. This document does not close either issue or approve alpha.2.

## Problem and required behavior

The previous rich surface waited for one complete media catalog. A changed diagram, formula, caption, image reference, theme or language removed the entire canvas until background preparation finished. Authors could not read or edit unrelated prose during that interval.

The canvas now stays mounted. Each exact canonical atom is independently ready, pending or failed. Unchanged acknowledged atoms keep their real output. Changed or missing output has a localized native status box at the original opaque-node boundary while the background renderer prepares its exact source and presentation identity. Authored captions remain in native document flow. Failure retains source and offers retry; it never becomes a fabricated final diagram or editable replacement paragraph.

## Ownership and integrity

CraftEditor's generic resolver supplies explicit placeholder geometry, opaque canonical paths and visible-atom hints. Inline formulas still occupy exactly one canonical scalar; their status labels never enter text, selections, saved JSON or history. Resolver generation changes rebuild disposable geometry and raster tiles without resetting semantic selection or creating document revisions.

WriterCraft owns source/theme/scale/language keys, project-authorized media snapshots, worker policy and translations. Per-atom acknowledgements are accepted only for the current request epoch and exact canonical identity. A source or presentation change cancels older work; leaving the document cancels its worker receiver. Late output cannot replace newer source or cross project ownership. Image loads use only the captured project snapshot or the explicit bundled showcase asset; missing media has an explicit fallback and no ambient URL/file lookup.

One process-wide renderer permit bounds CPU workers. The result channel holds at most one acknowledgement, and a native frame consumes at most four acknowledgements before returning to input. A fifteen-second batch budget yields unfinished work for a later batch rather than declaring a valid document failed. The existing per-atom input, pixel and output limits remain enforced by the renderer.

Validated positive, finite geometry metadata is retained for each exact current atom identity when disposable pixels are evicted or invalidated by a theme change. Pending formulas may scale this previous geometry as an explicit provisional em-size estimate; this never claims newly shaped output or reuses stale pixels. Initial geometry and general viewport anchoring still require native review under issue #6.

Derived output uses a viewport-aware LRU cache. Offscreen results are evicted first; evicted atoms return to pending and can render again when visible. A cache budget is not a canonical document capacity limit, and no atom is removed or rejected merely because the document's total derived images exceed the cache budget.

## Focused verification to run on GitHub

The engine fixtures cover pending/failure prose geometry, exact opaque-node paths, inline scalar/Unicode mapping, real native-canvas text events while media is unavailable, selection/history preservation after acknowledgement, viewport hints, independent single-atom preparation and source-preserving renderer failure.

Application fixtures in `app/rendering.rs` cover unchanged output across prose edits, retention of other atoms after a source change, caption identity, progressive acknowledgement before batch completion, stale source/theme rejection, LRU rerender state without canonical rejection, scoped failure/retry, document cancellation, explicitly shortened Unicode source previews, and actual application-resolver canvas input with pending or failed media.

The seven focused engine fixtures, complete engine workspace tests, strict Clippy and examples passed on hosted Linux at source `6a07005b07596eb569dd92f3a17f07cc39d38f34` in [run 42464125790](https://github.com/avacadorun-dev/craft-editor/actions/runs/42464125790). This includes compact status presentation and cached measured atom rectangles. The same run also passed its separate native clipboard probes on hosted macOS and isolated X11; these do not constitute an owner-device interaction review. The eleven application fixtures, including retained geometry across theme/LRU invalidation without stale pixels, are still unrun until the new engine is pinned.

Run the complete editor-enabled workspace tests and strict Clippy on the hosted runner after pinning the qualified engine commit. A source fixture is not an observed pass. Native operating-system interaction, accessibility, IME, all renderer syntax, complete editor parity and independent UX acceptance remain separate required evidence.
