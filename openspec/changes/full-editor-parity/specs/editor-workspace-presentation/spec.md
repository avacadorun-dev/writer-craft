## ADDED Requirements

### Requirement: Adaptive bounded manuscript surface

The editor SHALL use a centered bounded document surface when remaining center width permits and a joined pane under constraint. Both states and Focus SHALL preserve a readable maximum prose measure, continuous writing, one native app shell and stable document/selection/history state. Structural presentation SHALL follow the owner-reviewed adaptive design.

#### Scenario: Focus gains horizontal space
- **WHEN** the author enters Focus and side panes disappear
- **THEN** the document stays centered with exterior gutters and bounded prose width, a visible exit restores the previous panels, and no print page breaks are introduced

#### Scenario: Panels constrain the center
- **WHEN** the remaining center width falls below the approved document-and-gutter threshold
- **THEN** the surface joins the center pane while preserving useful internal gutters, local manuscript scrolling, reachable formatting and exact caret/selection/history

### Requirement: Consistent comfortable fields and modal grid

Native single-line fields and dialog actions SHALL have a 40-point minimum height. Fields SHALL use full available form width, 12/8-point horizontal/vertical padding and centered text. Modal header, body and footer SHALL share a 24-point inset grid, deliberate spacing and reachable actions at supported window sizes and languages.

#### Scenario: Create a document through the form
- **WHEN** the author opens New document and edits a localized or long title
- **THEN** its field is comfortable and bounded, title/label/field/footer share the inset grid, actions are reachable, and Enter/button submission creates an editable manuscript without a focus failure

#### Scenario: Cancel a contextual editor
- **WHEN** the author cancels with the labelled action or Escape
- **THEN** canonical manuscript content remains unchanged and focus returns to the original editing target

### Requirement: Exact-source presentation acceptance

The shared presentation SHALL remain unaccepted until independent review verifies the compiled native app at supported viewports, languages and appearances. Design illustrations and source geometry SHALL NOT substitute for observed field, modal, focus and adaptive-state behavior.

#### Scenario: A design concept passes visual review
- **WHEN** the owner reviews adaptive workspace or modal illustrations
- **THEN** that review selects an implementation direction while the compiled interaction and release gates remain pending

### Requirement: Live manuscript statistics

The editor SHALL expose word count, Unicode scalar character count and estimated reading time from the original editor's reader-text projection of the live document. Saved note bodies and nontext atom attributes SHALL NOT enter these counts. Unsaved native edits SHALL update statistics; changing selection or presentation SHALL NOT mutate document history.

#### Scenario: Inspect statistics before autosave
- **WHEN** the author edits text in a table or formatted paragraph and opens document statistics before autosave
- **THEN** the count reflects that exact live tree, the Unicode character count and reading estimate are available, and no save, annotation or Undo entry is created by inspection

### Requirement: Source backed native reading preferences

The editor SHALL provide the original three font roles, text sizes, line and paragraph spacing, independent list spacing, 60/70/80-character measures, toolbar modes and word-goal preference through usable persisted native controls. Character measure SHALL use the selected font's actual zero advance; font-size line-height semantics SHALL remain distinct from natural font metrics. Presentation changes SHALL preserve canonical content, selection and Undo.

#### Scenario: Change reading presentation
- **WHEN** the author changes manuscript family, text size, spacing, width or toolbar mode
- **THEN** the live native document reflows without resetting its editing session, constrained windows retain reachable overflow commands, and restart restores the acknowledged choices

#### Scenario: Existing custom size or invalid preference file
- **WHEN** the author opens an earlier native workspace with a custom size, or the saved editor preferences are invalid, newer or changed externally
- **THEN** a valid existing size is retained, unsupported data is preserved with a readable error, and no settings interaction silently overwrites the unacknowledged file

### Requirement: Opaque content participates in document flow

Image, diagram, block-formula and inline-formula paint, captions and status placeholders SHALL fit their allocated document flow. Font-size-based prose line spacing SHALL remain distinct from natural font metrics without clipping or collapsing object extent. Async render completion, retry, theme, typography and width changes SHALL preserve canonical content, exact selection, annotations and Undo.

#### Scenario: A large rendered object follows text or a table
- **WHEN** a mixed manuscript contains code, tables, a large image, two different Mermaid diagrams, block formulas and an inline formula, and its native rendering completes
- **THEN** painted objects and captions remain within allocated flow, following content does not overlap them, and reflow preserves the canonical document and editing history

#### Scenario: Inspect native evidence after asynchronous rendering
- **WHEN** automated verification targets a synthetic opaque block
- **THEN** capture waits for visible render and scroll geometry to settle, records actual viewport dimensions and render failures, and does not treat requested window size or startup-only screenshots as completed layout acceptance
