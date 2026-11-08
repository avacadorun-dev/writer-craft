# Clipboard media portability

## ADDED Requirements

### Requirement: Exact selected rich content and project-owned images travel together

Copy and Cut MUST publish exact selected canonical content, interoperable HTML, meaningful plain text and a registered host image capsule in one acknowledged native write. Only images referenced by the selected content may be exported. The capsule MUST bind a digest of the exact canonical rich bytes. Origin-project hints MUST NOT authorize an asset lookup. Cut MUST preserve source content when media preparation or any native representation write fails.

#### Scenario: Copy an owned image into another project

- **WHEN** the author copies selected rich content containing an owned image and pastes it into a different project
- **THEN** original encoded bytes, MIME, checksum, dimensions and length are validated before imports
- **AND** one atomic destination-owned import creates fresh identities where required
- **AND** only canonical image `assetId` and `src` bindings change; other known/opaque metadata remains intact.

#### Scenario: Reject stale or mismatched clipboard payloads

- **WHEN** a capsule digest, descriptor, original image header, duplicate identity or combined byte budget is invalid
- **THEN** no document or partial image-import batch is committed.

### Requirement: Native clipboard jobs preserve exact editing context

Heavy database, hashing and base64 work MUST run on a bounded host worker. Paste MUST use the one raw bundle captured at its command boundary, without rereading mutable OS clipboard contents. Deferred Cut/Paste completion MUST consume a one-shot session/revision/exact-selection guard. A successful Copy completion may not mutate writing. Dialog text inputs retain their normal native clipboard behavior.

#### Scenario: Writing changes while a media paste is prepared

- **WHEN** the destination revision, exact selection, session identity or composition changes before completion
- **THEN** completion refuses to insert at a different destination and preserves the author's current writing.

### Requirement: External images use explicit host acquisition policy

A clipboard image URL MUST NOT cause ambient network or filesystem access from the editor or renderer. HTTPS image references MUST be staged for the application's explicit asynchronous import policy before a complete paste is applied. A foreign UUID without portable bytes or current destination ownership MUST fail visibly rather than becoming a successful placeholder.

#### Scenario: External HTML contains an HTTPS image

- **WHEN** the captured clipboard HTML contains a permitted external image
- **THEN** the host receives a pending import requirement before any document mutation.

## Verification boundary

These requirements remain part of app issue #7 and full editor epic #4. The first implementation supplies deferred engine contracts and internal owned-image capsules. Full native app wiring, external HTTP acquisition, file drop, mixed/open wrapper fitting, rectangular table selection and cross-application paste UX remain required until verified. A synthetic fixture or OS format roundtrip alone does not close those issues.
