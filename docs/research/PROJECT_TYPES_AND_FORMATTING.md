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
