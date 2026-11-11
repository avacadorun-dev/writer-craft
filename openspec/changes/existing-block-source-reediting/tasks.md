# Existing-block source re-editing tasks

- [x] Record F-46/#7 behavior, exact-object safety and guard boundaries before implementation.
- [ ] Open the existing `BlockDialog` for the double-clicked Mermaid diagram, block formula, inline formula, code block or image.
- [ ] Keep single-click selection inert and keep the permanent toolbar unchanged.
- [ ] Offer the same command in the manuscript object context menu for the current measured selection and on `F2`/`Shift+F10`.
- [ ] Add focused source regressions for gesture routing, exact-object selection, cancel/stale preservation and save/reopen identity.
- [ ] Pass exact-source hosted checks (`cargo fmt`, workspace tests, strict Clippy) on the committed candidate.
- [ ] Observe double-click, right-click and F2 on a compiled native candidate at a compact window size and record the evidence.
- [ ] Obtain independent review of the exact compiled interaction and update #7 with facts.

## Evidence boundary

Source-only completion is partial. Dialog identity, history, stale and cancellation behavior are covered by the existing authoring tests; the new gesture-routing tests prove selection/flag routing, not operating-system mouse behavior. Native interaction evidence must name the exact installed candidate.
