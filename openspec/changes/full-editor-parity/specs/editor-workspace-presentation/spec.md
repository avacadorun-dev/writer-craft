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
