# Settings and storage recovery UX checkpoint

Status: source implementation for independent findings A2-02, A2-03, A2-04 and A2-06 from [the alpha.2 review](../design/UX_REVIEW_ALPHA2_2026-11-08.md). Hosted compilation, semantic execution and mounted native retest remain pending. This is not an acceptance or issue-closure record.

## Explicit backup prerequisites

Starting a backup flushes the existing manuscript/title path, but never silently saves or discards authored note drafts. Changed notes and paused new-note composers across projects produce a bounded prerequisite dialog. It names affected documents and projects, preserves long names through truncation/tooltips, and offers an explicit Save all notes and continue action, a per-document Review drafts action, and Cancel/Escape.

Save uses the existing guarded note commands. Each acknowledged note is removed from the pending collection; a failed or conflicting note retains its exact body and the preflight. Save-all is not represented as a new atomic multi-document operation. A failure does not authorize a backup or a discard. The existing error modal takes precedence, and dismissing it returns to the remaining preflight. Review opens the actual owning project/document and restores its paused composer or selected draft. Cancel dismisses only the backup attempt, leaving every draft available.

A verified backup receipt remains available as **Last verified workspace backup**, separate from the latest attempt's waiting/running/verified/cancelled/failed status. A late cancellation cannot replace an acknowledged successful publication with a cancellation claim. Failed preparation or worker completion cannot reuse the older green success as the latest result.

## Preference persistence retry

The editor preference store retains whether an explicit persistence attempt remains unacknowledged. An I/O failure preserves desired values; Retry save repeats that same write without forcing another setting change. Settings stays open after a failed change. Its footer explains the unsaved state and offers Continue for this session rather than implying persistence through Done. The writer can continue working while the pending state remains visible on return.

Invalid, future-version or externally changed files retain the existing write barrier. Retry is disabled when that barrier requires repair/reload, and the preserved-file explanation remains available in Editor settings. Successful persistence clears the pending flag; unknown current-version JSON fields remain preserved.

## Passage-note affordance

The selection menu and Notes header use the same availability reason. Read-only state, unavailable annotation metadata, unfinished composition and unsupported selection scope have distinct explanations. A cross-block range is never expanded, flattened or converted to a guessed note anchor. The disabled control has hover text and the relevant surface displays a readable hint. Exact same-block forward/reverse Unicode ranges retain their existing authority rules.

## Dependency decision: UTC backup filenames

The native Save dialog previously proposed a constant filename, prompting Replace on a routine second backup before the no-clobber backend refused it. Use an explicit UTC date/time with milliseconds and `Z` in the default name, while retaining the backend's no-clobber publication rule. The filename is presentation, not backup identity or write authority.

Proposed exact desktop dependency: `time = { version = "=0.3.55", default-features = false, features = ["std"] }`. Its [official tagged workspace](https://github.com/time-rs/time/blob/v0.3.55/Cargo.toml) declares Rust1.88, Rust2024 and MIT OR Apache-2.0, compatible with the application's Rust1.97 baseline. The [tagged package features](https://github.com/time-rs/time/blob/v0.3.55/time/Cargo.toml) keep formatting descriptions, parsing, macros and local-offset support disabled. Code uses `OffsetDateTime::now_utc` and numeric calendar/time getters through standard `format!`; it does not access timezone data or use the optional OS offset dependency.

The expected normal graph is time0.3.55, time-core0.1.9, deranged^0.5.8, num-conv^0.2.2 and powerfmt^0.2.0. These are proposed upstream constraints, **not a claimed resolved lock**. Root owns manifest wiring and exact hosted lock resolution/notice qualification. No local Cargo fetch or build was used for this decision.

## Pending verification

Named source regressions cover parked-note preflight/cancellation, explicit cross-project saving, conflicting-note preservation, draft review navigation, unsupported range identity, failed preference-write retry, readable timestamp names, actual SQLite backup receipts across failed/cancelled attempts and late cancellation after publication. Direct Rust formatting checks parse the source but do not execute these tests.

Mounted retest must exercise the preflight in compact/wide layouts and long German/CJK labels, restore file access after preference failure and retry unchanged values, inspect disabled-note explanations with keyboard/pointer, and compare current-attempt status against a retained older receipt. Full editor/project acceptance and product issues remain open.
