## ADDED Requirements

### Requirement: Schema-aware manuscript organization

Authors SHALL create and navigate nested folders, documents and notes using the saved project structure. Native projects SHALL support the original free structure without requiring a project-kind selector. Imported custom rules and unknown data MUST remain intact.

#### Scenario: Organize a new story
- **WHEN** A3-WRITE-01 creates chapter, scene and reference branches
- **THEN** each item appears in its chosen valid location with a stable identity and writable content only when its kind permits text

### Requirement: Complete recoverable hierarchy lifecycle

Authors SHALL rename, edit, move, reorder, trash and restore documents and folders. Commands MUST preserve text, descendants, revisions, annotations and references and refuse stale, cyclic, foreign or disallowed placements before mutation.

#### Scenario: Revise the manuscript structure
- **WHEN** an author moves a scene, renames a chapter, trashes and restores a branch
- **THEN** the saved tree reflects those deliberate actions and the restored content and history remain available under the same identities

#### Scenario: Cancel or reject a structural change
- **WHEN** a placement is cancelled, invalid or stale
- **THEN** the prior tree and unsaved writing remain intact with an actionable result

#### Scenario: Restore a nested branch
- **WHEN** a folder is trashed and restored
- **THEN** all active descendants participate, independently trashed descendants remain in Trash, and a missing, trashed or disallowed parent causes refusal before mutation

### Requirement: Direct tree organization

The hierarchy SHALL support native drag-and-drop into valid folders and before/after siblings, with highlighted targets and insertion lines. Full-width item rows and compact indentation SHALL remain usable at three/four nested levels. Right-click and keyboard context menus SHALL provide creation and lifecycle actions without permanent row buttons. Hover expansion and edge autoscroll SHALL support organizing larger projects.

#### Scenario: Move a scene into a collapsed chapter
- **WHEN** an author drags a scene over a collapsed valid chapter and drops it after the folder expands
- **THEN** the target preview SHALL match one guarded persisted move; stable identities, text and history remain unchanged

#### Scenario: Reject or cancel a drag
- **WHEN** a drag targets itself, a descendant, a schema-disallowed parent, leaves the tree or is cancelled with Escape
- **THEN** invalid placement SHALL be visible and no structure/content mutation SHALL occur

#### Scenario: Act on a tree item without permanent row buttons
- **WHEN** an author right-clicks an item or invokes its keyboard context menu
- **THEN** actions target that exact item, including creation inside a schema-valid container, while long titles retain the row's available width

### Requirement: Exact consultation return

An author SHALL consult and edit a reference document and return to the acknowledged originating scene with exact selection and measured scroll when its revision remains unchanged. Failed navigation MUST retain drafts and the return route.

#### Scenario: Consult a motivation while revising dialogue
- **WHEN** the author leaves the second repeated Unicode passage to inspect a character note and returns
- **THEN** subsequent typing operates on the original occurrence; changed source instead receives an explicit changed-context result without guessed positioning

### Requirement: Author-defined passage markup

Authors SHALL create, inspect, edit and remove tags and optional notes attached to exact passages, then filter and navigate those records across the current project. Labels MUST retain author text and meaning; no automatic extraction or name matching grants attachment authority.

#### Scenario: Follow a character through several scenes
- **WHEN** A3-MARK-01 tags selected occurrences in different documents and opens a filtered result
- **THEN** the intended passage and note are revealed by saved identity and exact anchor, with a return route and no change to unrelated prose

#### Scenario: Manage the author's labels
- **WHEN** a label is removed from one passage or explicitly renamed/removed across the project
- **THEN** the action names its scope, matches exact case-sensitive labels, guards the captured affected revisions and never silently merges differently authored labels

#### Scenario: Remove the last tag or restore deleted markup
- **WHEN** project-wide removal empties a tag-only mark, or deleted markup is restored
- **THEN** the empty mark becomes a recoverable tombstone; restoration retains its identity and marks a no-longer-valid anchor detached without searching for another matching quote

#### Scenario: A result or reference belongs to Trash
- **WHEN** the target document or an ancestor is trashed
- **THEN** navigation retains its identity and presents an unavailable/restore route instead of silently opening it as active manuscript content

### Requirement: Direct recoverable markup removal

Authors SHALL remove a passage mark in one direct action with nearby guarded Undo. Local label unassignment SHALL take one action without deleting the project label. Deletion MUST retain pending authored note data recoverably, preserve manuscript text and refuse stale mutations without losing drafts.

#### Scenario: Finish a marked correction
- **WHEN** an author revises the manuscript and removes its mark from the note list
- **THEN** the mark disappears without a menu or confirmation, the text/caret stay unchanged and Undo restores the exact mark with its latest authored note

#### Scenario: Remove one local label
- **WHEN** an author activates a label's remove control
- **THEN** that assignment is removed without a second Save, unrelated usages/definitions remain and an emptied label-only mark is recoverable

### Requirement: Durable compatible writing state

Tree changes and passage markup MUST survive save/reopen and canonical export/restore with unknown source fields, rich text, histories and original media retained. Older records without tags SHALL remain usable with empty tag metadata.

#### Scenario: Restore an organized draft
- **WHEN** the completed project is restored into an independent workspace
- **THEN** its supported structure, text, tags, notes and references retain their meaning and reader-export limitations remain explicit

### Requirement: Reusable colored project labels

Authors SHALL define, assign and manage named project labels with chosen colors. A note's primary label SHALL determine its manuscript mark and sidebar color; all assigned names SHALL remain visible without depending on color alone. Definitions, including unused labels, MUST survive reopen and canonical restore. Color changes MUST NOT rewrite prose or annotation anchors.

#### Scenario: Recognize a note without opening it
- **WHEN** an author assigns an existing colored label to a passage note
- **THEN** the text mark and compact note row show the same primary label color, with distinguishable quote, note preview and named labels

#### Scenario: Revise the project vocabulary
- **WHEN** an author creates, recolors, renames or removes a label
- **THEN** the operation shows its scope, refuses stale or conflicting changes before mutation, preserves pending note drafts and retains recoverable note state

#### Scenario: Read existing uncolored markup
- **WHEN** an older project contains manual tags without color definitions
- **THEN** exact tag names, notes, histories and anchors remain usable without an opening-time rewrite

### Requirement: Real native task acceptance

The named alpha.3 scope SHALL require independent review of a coherent substantial manuscript created and revised through the native interface. Compiler checks, API seeding or a rendered feature gallery MUST NOT establish user-journey completion.

#### Scenario: Engineering checks pass without a writing session
- **WHEN** only semantic fixtures or source inspection are available
- **THEN** native task acceptance and release readiness remain pending

### Requirement: Coherent native presentation

The application SHALL organize author actions into understandable contextual groups with consistent desktop icon treatment, bounded panes/dialogs and deliberate spacing. Every visited surface in the acceptance journey SHALL have a capture and independent visual/usability disposition. Blocking findings SHALL be corrected and observed again on compiled source before acceptance.

#### Scenario: A working action is hard to discover or visually unfinished
- **WHEN** the native author walkthrough reveals scattered controls, excessive spacing, unclear icons or inaccessible actions
- **THEN** acceptance SHALL require a recorded correction and fresh compiled visual and interaction evidence for the affected surface

### Requirement: Sequential marked-passage correction

The Review panel SHALL support previous/next navigation through captured marked identities, inline replacement and explicit Apply or Apply-and-next. It MUST preserve unapplied wording across navigation, retain a staged receipt after failure, and show actual changed fragments. Only explicit Apply may change canonical text.

#### Scenario: Correct a filtered sequence
- **WHEN** an author starts review from filtered project markup and applies a replacement before advancing
- **THEN** only that exact occurrence changes, the next mark opens by identity and Undo remains guarded by the applied revision

#### Scenario: Pause or navigate with unapplied wording
- **WHEN** the author closes the panel or visits another marked passage before applying
- **THEN** the authored replacement remains resumable against its captured revision without altering the manuscript or silently retargeting it

#### Scenario: Inspect the whole short selected passage
- **WHEN** a short marked range is revealed, the inspector changes manuscript width, or its correction is applied or undone
- **THEN** the complete range that fits the usable viewport is visible below the formatting toolbar without changing selection direction, text, history or input ownership

#### Scenario: Recognize a contained note row
- **WHEN** a note has authored text, a quoted passage and one or several labels
- **THEN** its named chips and overflow control remain within the row's background and hit area, with consistent scrolling and selected-row reveal

#### Scenario: Remove a mark after its correction is applied
- **WHEN** a fulfilled correction has no active unapplied draft and its mark is removed
- **THEN** the panel exposes removal Undo without a phantom Resume action; an actually retained unapplied replacement remains resumable
