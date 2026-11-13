# Exact editor selection and reviewed rich changes

Status: implementation design for existing editor rows E24/E25/E26; not an accepted capability.
Source: frozen Writer's Studio `manuscript-editor.tsx:296–320,935–966`, assistant attachment DTOs,
`internal/store/assistant_apply.go`, and the current CraftEditor typed transactions/replay contracts.
Provider connectivity and a new assistant product are not introduced by this work.

## Exact attachment boundary

Flush the live draft before capture and require the acknowledged document revision. Native leaf
carets are Unicode scalar positions, distinct from the source assistant's per-top-level-block text
projection. Capture supported top-level paragraph/heading/blockquote/code ranges only, in document
order; omit unsupported structures from the source-compatible quote rather than expanding into
unselected writing. Blockquote paragraph separators, hard breaks and omitted inline atoms use the
explicit `BlockTextProjection` compatibility policy. Reversed selections produce the same ordered
ranges. Empty or unsupported selection has an actionable failure and creates no attachment.

The portable attachment DTO is ordinary untrusted data: document, base, exact ranges and quote.
Every use validates scope, revision, order, bounds and literal text against that exact canonical tree.
It cannot grant replay, project or provider authority. Reveal converts validated projected boundaries
back to native leaf carets with explicit affinity; no text search or first-occurrence retargeting.
The native chip offers reveal, remove and replace, survives panel focus, and becomes visibly stale
on a real document edit. Capturing does not send a request, alter reader JSON or create Undo history.

## Reviewed rich changes

Use a separate rich proposal record, not the existing plain-text proposal's body replacements.
An additive migration stores frozen before/after canonical trees, bounded typed commands, reason,
status and applied/undone revisions. Preparation applies actual model commands to a detached tree;
it never saves canonical content. The UI and future headless/provider adapter call the same core API.
Stored JSON is not proof: Apply checks the current exact base/tree, revalidates and executes the typed
commands, verifies the prepared result and current project resources/runtime table capacity, and
atomically commits canonical revision, mapped annotations/history and proposal status. A pending
working draft must be flushed or explicitly preserved; it is never silently discarded.

Reject changes status only. Undo Apply requires the exact applied revision/tree and appends a
compensating revision, restoring proven prior attachments while preserving current author note text
and lifecycle. Later text or formatting invalidates Apply/Undo. Stale/corrupt/resource failures leave
all writing, notes, history and proposal state unchanged. Imported proposal/replay JSON remains
quarantined evidence and never populates active rich proposals or private replay receipts.

The review surface distinguishes manual/local edits from real provider output. Preview does not
enter the manual Undo stack or autosave. Native interaction/history reconciliation after Apply must
be qualified explicitly; reloading a widget is not evidence that manual-history behavior is correct.
Proposal summaries are paged and content-light; full trees load only for the selected review.
The same consistent export transaction captures rich review records as untrusted preservation data.

## Required verification

Use synthetic repeated text, non-BMP/combining Unicode, reversed/multi-block quotes, blockquotes,
hard breaks/inline atoms, ignored list/table structures, stale bases and forged range/order/text.
Prove capture/reveal do not mutate content/history. Rich review tests must cover formatting-only
staleness, structured multi-block edits, notes inside/outside changed text, unknown attributes/media,
resource isolation, over-capacity tables, failed atomic publication, Reject, compensating Undo,
post-Apply note edits, restart and imported-evidence quarantine. Then exercise the real compiled
context menu, chip, source preview, Apply/Reject/Undo, keyboard/IME and designer acceptance.


## Alpha.3 implementation boundaries

Use additive schema9 `rich_proposals` records, with scoped content-light cursor pages and explicit selected-record loading. The complete typed command batch is bounded to1,000 operations/16MiB and the reason to4,000 Unicode scalars. Core commands accept a fixed project scope separately from the record identity; deserialized proposal/attachment fields never establish project authority. Preparation and Apply enforce the current63-column native table bound and project-owned image references. An unresolved rich or plain working draft blocks canonical Apply/Undo; the UI must acknowledge its normal save first. A no-op proposal is refused rather than manufacturing an applied revision.

The native composer continues to identify local manual proposals accurately; provider connectivity is separate existing migration work. Detached engine preparation does not enter the live session's manual Undo stack. Apply/Undo must rebase or replace the native session only after the canonical transaction succeeds, invalidate stale attachment chips and refresh exact note context. Full native history behavior remains a required acceptance case.

### Completing the originating note

F-56 retains the exact originating note snapshot beside the applied proposal receipt. An exact selected subrange can retain its current note/queue identity; a standalone or unrelated selection cannot inherit a note from matching text. After Apply, resolving the note is an explicit metadata command using its post-Apply snapshot and the current acknowledged manuscript. Pending body, labels and lifecycle commit together under existing guards. Record the returned snapshot before fallible presentation work; resolution never replays the text change. Completed notes leave the active queue, remain in history and can reopen. Manuscript Undo and note resolution are separate actions. This is integrated source behavior awaiting hosted/native acceptance, not an automatic completion decision for every applied edit.
