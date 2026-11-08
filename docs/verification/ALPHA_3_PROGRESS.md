# Alpha.3 implementation and qualification

Active development, not a release or complete native acceptance. First fully green source: `e5018379498e0dcf1fd99ec07a6298e80dbbbe21`. Work is in [draft PR #13](https://github.com/avacadorun-dev/writer-craft/pull/13). Published alpha.1/alpha.2 tags and assets remain immutable.

## Issue disposition

Alpha.2 was tested on macOS and confirmed functional. Installer issue #3 is closed after complete package checks and acceptance; its milestone is closed. Issues #2, #4, #6–#12 retain their unfinished alpha.3 criteria. New #14 covers researched formatting, meaningful project types and stable user scenarios; its candidates are not implicitly selected features.

## Exact-source evidence

[Verify run42501608167](https://github.com/avacadorun-dev/writer-craft/actions/runs/42501608167) passed every required job: macOS313 default-workspace tests,428 all-feature-workspace tests and12 isolated-model tests; formatting; strict all-target/all-feature Clippy; strict OpenSpec and source/tool checks; shared core/transfer/CLI tests and strict Clippy on Windows and Linux. Default/all-feature counts overlap and are not749 unique tests. No local Cargo build/fetch/test was used.

Qualified pins are CraftEditor `26e71d1c2e0b611e0c46fc21980eccef4e01ab92` and CraftCanvas `ca8b74aed8df0ab5db38308a3cf0ef235f3443a1`. Generic/controller checks do not replace actual consumer/native acceptance.

| Workstream | Actual implementation/controller result | Remaining work |
|---|---|---|
| Tables | Native cell/node/bookmarks, selected structural ranges, header inheritance, explicit deletion, widths/alignment, column resizing and exact cell selection; frozen original-output comparisons. | Table-local wide overflow, complete input/clipboard and native interaction/history acceptance. |
| Structured review | Exact retained Unicode attachments; scoped detached proposals; atomic replayed Apply/Reject/compensating Undo; note metadata preservation; native structured before/after preview; core and desktop-controller navigation/stale/rollback checks. | Actual native review journeys, real provider/context integration and complete command parity. |
| Canvas/import | Metadata-confirmed pinned `.wstudio` import, cancellable atomic creation; native board navigation/edit/move/resize/history/ack/recovery; five focused controller regressions pass. | Full Canvas/type/media workflows, native/private-reference acceptance. |
| Portability | Bounded framing, format1 compatibility restore, content-addressed paged format2 export, verified no-clobber archive and immutable stream preflight. | Semantic format2 activation/roundtrip, selected-scope streaming, native export/restore and full-workload acceptance. |
| Project domains | Existing kinds/custom structure, hierarchy, dictionary/guide/memory and media are preserved. | Preserved JSON/flat navigation do not complete native hierarchy/domain workflows. Research must justify new creation profiles. |

## Requested isolated dev package

[Preview run42501773427](https://github.com/avacadorun-dev/writer-craft/actions/runs/42501773427) is pinned to this source. The requested local dev copy uses a separate name/workspace. Packaging or exploration does not grant full alpha.3 acceptance. Do not drive or replace the running dev window after handoff while the user explores it.

## Continuing acceptance

The independent `full-editor-parity` gate remains pending. Generic media mechanics are separately developed in [CraftMediaPreview](https://github.com/avacadorun-dev/craft-media-preview), with no unqualified consumer pin. PDF/audio/video/source playback, editable hierarchy, dictionary/guide/memory, remaining native controls/syntax/flow/input/IME/accessibility/performance, full compatibility and portable restore retain their requirements.

Read the [independent audit](../design/UX_ALPHA3_AUDIT.md), [release plan](../governance/NEXT_RELEASE.md), [type research/scenarios](../research/PROJECT_TYPES_AND_FORMATTING.md) and source matrices. Partial issues remain open.
