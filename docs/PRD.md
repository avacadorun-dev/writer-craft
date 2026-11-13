# WriterCraft product requirements

Status: target product contract with an incremental native MVP. Updated 2026-11-07. Implemented capability is tracked separately in [feature parity](migration/FEATURE_PARITY.md) and change verification; this document is not a completion claim.

## Product and author

WriterCraft is an open-source desktop workspace for authors of long-form text. An author installs an application, opens a local project, plans, writes or dictates, reviews changes and exports their work. The principal adoption problem is requiring an ordinary writer to install and operate Docker, a database and object storage before writing.

Primary users: individual authors of long-form text, including writers who use dictation and AI-assisted revision. The native MVP ships eight interface languages now: Russian, English, German, French, Spanish, Portuguese, Chinese and Japanese (ru/en/de/fr/es/pt/zh/ja). Interface language does not translate a manuscript or imply matching transcription/model-language capability. The product runs on the author's computer and owns local working data. Remote transcription or AI is an explicit provider choice, separate from offline writing.

## Outcomes

- A first-time author creates and saves a project through the installed application without server setup or an account.
- A returning author finds saved text and recoverable unfinished work after restart.
- Suggested changes remain reviewable until the author applies them; conflicts preserve newer work.
- Existing authors can migrate supported projects without silently flattening their rich content or losing original audio, provenance and history.
- Light and Dark modes plus the requested non-green legacy palettes make the workspace comfortable for sustained writing.
- Authors can switch any of the eight built-in interface languages immediately and reopen with the same choice; Chinese/Japanese controls render correctly offline.

Proposed measures, pending observation: time from first launch to first saved paragraph; successful restart recovery; repeat weekly writing; migration round-trip failures; proposal rejection/conflict rates. No adoption, performance or retention numbers have been established yet.

## Product scope and milestones

| Milestone | User-visible commitment | Acceptance boundary |
|---|---|---|
| Native MVP | Local projects and plain prose, revisions, durable drafts, exact reviewed replacements, archive inspection, native writing interface, eight built-in UI languages, Light/Dark and six non-green palettes | Native launch and UI evidence plus named core/transfer tests; no rich parity claim |
| Reliable author migration | Complete supported .wstudio project copy with rich histories, annotations, assets, audio and context | Versions1–8 fixtures, checksums, valid references, crash-safe import and semantic round trip |
| Workflow parity | Native rich editor, project structures, Canvas, dictionary, guide/memory, dictation/transcript review, assistant threads/providers and publishing exports | Each legacy workflow mapped to native scenarios and passing acceptance |
| Distribution | First full desktop release on macOS, with update/recovery behavior; Windows/Linux desktop delivery follows later | Signed/notarized macOS where credentials are supplied; verified native launch, upgrade and recovery; portable core CI continues before later platform-specific UI/installer acceptance |

Implementation scope consists of existing Writers Studio workflows plus explicitly requested native distribution, appearance and localization changes. New domain capabilities need a separate product instruction.

Full parity remains the target. The plain prose MVP is an independently useful first delivery, with an explicit migration inventory instead of implied equivalence.

## Essential journeys

1. Create local project → create document → type/paste Cyrillic and emoji → autosave → restart → recover or continue.
2. Review document revisions → inspect earlier text → retain immutable history.
3. Select a unique exact phrase → prepare replacement → inspect before/after → Apply or Reject → guarded Undo.
4. Inspect an existing .wstudio archive → see source metadata and current document previews → understand unsupported migration features before importing.
5. Choose Light/Dark and a palette → continue writing → restart with the same appearance.
6. Choose an interface language → observe existing statuses and controls switch immediately → retain the current draft → restart in the chosen language.
7. At later milestones: plan on Canvas → dictate with recoverable original audio → review transcript → edit rich manuscript → request real provider assistance → review grouped changes → publish/export.

## Product invariants

Saved text, imported source and original audio are user-owned data. A network/provider failure must not erase writing. Unsupported rich content remains preserved and read-only until editing is lossless. A stale save, Apply or Undo never silently overwrites newer work. A preview is labelled as a preview. Deterministic tooling is labelled as local tooling. Credentials do not belong in project files or exports.

Manuscript writing uses continuous text within a bounded, locally scrolling editor pane. Page layout and pagination belong to publication preview/export.

The application's name is WriterCraft. It uses a coherent native workspace rather than browser navigation. Appearance has independent brightness and palette settings. Retained legacy palettes: cedar-and-copper, dried-rose-and-wine, fog-graphite, linen-clay, dusty-lavender, porcelain-ink. Green-accent quiet and olive-ink palettes are excluded by the current product instruction.

## Decisions and unresolved hypotheses

Accepted architecture and alternatives are in [DECISIONS](architecture/DECISIONS.md); observable contracts are in [OpenSpec](../openspec/specs/). Agent change discipline is in [SPEC_WORKFLOW](governance/SPEC_WORKFLOW.md).

Open hypotheses needing evidence: native installation improves activation; the selected Rust text stack can reach rich editor/IME/accessibility parity; long-form Russian dictation quality is acceptable; users can understand provider setup. These are research questions, not promises or reasons to discard working data paths.

## Owner corrections: desktop shell and preview distribution

Automatic engineering preview builds target macOS Apple silicon, Windows x64 and Linux x64 on GitHub. macOS remains first for complete interaction acceptance; a successful Windows/Linux build is not a desktop support or installer qualification. Portable previews retain the same source version and separate target/build identity.

The command bar and visible navigator/inspector share vertical boundaries. Long project and chapter titles truncate within allocated regions with full-title tooltips, never overlap commands. Settings use a permanent category sidebar, bounded content and fixed footer with the spacing contract in `docs/design/DESKTOP_UX.md`. The eight sections reflect existing Writers Studio scope; unavailable sections are informational until the actual legacy capability is migrated. No inactive switches may imply a working provider or MCP connection.

The current brand is the approved lion writer with a quill and book (#16), replacing the earlier monk identity. Preserve the supplied artwork; use separate full-tile toolbar/banner and padded OS-icon exports. Utility icons remain Lucide controls rather than substituting for the product mark.
