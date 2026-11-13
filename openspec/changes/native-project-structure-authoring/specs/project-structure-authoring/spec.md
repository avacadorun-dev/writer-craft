## ADDED Requirements

### Requirement: Saved structure determines creation

Root and child creation in an imported project SHALL use its saved custom type and placement rules, not an unrelated default profile. A stale structure revision, invalid parent/type/title or exceeded source bound SHALL refuse before canonical mutation. Content-bearing types SHALL receive one valid initial rich revision; structural types SHALL retain a stable identity without invented text.

#### Scenario: Create a custom section and its allowed child

- **WHEN** an author creates a contentless section and then a supported content-bearing child using the current structure
- **THEN** both appear at the selected hierarchy locations with scoped stable identities, only the child owns revision1, and existing text, history and unknown schema fields remain unchanged

#### Scenario: Placement or structure is no longer valid

- **WHEN** a creation request uses an older structure revision, a foreign/trashed parent or a disallowed root/child type
- **THEN** no node, alias, manuscript or revision is partially inserted and the author can refresh the current structure

### Requirement: Exact first-content identity

Initializing an imported content-bearing node without saved text SHALL validate its exact current metadata and reuse its reserved scoped native alias. Contentless, trashed, stale or repeated initialization SHALL not create an unrelated duplicate document.

#### Scenario: Write first content after import

- **WHEN** a current content-bearing node at head0 receives its initial manuscript
- **THEN** its original logical identity and existing references remain valid, its native text head becomes1 atomically and the original imported source is retained unchanged

### Requirement: Compatible structural preservation

Known structural/head changes SHALL preserve original source bytes, unknown fields and unrelated project domains. Native canonical export/restore SHALL retain new nodes and first-content heads with fresh isolated native handles; supported legacy reading export SHALL remain valid without claiming unknown editing semantics.

#### Scenario: Restore a project after structure authoring

- **WHEN** the author exports and restores a project after creating a custom child and initializing an old head0 node
- **THEN** the current hierarchy and saved text heads survive, scoped aliases resolve only in the restored namespace and the old immutable source baseline stays available without nested copies

### Requirement: Creation prerequisites and aggregate acknowledgement

Project and document forms SHALL check current save/navigation prerequisites before inserting any record. Creating a new native project and its first manuscript/history SHALL be atomic. A refusal SHALL retain form input without duplicate records; a completed creation receipt SHALL close the form even if later view navigation fails.

#### Scenario: Create while a card cannot be left

- **WHEN** an author submits either creation form while a card is dirty, composing, pending or failed
- **THEN** creation refuses before mutation, retains the current card and form fields, and a clean retry creates exactly one intended record

#### Scenario: The first manuscript history cannot be inserted

- **WHEN** a project creation fails after its project row was inserted but before all initial manuscript history is saved
- **THEN** the transaction rolls back the entire new aggregate and the previous library remains unchanged
