# assistant-review

## Purpose

Keep an author in control of suggested manuscript changes through visible staging, exact validation, atomic application and safe undo.

## Requirements

### Requirement: AR-01 Noncanonical proposals

WriterCraft SHALL keep proposed text separate from the canonical document until the author explicitly applies a reviewed proposal.

#### Scenario: Stage an exact replacement

- **WHEN** a unique phrase is proposed for replacement
- **THEN** the author can review the before and proposed text while saved content and history remain unchanged

### Requirement: AR-02 Unambiguous exact targets

WriterCraft SHALL reject an exact replacement target that is missing or occurs more than once, including overlapping occurrences.

#### Scenario: Overlapping phrase

- **WHEN** the requested phrase is ana in banana
- **THEN** WriterCraft reports an ambiguous target and stages no replacement

### Requirement: AR-03 Validated apply and reject

WriterCraft SHALL apply a proposal only while its base revision and expected content match, and SHALL reject a staged proposal without changing canonical text.

#### Scenario: Source changes before apply

- **WHEN** the author changes the document after a proposal was staged
- **THEN** Apply reports a stale source and preserves the newer document and its history

### Requirement: AR-04 Guarded compensating undo

WriterCraft SHALL undo an applied proposal by adding a compensating revision only while the result of that application is still current.

#### Scenario: Newer manual work exists

- **WHEN** an author requests proposal undo after saving a later manual change
- **THEN** WriterCraft reports an undo conflict and preserves that later change

### Requirement: AR-05 Truthful assistant origin

WriterCraft SHALL identify whether output comes from a configured AI provider or deterministic local tooling, show its actual scope, and keep incomplete or cancelled output from applying edits.

#### Scenario: No AI provider is configured

- **WHEN** the assistant panel is opened without a configured provider
- **THEN** it offers truthful setup or local-tool actions and does not label scripted text as an AI answer

### Requirement: AR-06 Grouped assistant parity milestone

At the parity milestone, WriterCraft SHALL support reviewed multi-document changes with scoped context, protected ranges, source citations, persistent task state and atomic grouped apply and undo.

#### Scenario: One target in a grouped proposal is stale

- **WHEN** any selected target changes before a grouped proposal is applied
- **THEN** no selected document is partially committed and the author receives an actionable conflict
