# Existing-block source re-editing tasks

- [x] Record F-46/#7 behavior, exact-object safety and guard boundaries before implementation.
- [x] Open the existing `BlockDialog` for the double-clicked Mermaid diagram, block formula, inline formula or image, and for a code block through the context-menu/F2 routes.
- [x] Keep single-click selection inert and keep the permanent toolbar unchanged.
- [x] Offer the same command in the manuscript object context menu for the current measured selection and on `F2`/`Shift+F10`.
- [x] Add focused source regressions for gesture routing, exact-object selection, cancel/stale preservation and save/reopen identity.
- [x] Pass exact-source hosted checks (`cargo fmt`, workspace tests, strict Clippy, strict OpenSpec) on the committed candidate.
- [ ] Observe double-click, right-click and F2 on a compiled native candidate at a compact window size and record the evidence.
- [ ] Obtain independent review of the exact compiled interaction and update #7 with facts.

## Evidence boundary

Source `91abc734068b43a75b3edd9b6cbf2b7c98bba1c0`: [Verify 43087405526](https://github.com/avacadorun-dev/writer-craft/actions/runs/43087405526) and [Preview 43087397599](https://github.com/avacadorun-dev/writer-craft/actions/runs/43087397599) are green. The compiled-widget probes exercise synthetic egui pointer/key input through actual widgets, not operating-system events. Dialog identity, history, stale and cancellation behavior are covered by the existing authoring tests. Native interaction evidence must name the exact installed candidate and remains outstanding.
