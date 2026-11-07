# Native interface localization

## Purpose

Let authors use the complete native interface in eight built-in languages while preserving their own writing and reliable offline operation.

## Requirements

### Requirement: LO-01 Eight built-in interface languages

WriterCraft SHALL provide Russian (ru), English (en), German (de), French (fr), Spanish (es), Portuguese (pt), Chinese (zh) and Japanese (ja) interface catalogs in the installed application.

#### Scenario: Select any supported language offline

- **WHEN** the author opens language settings without a network connection
- **THEN** all eight languages are available by recognizable names and selecting one does not require a download

### Requirement: LO-02 Immediate language switching

WriterCraft SHALL apply a selected language immediately to navigation, settings, dialogs, tooltips, accessibility labels, empty states and currently visible messages without discarding the current editor buffer.

#### Scenario: Switch during an unfinished edit

- **WHEN** the author changes interface language while a document contains unfinished text
- **THEN** the interface changes language in the current session and the working text remains unchanged

### Requirement: LO-03 Persistent language choice and safe fallback

WriterCraft SHALL restore a supported saved language. A missing preference defaults to Russian; an invalid or unreadable preference uses Russian with a visible recovery notice and preserves the original configuration until an explicit preference change.

#### Scenario: Reopen a chosen language

- **WHEN** the author selects Japanese and restarts the application
- **THEN** the interface opens in Japanese

#### Scenario: Invalid stored language

- **WHEN** the saved language identifier or preference file cannot be used
- **THEN** the interface remains usable in Russian, explains the recovery state and leaves the original preference file unchanged until an explicit settings change

### Requirement: LO-04 Localized dynamic messages

WriterCraft SHALL format application statuses, counts, proposal states and actionable errors in the selected language with complete parameters; switching language SHALL also update messages already visible.

#### Scenario: Switch with an existing save conflict

- **WHEN** a save conflict is visible and the author switches language
- **THEN** its explanation and actions use the new language while the document identity and revision values remain correct

### Requirement: LO-05 Chinese and Japanese glyph coverage

WriterCraft SHALL display Chinese and Japanese interface text legibly in the packaged native application, with the required font coverage available offline.

#### Scenario: Inspect Chinese and Japanese controls

- **WHEN** the author opens each dialog and writing pane in Chinese and Japanese
- **THEN** translated labels render with supported glyphs and without missing-character boxes or clipped essential actions

### Requirement: LO-06 Preserve author content and technical source

WriterCraft SHALL change interface language without translating manuscript text, user titles, exact quotes, source files or original technical diagnostics; diagnostics remain separately identified by a localized label.

#### Scenario: Switch a multilingual manuscript

- **WHEN** the interface language changes while a manuscript and its user-defined title contain several languages
- **THEN** their text, revision and exact proposal anchors remain unchanged

### Requirement: LO-07 Complete catalog and interface coverage

WriterCraft SHALL provide every required interface message and parameter in every supported catalog. Application-owned visible strings SHALL use the localization system; an unexpected missing key SHALL display a visible neutral diagnostic rather than silently substitute another language.

#### Scenario: A required translation is missing

- **WHEN** catalog validation finds a missing key, empty translation or inconsistent parameter set
- **THEN** the acceptance check fails before release and identifies the language and message

#### Scenario: Inspect application-owned strings

- **WHEN** the interface coverage check and native walkthrough encounter controls or dynamic messages
- **THEN** all application-owned text resolves through the selected catalog, with user content and explicitly labelled technical diagnostics treated as preserved source
