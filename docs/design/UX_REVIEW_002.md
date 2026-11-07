# Independent UX review 002 — foundation retest

Status: source re-audit in progress; native-foundation acceptance pending fresh visual and interaction evidence. Review date: 2026-11-07. This report does not approve Author pilot or full legacy parity.

## Named scope

Version `0.1.0-alpha.1`, macOS native foundation: local project/document navigation, plain-text writing and recovery, explicit exact-selected-range proposals with Apply/Reject/guarded Undo, archive preview, Light/Dark/System palettes, eight interface languages and bounded desktop panels. Existing rich editing, anchored notes/dictionary, voice, real model providers, Canvas and publishing remain tracked parity work. No new NER or local-inference capability is implied.

## Source re-audit

The implementation is now separated into workspace orchestration and commands, navigator, library, manuscript, review and dialogs components. The source addresses the main review001 issues:

- Explicit selection action freezes document/revision/body/scalar anchor; `stage_replace_range` preserves a repeated occurrence instead of searching for a unique quote. Selection alone no longer fills the tool.
- Review cards use bounded changed context before the composer, offer expanded review and visibly disable stale Apply.
- Pane resolution preserves a useful manuscript width and permits an explicitly requested inspector to replace navigation in a compact window.
- Focus has a visible reversible exit and retains the requested panel arrangement.
- Settings and review use modal backdrops; global application shortcuts are guarded while a modal is active, and closing returns editor focus.
- Prior stale-navigation and recovery issues remain guarded in the shared core.

These are source findings. Parent-reported checks include core/controller regressions for repeated selections, stale scopes, bounded excerpts, panes, focus and modal shortcuts. They do not by themselves establish native pointer, IME or screen-reader behavior.

## Remaining checks before scoped acceptance

1. Observe new actual dark/light/settings and compact screenshots, including long project identity and visible utilities.
2. Review an isolated interaction record for selection/context action/prepare/apply/undo, focus and modal behavior; distinguish actual native interaction from controller tests precisely.
3. Confirm selection-save failure does not display a false successful attachment status.
4. Confirm a large frozen quote has a bounded preview/expand path and ordinary action reachability.
5. Record exact current source/design fingerprint and screenshot hashes only after evidence is stable.

The historical review001 remains unchanged as historical evidence except repository-language normalization. The authoritative release decision is `UX_GATE.json`; it stays changes-required until this retest is complete.

## Independent retest update — 2026-11-08

Observed actual `docs/images/workspace-dark.png` and `workspace-light.png`. Both show a coherent bounded workspace, readable typography and utility controls. Dark review shows a scoped old/new excerpt with Apply/Reject adjacent and the composer collapsed; it no longer duplicates complete chapters. The light image shows the local composer state and legible light surfaces. These are actual renderer outputs; OS chrome/native menu behavior is not proved by them.

Independently executed `cargo test -p studio-desktop ux_regressions --locked`: **7 passed**, covering the second selected occurrence, stale frozen selection refusal, stale navigation snapshots, bounded long-chapter excerpts, compact pane policy, focus restoration and modal shortcut protection. Separately executed `app::tests`: **2 passed** for interface/manuscript glyph coverage and translated static messages. The selection regression constructs a frozen selection programmatically; it is not a pointer/context-menu interaction test. The focus regression checks state resolution, not an actual button click. These boundaries remain explicit.

Current source additionally truncates toolbar/document identity, bounds a large frozen quote in a local scroll area and starts the inspector quietly closed. Selection attachment success status is now inside the successful-save branch. Fresh compact/settings and native interaction evidence are still pending.

### R002-03 — persistent save-failure modal trap (P1)

The automatic save loop retries whenever the original dirty timestamp is older than650ms. A failed save retains that timestamp, so another attempt occurs on every100ms repaint and reopens the error modal. Dismissing the alert therefore does not leave a stable route to Export/recovery. Suspend timer-driven retry after failure while retaining the unsaved body/draft; provide an explicit retry/manual Save/export path. Verify with a persistent failure that dismissal does not automatically reopen the modal and the author can reach recovery controls. This is in the named foundation scope because dependable local writing/recovery is a declared capability.
