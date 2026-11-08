# Native media integration for alpha.3

Status2026-11-08: separately implemented image/PDF provider foundation under hosted qualification. Original audio/video/source playback and full native acceptance remain required under #10/#4. No media dependency is yet pinned into this application.

## Reusable boundary

[CraftMediaPreview](https://github.com/avacadorun-dev/craft-media-preview) contains generic provider/request/texture mechanics only. WriterCraft owns project scope, exact selected database, logical resource aliases, source metadata, storage, localization, navigation and the normal app shell. Never clone the whole original app or insert a WebView. A generic parsed source/request is not project or filesystem authority.

The native adapter obtains a narrow `Workspace::project_media_source(project,logical_id)` capability on a separately opened media-worker Workspace. It verifies captured original MIME/length/SHA through the fixed project before supplying immutable bytes to the provider. It must not read every manuscript through a complete project export merely to preview one attachment, use the image-only12MiB transfer limit for PDF/audio/video, reopen source paths or fetch inert Canvas URLs implicitly.

Each source/project/page/geometry change begins a fresh non-deserializable provider request. Cancelling/leaving stops its worker; stale results cannot replace another resource even when the bytes/hash happen to match. Existing data and original artifacts remain unchanged and exportable after every preview/playback error. Native texture/cache state is disposable presentation data.

## Actual first provider slice

The first implementation returns real PDF page rasters and PNG/JPEG/GIF/WebP first-frame pixels, actual page counts, correct premultiplied-versus-straight alpha metadata and known backend diagnostics. It has independent finite source/decoded-image/output bounds. PDF parser/interpreter cancellation is cooperative; codec calls, total scratch memory and process allocation failure are not falsely described as universally preemptible or bounded by the output size alone. Empty diagnostics do not certify PDF fidelity. GIF animation, audio/video and host-native controls are not established by this raster slice.

Selected exact source backend: Craft PDF at `0a9d1f48da9e0e61a2c7b602ba96ef1c4672bcd9`, raster only, with six matching root patches for hayro, hayro-interpret, hayro-syntax, hayro-jbig2, hayro-cmap and hayro-postscript. Dependency-local patches are ignored by Cargo; the consuming root must repeat the reviewed source pins. The independent graph contains143packages, one egui0.36.2 and no eframe/WebView/FFmpeg runtime. Metadata is not a binary-size measurement. Consumer adoption requires exact-source all-platform Rust1.95 tests/fmt/strictClippy, dependency/notices review and a hosted release-size/graph delta.

## Audio/video continuation

Use the same immutable source boundary for Craft audio/media at0a9d1f48 and individually gated FilmCraft codecs at4a56be9c. Do not import the upstream codec hub/UI or its egui0.37 fork. Actual MP3/AAC and H.264 source examples are required minimum corpus members, not a replacement for the original WebM/Ogg and supported MIME/codec table. Preserve explicit unsupported-codec states and original bytes; do not silently remove a previously agreed original format from release acceptance.

The playback layer must provide bounded decoder queues, a latest-frame video queue, device-consumed sample position, Play/Pause/Seek/Volume/End and exact bounded source segments. No GUI elapsed-time simulation, decoder/database work in the audio callback, unbounded sample cache or stale output across project changes. Device absence/error is visible and actionable. Seeking must handle codec preroll, presentation timestamps and actual decoded positions without silently targeting another segment.

Output candidateCPAL0.15.3 uses older CoreAudio-rs0.11 and default WASAPI/ALSA backends, avoiding silently adopting the newer CPAL0.18.2 documentation's macOS14.2 baseline. Keep the app's14.0 deployment target, default native backends and actual native validation; do not claim minimum-OS runtime acceptance from a compile onmacOS15. Disable ASIO/JACK/realtime optional features. Linux ALSA development/runtime dependencies must be reflected in hosted builds and .deb verification before adoption.

## Licenses and acceptance

This app and the reusable wrapper are MIT. Craft PDF/audio/media, FilmCraft and Hayro retain MIT OR Apache-2.0 terms plus embedded font/CMap notices. Symphonia0.6.1 (MP3 and other decoding) is MPL-2.0; retain its unmodified source/license reference. CPAL is Apache-2.0. Primary immutable manifests/contracts and actual resolved license metadata are recorded in the media repository; no upstream license is replaced by the wrapper license.

Before closing #10, exercise the actual compiled normal-app Canvas media controls/resource navigation, PDF page/open and original MP3/MP4/WebM/Ogg supported journeys, cancellation/stale/failure/no-device cases, source-segment endpoints, restart and private reference compatibility. Media controls own their input rather than dragging Canvas cards. Independent design review covers compact/wide Light/Dark and keyboard reachability. Provider tests, source preservation or archive checks alone do not close native full-project acceptance.
