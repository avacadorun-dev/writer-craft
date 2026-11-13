# Alpha.3 milestone: authoring and manuscript delivery

Selected 2026-11-10. [GitHub milestone 0.1.0-alpha.3](https://github.com/avacadorun-dev/writer-craft/milestone/2) is the live task list. This latest scope supersedes earlier scheduling of complete project recovery or full parity as alpha.3 release prerequisites. It is a plan, not an implementation or acceptance claim.

## Delivered authoring baseline

The twelve document/hierarchy/reference/markup/review/save-reopen scenarios were manually passed on `15bc85a16149c7161f07d45f123bd16830693ee4`; see the [exact acceptance record](../verification/ALPHA3_OWNER_ACCEPTANCE_15BC85A.md). The substantial manuscript was authored through native UI. Preserve that evidence and owner-used project; do not recreate it, upload private edits or repeat the slow automated twelve-case matrix. The selected #14 slice is delivered; its broad issue remains open.

## Remaining release work

| Priority | Issue | Concrete result |
|---|---|---|
| Required | [#7](https://github.com/avacadorun-dev/writer-craft/issues/7) | Existing formula/Mermaid source editing by double-click, exact object context menu and keyboard, with guarded Apply/Cancel/history/reopen and compact chrome. |
| Required | [#18](https://github.com/avacadorun-dev/writer-craft/issues/18) | Responsive home/library with truthful project continuation, compatible themes and optimized artwork. |
| Required | [#20](https://github.com/avacadorun-dev/writer-craft/issues/20) | One selected manuscript compiled in tree order into UTF-8 Markdown, plus portable relative assets. Reuse existing Rust Markdown support. |
| Required | [#19](https://github.com/avacadorun-dev/writer-craft/issues/19) | The same ordered selection exported as a real editable DOCX, with an explicit supported-content profile and qualified output. |
| Stretch, nonblocking | [#21](https://github.com/avacadorun-dev/writer-craft/issues/21) | Reflowable EPUB 3 from the shared manuscript plan, only after backend, conformance and reader qualification. Move the still-open issue out of this milestone at freeze if unready. |

Work one product issue at a time: #7 → #18 → #20 → #19, then assess #21. Preserve existing TXT export. Share selection/order/revision/asset planning across formats; do not build separate conflicting manuscript trees. No deadline is asserted.

## End-to-end delivery scenario

An author opens the organized story, creates/revises/reorders chapters and scenes, consults a character motivation and returns to the exact text, then finishes marked corrections. The author chooses a manuscript document/subtree, reviews actual included chapters and their order, and exports one Markdown document or DOCX to a chosen destination. Character research, notes, labels, Trash, history and assistant context are not included implicitly. Opening the real exported artifact in an independent reader completes delivery. The project, rich source, revisions and original assets remain intact.

An output profile must state supported headings, paragraphs, marks, lists, links, tables and images, plus explicit fallback/refusal for formula/Mermaid/unknown content. Reader exports are not lossless project backups. Missing resources, stale snapshots, cancellation and failed writes cannot produce false success or overwrite unrelated files.

## Rust backend qualification

- DOCX candidate: [docx-rs 0.4.22](https://docs.rs/docx-rs/0.4.22/docx_rs/), MIT, using its native Rust API. Qualification and an exact dependency lock precede adoption; no app dependency has been added by this plan.
- Markdown: reuse `studio-transfer` rendering and safe asset paths before introducing a library.
- EPUB candidate: [epub-builder 0.8.3](https://docs.rs/epub-builder/0.8.3/epub_builder/), MPL-2.0, with the Rust `ZipLibrary` backend and system-command backend disabled. WriterCraft must still generate valid XHTML/CSS. Use official [EPUBCheck](https://github.com/w3c/epubcheck) in GitHub CI and real reader checks if shipping it.

Library availability is feasibility evidence, not a guarantee of format fidelity. The installed app must not require Chromium, Node, Pandoc, Word, LibreOffice or external ZIP tools to generate these formats. Independent reader applications and CI-only validators are verification tools. PDF is not selected for alpha.3.

## Separate acceptance gate

`docs/design/UX_GATE.json` remains **pending**, scope `structured-writing-alpha`. The updated A3-HANDOFF-01 covers mandatory final-manuscript delivery in #19/#20, not complete project restoration. Before release:

1. Finish each required issue and its actual acceptance checks; retain broad unfinished criteria without partial closure.
2. Qualify the exact committed candidate through GitHub checks and native installers, preserving source/lock/artifact identity and stating actual platform/signing limits.
3. Export the substantial UI-authored manuscript through native controls. Inspect the actual DOCX in a named/versioned independent word processor without repair warnings; inspect Markdown and moved relative assets in an independent reader. Verify targeted failure/cancel/source-preservation behavior and eight UI locales. Existing hosted checks plus focused regressions suffice; do not add an exhaustive matrix.
4. Obtain concise owner checks of changed features/output and independent review of the same completed candidate. Attribute the existing twelve-case owner report honestly; it is not invented agent observation or automatic acceptance of later source.
5. Bind the named gate to exact matching source and evidence. Only then publish packages/release notes/screenshots for the actual scope. Existing alpha.3 publication authorization remains valid; no repeated publication question is required.

## Outside this release gate

Keep [#17](https://github.com/avacadorun-dev/writer-craft/issues/17) complete Canvas, [#4](https://github.com/avacadorun-dev/writer-craft/issues/4) full editor parity, [#10](https://github.com/avacadorun-dev/writer-craft/issues/10) full original-project compatibility, unfinished broad [#14](https://github.com/avacadorun-dev/writer-craft/issues/14) and [#11](https://github.com/avacadorun-dev/writer-craft/issues/11) complete portable-project/storage/canonical recovery open outside this milestone. Preserve prepared work and criteria. The prior A3-HANDOFF-01 canonical restoration requirement is deferred to #11, not satisfied by DOCX/Markdown. Existing canonical preservation/label round-trip invariants still apply to changes that touch them.

No new Canvas development or unqualified integration. Resume only after the separate library's exact SHA/version handoff, migration notes and native evidence. Closed historical milestone tasks, including the earlier #15 Canvas slice, remain as delivered history; they do not expand this release. Never name this slice `full-editor-parity`. Preserve published alpha.1/alpha.2 tags/assets, all owner-used previews/projects and the prepared unverified #10 test diff.
