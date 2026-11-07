# Native MVP capability delta

## Purpose

Offer a coherent native writing workspace whose independent brightness and palette settings remain comfortable across sessions.

## ADDED Requirements

### Requirement: AP-01 Light and dark modes

WriterCraft SHALL provide explicit Light and Dark appearance modes, preserving the selected mode across application restarts without changing manuscript content.

#### Scenario: Reopen in Light mode

- **WHEN** the author selects Light, closes WriterCraft and reopens it
- **THEN** navigation, editor, inspector and dialogs use the selected light appearance

### Requirement: AP-02 Non-green legacy palettes

WriterCraft SHALL offer cedar-and-copper, dried-rose-and-wine, fog-graphite, linen-clay, dusty-lavender and porcelain-ink palettes in both brightness modes, alongside its native charcoal/blue default.

#### Scenario: Select a legacy palette

- **WHEN** the author selects a supported palette and switches brightness mode
- **THEN** the palette identity remains selected and all active controls use the corresponding mode tokens

#### Scenario: Encounter an excluded green preference

- **WHEN** legacy settings refer to quiet or olive-ink
- **THEN** WriterCraft offers the native default and does not add those green palettes to its selectable palette list

### Requirement: AP-03 One native workspace

WriterCraft SHALL present one shared command bar, bounded navigator and inspector panes, and a central writing surface with contained scrolling and visible save state.

#### Scenario: Resize a writing window

- **WHEN** an author resizes the window or toggles the inspector
- **THEN** the writing surface remains usable, panes stay bounded and the editor draft is preserved

### Requirement: AP-04 Accessible controls and typography

WriterCraft SHALL provide readable Cyrillic typography, visible keyboard focus, coherent vector icons with accessible names, and working text selection and clipboard behavior in each supported appearance.

#### Scenario: Use an icon command by keyboard

- **WHEN** the author focuses and activates an icon-only command
- **THEN** the command exposes its functional name, shows focus and performs the same action as its labelled counterpart
