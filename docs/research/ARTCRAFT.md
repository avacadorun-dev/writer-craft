# Artcraft research and reuse decisions

Research date: 2026-11-07. This is source inspection, not a claim that Artcraft binaries passed our acceptance tests. Upstream main branches change; record and test an immutable revision before adding an upstream dependency. No Artcraft application source or branding is vendored in this MVP.

## What the approach demonstrates

The [Artcraft site](https://getartcraft.com/) describes native Rust tools, local files, agent interfaces and desktop installers. [Craft Anything](https://getartcraft.com/apps/craft-anything) is an investigation/build workbench, not an automatic porting engine or a rich-text widget. [PhotoCraft](https://github.com/storytold/photocraft) demonstrates an application architecture and distribution approach; its image engine is not a manuscript foundation.

The useful common pattern is a domain model and command engine independent of UI, with UI, CLI and MCP using the same behavior. WriterCraft adopts that pattern in its own small core. A native app without WebView is feasible; it does not establish feature parity, document fidelity, or a native system-widget appearance.

## WordCraft: rich documents

[Workspace manifest](https://github.com/storytold/wordcraft/blob/main/Cargo.toml) identifies version 0.15.1, edition 2024, Rust 1.95 and egui/eframe 0.37. The [README architecture](https://github.com/storytold/wordcraft#architecture) separates document/font/proofing data, layout/formats, render/PDF, command engine, MCP and egui UI. [Engine source](https://github.com/storytold/wordcraft/blob/main/crates/engine/src/lib.rs) routes UI, shortcuts, CLI and MCP through a common registry. This is a strong reference for our command boundary.

The document model has paragraphs, formatting runs, tables, notes, comments and tracked revisions. See [doc source](https://github.com/storytold/wordcraft/blob/main/crates/doc/src/lib.rs). However the [engine dependencies](https://github.com/storytold/wordcraft/blob/main/crates/engine/Cargo.toml) include more than ten domain/render/format modules, proofing, fonts and native font-store integrations. It is not a small drop-in text component.

The [canvas](https://github.com/storytold/wordcraft/blob/main/crates/ui-egui/src/canvas.rs) paints pages and editing affordances itself. The [clipboard adapter](https://github.com/storytold/wordcraft/blob/main/crates/ui-egui/src/system_clipboard.rs) handles formatted HTML/RTF beyond egui's text paste. A custom [AccessKit adapter](https://github.com/storytold/wordcraft/blob/main/crates/ui-egui/src/a11y.rs) exposes the document drawn beneath the bitmap canvas. These are real costs hidden by an attractive screenshot.

WordCraft calls itself alpha. Its README's readiness percentages are the authors' estimates, not our benchmark. [ROADMAP](https://github.com/storytold/wordcraft/blob/main/ROADMAP.md) records unresolved native IME acceptance, file fidelity and stability work; historical gap summaries and newer progress entries must be checked together. We have not run those workflows. Decision: study the engine and evaluate an isolated adapter later; do not base the first editable manuscript on an unverified whole-application fork. Imported Writers Studio rich JSON is preserved and read-only in our current MVP.

## FigCraft: canvases

[Manifest](https://github.com/storytold/figcraft/blob/main/Cargo.toml): 0.15.0, Rust 1.95, MIT OR Apache-2.0. The [model](https://github.com/storytold/figcraft/blob/main/crates/doc/src/lib.rs) has stable node IDs and copy-on-write document snapshots. Its [engine](https://github.com/storytold/figcraft/blob/main/crates/engine/Cargo.toml) brings approximately fifteen app-domain crates including layout, rendering, collaboration, plugins and exports.

Decision: reference command dispatch, stable IDs and canvas interaction architecture. Do not introduce the Figma document model, collaboration stack or renderer to implement simple story cards. A full writer board and original Canvas migration are deferred; preserving source data does not mean providing an editable board.

## DesignCraft and DeckCraft

[DesignCraft manifest](https://github.com/storytold/designcraft/blob/main/Cargo.toml): 0.17.0, Rust 1.95, MIT OR Apache-2.0. Its [EPUB exporter](https://github.com/storytold/designcraft/blob/main/crates/epub/Cargo.toml) is relatively focused but accepts DesignCraft's own document/color model. Decision: possible export reference after a canonical book model exists; no dependency in this MVP.

[DeckCraft manifest](https://github.com/storytold/deckcraft/blob/main/Cargo.toml): 0.15.0, Rust 1.95, MIT OR Apache-2.0. Its [text layout](https://github.com/storytold/deckcraft/blob/main/crates/text/Cargo.toml) depends on the slide model, fonts, geometry and colors. Decision: no current benefit over the WordCraft reference for manuscript editing; do not import a slide engine.

## craft-libs: narrow reusable foundations

[craft-libs](https://github.com/storytold/craft-libs) is edition 2024, version 0.1.0, Rust 1.95, MIT OR Apache-2.0. [Its manifest](https://github.com/storytold/craft-libs/blob/main/Cargo.toml) patches the egui family to storytold/egui revision `6d518fc799681571963842f95b55a6b68431b15a`. Root Cargo patch settings are not automatically inherited by consumers. Mixing the suite's current helpers with a different egui release requires a build spike, not an assumption.

| Library | Useful boundary | Decision |
| --- | --- | --- |
| [craft-ui](https://github.com/storytold/craft-libs/blob/main/crates/ui/README.md) | SVG catalogs, accessible icon buttons, tabs, splitters, docking and theme mechanisms; application supplies icons, fonts and palette | Defer direct use while MVP uses published egui 0.36.2; use coherent local vector icons |
| [craft-atomic](https://github.com/storytold/craft-libs/blob/main/crates/atomic/README.md) | App-independent atomic file replacement, exclusive creation and failure injection | Best small candidate for future project/asset writes; SQLite currently owns transactional persistence |
| [craft-keymap](https://github.com/storytold/craft-libs/blob/main/crates/keymap/README.md) | Shortcut chords, scopes, conflict detection and overrides | Evaluate once native command catalog grows |
| [craft-font-catalog](https://github.com/storytold/craft-libs/blob/main/crates/font-catalog/README.md) | Platform font discovery and lookup | Evaluate for full rich-text font support |
| [craft-update](https://github.com/storytold/craft-libs/tree/main/crates/update) | Suite update behavior | Defer until our own release identity, signatures and channels exist |

`craft-ui` is not a bundled icon pack or complete design system. Its mandatory dependencies include fonts-build/i18n, egui/epaint and skrifa; native features add platform integration. `craft-atomic` is much smaller: its manifest lists only an Apple-specific rustix dependency. Its API exposes `atomic_write`, `write_new` and `WriteOptions`; private recovery/settings data must explicitly select private permissions. These are candidates, not incorporated dependencies.

Upstream code licenses permit reuse subject to their terms. Fonts, icons and other assets keep their own licenses; review [ATTRIBUTION](https://github.com/storytold/craft-libs/blob/main/ATTRIBUTION.md), [NOTICE](https://github.com/storytold/craft-libs/blob/main/NOTICE), and applicable license files before copying. No app trademark, screenshot or logo should become our product identity merely because source code is open.

## Rust engineering skill

The repo-local `.agents/skills/rust-best-practices` comes from [Apollo GraphQL skills](https://github.com/apollographql/skills/tree/main/skills/rust-best-practices), skill version 1.1.2, MIT. Its provenance is tracked in the repository. It supplies Rust ownership/error/lint/test/documentation guidance; domain data safety and migration contracts remain ours. This is an organization-published reference skill, not a Rust language standard or independent product certification.

## Required proof before deeper reuse

- Build the selected immutable upstream revision with our toolchain and target UI version.
- Retain representative Cyrillic, emoji, marks, tables, formulas, images, block IDs and source refs after repeated save/reopen.
- Verify IME, selection, clipboard, keyboard access and screen-reader behavior on actual target systems.
- Test a long manuscript and crash recovery; compare exported documents on a real corpus.
- Keep code reuse, opaque preservation and completed user workflows separately stated in release notes.
