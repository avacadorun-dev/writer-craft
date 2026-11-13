# Host-owned Canvas integration

## Dependency decision

Pin `craft-canvas-model` and `craft-canvas-egui` to `ca8b74aed8df0ab5db38308a3cf0ef235f3443a1`. Exact hosted run42499304520 passed locked Rust1.95 tests, rustfmt and strict Clippy on macOS, Windows and Ubuntu. This includes the original-compatible2048-byte link-URL correction over63845f9 and qualifies model/egui event contracts, not full native application interaction. The crates reuse the host's egui0.36.2/serde/serde_json/thiserror graph and introduce no window or media runtime. Retain their MIT license and upstream connector attribution in generated notices.

## Authority

`CanvasState` owns a fixed project/board pair, the acknowledged `ProjectCanvasSnapshot`, one `CanvasSession` and disposable `CanvasView`. Native pointer previews never write canonical content. Each complete intent prepares against the exact session identity, validates the source dialect, persists the complete candidate through `save_project_canvas` at the acknowledged saved head, then acknowledges the already-prepared session. A failure leaves the previous canonical/session unchanged and retains the exact candidate for explicit retry or preservation. Undo/Redo follow the same durable append-before-ack order and create compensating revisions. Never reset the session to simulate successful Undo.

The host's dialect recognizes source `x-writers-studio` lock, group/frame/element/resource metadata. Generic file and URL strings never trigger ambient filesystem or network access. Navigation uses explicit project-scoped resource references. Unknown fields remain preserved; unsupported known preview capabilities remain identified rather than shown as operational empty cards.

## Source and recovery

Use existing canonical Canvas revision/draft APIs. Opening a stored native draft does not silently replace the saved graph: the author must explicitly recover or keep saved content; stale drafts remain preserved. A text-source editor retains its captured exact base and complete draft until successful acknowledgement or explicit Cancel. Existing user data, original source and imported external evidence are never made replay authority.

## Review

Required source/controller tests cover save before acknowledgement, stale/foreign bases, failed durable save, unknown metadata, exact Unicode text, Undo/Redo and restart. Actual compiled native pointer/keyboard, source dialog, compact/wide Light/Dark and recovery/Cancel paths need separate designer acceptance. Native media and full drawing/card/connector controls remain required by the parent contract.
