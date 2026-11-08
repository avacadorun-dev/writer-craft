# Full-editor architecture and acceptance design

## Existing foundation

WriterCraft uses egui/eframe 0.36.2 and a shared SQLite core. Its current editable canonical value is plain prose; imported JSON is opaque/read-only. The earlier prosemirror 0.5.2 experiment proved 12 model properties, not a native rich view. See the complete source/UX matrices and `docs/research/EDITOR_SPIKE.md`.

## Boundary

CraftEditor owns generic lossless structured documents, explicit scalar/UTF16/byte positions, revision-guarded transactions, history and a single native layout/input/selection session with block rendering. WriterCraft supplies its schema/compatibility adapter, projects, persistence/media, note/dictionary meaning, source playback, assistant review authority and localization. Pin actual reviewed library commits/packages; do not depend on a developer's absolute checkout path in distributed source.

The original PM-shaped JSON remains canonical. A transient layout projection or a reused WordCraft layout/input algorithm cannot become a lossy alternate document format. Unknown metadata stays preserved, unknown/future unsupported structures are refused before editable conversion, and plain schema 1 is not eagerly upgraded. Maintain exact mapping between document/block identities, Unicode scalars, PM UTF16 structure positions and native view geometry. Pixels are never persisted anchors.

## Native view

There is one document-level caret/selection/history, including cross-block and table interaction. Independent per-block text fields are not a substitute. Input, clipboard, composition, layout, hit testing and accessibility share that session. Renderer plugins must cover every current atom/block, including real Mermaid/formula previews and their safe source fallback. Reuse focused upstream components only after checking API/license/provenance; do not vendor a complete word processor application or introduce a browser runtime.

## Persistence and command authority

Persist canonical rich snapshots, draft bases, annotations and history atomically with optimistic revision checks. Existing plain/opaque data and histories remain readable. Prevent legacy plain-text save paths from flattening an edited structured document. UI and headless adapters call the same domain commands. Assistant proposals remain staged until explicit Apply; apply/undo include rich data and mapped anchors in one transaction. User edits win over late work.

Media must be durably stored and integrity/ownership-checked before an image node references it. Keep original audio references and bounded playback semantics. Annotation lifecycle and attachment validity are independent; a deleted/uncertain quote retains its note and original context without automatic retargeting.

## Internal milestones versus release

1. Complete current-source inventory and freeze fixture identity.
2. Implement generic model/typed transaction and compatibility fixtures.
3. Prove a representative native document with mixed marks/Unicode/repeated quote, sourceRefs and a table boundary across input/selection/split/join/Undo/save/reopen.
4. Complete every remaining block/input/integration case in the matrices.
5. Independent native UX retest and exact-source full-editor gate.

Only step 5 can authorize an alpha.2 candidate. Steps 1–4 are development checkpoints and must expose gaps, not reduce the agreed scope. Windows/Linux compilation remains separate from macOS-first native interaction evidence; native input assumptions must not be reported as verified on an unobserved platform.


## Isolated table and formatting preview

The owner requested the first compiled table/formatting checkpoint for hands-on testing before the full alpha.2 candidate. An optional `editor-preview` feature mounts the separate engine in the existing bounded manuscript pane. The `--editor-preview` flag or bundle environment selects a separate `WriterCraft/Editor Preview` workspace and bundle identity; the normal running alpha.1 preview is not replaced.

The host owns structured draft CAS and canonical SQLite revisions; the engine owns an independent session/history clock. Save acknowledges the whole structured tree and keeps the live caret/history. Formatting-only edits are persisted; plain-text save/proposal paths cannot flatten rich documents. A separate hosted workflow compiles and packages this development checkpoint, records the exact source and lock, and does not tag or publish alpha.2. Native interaction and independent design review remain required before owner handoff. This milestone does not waive any remaining editor parity requirement.


## Clipboard verification boundary

These requirements remain part of app issue #7 and full editor epic #4. The first implementation supplies deferred engine contracts and internal owned-image capsules. Full native app wiring, external HTTP acquisition, file drop, mixed/open wrapper fitting, rectangular table selection and cross-application paste UX remain required until verified. A synthetic fixture or OS format roundtrip alone does not close those issues.
