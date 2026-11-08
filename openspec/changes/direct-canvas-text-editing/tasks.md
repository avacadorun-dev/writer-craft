- [x] Record original textarea/blur/Cmd-Enter/Escape behavior and issue #15.
- [x] Specify direct input, child input ownership, IME, two histories and guarded checkpoint/retry semantics.
- [x] Qualify the separate generic inline widget before dependency adoption (exact `58ce123`, all-platform Rust1.95 run 42505477358).
- [x] Integrate direct card input and new-card focus without a primary modal.
- [x] Add exact draft checkpoint before canonical save and preserve later typing through acknowledgement.
- [x] Add seven host/global-Save/native-event acknowledgement/restart/checkpoint conflict/retry/new-card/composition regressions and complete localization; native OS journeys remain pending.
- [ ] Pass exact-source hosted tests/fmt/strict Clippy.
- [ ] Obtain actual compiled native input/zoom/selection/IME/failure and independent UX acceptance.

## Bounded implementation handoff

Host source commits: 0f3ee84 (direct card integration and checkpoint CAS), 9a4d57d (actual host event/explicit Save), b51ddf9 (global app Save and native Undo/Redo across acknowledgement), followed by the explicit preedit navigation guard. Root app Save hook 49bf627 is required by the global regression. Generic pin 58ce123 is qualified as recorded in design; no new packages were added by that pin. OpenSpec strict validation, direct rustfmt and diff checks passed locally; Cargo checks are hosted only and the latest complete host batch remains under verification. Actual native double-click/new-card focus/selection/paste/IME/zoom/Done/Cancel/retry/restart and independent design review remain pending. No issue closure or native acceptance is implied.
