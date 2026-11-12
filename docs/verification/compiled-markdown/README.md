# Compiled Markdown manuscript — native captures

Synthetic demo project (`--demo`) in an isolated throwaway database; no owner manuscript entered these runs.

Evidence files:

- `01-export-mode-ru.png` and `01-export-mode-ru.capture.json` — manuscript export mode with scope selector, folder name and the preview action, Russian catalog. Command: `WRITERCRAFT_CAPTURE_LANGUAGE=ru target/debug/writercraft --database <tmp>/workspace.sqlite --demo --capture 01-export-mode-ru.png --capture-transfer`.
- `02-order-preview-en.png` and `02-order-preview-en.capture.json` — order preview with the disclosed reading scope, English catalog, whole-project scope. Command: same with `--capture-transfer-preview`.

Both runs report `status: captured`, `canonical_unchanged: true`, 11 stable frames, one readback request and a 2880x1846 pixel viewport at 2.0 pixels per point. The committed PNGs are downscaled to 1600x1026 for repository size; the capture JSON keeps the original pixel size.

These are engineering captures of settled native output. They are not interaction, owner or independent UX acceptance, and `editor_acceptance`/`native_interaction_acceptance` remain false in both JSON files. The binary that produced them is `target/debug/writercraft` SHA-256 `fc9fcf329f1fe88ddbdd2300761a13cdad35c842223387d43410bf06bab0c07a`.
