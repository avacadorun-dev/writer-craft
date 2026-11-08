## Implementation

- [x] Qualify a separate immutable native provider before host pinning.
- [x] Resolve narrow same-project source capabilities on one bounded worker.
- [x] Preserve original bytes and reject stale/foreign host results.
- [x] Add actual image/PDF board rendering and explicit page viewer.
- [x] Add complete eight-language catalog rows.
- [x] Add synthetic host pixel/preservation/cancellation/queue tests.
- [x] Resolve and review the exact host lock on a hosted runner (run 42503543183, source e6bd6bc; SHA-256 `a5dda16e41283d781070a152bed538132b371f209d1768d69e7d6f80d3f90926`).
- [ ] Pass exact-source host tests, strict Clippy, formatting and all-platform package checks.
- [ ] Review resolved license notices and release graph/size changes.
- [ ] Obtain independent actual compiled/native compact/wide, Light/Dark, keyboard/page/error/retry acceptance.

## Required continuation under issue #10

- [ ] Complete original animated-image, audio/video and bounded source-segment playback, supported original codec corpus and no-device/seek/end journeys.
- [ ] Complete original PDF controls/fidelity and private reference verification.
- [ ] Complete full Canvas card/drawing/connector workflow acceptance.

## Lock review evidence

The hosted metadata-only resolver used rustc 1.99.1; its intended stale-lock gate failed after retaining the actual resolved lock. This is lock provenance, not Rust1.97 compilation or native acceptance. The graph changes from 639 to 660 packages: 21 additions, no removed/upgraded pre-existing package identities/checksums. Existing changed dependency rows only disambiguate `font-types` versions and add the two desktop media dependencies. The graph retains one egui 0.36.2 and the exact qualified f370512 provider/six 0a9d1f48 reader-raster source pins. Exact-source all-platform host checks remain pending.
