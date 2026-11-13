# Native Canvas integration tasks

- [x] Audit exact generic source63845f9 and hosted Rust1.95 qualification; record dependency/authority design.
- [ ] Integrate one fixed-project board session, native viewport and explicit text editor.
- [ ] Implement durable prepare/checkpoint/ack, guarded failure/retry and compensating Undo/Redo.
- [ ] Integrate board listing, creation and resource navigation into the current workspace.
- [ ] Implement explicit native board draft recovery and preserved conflict paths.
- [ ] Cover synthetic source extensions, foreign/stale state, cancellation, save/Undo/reopen and controller behavior in hosted checks.
- [ ] Finish source-equivalent connector/card/drawing controls and real media workflows under #10.
- [ ] Exercise exact compiled native workflows and independent UI/UX review before accepting the complete parent issue.


## Exact first consumer check

Consumer source69346fa/run42500279200 compiled the real desktop integration. The3Canvas durable-acknowledgement/stale/dialect tests and2pinned native-import/cancellation tests passed in the all-feature desktop run. Both native-import tests also passed in the default workspace run. That overall run failed in a separate table-selection regression, so it is not an accepted consumer checkpoint. The unused `preserved_draft` accessor is removed in the next focused source increment before strict Clippy. Native OS interaction, source dialog/recovery paths, media/drawing/GFM and complete project acceptance remain pending.
