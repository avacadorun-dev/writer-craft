# Compiled Markdown tasks

- [x] Record the ME-01..ME-06 delta and reuse decision before implementation (`proposal.md`, `specs/manuscript-export`, `design.md`).
- [x] Add `ManuscriptSelection`, tree-order plan, chapter/folder headings and pre-export disclosures in `studio-transfer` (`compiled_markdown.rs`, source only).
- [x] Compile one ordered document with heading offsets and in-document document anchors through the existing renderer (`heading_offset`/`anchors` inputs).
- [x] Copy only referenced original media into a deterministic adjacent folder with read-back verification and relative links (`referenced_assets_should_be_copied_adjacently_with_relative_links_and_matching_bytes`).
- [x] Enforce exclusive destination, incomplete-marker completion, cancellation and concurrent-snapshot refusal (`existing_destination_should_be_refused_without_overwriting`, `cancelled_compilation_should_not_create_the_destination_or_report_completion`, `moving_the_document_and_assets_should_preserve_resolvable_relative_links`).
- [ ] Expose the actual native export mode with scope, order preview and disclosure in eight catalogs behind the existing transfer worker. No UI is wired yet; the library API is not reachable from the app.
- [ ] Run focused tests, rustfmt, strict Clippy and workspace tests, then hosted Verify/Preview on the exact candidate. Local `cargo test -p studio-transfer --locked`, `cargo fmt --all` and strict Clippy passed; workspace/hosted runs are still pending.
- [ ] Observe the native-generated manuscript in an independent Markdown reader; check moving the `.md` plus assets, Unicode, links, tables, code and disclosed formulas/Mermaid.
- [ ] Obtain independent scoped review, hand the native artifact to the owner, and update #20/progress/handoff with exact source/build evidence only.
