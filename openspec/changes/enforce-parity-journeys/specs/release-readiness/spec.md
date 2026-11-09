## ADDED Requirements

### Requirement: RR-1 both completeness and task acceptance
The release checker SHALL require independent acceptance of every authored source inventory, capability and role journey for full-editor-parity and full-parity, including prerelease versions below1.0.

#### Scenario: Attractive but incomplete candidate
- **WHEN** screenshots and a named UX review exist but a capability or journey is missing, pending, rejected or unknown
- **THEN** release qualification fails and no full parity claim is authorized.

#### Scenario: Partial source inventory
- **WHEN** a source inventory has not received independent review for omitted behaviors
- **THEN** its implementation evidence cannot substitute for completeness acceptance.

### Requirement: RR-2 exact requirements and evidence
Acceptance SHALL bind current source, lock, required contracts, validator and narrative evidence. Changed requirements, modified reports, duplicate IDs and stale observations SHALL fail qualification.

#### Scenario: Changed task after review
- **WHEN** a required journey changes after the candidate review
- **THEN** the fingerprint changes and the prior review cannot qualify the new candidate.

#### Scenario: Rewritten interaction report
- **WHEN** a reviewed interaction report changes without a new review
- **THEN** its recorded hash no longer passes the evidence check.

### Requirement: RR-3 honest progress and research
Source validation SHALL allow explicitly pending acceptance while preserving release refusal. Role stories SHALL distinguish legacy requirements, requested enhancements and unselected hypotheses; documenting a story or competitor feature SHALL NOT count as observed user validation.

#### Scenario: Ongoing development
- **WHEN** source contracts are valid but their native evidence is pending
- **THEN** source checks can pass while release readiness remains false.

#### Scenario: Unselected research idea
- **WHEN** a relationship model or profile is only a research hypothesis
- **THEN** it remains pending discovery and cannot be silently treated as a selected implementation requirement or passed user study.
