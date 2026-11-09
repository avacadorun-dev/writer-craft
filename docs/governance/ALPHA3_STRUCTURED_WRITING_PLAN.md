# Alpha.3 structured writing delivery

Selected 2026-11-09 after F-49/F-50/F-51. Active application issue: #14. This plan supersedes a literal character-form deliverable. It does not replace the pending full migration inventory or authorize Canvas engine changes.

## Outcome and sequence

Deliver `structured-writing-alpha`: an author can organize, draft, annotate, revise, consult references, reopen and hand off a substantial manuscript. Project kinds remain an open research decision; do not add a mandatory taxonomy selector. Existing imported kinds/custom schemas remain authoritative for their projects.

1. Reconcile current source and preserve unfinished work. Done at `fc49359789ad3997a251db6284ecc3c49e196690`; behavior base `fb87372`, Canvas pin `6b68eb4`, editor pin `fd96a30`. The retained 312-line #10 diff has SHA-256 `6b72cc0c11984b0c93367a7724ea224dc519f306ccb3378fb654dcb885018083` (Git diff bytes). Verify 42626161032 was queued at reconciliation; no new passing build is inferred.
2. Complete hierarchy-led creation and full document/folder lifecycle under #14: read, create, edit, rename, move/reorder, trash and restore. Reuse original free folder/document/note semantics for native projects and saved schemas for imported projects. Preserve stable identity, current text, histories, annotations and unknown data.
3. Complete author-defined passage tags and notes under the same #14 task: create/read/update/delete, project-wide filtering, exact occurrence navigation and return. F-54 adds reusable project label names/colors and a compact note inspector; no automated entity extraction or guessed relationships.
4. Independently accept the complete structured drafting/marking task on an exact hosted native candidate. Keep #14 open for unfulfilled research, sizing and board-related criteria. Only after this active issue slice is accepted, fix and verify #7 double-click/context/keyboard source editing.
5. Select #11 as the next independent issue: finish the existing readable Markdown/assets and canonical folder/restore path needed by this manuscript. Verify actual native export/import, privacy scope, hierarchy/annotation preservation and an independent restored copy. Close only criteria actually completed.
6. Freeze the release source, verify the complete working project on it, obtain independent `structured-writing-alpha` acceptance, package native installers on GitHub, update actual compiled screenshots and publish alpha.3. Preserve alpha.1/alpha.2. Full parity #4 and Canvas #17 remain open.

Root owns app integration, hosted dispatch, exact native observation and release. Delegation names one active issue, owned files and a stop condition. Rust builds/tests/packages run only on GitHub. Public CI fixtures remain invented and contain no private manuscripts. The running Preview68db76e, user projects and CraftCanvas checkout are untouched.

## End-to-end scenarios

### A3-WRITE-01 — build and revise an organized story

Joins PT-NOV-01 and the non-Canvas consultation part of PT-CHAR-01/PT-KNOW-01. Start in the native Library with an empty project. Create a manuscript folder with several chapter folders and scene documents; create a separate materials branch with character motivations, descriptions and setting notes. Write actual connected prose. Rename a chapter and scene, move/reorder a scene, trash an abandoned scene, restore it and confirm its text/history. While revising a repeated passage, open a reference document, edit it, return to the same selection/scroll and continue typing. Save, quit, reopen and continue. Invalid placement, cancelled edits and failed/stale saves must retain work. F-53 direct highlighted drag-and-drop remains required; F-54 replaces permanent row buttons with full-width titles and right-click/keyboard contextual actions.

### A3-MARK-01 — follow a character or unresolved story question

Joins PT-EDIT-01/PT-KNOW-01 and the explicitly requested manual tagging extension. Select an exact occurrence, attach author-defined tags and an optional continuity note, then mark related occurrences in other scenes. Filter the project by a tag, inspect a result in its document, revise, return, change/remove a tag and delete/restore the relevant note through the supported lifecycle. A repeated name never substitutes for the captured identity/range. Deleted text produces an explicit detached state. Define a project label and its color before assigning it, recognize the same color in text and the note list, and revise the shared label deliberately. Multiple labels remain named and ordered, with the first driving the text mark. Save/reopen and canonical transfer retain labels (including unused definitions), tags, anchors and note history. This is author-controlled markup, not NER inference or automatic character synchronization.

### A3-HANDOFF-01 — deliver and recover the organized work

Joins the selected #11/PT-MIG-01/PT-NONF-01 transfer path. Export the intended manuscript/reference scope as readable Markdown with actual assets, inspect the resulting organization, and separately export/restore a complete canonical project. The restored copy must retain tree order, stable logical references, rich text, passage tags/notes, histories and original assets. Explain any reader-format limitation and private annotation inclusion before exporting. Cancel/failed output must not publish a misleading partial success.

### A3-REEDIT-01 — revise an existing technical/story object

After #14 acceptance, complete #7 on existing rendered Mermaid and inline/display formula objects: single click selects, double click opens the exact source, context-menu and keyboard paths reach the same guarded editor. Apply rerenders the same object, Cancel leaves it unchanged, Undo/Redo and save/reopen preserve the result and surrounding prose. Failed/stale edits retain the draft. Keep these actions contextual without growing the permanent toolbar.

### F52-VISUAL-01 — finish the desktop presentation during the author task

Capture each visited screen, pane, menu and dialog while performing the scenarios. Record its layout, spacing, content density, action hierarchy, discoverability, icon treatment and keyboard usability. Improve observed defects, then repeat the affected task and capture the changed compiled candidate. Evaluate coherent filled/custom desktop icons where recognition benefits. Independent visual review is a release requirement and does not stand in for functionality or unvisited-platform acceptance.

## Acceptance manuscript

Use an original coherent story, **The Last Light / Последний огонь**, created for this acceptance without private user data: a coastal town prepares to decommission its lighthouse; restorer Asya, engineer Lev and archivist Nina disagree about a missing signal log. Four chapters, at least two meaningful scenes per chapter, and distinct character/reference branches provide actual motivation, conflicting knowledge and revisions to track. The prose, motivations and notes must connect across the story; placeholder strings, a control gallery or a collection of test cases cannot qualify it.

Create and change this project through the native interface. Keep a concise action log and exact-candidate captures; inspect durable state/export afterward only as corroboration. API-created data, injected database rows or scripted internal commands do not prove the user journey. Automated semantic fixtures remain separate engineering evidence. The demonstration is newly authored acceptance material, not an observed human research session or an existing person's manuscript.

## Release truthfulness

`structured-writing-alpha` is a bounded engineering prerelease, not `full-editor-parity`, professional screenplay pagination, exhaustive migration certification or three-platform GUI acceptance. The independent reviewer must inspect the actual candidate and author journey. Retain the pending full-parity gate/inventories and all unresolved criteria. Owner authorization covers publication after concrete verification; no duplicate publication permission is needed.
