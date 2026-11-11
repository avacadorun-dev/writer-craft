# Independent home body source preflight

Date: 2026-11-12. Reviewer: separate `home18_review` agent. Candidate: `3e1b36c4d6b6b0fb0935525d04896de31816e109`. Scope: issue #18 source only. Result: no remaining blocking source finding after the P2 correction; execution and native acceptance pending.

Reviewed library.rs, library/view.rs, library/artwork.rs, library/tests.rs and the two new eight-language catalog rows. Continue resolves the current project's actual ID still present in the library and routes through existing guarded opening/save/navigation. The implementation uses available body dimensions, one local vertical scroll, native text/controls and horizontal rows. Artwork adaptation targets blue robe pigment, retaining neutral paper/skin/ink and alpha, and caches by resolved accent. No shell, engine, persistence or manuscript-typography changes were found in scope.

Required P2 correction: offscreen rows and banner actions could receive keyboard focus without becoming visible. Both paths now call scroll_to_me on gained focus. The new regression sends Tab through thirty rows and Shift+Tab back to the banner, checking endpoint visibility. This is fixed in source; the regression has not yet executed. Existing egui Sense::click provides Enter/Space activation.

The root ran scoped rustfmt/diff checks, OpenSpec 1.16.1 strict validation for home-library-body, and source metadata validation. No local Rust builds/tests ran. The candidate still requires hosted formatting/tests/Clippy, exact compiled captures and actual native actions/theme/resize/keyboard review. No issue closure, native usability acceptance or release readiness is asserted.
