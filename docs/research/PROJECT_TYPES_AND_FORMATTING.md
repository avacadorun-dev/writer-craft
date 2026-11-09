# Meaningful project types and manuscript formatting

Status: active discovery for [issue #14](https://github.com/avacadorun-dev/writer-craft/issues/14), started 2026-11-08. This records initial evidence and proposed scenarios, not completed user research, an accepted taxonomy or implemented capabilities. The prior seven project kinds are a compatibility inventory; a new creation model must earn its distinctions through observable user tasks. Native alpha.3 engineering continues independently of unselected research additions.

## Research questions

1. Which distinctions change the author's actual work, rather than only names/icons/folders?
2. Which belong to a project workflow, reusable starting template, document editing mode, metadata or output profile? Can one project contain multiple document modes?
3. What do screenwriters, prose writers, nonfiction writers and editors need at drafting, restructuring, review and handoff stages?
4. Which needs are already source-backed migration obligations; which are new researched candidates; what direct evidence supports each?
5. How do old Studio kinds/custom schemas map into a refined model without rewriting or losing their data?

## Initial primary-source comparison

Sources checked 2026-11-08. Product documentation establishes documented behavior, not demand prevalence or proof that WriterCraft should clone it. No competitor application was operated for this initial comparison.

| Product | Documented comparison anchor | Research implication to test |
|---|---|---|
| [STARC](https://starc.app/download/) | Distinct editing modules for text, treatment, screenplay, comics, plays and novels; character/location/world support is described separately. | Test whether workflow modes need real editor semantics and linked domain views rather than a project label. |
| [Final Draft](https://kb.finaldraft.com/hc/en-us/articles/27613546469652-How-do-I-use-the-Outline-Editor) | Connected beats, customizable outline lanes, scene navigation and script-page planning. | Evaluate planning-to-script relationships, while keeping planning goals distinct from manuscript pagination. |
| [WriterDuet](https://www.writerduet.com/article/234-write-offline) | Local/offline and server-sync states are distinguished in documented writing workflows. | Distinguish saved local writing, pending sync and editorial collaboration instead of deriving behavior from a genre label. |
| [WriterSolo](https://writersolo.com/) | Official application entry point identified; its dynamic page supplied no readable feature documentation in this pass. | Evaluate separately from WriterDuet. Do not assume their collaboration/offline models are identical; primary verification remains pending. |
| [Fade In](https://www.fadeinpro.com/page.pl?content=features) | Screenplay element transitions while typing, character/location completion, nested navigation and rewrite/revision tools. | Test a concrete scene-writing and revision workflow, not a folder named Screenplay. |
| [Celtx](https://support.celtx.com/hc/en-us/articles/216076228-Storyboard) | Storyboard sequences can link to script scenes with explicit synchronization and return navigation. | Determine whether visual planning is a linked view of narrative data or an independent board; do not imply automatic mutation. |
| [Scrivener](https://scrivener.tenderapp.com/help/kb/features-and-usage/project-and-document-templates) | Customizable project/document templates seed reusable organization and reference material. [Default template details](https://www.literatureandlatte.com/blog/understanding-scriveners-default-templates-what-each-one-contains) also distinguish metadata/compile choices. | Test flexible templates and output configuration separately from immutable project kinds. |
| [Obsidian Canvas](https://help.obsidian.md/plugins/canvas) and [properties](https://help.obsidian.md/Editing+and+formatting/Properties) | Canvas connects notes/media; note properties carry typed metadata. | Test lore, dictionary and source relationships; distinguish built-in behavior from community extensions. |
| [FocusWriter](https://github.com/gottcode/focuswriter) | Focused drafting, themes, goals, live statistics and session continuation. | Evaluate a cross-project writing mode, rather than inventing a Focus project kind. |
| [Google Docs suggestions](https://support.google.com/docs/answer/6033474?hl=en) | Suggestions can be reviewed without immediately replacing original text, then accepted/rejected. | Validate editor/author review journeys independently of the manuscript's genre. |
| [Word sizing](https://support.microsoft.com/en-us/word/resize-a-table-column-or-row) and [Scrivener images](https://www.literatureandlatte.com/blog/working-with-images-in-scrivener-projects) | Direct/exact table sizing and proportional image dimensions give concrete formatting comparisons. | Define object size, workspace zoom and output size explicitly; preserve assets and legacy dimensions. |

The initial first-hand image/table requests linked in issue #14 are limited historical evidence. Broader persona/task validation, contrary evidence, direct native evaluation and prioritization remain open. Pricing, market-share claims and marketing language are not functional acceptance criteria.

## Legacy kind hypotheses

These are questions, not a decision to create new features or remove old kinds.

| Legacy kind | Hypothesis to validate | Evidence needed |
|---|---|---|
| `novel` | Chapter/scene restructuring, narrative references and delivery profiles form a coherent long-manuscript workflow. | Actual drafting/reordering/review/handoff scenarios and useful defaults without forced structure. |
| `story` | It may be a lighter starting template sharing the novel workflow, or need distinct collection/submission behavior. | A task-based reason for a separate type; compare single story and collection tasks. |
| `screenplay` | Professional usefulness depends on script element behavior, navigation and handoff, beyond acts/scenes folders. | Concrete scene/dialogue/rewrite/output journeys across the screenplay tools above. |
| `nonfiction` | Research organization, source relationships, illustrated content and output needs may justify a distinct starting workflow. | Relevant writer/editor tasks; distinguish actual requirements from speculative citation/publishing features. |
| `game_design` | General game-design documentation and interactive narrative may be different jobs. | Evidence from narrative writers and relevant professional tools before selecting either interpretation. |
| `free` | A minimal flexible starting point may serve several audiences without genre constraints. | A complete first-session task and progressive discovery, with no hidden essential tools. |
| `custom` | Reusable structures/profiles may be more useful than a nominal custom type. | Create/reuse/change a schema/template while preserving existing projects and references. |

## Scenario record contract

Every scenario has a stable ID, status, role/job, evidence/hypothesis, context and preconditions, trigger, main steps, observable outcome, alternatives, failure/cancel/stale/recovery paths, compatibility cases, related requirements/issues, semantic tests and exact compiled native/UX evidence. Do not turn a written scenario into a passed test. A proposed scenario may be refined or rejected without erasing its decision history.

## Initial proposed scenario catalog

| ID | Role and task | Main observable journey | Alternatives/failure/compatibility | Traceability/status |
|---|---|---|---|---|
| PT-NOV-01 | Prose writer restructures a long draft | Create or import chapters/scenes; move a scene to another chapter; continue editing at its exact passage; save/reopen. | Cancel move; refuse cycles/incompatible placement; retain notes, dictionary/source links, IDs and history; old custom nesting survives. | Existing hierarchy migration #10 plus template hypothesis; proposed native acceptance. |
| PT-SCR-01 | Screenwriter drafts and revises a dialogue scene | Start a scene, enter headings/action/character/dialogue using predictable keyboard behavior; navigate/reorder; hand off a correctly formatted selected output. | Undo transitions; preserve dialogue/source metadata; distinguish draft/rewrite versus production-specific behavior; test old screenplay projects. | New-profile research #14; exact element/output scope not selected. |
| PT-EDIT-01 | Editor proposes a change and the writer decides | Mark the second repeated passage, attach a note, prepare an exact proposal, inspect, reject or Apply and guarded Undo. | Formatting-only staleness; detached notes; concurrent revisions; preserve later note text; save/reopen. | Existing #2/#4; core/controller evidence does not yet establish full native acceptance. |
| PT-NONF-01 | Nonfiction writer organizes research and an illustrated chapter | Link a source/reference, insert a captioned image and explanatory table, set readable dimensions, reopen and export with real artifacts. | Missing resource, narrow window, resize cancellation, unsupported reader-export semantics; preserve original media and Studio data. | Existing #7/#10/#11 plus researched formatting #14; proposed workflow. |
| PT-KNOW-01 | Writer checks lore and project context while revising | Follow a term link to its dictionary record; inspect relevant scoped memory/guide material; return to the exact passage. | Renamed/deleted term, inactive memory, stale updates, detached note; preserve snapshots and history instead of guessing links. | Existing #4/#10; native domain workflows still incomplete. |
| PT-GAME-01 | Narrative writer distinguishes story planning from design documentation | Organize a narrative branch/scene reference or a system-design section, then verify the chosen project profile supports the actual task. | A branch diagram alone does not prove interactive execution; no scripting/game-runtime feature is implied. | Discovery #14; audience/problem validation pending. |
| PT-CUSTOM-01 | Writer reuses a familiar project setup | Define a useful allowed hierarchy/reference setup, create another project from it, then customize that project independently. | Invalid child rule, schema change affecting live nodes, cancelled creation; original custom projects continue to open/edit. | Existing custom-structure compatibility #10; reusable-profile improvements await research selection. |
| PT-FOCUS-01 | Writer continues a daily session without distractions | Open the last passage, enter bounded Focus mode, write, save and return to structure/review without losing position. | Compact window, different locale/theme, failure acknowledgement and restart recovery. | Existing #4/#6/#9; cross-project mode, not a proposed extra project type. |
| PT-MIG-01 | Existing Studio user opens an old project | Import each supported kind and a custom structure with notes/guide/memory/media; inspect/edit/save/reopen; export and restore independently. | Unknown optional fields, versions1–8, missing/corrupt assets, duplicate import and interrupted writes leave original/private data intact. | Mandatory #10/#11; current import/Canvas tests are partial evidence only. |

### Worked scenario PT-NOV-01

- **Status/role:** proposed native acceptance; writer revising a chapter-based manuscript.
- **Starting state:** a synthetic imported Studio project with two chapters, three scenes, a note on a repeated quote, a dictionary link and a source reference. All IDs, original JSON/assets and baseline histories are captured.
- **Trigger:** the writer decides that the second scene belongs in the other chapter.
- **Main path:** select the scene in the hierarchy; choose/drag to a schema-valid position; review its new location; open the same scene; inspect its marked passage/references; edit and save; restart and reopen.
- **Success:** one authoritative node identity remains, the hierarchy/order is correct, writing/marks/links/history are retained and the same scene remains reachable. Document order in supported output follows the chosen structure.
- **Alternatives:** keyboard move and pointer move use the same validated command; cancel leaves the original order unchanged; a new project seeded from a template exercises the same commands.
- **Failures:** a cycle, forbidden parent type, stale structure revision or failed transaction refuses before partial mutation. If unsaved work blocks navigation, it remains recoverable and visible.
- **Compatibility:** legacy/custom kinds retain raw metadata and their valid structure; changing a creation profile never silently reclassifies an imported project.
- **Required evidence:** schema/move/atomicity and exact preservation tests; actual native pointer/keyboard/cancel/reopen record; independent hierarchy/spacing/focus review; links to the exact source/build in #10. No such full native acceptance is claimed by this document.

## Decision and implementation gate

Produce a justified profile model and prioritized capability set, with contrary evidence and open questions. Map retained old kinds/custom structures explicitly. Selected capabilities require precise OpenSpec deltas/design/tasks before implementation; native UX and semantic preservation checks must match the final source. Generic editor/Canvas/media mechanics remain external engines; business project profiles, dictionary, guide and memory belong in WriterCraft. No selector is accepted from labels or screenshots alone.

## Read-only source identity

The source application working tree is not pristine. These hashes identify the exact relevant product contracts inspected; no manuscript/archive/media is included.

| Source path | SHA-256 |
|---|---|
| `src/entities/project/model/types.ts` | `4a68cfac444a6b491e1659788aa795b85fad973aac31de9d237c7b0830b96bb6` |
| `internal/store/structure.go` | `6a2c934f79411ab3f679f1dbe28daa2b98b15864d89b788f890ad5979de757d9` |
| `src/widgets/project-tree/model/project-outline.ts` | `a0d3c2f8e697077875d48e1d62271a8120f3326d641a68ff2a9368f76fc75b54` |
| `src/entities/assistant/model/types.ts` | `ed65db65bc9066ca946e977301207f189aafc202cb72ff296f2ecad26e505cff` |
| `src/pages/assistant-context/model/memory.ts` | `2368c30b6aca35ebf0f0554af83c7e7637fdc0bd0dc4f2a4861a34464ba9be28` |
| `internal/store/dictionary.go` | `d1676948720c368fa0d8a35e6d07c93c5c09173d41dbbaf5b7420cef884caf45` |


## Connected boards, manuscripts and narrative knowledge — 2026-11-09

This requested extension is tracked in #14/F-43. Status: initial primary-source and read-only source audit; hypotheses and scenario drafts remain unvalidated. No competitor application was operated and no new automatic synchronization behavior is selected here.

### Existing Studio baseline

Read-only source shows `CanvasResourceRef` supports document/media/canvas IDs; cards can contain typed document/media/canvas fields and `promotedDocumentId`. `CanvasDocumentCard` displays the current document title/status/plain-text excerpt and document cards carry `writers-studio://documents/<id>` plus the typed resource ID. The board exposes `onOpenDocument` and guarded promotion of a saved card into a document; `internal/store/canvas_promotions.go` checks the base revision and writes document and Canvas revisions transactionally. These facts establish an existing link/promotion baseline, not a complete character/lore synchronization contract or observed native parity.

Source hashes in the original working tree (read-only, 2026-11-09):

| Source path | SHA-256 |
|---|---|
| `src/widgets/canvas-board/ui/canvas-board.tsx` | `be67e4c9d97a490df4053844e1cc317ce77d7098c8383299f5a3940300940d87` |
| `src/entities/canvas/model/types.ts` | `af9c5181b6eb459d13c78e4832974476f3a9a8cabeb451668a189b7eade30e93` |

### Primary comparison evidence

Checked 2026-11-09; documentation describes vendor behavior, not observed WriterCraft demand or acceptance.

| Product/source | Documented behavior | Hypothesis and boundary |
|---|---|---|
| [Final Draft Outline Editor](https://kb.finaldraft.com/hc/en-us/articles/27613546469652-How-do-I-use-the-Outline-Editor) | Beats appear on both board and outline; editing a beat updates those representations. A separate script lane exposes written scenes; scene navigation and explicit Send to Script are described. | Distinguish shared planning records from copying an outline into prose. Do not infer universal bidirectional manuscript synchronization. |
| [Celtx Storyboard](https://support.celtx.com/hc/en-us/articles/216076228-Storyboard) | Explicit Sync Script adds scene-linked sequences and updates their headings; linked sequences navigate to the corresponding script scene. Independent sequences are also supported. | Evaluate author-controlled refresh and clear live/independent states. This does not establish that a visual reorder edits script order. |
| [STARC character relationship map](https://old.starc.app/help/characters-relations) | The map exposes character profile and relationship panels, per-character viewpoints, grouping and focus. | Evaluate profile identity, directional relationship meaning and contextual editing; do not mistake map geometry for story events. |
| [Obsidian Canvas](https://help.obsidian.md/plugins/canvas) | Distinguishes independent text cards from existing note-file cards and supports narrowing a note to a heading/block. Text cards can be converted to files; their backlink behavior differs. | Test canonical references versus independent writing and explicit conversion; filenames/labels alone are not sufficient persistent identity. |

### Relationship models to test

| Hypothesis | User benefit to test | Failure or contrary case | Decision still needed |
|---|---|---|---|
| H-LINK-1: one canonical record, several views | A character/profile or beat edited from either surface is consistent everywhere. | An author expects a private planning variant or historical state; shared mutation surprises them. | Which entities are shared and where editing is allowed; exact refresh and Undo authority. |
| H-LINK-2: live stable reference | A card follows a scene/character/lore record without duplicating its text. | Target is deleted, access scope changes or a passage anchor detaches. | Target granularity, backlinks, missing-target state and exact return context. |
| H-LINK-3: revision-pinned snapshot/excerpt | A writer preserves a review state or research quotation while the source evolves. | It appears current after the original changed. | Visible provenance/revision, explicit refresh/replace and conflict behavior. |
| H-LINK-4: independent note | Free planning remains independent of the current draft. | The author assumes a matching name means automatic synchronization. | Clear conversion/link commands and what remains independently editable. |

These are candidate models, not four mandatory card types. Evaluate the smallest understandable design against real tasks. A plain Open document action establishes navigation only; it cannot by itself satisfy the requested connected-writing outcome.

### Proposed relationship scenario records

All four are draft research scenarios, with no native pass recorded. Public fixtures use synthetic scenes, characters and rules. Trace each selected path to #14 → this OpenSpec change → implementation → semantic tests → exact compiled independent review.

**PT-LINK-01 — plan a beat, write its scene and return.** Role/job: screenwriter connects visual planning to daily scene drafting. Start: a saved project has a board, a beat card, a manuscript scene and two identical passage quotes. Trigger: associate/open the intended scene or exact passage. Main: make the relationship explicit, open the intended target, revise/save, reveal its board context, return with the original board pan/zoom/selection and manuscript caret/scroll. Outcome: stable intended identity, understandable edit effect, no divergent copy, no change to scene order from layout alone. Alternatives: create a scene from a free note or link an existing one under the chosen model. Cancel leaves both canonical records unchanged; stale target or failed save preserves local work and gives an actionable state. Old Studio document cards/promotion, rename/move and export/restore must preserve meaning. Proposed evidence: exact second-quote navigation, semantic layout-only no-mutation test, save/reopen and independent pointer/keyboard return journey.

**PT-CHAR-01 — inspect a character and relevant scenes.** Role/job: writer checks a character's profile and behavior while revising dialogue. Start: one identified character record, two scene references and a board card with an independent planning note. Main: open the profile from the card, inspect scene/behavior references, edit the selected canonical field or explicitly independent note, return to the same scene/passage. Outcome: the author knows which record changed and sees consistent selected views without losing the independent note. Alternatives: a revision snapshot is deliberately retained instead of refreshed. Cancel/stale edits preserve the saved profile and draft; deleted or out-of-project targets are not guessed from names. Migration/export retain existing dictionary/card metadata and original media. Proposed evidence: shared-versus-independent update assertions, duplicate-name target test, actual contextual native editing and return navigation. Profile field set and behavior-note semantics remain unselected.

**PT-LORE-01 — verify a world rule while writing.** Role/job: writer checks consistency without abandoning a passage. Start: a manuscript has an explicit reference to one world rule and a related planning board; the same term appears elsewhere. Main: follow the exact reference, inspect its source/provenance and related knowledge/card, choose the selected allowed edit/refresh action, then return to the original passage. Outcome: known canonical/snapshot authority, retained writing context and no inference that inactive memory is an instruction. Alternatives: a standalone note stays standalone; an unknown imported reference remains visible as preserved evidence. Missing/stale target, provider proposal, cancel and failed save never overwrite prose or silently retarget by quote/title. Proposed evidence: exact scope/identity tests, annotation-preserving updates, export/restore reference checks and native keyboard/accessible navigation. Any automatic relevance or consistency suggestion needs separately selected scope.

**PT-LINK-02 — maintain relationships through lifecycle and transfer.** Role/job: writer reorganizes a project while keeping connected planning usable. Start: linked scene/profile/rule records, an independent note, a pinned snapshot and an old custom Studio structure. Main: rename/move/edit targets; trash/restore one; save/reopen; canonical export/restore into a fresh project; follow each relationship. Outcome: identities and chosen lifecycle survive, preserved snapshot revision stays explicit, no title/quote guessing or silent divergent text. Alternatives: remove a card without deleting its canonical target, or explicitly unlink while retaining independent text. Cancel/stale/failed operations leave prior canonical state/history intact; unavailable references remain understandable and recoverable. Proposed evidence: transactional lifecycle/Undo tests, old-format and unknown-field/media preservation, repeated transfer stability and independent actual native lifecycle review. Permanent-deletion/reference policy must be decided before implementation.

## Operational task and evidence decision — 2026-11-09

Use the existing PT IDs in `docs/design/ROLE_JOURNEYS.md` for complete cross-surface acceptance, alongside the separate exhaustive capability inventory. Add a per-finding evidence kind (direct request / observed session / first-hand report / vendor documentation / design inference), source/date, falsifiable benefit hypothesis, contrary case, selection decision, test criterion and observed outcome. Do not relabel a proposed story as a completed user session. Existing comparison references remain initial evidence; no new interviews or representative demand estimates are claimed.

Source baseline, explicitly requested enhancement and unselected hypothesis are separate statuses. H-LINK alternatives, specialized screenplay elements and creation-profile refinements remain pending decisions until researched and selected; source document references/promotion/types remain mandatory baseline #17. Research informs how a workflow becomes coherent without deleting its required abilities. #14 acceptance needs the justified choices and verified selected implementation, not only this document.


## Kind audit verdicts and legacy mapping — 2026-11-09 decision record

Each original kind is classified by its real audience/task distinction and mapped to the minimal mechanism that preserves it. Verdicts are evidence-labeled: [vendor documentation], [design inference], [first-hand report] from the comparison table above; no hands-on competitor evaluation is claimed.

| Original kind | Distinction it carries | Verdict | Mechanism |
|---|---|---|---|
| `novel` | Long-form prose, chapter hierarchy, revision milestones | Backward-compatibility data only | Import whitelist + verbatim manifest preservation; native creation stays kindless |
| `story` | Shorter prose, fewer structural expectations | Backward-compatibility data only | Same as `novel`; no separate behavior in the native app |
| `screenplay` | Scene/element semantics, pagination by dialogue | Unselected hypothesis for alpha.3 | Element mode is not implemented; imported structure preserved as data |
| `nonfiction` | Section/reference-oriented hierarchy | Backward-compatibility data only | Same import-preservation contract |
| `game_design` | Entity/state/quest records alongside prose | Baseline via board/document types (owned by #17) | Existing card/relation types and promotion; no new entity engine |
| `free` | No imposed structure | Current native creation default | Native projects export as `kind:"free"`, flat `document` hierarchy |
| `custom` (+ custom schemas) | Owner-defined types/fields | Baseline, preserved losslessly | Unknown fields/types round-trip; schema extension requires explicit compatible migration |

Decision: the seven keys are an import-compatibility inventory, not a validated creation taxonomy. Native project creation remains kindless; no reclassification happens on export; original manifests travel verbatim through `.wstudio` v1–8 and canonical format-2. A creation selector is an unselected hypothesis — adding one is a product decision requiring its own evidence, not proof of professional workflow completeness.

## Baseline / requested / unselected classification

- **Baseline (mandatory, tracked elsewhere):** seven-kind import contract above; existing document cards, references, promotion, rename/move and export/restore semantics (source document references and promotion belong to #17's board baseline).
- **Requested enhancement (selected for alpha.3):** direct object sizing — image/visual-block exact-size with aspect lock and table column resize — to be implemented as guarded commands on the already-round-tripped `width`/`height`/`colwidth`/`align` attributes; no schema extension, so the v1–8 and editor-schema 1–4 contracts stay untouched. Connected-writing relationship scenarios PT-LINK-01/02, PT-CHAR-01, PT-LORE-01 remain draft research records; only navigation semantics are baseline, synchronization models stay unselected.
- **Unselected hypothesis (alpha.4 material):** creation taxonomy selector; screenplay element mode; per-kind output/compile profiles; automatic board↔document/character/lore synchronization (H-LINK model choice beyond navigation); competitor-app hands-on evaluation; persona/task validation.

## Formatting needs matrix (selected scope)

| Capability | Evidence kind | Source | Selected? |
|---|---|---|---|
| Image exact width/height with aspect lock | Vendor documentation | Word resize docs; Scrivener image handling | Yes — alpha.3 |
| Table column resize (drag + exact-size keyboard entry) | Vendor documentation + observed native parity need | Original editor colwidth support | Yes — alpha.3 |
| One-gesture Undo, cancel/stale/read-only safety, durable save/reopen | Design inference from shared command rules | Existing guarded-command contract | Yes — acceptance criteria |
| Screenplay element pagination | Unselected | — | No |
| Compile/output profiles | Unselected | — | No |

Implementation boundary: sizing commands reuse existing canonical attributes (`colwidth`, image `width`/`height`); unknown-field and original-media preservation is inherited from the round-trip suites; no `.wstudio`/schema version change is introduced.
