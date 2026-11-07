# Independent UX review 003 — native foundation acceptance retest

Review date: 2026-11-08. Reviewer: independent UI/UX designer (`ux_acceptance`). Version: `0.1.0-alpha.1`. **Disposition: accepted for the named `native-foundation` engineering-alpha scope.** No Author pilot or complete migration approval is granted.

## Named scope and evidence boundaries

The scope is the macOS native foundation: local project/document navigation, plain-text writing with local revisions and recovery, exact-selected-range local proposals with explicit Apply/Reject/guarded Undo, read-only archive preview, bounded panes, appearance preferences and eight interface languages. This is an engineering alpha. It is not Author pilot, complete Writers Studio migration, rich-editor qualification, a signed/notarized distribution certification or complete accessibility qualification.

The reviewer read `AGENTS.md`, `UX_ACCEPTANCE.md`, the append-only owner feedback, desktop/workflow contracts, historical reviews001/002 and current shell/manuscript/review/dialog/settings/icon source. The reviewer did not manipulate the owner's running application or projects. Native computer-use observations below were performed and recorded by the implementation owner; they are explicitly distinguished from tests and image inspection independently performed by this reviewer.

## Independent automated execution

The reviewer independently executed `cargo test -p studio-desktop --locked` during final integrated Lucide/settings verification: **32 passed, zero failed**. The implementation owner subsequently recorded the frozen-source full check: **71 tests passed** (28 core, 32 desktop, 11 transfer), formatting and strict all-target/all-feature Clippy. The last title/brand allocation adjustments were also checked in the final compiled captures below. The independent desktop suite includes:

- Eleven UX regressions: exact second occurrence, stale frozen-scope refusal, refreshed navigation after saving, compact pane policy, reversible focus state, bounded changed excerpts, persistent save-failure retry suspension and explicit successful retry, modal shortcut ownership, same-frame shortcut batching, and project creation with Enter while every AccessKit focus ID belongs to an emitted node.
- Two raw-egui input regressions: secondary press/release preserves selection and opens its menu; Shift+F10 opens selection actions without changing text.
- Two Lucide checks: every mapped utility has a pinned official source; native geometry has finite coordinates and valid triangle/contour references.
- Settings/catalog completeness, preference persistence/defaults/invalid-file preservation, interpolation parameters, seven-palette mode resolution/contrast and actual bundled-font character ownership/coverage.

These tests exercise controller/storage state and raw egui events. They do not prove native keyboard layout/IME behavior, VoiceOver usability, file-dialog behavior or full system-menu integration. The persistent failure test proves stable dismissal over subsequent frames, preserved unsaved body/durable draft and newer canonical writing, and export of preserved text; it is not a native disk-full simulation.

## Native interaction record reviewed

Reviewed `docs/verification/NATIVE_INTERACTIONS.md`, supplied by the implementation owner from a separate QA app with synthetic data:

- The sample contains two identical `the lighthouse` phrases. Native selection/secondary-click/Prepare freezes the second occurrence at Unicode scalars 486–500. Replacement typing goes to Proposed text. Stage leaves revision 1 unchanged; Apply creates revision 2 and changes only the second occurrence; Undo creates revision 3 and restores the full original sample.
- Command-N focuses the creation field; typing and Enter create a project and First chapter while the app remains running. The new manuscript is focused, accepts text and saves.
- Categorized preferences block Command-N when open. All eight language choices are available; Japanese selection changes UI labels while preserving English author text/names. Escape closes the language popup before Settings and returns focus without an accessibility crash.
- Focus mode has a visible exit; native and controller observations support reversible panel state.

The earlier Enter crash was traced to requesting a new editor focus ID before its AccessKit node existed. Focus now waits until that TextEdit renders; the new tree regression and successful native creation close that defect. Same-frame shortcut stacking is covered by an explicit dispatcher-return rule and its regression; the already-open-modal native route was checked separately.

Native Shift+F10 injection remained inconclusive. Its raw-event test passes and the native secondary-click journey is complete. This is an explicit keyboard qualification limit, not a claim that native F10 was observed working.

## Historical findings and owner corrections

| Finding | Current source/evidence disposition |
|---|---|
| R001-01/05 selected occurrence and explicit scope | Native repeated-second journey plus independent controller/raw-input tests verify the declared pointer route. Selection alone does not attach scope. |
| R001-02/06 review size and stale state | Bounded old/new excerpts, nearby Apply/Reject, expanded review and visible disabled stale Apply are implemented; final compiled Dark review capture verifies the implemented presentation. |
| R001-03/09; F-19 geometry and titles | Toolbar identity and navigator derive from shared pane geometry; title regions clip/truncate with full-value access. The final compact German capture verifies title truncation before the inspector boundary; normal Dark/Light captures verify the visible navigator/toolbar boundary. |
| R001-04/08 focus and quiet writing | Visible reversible focus and quietly closed inspector are implemented and exercised; final Dark/Light captures verify the quiet/shared shell. |
| R001-07 modal routing and focus | Native Enter creation, language-popup/Settings Escape, already-open-modal guard and AccessKit/same-frame tests close the observed failures. |
| R002-03 repeated save-failure alerts | Independently executed regression confirms automatic retries pause and dismissal stays stable; unsaved text/draft remain preserved and manual retry/export are possible. |
| F-20/21 structured settings and fixed geometry | Eight categories, bounded content, fixed header/footer and honest parity scaffolds are implemented. Final normal/minimum Settings captures verify bounded header/footer, aligned controls and consistent category/content spacing, closing the observed visual rejection for this scope. |
| F-22/23 appearance presentation and hierarchy | Miniature-window mode/palette cards and left-aligned category groups are implemented; final English and Japanese Settings views verify the selected presentation. |
| F-24 actual updated preview | The final launch record confirms the updated app was observed through native screenshot/AX and left running with a separate persistent synthetic workspace. |
| F-25/26 utility family | Official-family comparison inspected; designer recommended Lucide and owner explicitly selected it. Thirty official Lucide utilities replace the hand-drawn set. Final native toolbar/sidebar captures verify the selected geometry is readable and coherent in both appearances. |

Missing legacy workflows are documented scope, not silently approved omissions: rich editing, persistent annotations/dictionary, real model/MCP/provider setup, voice/audio, Canvas, full project migration and publication still require later contracts and their own UX acceptance. Informational settings scaffolds are visibly unavailable and contain no fake operational controls.

## Utility family comparison

The independent reviewer inspected both actual official-SVG contact sheets at original resolution: seven meanings, identical foregrounds/surfaces, 24/20/16pt at 2× Retina. These are comparison renders, not compiled egui mesh evidence.

| Family | Observed fit and tradeoff |
|---|---|
| Lucide Outline | Best balance for this interface: consistent optical presence, readable 16–20px details, restrained rounded geometry beside Source Sans and the quiet manuscript surface. |
| Tabler Outline | Credible close alternative with strong recognition; the shown palette/microphone/diagonal plug feel somewhat more technical/heavier. |
| Phosphor Regular | Delicate and attractive at 24px; shown 16px document/sidebar have less optical weight and the selected connection glyph is busier. Another glyph could improve that meaning, so this is not a universal family limitation. |

The recommendation is specific to the actual rendered interface and is not a universal ranking. F-26 records the owner's subsequent Lucide choice. Product branding remains the separate monk/book mark. Source SVGs, pinned commit, license notices and native mesh validation are present; final compiled captures confirm readable antialiased forms at the actual toolbar/sidebar sizes.

Comparison hashes (local selection evidence, not release captures):
- `artifacts/icon-comparison/comparison-dark.png`: `9676436a085f99ff463722446aa701db3f75e619a60a51bad1802e8013c92621`.
- `artifacts/icon-comparison/comparison-light.png`: `faaa2470b2723f210336b6cb68d093e7121c911aa782f35e6206fac17a96ebf4`.

## Final compiled visual inspection

The reviewer inspected all nine final checked-in PNGs at original resolution. They are real application renderer captures, not mockups. Normal client captures are 2880×1846 pixels (1440×923 logical at 2×); compact captures are 2000×1360 pixels (1000×680 logical at 2×). OS traffic lights are excluded from renderer captures; native launch/window evidence comes from the separately recorded computer-use session.

| Capture | Independent visual disposition |
|---|---|
| `workspace-dark.png` | One flat workspace; visible navigator and inspector boundaries continue into toolbar separators; monk/name gap is compact; Lucide forms are clear. Scoped excerpts and Apply/Reject are nearby and visible; the composer is collapsed. |
| `workspace-light.png` | Legible light surfaces and serif manuscript; inspector quietly closed; stationary navigator/header/status and bounded manuscript composition. |
| `qa/compact-de.png` | Minimum-width editor, deliberately long German project/document identities truncate without crossing the inspector boundary or colliding with utilities. Navigation gives way to the requested inspector and useful manuscript width remains. Vertical review overflow stays within its local scrolling pane. |
| `qa/settings-en.png` | Left-aligned category hierarchy, regular spacing, miniature-window mode/palette previews, clear selected state and fixed reachable Done. |
| `qa/settings-ja-light.png` | CJK labels render without missing glyphs; selected Light/palette states remain distinct; long shortcut-category label truncates within its row and has full-value access. |
| `qa/settings-compact-en.png` | Preferences fit the minimum client size with visible close/Done controls; sidebar/content/footer remain bounded. |
| `qa/settings-general-en.png` | Language setting belongs to General, with a clear statement that author text is preserved. No decorative nonfunctional fields. |
| `qa/settings-editor-en.png` | Actual manuscript-size control belongs to Editor and stays within a stable settings shell. |
| `qa/settings-mcp-en.png` | MCP orientation is present as requested, explicitly unavailable in this alpha; no fake endpoint, connection test or operational switch. |

The two final semantic corrections are present in source: the modal X is labelled with localized `Close`, and the toolbar gear is labelled Settings rather than Appearance settings. F-27 is verified in the final normal/compact images: the name sits directly beside the monk while preserving the native macOS leading exclusion region.

## Final launch and qualification limits

The implementation owner's final record in `docs/verification/NATIVE_INTERACTIONS.md` confirms the packaged updated debug app opened through native computer use and remained visible, showing the monk, Lucide controls, Russian UI, saved continuous manuscript and three-document synthetic project. It was left running for the owner, with no further automated clicking and no original Writer Studio project mutation. The record also reports ad-hoc code-signature and bundle-metadata checks. This is launch evidence, not Developer ID/notarization/update certification.

No blocking P0/P1/P2 issue remains **within this declared engineering-alpha scope** after the source, independent tests, supplied native interaction record and final visual inspection. The release gate may accept `0.1.0-alpha.1` for `native-foundation`.

The following are not approved by this decision: full author-pilot/daily-use or legacy parity; rich editing/annotations/voice/real AI/MCP/Canvas/publishing; full IME/VoiceOver/system-menu qualification; native Shift+F10 behavior; Windows/Linux desktop interaction; exhaustive native journeys in all eight languages and every palette/mode combination. Existing catalog, glyph and contrast tests plus English/German/Japanese captures and Russian launch evidence must not be described as that exhaustive qualification. Each later capability earns its own acceptance.

## Accepted client baseline and retained visual evidence

Accepted client-surface baseline fingerprint: `b50be2e95ba105ce0589a0e07c817cc5832ba7d76f0887ab44c0e08c23c3cf60`. The reviewer independently recomputed it after inspecting these captures. The nine captures below remain the unchanged client-geometry/theme/icon evidence; the bounded startup-icon delta is reviewed separately below. A source/design change requires the relevant retest and gate update.

| Checked-in evidence | SHA-256 |
|---|---|
| `docs/images/workspace-dark.png` | `1583f065b70ed40ffc93510af344d44a4e076e31a7b782377be70ee55b7b176e` |
| `docs/images/workspace-light.png` | `956e809d1e817c9d85be559d8c6c51b0b7f41ee6f76095f1170fb3382085174e` |
| `docs/images/qa/compact-de.png` | `3e1593208bf0c20499f99affd23c03e81043544f05afa98223779eadf56e3ac8` |
| `docs/images/qa/settings-compact-en.png` | `a32c32f4881862236c97b2942934cb772d4b2b1fda84722ed77401a1980c2a30` |
| `docs/images/qa/settings-editor-en.png` | `c7fdae7acb76bef114e4cba1544ef9c98bb74f70976e63c03c48e8f9c8d053d0` |
| `docs/images/qa/settings-en.png` | `7c88c4dc23e9ff1f7543afd89cb52966661ad248c70f8bb9fc32ab9e5f39825b` |
| `docs/images/qa/settings-general-en.png` | `962aa121378feaa8a9baf959579d72da23cf2f0e0e4d125907daa0b81e0b7ddc` |
| `docs/images/qa/settings-ja-light.png` | `542f8292af578e2021fcd944839b2ce1788ffe50d593f4fab058e1cb14d29d28` |
| `docs/images/qa/settings-mcp-en.png` | `168a92d57636db3ba1dea093c15b5964417825c783ea812ad469999271157072` |

## F-28 bounded application-icon startup delta

**Delta disposition: accepted for the same `native-foundation` engineering-alpha scope; this does not close issue #1 or claim direct Dock visual confirmation.**

The owner reported an unfamiliar application/Dock icon. The reviewer inspected the exact change against the accepted client baseline: the only runtime crate change is startup loading of the embedded PNG through `eframe::icon_data::from_png_bytes`, propagated as an application-creation error on decode failure, and explicit `ViewportBuilder::with_icon`. No manuscript, pane, preference, utility-mesh or client-rendering code changed. The new PNG and the existing ICNS are reproduced by `scripts/make-icon.sh`. The owner-feedback update records F-28 without expanding feature scope.

The reviewer independently read the installed primary eframe 0.36.2 implementation: `src/native/epi_integration.rs` falls back to the framework icon when no viewport icon is supplied, and `src/native/app_icon.rs` applies the selected image to the macOS application at runtime. Supplying the intended icon directly addresses that startup path. This is a source-based causal conclusion, not a claim that an ICNS file alone proves running-process identity.

The reviewer viewed `assets/WriterCraft.png`: a valid 512×512 monk/book application mark matching the approved identity, byte-identical to the generated 512px iconset output. Asset SHA-256: `9f14929016a6991929e337fbae0932302aebd2da7c963c409c95096360394b70`.

Reviewed the new F-28 section in `docs/verification/NATIVE_INTERACTIONS.md`: after rebuilding and re-signing the proper packaged debug app, the implementation owner performed a fresh native launch and observed the standard macOS About panel showing the monk/book application image with WriterCraft/version metadata. Closing About returned to the saved manuscript, and the updated app was left running. That native OS observation was supplied by the implementation owner; the independent reviewer personally inspected the source/asset/evidence record. The current source's 71 tests, formatting and strict Clippy pass are recorded there.

The automation surface did not independently capture the Dock. Direct Dock appearance and the hosted candidate still need the confirmation tracked in [issue #1](https://github.com/avacadorun-dev/writer-craft/issues/1); do not describe About evidence as a direct Dock screenshot or close the issue on the compiler result alone. The source fix plus asset validation and fresh native OS About observation are sufficient to accept this bounded startup delta for the engineering alpha. Existing full-parity/accessibility/platform limitations above remain unchanged.

The nine client captures were not repeated: they predate only this startup/OS-icon delta and their code/content hashes remain unchanged. They prove the previously reviewed client geometry, not a recaptured OS icon. Repeating them while the owner explores would add no relevant icon evidence; the fresh native launch/About check is the relevant retest.

Accepted application-icon startup-delta fingerprint: `5f1dd22c19a24da04045377200aebf88d415ff39e363c5f62a08e7b1aea125fb`. The reviewer independently recomputed this value for that delta. The subsequent metadata-only refresh below is the current gate fingerprint.

## Owner-authorized MIT metadata refresh

**Metadata-delta disposition: accepted without changing the `native-foundation` engineering-alpha scope.** The owner explicitly selected MIT for WriterCraft. Root package metadata, the editor-compatibility harness metadata, license file and release documentation/checks were updated to that instruction; this adds no application behavior or UX capability.

The reviewer independently verified the source/design fingerprint relationship without modifying files: substituting only the previous `Cargo.toml` workspace-license value into the current fingerprint computation exactly reproduces the accepted startup-icon fingerprint `5f1dd22c19a24da04045377200aebf88d415ff39e363c5f62a08e7b1aea125fb`. The remaining fingerprinted client code, native icon startup code, assets and design contracts therefore match the accepted baseline. Offline Cargo metadata independently reports MIT for all four workspace crates. The nine retained PNG hashes are unchanged.

The implementation owner records passing release-helper 9, portable-packager 4 and release-staging 5 tests after the metadata update. The reviewer reran the source release check against the refreshed gate. No additional native launch or client recapture is warranted for this administrative metadata delta: the accepted client visual evidence and fresh application-icon/About evidence remain applicable, with their existing explicit limits. Direct Dock confirmation in issue #1, future annotations and full migration remain outside this refresh.

Current accepted source/design fingerprint: `6bb777e7ced2e4559bc14312ab54beea2f28160b63ccaaf7bb770d2bffd14fd0`. The reviewer independently recomputed it before refreshing the gate.
