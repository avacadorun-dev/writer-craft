# Compiled home body correction captures

Source: `8c61a8872810c7e1a5b84bd7bf329f8a9a268a1a`, 2026-11-12. These PNGs are actual native application readbacks from the user-authorized local debug build, not generated concepts. All projects/content are new synthetic fixtures. `manifest.json` binds source, executable and image hashes to observed viewport, palette and appearance. Sidecars preserve native capture metadata and explicitly do not accept native interactions.

The previous 399a477 preview exposed P1 left/right clipping beneath the navigator. Corrections clamp centered width to the real body, fix the text-column allocation, restore trailing section actions, increase the local heading hierarchy and align the visible scribe with the banner's right/bottom contour. Transparent PNG export margins are trimmed at decode; source artwork bytes, neutral pigment and alpha are preserved.

- `blue-light.png`: blue palette, Light, observed 1440 by 923 points at 2x.
- `copper-dark.png`: Cedar and copper, Dark, observed 1440 by 923 points at 2x.
- `compact.png`: blue palette, Dark, observed 1000 by 680 points at 2x.

Each sidecar reports unchanged canonical synthetic content. The owner is exploring a separate isolated `WriterCraft Home Contour` copy; preserve that running window and its database. The local build/launch exception was explicitly authorized after GitHub Verify43097727701 and Preview43097731613 ended in startup_failure without jobs or artifacts. Hosted tests/Clippy and the remaining full #18 native state/locale/keyboard matrix remain pending. No release or issue closure follows from these limited visual captures.
