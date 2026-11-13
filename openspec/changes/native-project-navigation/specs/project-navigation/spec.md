## ADDED Requirements

### Requirement: Source-backed active hierarchy

Native navigation SHALL expose imported active sections and documents with their saved parent and sibling organization, scoped identities and current manuscript labels. Missing/inactive parents SHALL not hide otherwise active children. Newly created native documents SHALL remain reachable without duplicating mapped imported documents or exposing trashed source rows as fallback entries.

#### Scenario: Navigate nested source documents with repeated names

- **WHEN** a project contains two nested documents with identical titles and a contentless parent section
- **THEN** the navigator shows their respective hierarchy and opens only the exact selected native document without title-based guessing or manuscript mutation

#### Scenario: An imported document is trashed

- **WHEN** the active navigator is built while the corresponding native document is still retained in storage
- **THEN** the trashed node does not reappear as an unmapped fallback document and active descendants remain reachable according to the source hierarchy contract

### Requirement: Bounded search and disclosure

The hierarchy SHALL use bounded native scrolling, stable presentation identities and comfortable labelled controls. Search SHALL expose matching ancestry without changing canonical data or the saved collapsed presentation state. Deep or cyclic input SHALL not overflow a recursive call stack or create duplicate infinite rows.

#### Scenario: Search inside a collapsed section

- **WHEN** an author searches for a matching descendant of a collapsed section
- **THEN** the result shows its meaningful ancestry and clearing search restores the earlier collapse choice

### Requirement: Guarded context navigation

Opening a hierarchy row SHALL use the existing save/recovery prerequisites and exact project-scoped handle. Failed navigation SHALL retain the current manuscript and recoverable drafts. Contentless or not-yet-initialized nodes SHALL not silently acquire unrelated text documents.

#### Scenario: Opening requires an unresolved recovery choice

- **WHEN** a selected document cannot be opened until its saved and draft state is resolved
- **THEN** no new project context, selection or unrelated document is published as a successful navigation
