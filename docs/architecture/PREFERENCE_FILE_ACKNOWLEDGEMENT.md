# Preference file acknowledgement and recovery

Status: implementation design for UX finding A2-08 under existing issues #4/#9. It extends the
A2-03 editor retry correction to appearance, interface and editor preferences. Native retest and
independent acceptance remain pending.

## One file publication rule, three existing formats

Use one small desktop-owned acknowledged JSON-file primitive. It bounds reads and desired output
to 64 KiB, rejects duplicate keys and nonobject roots, and keeps the exact acknowledged bytes. A typed
adapter validates the existing flat appearance/interface formats or editor envelope version 1.
Interface's bounded legacy font-size migration remains supported. Editor typography is authoritative
in editor.json; changing it no longer rewrites the interface file's migration-only font-size field.

Unknown root and nested fields stay in raw JSON envelopes rather than becoming runtime settings.
Known-field updates merge only the serialized known fields; untouched raw values, including large
numeric lexemes, are retained. A save that neither changes nor adds known fields preserves complete original bytes and framing.
Malformed, unreadable, oversized or explicitly unsupported files keep a write barrier. Runtime choices
remain usable for the current session; changing a choice cannot silently grant overwrite authority.

Prepare a flushed sibling temporary file, recheck the exact acknowledged source immediately before
publication, and publish a first-created file without clobbering a path that appeared meanwhile.
For an existing acknowledged file use atomic replacement. A pre-write check plus rename is **not**
a universal atomic compare-and-swap against noncooperating writers. The implementation refuses
changes it observes before publication and makes no broader filesystem-race/power-loss guarantee.
Symlink preference inputs are preserved rather than followed for writing. SQLite/invalid aliases
cannot become acknowledged JSON; atomic replacement does not write through hardlinks.

## Independent acknowledgement and recovery

Each group owns its desired runtime values and last acknowledgement. A failed save retains its
pending values. Retry repeats unchanged pending choices; success in one group cannot clear another
group's failure. Errors and cancellation never mark a save acknowledged. Externally changed files
retain the original acknowledgement and a conflict state; Retry cannot replace the newer file.

Settings names every group needing attention and uses Continue for this session while any desired
choice or source barrier remains unacknowledged. Retry saves only pending retryable groups. A group
with an invalid or changed file exposes Reload saved settings with an explanation that it replaces
only that group's session choices. Reload requires an existing saved file and validates it before
changing runtime values; failure retains every choice and pending state. Reload never writes the file or touches another group.

Manuscript-save status remains separate from session-only preferences and unsaved author notes.
Appearance, language and editor reflow still leave canonical content, exact selections and Undo intact.
Footer actions keep 40-point targets and the shared 24-point grid, with bounded content at compact sizes.
No reset/force-overwrite action or new preference capability is introduced.

## Verification

Synthetic semantic/controller tests must cover missing and partial legacy files, valid choices,
unknown raw values, invalid/oversized/future inputs, denied writes and unchanged-value retry, independent
partial failure, file creation/replacement/deletion around staging/publication, SQLite/symlink/hardlink
aliases, reload success/failure, restart and per-group file isolation. Hosted all-feature tests and
strict Clippy/formatting are required. Native compact/wide long-language/CJK footer and interaction
checks, and independent UX review, remain separate acceptance gates.

Implemented source verification covers shared-file staging/publication, all three adapters, controller
partial failure and scoped reload, migration-only interface size, manuscript status isolation and
headless accessibility geometry for all eight catalogs at compact/wide sizes. These newly added tests
have not been executed locally: local compilation is prohibited for this review. Direct formatting
and source checks are recorded separately; hosted CI and native visual/interaction checks remain required.
