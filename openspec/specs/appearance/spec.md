# appearance

## Purpose

Offer a coherent native writing workspace whose independent brightness and palette settings remain comfortable across sessions.

## Requirements

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

WriterCraft SHALL present one shared command bar, bounded navigator and inspector panes, and a central continuous-text writing surface with contained scrolling and visible save state.

#### Scenario: Resize a writing window

- **WHEN** an author resizes the window or toggles the inspector
- **THEN** the writing surface remains usable, panes stay bounded and the editor draft is preserved

#### Scenario: Edit a long manuscript continuously

- **WHEN** the author writes or scrolls through a long manuscript
- **THEN** text flows continuously inside the bounded editor pane and page layout is presented only in publication preview or export

### Requirement: AP-04 Accessible controls and typography

WriterCraft SHALL provide readable Cyrillic typography, visible keyboard focus, coherent vector icons with accessible names, and working text selection and clipboard behavior in each supported appearance.

#### Scenario: Use an icon command by keyboard

- **WHEN** the author focuses and activates an icon-only command
- **THEN** the command exposes its functional name, shows focus and performs the same action as its labelled counterpart

### Requirement: Categorized native preferences

The application SHALL expose General, Appearance, Editor, AI and providers, Agents and MCP, Voice, Files and recovery, and Keyboard shortcuts as stable settings sections. Working controls SHALL persist in their corresponding sections. Sections whose legacy capability is not yet migrated SHALL explicitly describe their unavailability without functional-looking connection switches.

#### Scenario: Appearance remains usable in a compact desktop window

- **WHEN** the author opens Appearance at the minimum desktop size
- **THEN** mode and palette controls scroll only within the content region
- **AND** the category sidebar and Done action remain reachable without scrolling the whole window

### Requirement: Coherent desktop shell boundaries

The command bar SHALL align its dividers with visible navigator and inspector boundaries. Titles SHALL remain confined to their allocated region at long translated labels and narrow window widths.

#### Scenario: Long project title

- **WHEN** a long project name is shown in the minimum-width window
- **THEN** it truncates with a full-title tooltip
- **AND** the file, save, review and settings commands remain separate and accessible

### Requirement: Recognizable appearance previews

The application SHALL present Light, Dark and System modes separately from the seven palettes. The choices SHALL preview actual resolved workspace colors in native miniature-window cards with an accessible name and a visible selected outline.

#### Scenario: Selecting a palette while using System mode

- **WHEN** the author chooses a palette in System mode
- **THEN** the palette changes while System mode remains selected
- **AND** the preview and workspace use that palette's resolved colors for the current OS appearance
