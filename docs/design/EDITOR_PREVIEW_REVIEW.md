# Independent editor preview review

Review date: 2026-11-08. Reviewer: independent UI/UX designer (`preview_designer`).

**Disposition: engineering preview; source fixes reviewed, native retest pending.** This record does not approve Author pilot, alpha.2, complete editor parity or a release. `UX_GATE.json` is unchanged.

## Scope and evidence boundaries

The reviewed checkpoint is the isolated macOS native table/formatting preview: continuous manuscript canvas, real wrapped and merged table cells, inline formatting, structural table actions, document-level Undo/Redo, durable drafts and canonical rich saves. It is an implementation checkpoint within the unchanged full-editor-parity milestone.

Reviewed application source: [aa2237e0f971544338e67c8856632d86156f4f78](https://github.com/avacadorun-dev/writer-craft/commit/aa2237e0f971544338e67c8856632d86156f4f78). The application pins CraftEditor [418cc687e2d80a26753b2ca76c52a6c272ad7255](https://github.com/avacadorun-dev/craft-editor/commit/418cc687e2d80a26753b2ca76c52a6c272ad7255). Source inspection covered the rich editor, manuscript/Review integration, save/open dispatch, structural caret API and relevant regression test definitions.

The implementation owner reported displaying the actual native application through CUA at 1440 × 940, Russian interface, Dark appearance: shared monk/book shell and a raster table with merged cells. This was application source [99bb2d0011450721b55cfa2f29bc11def71e7184](https://github.com/avacadorun-dev/writer-craft/commit/99bb2d0011450721b55cfa2f29bc11def71e7184), from [hosted run 42451672199](https://github.com/avacadorun-dev/writer-craft/actions/runs/42451672199). The implementation owner reported 89 tests, strict Clippy, archive checksum and ad-hoc code-signature verification for that run. These are supplied results, not checks independently executed by this reviewer.

That first capture was inline CUA output only. No retained capture file or image hash was supplied; none is invented here. This reviewer has not independently inspected a retained native capture or operated that application. Display evidence for source 99bb2d0 does not verify later source aa2237e.

The owner is actively testing the open preview and explicitly requested that it remain undisturbed. No reviewer CUA interaction, app replacement/restart or owner manuscript access was performed. Hands-on independent retest awaits an explicit handoff after that testing ends. No local Cargo build or test was run for this review.

## Findings and source response

| ID | Severity | Source finding | Implementation response | Disposition |
|---|---|---|---|---|
| EP-P01 | P1 | A rich document exposed the plain-only Review composer. A user could enter an exact edit and then meet the core rich-document refusal. | `review.rs` now returns an honest localized preview explanation for rich documents and does not expose the incompatible composer. | Fixed in source; native verification pending. This temporary preview limitation does not remove rich proposals from alpha.2 scope. |
| EP-P02 | P1 | Deleting the current table row/column could remove its text-block identity, clear selection and let the canvas initialize the next typing target at the manuscript start. | The host evaluates the resulting table, retains a surviving target or chooses an adjacent surviving cell, and commits that caret through the shared structural selection/history API. | Fixed in source; native row/column, merged-cell and Undo/Redo retests pending. |
| EP-P03 | P1 | Table insertion retained the preceding paragraph caret, so subsequent typing did not enter the new table. | The host commits the first new cell as the post-edit caret and requests caret scrolling. | Fixed in source; native focus, scroll, typing and Undo/Redo retests pending. |

CraftEditor's `apply_commands_with_selection` validates the proposed post-edit target against a disposable result before changing the live document. It records that target in the same history entry, allowing Undo to recover the prior selection and Redo to recover the post-edit selection. This addresses the source-level integrity seam; its native caret/hit behavior remains unqualified.

The source also routes recovered rich draft text into the host's plain-text projection, protects rich documents from editable plain fallback builds, and applies a paragraph/heading change across selected text blocks as one transaction. Source tests name these persistence and selection invariants; their presence is not an independent execution result.

No additional confirmed P0 was identified during this source review. Rich saving retains the live EditorSession and its manual Undo history; draft compare-and-swap failure pauses autosave; document switching stops on a failed save. Native persistence/recovery and input ordering still require execution.

## Required native retest after owner handoff

Use a separate synthetic workspace and identify the exact application source, engine pin, hosted run and artifact checksum. Record window size, locale, appearance and palette with retained actual native captures. Do not reuse the owner's active manuscripts.

| Journey | Required observable result |
|---|---|
| First table insertion | Place the caret in an existing paragraph, insert a table, type immediately. Text enters the first new cell; caret and table are visible. Undo removes the insertion; Redo restores the table and the same target. |
| Row and column deletion | Delete active first/middle/last rows and columns, including span-adjacent cases. Next typing stays in the intended surviving table cell. Undo/Redo restores the exact tree and expected semantic selection; unrelated paragraphs remain unchanged. |
| Real grid interaction | Click inside each wrapped and merged cell, select forward and backward across cells and into surrounding prose. Caret, hit testing, selection and painted glyphs agree at the native Retina scale. |
| Formatting | Apply Bold/Italic/Underline and paragraph/heading changes by toolbar and supported keyboard shortcuts. Toolbar focus preserves the exact range. Active/mixed state agrees with the text; a collapsed caret formats subsequent typing. Undo/Redo crosses formatting and structural edits coherently. |
| Save and reopen | Edit cells and surrounding text, wait for autosave, explicitly save, switch documents and reopen. Canonical tree, spans, marks, unknown fields and correct host text projection remain exact. Save status must not claim success after failure. |
| Draft/recovery failure | Use isolated test data to exercise recovered rich drafts and a stale/conflicting writer. Preserve the unsaved rich content, pause repeated failing autosaves, and show an actionable status. Failed save must not switch away and lose the session. |
| IME and focus | Type Russian, emoji and Chinese/Japanese composition; exercise toolbar/modal/document switching during composition. Preedit is not canonical content; commit/cancel must not duplicate, disappear silently or reach another document. Glyph presence alone does not qualify IME. |
| Preview honesty | Open Review on a rich document. The incompatible composer is absent and the limitation is explained. Plain document Review still follows its existing path. Plain-text history display must not be treated as proof of preserved rich revision structure. |
| Supported bounds and appearance | Check 1440 × 940 and 1000 × 680, inspector shown/hidden, Dark/Light and representative palettes, long German labels and Japanese glyphs. Keep a fixed reachable format row, bounded manuscript scroll, useful text measure, legible cell/header/selection contrast and no clipped actions. |
| Capacity | Exercise 63 logical columns; refuse widening to 64 before mutation. Preserve wider canonical tables whole with an explicit read-only capacity state. This is the owner-approved F-30 exception, not a generic model/parser restriction. |

Backend font inspection supplied by the engine implementer confirms that the pinned WordCraft backend has its own bundled Source Serif 4 family, independent of egui font installation. CJK fallback and real composition remain native qualification work.

## Unchanged release requirement

The owner requires a demonstration covering all 21 canonical node types, all six marks and their existing editor workflows, including images, Mermaid and formulas. The current table/formatting sample is incomplete against that requirement.

[EDITOR_UX_PARITY.md](EDITOR_UX_PARITY.md), [the parity matrix](../migration/EDITOR_PARITY_MATRIX.md) and the full-editor OpenSpec contract remain authoritative. Persistent text annotations, rich proposals/assistant handoffs, source references, clipboard, composition, accessibility, rich history/recovery and the remaining block interactions still need their full native evidence. Missing full-parity features are tracked scope, not waived by this preview or by the source fixes above.

No issue is closed and no acceptance gate is changed by this review.

## Later workspace correction

The owner subsequently rejected the source-99 workspace's excessive top whitespace and formatting-strip presentation. [F-31 and owner issue #6](https://github.com/avacadorun-dev/writer-craft/issues/6) track that finding. The designer supplied nonfunctional Comfort/Balanced concepts, and the owner selected **Balanced** before implementation. The [workspace design contract](WORKSPACE_EDITOR_REDESIGN.md) records the approved 48/56/40 px fixed bars and 28 px initial manuscript gutter. Concept approval does not qualify the rebuilt app, prove the renderer or close the finding; independent native retest remains pending. The reviewer did not interact with or restart the owner's source-99 app.
