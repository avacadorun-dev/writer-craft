## ADDED Requirements

### Requirement: PM-1 project-scoped immutable media preview

The host SHALL resolve media from the selected database and exact project/logical identity on a separate bounded worker. Raw Canvas file/URL values MUST NOT grant filesystem or network authority. Original bytes, metadata, revisions and unknown fields MUST remain unchanged after preview.

#### Scenario: Imported PDF or image is opened
- **WHEN** an imported card references a valid same-project PDF or supported image
- **THEN** actual source pixels SHALL appear inside its reserved native card rectangle, and explicit opening SHALL display a native preview with actual PDF pages where applicable
- **AND** another project with the same logical alias SHALL not supply its bytes

#### Scenario: Decode fails or content is unsupported
- **WHEN** the exact selected source cannot be previewed
- **THEN** the host SHALL show a localized actionable failure and Retry while preserving the complete original artifact
- **AND** the failure SHALL not be reported as supported playback or complete migration parity

### Requirement: PM-2 bounded cancellable presentation

The host SHALL bound queued/running work and acknowledged texture memory independently, and SHALL refuse any stale or foreign completion before native texture acceptance. Page and geometry changes SHALL use explicit request identities.

#### Scenario: User leaves a pending board
- **WHEN** the selected board context is dropped during a queued/rendering request
- **THEN** its transfer and provider cancellation SHALL be requested and its late result SHALL not enter another context

#### Scenario: A page or source changes before completion
- **WHEN** a result has an older ticket or different project/resource/page/geometry key
- **THEN** the current preview SHALL remain unchanged
