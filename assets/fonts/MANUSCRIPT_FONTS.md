# Native manuscript fonts

The original editor offers Sofia Sans, Inter and Source Serif 4. Native writing retains all three choices with bundled Latin/Cyrillic styles and a separately acknowledged Noto Sans CJK fallback.

- Sofia Sans 4.101: five unmodified full TrueType faces (Regular, Bold, Italic, Bold Italic, SemiBold), official Lettersoup source revision [`9a1f0ba30f0139b011f9e69c9c462728fa5ef725`](https://github.com/lettersoup/Sofia-Sans/tree/9a1f0ba30f0139b011f9e69c9c462728fa5ef725/fonts/ttf/Normal), SIL OFL 1.1 in `SofiaSans-LICENSE.txt`.
- Inter 4.001 `git-9221beed3`: the official [4.1 release archive](https://github.com/rsms/inter/releases/tag/v4.1) contains this precise internal build. Its Regular SHA256 exactly matches the already bundled WordCraft face, so only the missing Bold, Italic and Bold Italic faces are added. Existing Regular/Medium/SemiBold are reused. The archive SHA256 and per-face hashes are in `manuscript-fonts.json`; original OFL is in `Inter-LICENSE.txt`.
- Source Serif 4: the pinned WordCraft dependency already bundles Regular, Italic, Semibold, Bold and Bold Italic. No duplicate rich-editor registration is needed. The separate regular file retained here also serves the existing egui plain-text fallback.
- Noto Sans CJK JP Regular: existing bundled, explicitly registered CJK fallback. Cmap coverage is checked separately from Western-family style coverage; this does not pretend CJK has a bundled italic/bold face.

No font is downloaded or installed into the operating system at runtime. Host registration is idempotent and performed before layout. Existing ambient fallback is not silently disabled: unsupported scripts and emoji need their actual native fallback behavior qualified independently. Coverage checks are not shaping, IME or accessibility acceptance.

The source browser disables standard ligatures and Sofia localized forms. Standard ligatures have an explicit native projection setting. Language-neutral shaping equivalence to `locl=0` remains a named qualification step rather than a no-op setting. Font files remain unmodified; no reserved font names are changed.
