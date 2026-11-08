# Persistent text marks and attached notes

Status: selected within the owner-approved full-editor scope for `0.1.0-alpha.2`, implementation in progress, not accepted. Updated 2026-11-08. Explicitly outside the published `0.1.0-alpha.1`. Read `openspec/changes/full-editor-parity/` and the full source/UX matrices; a model-only or visual-highlight subset is insufficient.

## Author workflow

The author selects an exact passage and attaches authored note/reminder text. Saving creates a durable visual mark tied to that passage. Moving the caret or focus elsewhere does not remove the mark. Reopening the document or application restores the mark and attached text. Clicking the mark or finding it in the document/project notes view returns to the exact annotated passage.

A temporary editor selection or captured proposal target is not this persistent mark. The annotation is author metadata; it does not become manuscript text or alter ordinary reader exports. A reminder here is the attached author-written note.

## Source-backed parity

The existing Writers Studio contract is `docs/specification/MANUSCRIPT_ANNOTATIONS.md` in the original repository, particularly its model/invariants, Studio workflow, project list and acceptance criteria. It separates notes/dictionary links from document JSON, uses stable block identity plus exact Unicode ranges, keeps annotation lifecycle independent of attached/detached anchor state, and preserves history through text edits and undo. Existing `.wstudio` annotations begin at transfer version 2; current source accepts versions 1–8.

The native migration must preserve those established guarantees rather than treating a colored selection as feature parity. This proposal focuses on durable passage notes; broader dictionary-link, assistant-fix and full-project-transfer parity stays in the complete migration inventory.

## Anchor and lifecycle contract

- Identify the document, relevant stable text/block identity, source revision and half-open Unicode-scalar range. Retain exact quote and bounded surrounding context. Never mix UTF-8 byte, UTF-16 or grapheme positions.
- Verify the exact selected text at the stated revision before creating or reattaching a note. Repeated quotes are distinguished by their actual identity/range, not a first matching substring.
- Known edits before an attached range shift its anchor; edits after it preserve that range. Exact proven native edits inside the passage, including whole-range replacement, regenerate its current quote while retaining note identity and start-right/end-left boundary affinity.
- Whole-passage deletion, deletion strictly across an endpoint or uncertain mapping preserves the note, original quote/context and lifecycle, marks the anchor unresolved/detached and keeps the note reachable. An interior edit with exact sealed mapping remains attached. Do not move it to another occurrence merely because the words match.
- A manual reattachment is an explicit author selection checked against a current revision. A stale document or note revision cannot silently overwrite a newer anchor or note.
- Save text, mapped annotation state and related history atomically. Undo/redo and reviewed assistant changes preserve the corresponding anchor state and compensating history.
- Note text edits, resolve/reopen and explicit note deletion operate on annotation metadata. Resolved status and anchor validity are independent. Duplicating a document or preserving a conflict copy must not silently discard its notes.

## Proposed acceptance matrix

| Situation | Required future result |
|---|---|
| Select the second repeated quote after Cyrillic and emoji | Mark and note attach only to that occurrence. |
| Move focus, select another passage, switch panes | Existing mark stays visible without retaining an active selection. |
| Save, close and reopen | Note text, mark, lifecycle and anchor restore. |
| Insert/remove text before the passage | Anchor follows the deterministic edit mapping. |
| Edit the passage interior through exact native commands | Mark follows the proven endpoints and exact current quote. |
| Delete the passage or delete across its boundary | Note survives as unresolved; original quote/context remain readable. |
| An identical quote remains elsewhere | No automatic first-match retargeting. |
| Explicitly reattach to a current selected passage | New exact anchor is verified and history retained. |
| Save/apply from a stale revision | No partial text/note mutation; conflict is actionable. |
| Undo an edit or reviewed apply | Text and annotation state remain consistent, preserving history. |
| Ordinary manuscript export | Metadata decorations and private notes stay outside reader text. |

## Unselected implementation decisions

The current native core stores plain prose and read-only imported rich JSON. The owner selected the complete native editor as the next milestone. Technical choices still need validation against the full original editor; no capability may be dropped to fit a smaller implementation. Stable block identity, native decoration/caret mapping, exact schema migration and the first supported selection granularity need review in that scope. No table, editor dependency, API or release version is committed here. Read the [proposed change](../../openspec/changes/persistent-text-annotations/design.md) before eventual implementation.

Tracking: [persistent text marks and author notes — issue #2](https://github.com/avacadorun-dev/writer-craft/issues/2).

The executed original-plugin probe, corrected partial-edit expectations and explicit parent/leaf projection contract are recorded in [Native annotation mapping](NATIVE_ANNOTATION_MAPPING.md). Original locator/projection bugs are documented there and must not be reproduced.
