# Native annotation mapping and original-source evidence

Source checkpoint, 2026-11-08. This records an executed headless source probe and implementation contracts; it is not compiled native workflow acceptance or permission to close issue #2.

## Actual original behavior

The probe transpiled the installed original `src/features/editor/model/annotations.ts` and invoked its actual `DocumentAnnotationsExtension` plugin, `annotationAnchorFromSelection`, `setDocumentAnnotations` and `mappedAnnotationAnchors` with the installed ProseMirror `Schema`, `EditorState`, `TextSelection` and transactions. The explicit-ID schema uses the same relevant paragraph, heading, blockquote, code, text and inline-atom shapes. No DOM, block-identity append transaction or original/user project was modified.

Source SHA-256: `e846b56f3f0b7555e84751400c79e32e650a210b9734ce9358e05a64c9806349`. Local reproducible probe and report, outside the repository: `/Users/avacado/tmp/craft-renderer-research/annotation-mapping-probe.cjs` and `.json`. The following results are source evidence from that execution; Rust expectations live in `crates/studio-core/tests/annotation_mapping_parity.rs`.

Starting text: `Я😀 echo echo tail`. Mark: the second `echo`, Unicode-scalar range `[8,12)`. UTF-16 positions were converted separately before passing transactions to ProseMirror.

| Exact edit | Original attached result |
| --- | --- |
| Insert `X` at 10 | `[8,13)`, `ecXho` |
| Delete `[9,10)` | `[8,11)`, `eho` |
| Replace `[9,10)` with `XY` | `[8,13)`, `eXYho` |
| Replace `[8,12)` with `новое😀` | `[8,14)`, `новое😀` |
| Delete `[8,12)` | Detached; original range and quote retained |
| Insert `X` at start 8 | `[9,13)`, `echo`; insertion excluded |
| Insert `X` at end 12 | `[8,12)`, `echo`; insertion excluded |
| Delete first marked scalar `[8,9)` | `[8,11)`, `cho` |
| Delete last marked scalar `[11,12)` | `[8,11)`, `ech` |
| Delete across start `[7,10)` | Detached; original quote retained |
| Delete across end `[10,13)` | Detached; original quote retained |
| Replace `[10,12)` with `tail` | `[8,14)`, `ectail` |

A second actual plugin case starts `先😀 東京 京都`, marks `[6,8)` (`京都`) and replaces `[7,8)` with `大阪`: the result remains attached at `[6,9)`, `京大阪`. Thus interior modification is part of ordinary native editing; treating every partial overlap as unresolved was an implementation gap.

The original Go assistant helper `internal/documentannotation/annotation.go::RemapAnchor` separately detaches partial overlaps. That is a conservative assistant-edit policy, not evidence that native author typing should lose its mark. Native typed commands with exact replay provenance use endpoint mapping. An unproven recovered snapshot never receives that authority.

## Explicit projections and original bugs

The native leaf projection retains hard breaks as newline and inline math as one U+FFFC scalar. `BlockTextProjection` adds one explicit newline between native text leaves and converts parent offsets to exact identified leaf points. `Node::inline_text` still has its original concatenation semantics. Canonical JSON, leaf offsets and unknown metadata are unchanged.

The frontend source uses `textBetween(..., "\n")`, which inserts paragraph separators. However, `locateAnchorRange` adds a flattened UTF-16 offset directly to a container's ProseMirror start and ignores nested paragraph boundary tokens. In the executed two-paragraph blockquote (`First α`, `Second 😀`), initial source capture is `[3,14)`, `st α\nSecond`; setting/re-reading the same note drifts to `[2,12)`, `rst α\nSeco`. The native implementation preserves the intended exact parent quote through explicit leaf points rather than copying this locator bug.

The frontend atom projection also differs from native coordinates: inline math and hard breaks have no ProseMirror `leafText`; the actual `textBetween` path omits both. The executed `中😀` + inline math `x^2` + hard break + `後の文` fixture projects `中😀後の文`. The initial selected `後の` captures `[2,4)` but the original locator detaches it on re-read. Tiptap's hard-break `renderText` is assigned to `NodeSpec.toText`, not `leafText`, and is not used by this annotation projection. Captions and block-atom authored source do not become the annotation's text.

The Go backend `internal/editor/document.go::appendInlineText` uses another projection: child paragraphs concatenate without separators, hard breaks contribute newline, and inline math contributes `[Формула: <latex>]`. Neither legacy projection can be silently interpreted as native U+FFFC/newline offsets.

The pure core adapter `AnnotationAnchor::adapt_legacy` now checks the two explicit compatibility policies, preserves the complete original anchor JSON and returns either an exact native anchor or a detached result. It verifies the stored quote and both bounded contexts at the stored offsets; it does not run the broken DOM locator. The transfer caller must retain that original JSON in import history and mark unresolved results as needs-attention. If an untagged old anchor matches more than one finite known projection at different physical occurrences, preserve it as detached/needs-attention for explicit reattachment; do not choose a matching quote or search another occurrence. Durable transfer integration is still a tracked requirement, not complete merely because this pure adapter exists.

## Native authority and immediate paint

`AnnotationPreviewSnapshot` has private validated fields. It captures one guarded document/notes/receipt context outside canvas paint. Preview and save share sealed replay validation, exact complete result comparison, receipt ownership, contiguous history, stale-base checks and protection against undoing a later explicit reattachment. Preview is purely in memory and does not update SQLite, revisions or the shared receipt cache. Refresh after canonical save, note metadata mutation or guarded external refresh.

For each verified native transition, map the start with right affinity and end with left affinity. Strict deletion across either endpoint or invalidated/removed identities detaches. Regenerate the exact current quote from mapped leaf endpoints; this is deterministic replay, not quote matching. Parent blockquote endpoints must still resolve inside the same parent; other cross-block selections remain subject to the source's one-supported-block contract. Undo/Redo restores the exact cached attachment state, independently of current authored note text or resolved status.

The generic canvas `DecorationSource` receives canonical document/revision and newly pending proofs after native input. The application combines already-drained and fresh proofs and performs pure preview, then supplies opaque IDs and exact current leaf selections. The editor paints ephemeral marks without adding canonical text/marks. No database or filesystem access is permitted in that callback. A source error is explicit, keeps prose editable, and refuses potentially stale marks.

## Remaining acceptance

The native notes inspector, exact capture/edit/resolve/reopen/delete, detached reattachment, click-to-note and all eight catalogs have source implementations awaiting compiled integration qualification. Attached note quotes display authored math source instead of internal U+FFFC; detached unavailable embeds have a localized textual label. Durable proof-derived draft recovery has a separate core checkpoint in [Durable annotation drafts](DURABLE_ANNOTATION_DRAFTS.md); its hosted qualification/application wiring, legacy transfer integration, real IME mark continuity, native interaction and independent UX acceptance remain required before issue #2 or alpha.2 can close. A headless model/raster pass alone is insufficient.

Compatibility tests additionally cover an intentionally ambiguous untagged blockquote: a hard break at the end of its first paragraph makes frontend and backend text both `echo\necho`, but their selected newline belongs to different native leaf/container locations. The adapter preserves the original unresolved anchor rather than choosing either. Full source metadata, including unknown fields, remains available in the returned original JSON.
