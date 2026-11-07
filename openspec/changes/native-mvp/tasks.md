# Native MVP tasks and evidence

Status on 2026-11-07: core tests28/28 passed; core strict Clippy and own-file formatting passed. Other unchecked items require actual integration evidence. This change remains open.

## 1. Core writing and recovery

- [x] 1.1 Implement local library and revisioned prose commands (NW-01, ME-01).
- [x] 1.2 Persist revision-pinned working drafts and preserve newer differing recovery text (NW-02).
- [x] 1.3 Refuse foreign/newer databases and add schema1→2 migration without rewriting records (NW-03).
- [x] 1.4 Preserve supplied imported JSON read-only (ME-03).

## 2. Reviewed changes

- [x] 2.1 Stage unique exact replacements with Unicode-scalar anchors and snapshot integrity (ME-02, AR-01, AR-02).
- [x] 2.2 Implement atomic Apply/Reject and guarded compensating Undo (AR-03, AR-04).
- [ ] 2.3 Verify real desktop before/after review, save-error/conflict messages and truthful no-provider state (AR-05).

## 3. Native integration and appearance

- [ ] 3.1 Verify native launch, reopen, draft recovery, typing, paste, selection and bounded pane behavior (NW-01, NW-02, AP-03, AP-04).
- [ ] 3.2 Verify archive inspection limits/integrity and preview wording with named transfer tests (NW-04).
- [ ] 3.3 Implement and verify persisted Light/Dark plus six retained non-green palettes; check editor, panes, dialogs and focus in each pair (AP-01, AP-02, AP-04).
- [ ] 3.4 Run integrated workspace formatting, tests and strict Clippy; capture actual commands/results.
- [ ] 3.5 Record native screenshots and manual interaction results; mark distribution/platform limitations explicitly (NW-05).

## 4. Governance

- [x] 4.1 Add PRD, accepted ADRs, capability scenarios, native-mvp delta and this evidence map.
- [x] 4.2 Run official OpenSpec1.16.1 strict validation: five specs and one change valid, zero issues; see `docs/governance/OPENSPEC_VALIDATION.md`. CLI initialization/tool generation remains separate.
- [ ] 4.3 Archive only after the declared MVP scope is verified; carry full-parity work into named follow-up changes.

## Core evidence map

All names below refer to `crates/studio-core/tests/workspace.rs` and passed in the twenty-eight-test run. Command: `cargo test -p studio-core --locked`. Strict core check: `cargo clippy -p studio-core --all-targets --all-features --locked -- -D warnings`. The host run used a direct installed toolchain and `CARGO_PROFILE_DEV_STRIP=none` to avoid a host rust-objcopy library problem; this is not an application runtime dependency.

| Spec | Actual test names |
|---|---|
| NW-01 | `saved_documents_and_revisions_survive_reopening`, `invalid_creation_does_not_leave_an_orphan_document` |
| NW-02 | `working_draft_survives_restart_without_mutating_canonical_history`, `canonical_save_clears_only_its_matching_draft`, `canonical_save_does_not_delete_a_newer_differing_working_draft`, `stale_draft_write_does_not_overwrite_recovery_text`, `new_base_draft_cannot_overwrite_older_distinct_recovery_text`, `draft_recovery_creates_editable_copy_without_changing_original_and_survives_reopen`, `recovering_a_missing_draft_does_not_create_a_document`, `recovered_document_title_preserves_suffix_within_title_limit` |
| NW-03 | `opening_another_database_preserves_its_schema_and_content`, `schema_one_migration_adds_drafts_without_rewriting_existing_documents`, `newer_workspace_schema_is_refused_without_downgrading` |
| ME-01 | `stale_save_from_a_second_connection_cannot_overwrite_text`, `identical_save_does_not_create_spurious_revision` |
| ME-02 | `anchors_use_unicode_scalars_and_apply_exact_replacement` |
| ME-03 | `imported_rich_content_is_preserved_and_read_only_after_restart` |
| AR-01 | `staging_does_not_mutate_canonical_text_or_history`, `proposal_lifecycle_persists_after_restart` |
| AR-02 | `overlapping_exact_matches_are_ambiguous`, `selected_second_repeated_phrase_applies_and_undoes_exact_unicode_range`, `selected_invalid_scalar_ranges_stage_nothing_and_preserve_history`, `selected_literal_mismatch_stages_nothing_and_preserves_history`, `selected_stale_revision_stages_nothing_and_preserves_newer_work` |
| AR-03 | `stale_apply_changes_neither_document_history_nor_proposal`, `rejected_proposal_cannot_be_applied` |
| AR-04 | `undo_appends_a_compensating_revision_and_preserves_history`, `undo_refuses_to_overwrite_a_later_manual_edit` |

## 5. Eight-language interface delivery

- [ ] 5.1 Verify complete ru/en/de/fr/es/pt/zh/ja catalogs and matching message parameters with actual named tests (LO-01, LO-07).
- [ ] 5.2 Verify immediate language switch of controls, tooltips, accessibility labels and existing dynamic/error messages while retaining an unfinished draft (LO-02, LO-04, LO-06).
- [ ] 5.3 Verify restart persistence, missing preference defaults and visible invalid-preference recovery that preserves the original file (LO-03).
- [ ] 5.4 Verify packaged offline Chinese/Japanese glyphs, dialogs and long labels with native screenshots; audit missed hardcoded strings across all active views (LO-05, LO-07).

These localization checks remain pending until the integrated implementation and observed evidence are recorded; catalog size alone is not whole-interface acceptance.

## 6. Dependency quality update

- [x] 6.1 Verify official stable metadata, upgrade zip9.4.1/rusqlite0.41.1/rfd0.18.0, preserve the matching egui/eframe0.36.2 compatibility pin and update Cargo.lock.
- [x] 6.2 Run core24 + transfer11 tests and strict module Clippy after the upgrade;35 tests pass.
- [x] 6.3 Query all395 locked registry package/version entries against OSV, document remaining conditional-platform maintenance notices and macOS reverse-tree evidence in `docs/architecture/DEPENDENCY_AUDIT.md`.
- [ ] 6.4 Verify updated native file dialogs and the integrated localized macOS application before release.

## 7. Selected-occurrence parity correction

- [x] 7.1 Add the revision-pinned explicit Unicode range staging API with the same proposal transaction and snapshot checks, while retaining phrase-only uniqueness as a fallback (AR-02).
- [x] 7.2 Verify repeated second-occurrence Apply/Undo, invalid range/literal rejection and stale source no-mutation regressions: core28/28 tests and strict core Clippy passed.
- [ ] 7.3 Verify native selection rebinding only when the editor buffer still matches the captured source, including an unsaved selected edit and a repeated phrase.

## Owner corrections and hosted previews

- [x] Add hosted engineering preview jobs for macOS ARM64, Windows x64 and Linux x64; retain macOS-first interaction qualification.
- [x] Align shell dividers and constrain long titles to their allocated areas.
- [x] Split preferences into eight legacy-backed sections with bounded content and fixed actions.
- [x] Defer manuscript focus until its accessibility node exists after document creation.
- [ ] Independently recheck shell/settings/brand captures and native creation/review interactions.
- [ ] Verify the first actual hosted preview artifacts on all three platforms.
