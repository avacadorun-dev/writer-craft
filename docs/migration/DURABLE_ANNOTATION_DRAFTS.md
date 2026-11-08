# Durable native draft attachment recovery

Status: implementation checkpoint for alpha.2 issue #2/#4, awaiting hosted Rust checks, application wiring and native interaction acceptance. The separate original Go/web repository and user projects are unchanged.

## Authority and storage

Schema 6 adds the private `rich_draft_annotations` table, owned by its exact `rich_working_drafts` row. The checkpoint is generated only by `Workspace::save_rich_editor_draft` after the existing sealed native replay verifier has reproduced the complete expected rich tree. No `ReplayProof`, `HistoryReceipt`, public caller map, authored note body or serialized native history is accepted as recovery authority.

The checkpoint binds format version, document identity, canonical base revision and hash, exact draft content hash, optimistic draft version and the complete note set. Each note has its original attachment hash and note revision. Attached targets use exact stable parent IDs and scalar boundaries plus a derived attachment hash. Detached targets retain the complete last-safe attachment snapshot and its hash. This is necessary when newly inserted text was marked and subsequently deleted: neither the canonical base nor the final draft contains the last-safe quote/context.

Saving a draft does not publish canonical prose, annotation changes, attachment history, receipts or acknowledged revisions. It replaces draft and checkpoint in one immediate SQLite transaction under the existing draft-version guard. The older `save_rich_draft` API deliberately deletes any previous checkpoint when replacing a draft without verified replay. `RichDraft.has_annotation_checkpoint` is only a read hint; recovery revalidates the private row and never trusts a caller's serialized boolean.

## Recovery transaction

`Workspace::recover_rich_editor_draft(document_id, expected_revision, expected_draft_version)` reads and validates the exact draft and checkpoint in one transaction. It refuses stale canonical revisions or draft versions, mismatched base/content bindings, changed note sets, later reattachments and missing/inconsistent attachment history. Failure retains the complete draft, canonical document, notes and history.

An intervening authored note text or lifecycle update is allowed only when every intervening append-only event is contiguous, has the metadata-only update origin and leaves the attachment unchanged. Recovery preserves those newer authored fields. Reattachment followed by a return to the original coordinates still conflicts; equal current JSON does not erase the intervening author decision.

On success, canonical rich content and recovered attachment states share one new revision and corresponding annotation/history snapshots. The exact matching draft is removed, cascading its private checkpoint. A sealed edit/Undo roundtrip may have equal final JSON but still creates a compensating revision because its acknowledged attachment history matters. Old in-process history is not resurrected; the application starts a fresh native editor session against the recovered canonical revision.

A draft without a verified checkpoint uses the conservative snapshot reconciliation path. It never receives synthesized proof authority or repeated-quote matching. Notes remain reachable and may become detached when the old exact attachment cannot be established.

## Application wiring contract

Before writing a native draft, drain newly pending session changes into the retained proof journal. For an ordinary native session, pass the complete unacknowledged journal to `save_rich_editor_draft`, retaining it on failure. The existing canonical save still owns receipt acknowledgement; do not consume or serialize those proofs after only a draft save.

On document open, a rich draft with a verified checkpoint is recovered before mounting a fresh editor session and preparing the note snapshot. Use the returned canonical document/revision, including when final JSON equals the previous canonical JSON. A recovery conflict must preserve its draft and note context and offer an actionable recovery path; it must not silently fall back to guessed anchors. Legacy unproven drafts keep their conservative path.

This checkpoint protects persistent passage attachments through a crash/restart. Unsaved authored note/composer text uses its separate explicit Save/Discard/Cancel close policy and retained per-document state; this schema does not pretend to serialize those UI drafts.

## Verification and remaining gate

The focused Rust suite contains ten regressions: exact Unicode attachment after restart; failed proof/draft guards; intervening authored note/status preservation; explicit reattachment back to equal coordinates; later note creation; clearing authority on unproven draft replacement; deleted-passage Undo; corrupt draft/content binding; schema-five upgrade; and inserted-then-deleted last-safe detached quote/context. Rust checks run on GitHub, not locally.

Five synthetic SQLite executions of the actual migration scripts for schemas 1–5 to 6 preserved existing document rows. Existing migration fixtures now expect schema 6 and accurately remove later tables when constructing an older database. This SQL evidence is not a substitute for the hosted Rust suite, full application restart flow, real IME/clipboard behavior or independent native UX acceptance.

## Explicit preservation route after a conflict

A refused recovery must not prevent the author from accessing either preserved copy. `rich_recovery_snapshot` freezes the canonical document, exact draft/version, all saved note records (including deleted notes), their raw append-only history and optional private attachment-checkpoint JSON in one read transaction. Unknown JSON is retained. Its fields are private, it implements serialization only, and an internal complete-data guard is revalidated before an explicit open/export action. Revalidation failure keeps the captured copy available and requires an explicit reload; there is no automatic retry loop.

The recovery dialog compares bounded, independently scrollable, read-only plain-text previews. Its choices are Export recovery JSON, Open saved manuscript read only and Cancel. Preview text never becomes canonical content. Cancel retains the previous application context. The saved read-only view bypasses the draft and gates all authoring, note, deferred clipboard, property and autosave writes without changing the persisted document's read-only flag or deleting its draft.

The versioned `writercraft.document-recovery` JSON is data-only preservation evidence, not a portable workspace backup or an authority to remap attachments. It retains raw checkpoint evidence because the checkpoint can contain the only surviving inserted-then-deleted quote. No future import may treat that JSON as a native replay proof or trusted checkpoint. Media bytes are excluded; the artifact explicitly states that its references still depend on the original workspace. Export is an explicit file-dialog action and refuses the active database, WAL/SHM paths, canonical aliases and equivalent existing files through a safe portable identity check. An export failure leaves both copies and the frozen dialog intact.
