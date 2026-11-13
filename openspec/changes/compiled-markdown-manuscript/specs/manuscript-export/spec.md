# Compiled manuscript export delta

## ADDED Requirements

### Requirement: ME-01 Ordered single-manuscript scope

The desktop SHALL compile one selected manuscript into one Markdown document in the current project-tree order, offering the whole project, the current document, a chosen subtree or explicit documents, and SHALL preview actual inclusion, chapter order and reader conversions before writing.

#### Scenario: Reordered chapters

- WHEN a multi-chapter manuscript is reordered and compiled
- THEN the single document lists included chapters in the shown tree order, using the same inclusion plan as the final-document DOCX export.

#### Scenario: Excluded content

- WHEN the user compiles the current document or a subtree
- THEN sibling documents, character research, notes, annotations, labels, histories and deleted content outside the selection never appear in the output.

#### Scenario: Deleted writing

- WHEN the project contains a trashed document or section
- THEN the whole-project and subtree scopes skip that branch, deleted text never appears in the manuscript, and the preview never lists it.

### Requirement: ME-02 Documented Markdown reading profile

The compiled document SHALL represent headings, paragraphs, supported marks, lists, links, code languages, tables, and formula/Mermaid source according to a documented reader profile, and SHALL disclose every conversion or unrepresentable object instead of dropping it silently.

#### Scenario: Unrepresentable rich content

- WHEN an included revision contains a node or mark that the reading profile cannot express
- THEN the output keeps the readable remainder, the pre-export disclosure or completion report names the affected object, and no canonical source is discarded or rewritten.

#### Scenario: Missing referenced artifact

- WHEN an included revision references a media object absent from the captured snapshot
- THEN compilation refuses before creating the destination and reports the missing artifact.

### Requirement: ME-03 Adjacent original assets

Referenced images and attachments SHALL be copied into a deterministic adjacent assets folder with portable collision-safe names and relative links from the one document, preserving original bytes.

#### Scenario: Moved export pair

- WHEN the compiled `.md` and its adjacent assets folder are moved to another directory
- THEN every included relative link resolves, filenames remain safe and collision-free, and copied asset checksums match the originals.

### Requirement: ME-04 Manuscript-only output

The compiled export SHALL contain only selected manuscript text and its referenced artifacts; it SHALL NOT leak project research, annotations, label catalogs, revision history, deleted content, credentials or machine-specific paths, and SHALL NOT present itself as a lossless project backup.

#### Scenario: Source preservation

- WHEN a manuscript is compiled
- THEN the source project, its revisions, histories, notes and original artifacts remain byte-identical, and the completion report states the reader scope rather than backup fidelity.

### Requirement: ME-05 Atomic cancellable publication

Compilation SHALL use a new exclusive destination, write an explicit incomplete marker first, verify and sync every payload, and remove the marker only after complete success; cancellation, concurrent snapshot change, missing assets, destination collisions and failed writes SHALL never leave a completed-looking partial export.

#### Scenario: Cancelled compilation

- WHEN cancellation is requested while payloads are being written
- THEN writing stops, the destination remains explicitly incomplete, no completion is reported, and unrelated existing files are untouched.

### Requirement: ME-06 Localized native flow

The native export flow SHALL use the established transfer modal behavior, complete eight locale catalogs, bounded scrolling and reachable actions, and synthetic public fixtures only.

#### Scenario: Every supported locale

- WHEN the export mode is opened in ru/en/de/fr/es/pt/zh/ja
- THEN all visible labels resolve without placeholders, the order preview and disclosures remain readable, and user manuscript text is never translated.
