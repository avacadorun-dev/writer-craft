# Native editor typography and proofing

Status: implementation checkpoints; full native UX acceptance pending. Source contracts: E14/E15/E19 in the editor parity matrix; exact original values are read from `src/shared/lib/preferences.ts` and `src/shared/styles/globals.css` in the frozen web edition.

## Preferences and content boundary

`editor.json` version1 is separate from appearance, interface language, workspace SQLite and author projects. It retains Sofia Sans/Inter/Source Serif4; 17/19/21 logical-pixel sizes; line-height1.5/1.75/2; paragraph .65/.9/1.1em; list0/.25/.5em; width60/70/80ch; toolbarAuto/Expanded/Compact; spelling, word goal and source-autoplay intent. First launch uses original-editor defaults. An existing valid native font size16–28 migrates exactly, including custom values not offered by the original presets.

Malformed, unsupported future-version, oversized or externally changed preference files remain intact. The current process may use safe defaults, but a blocked store refuses persistence and settings controls explain that repair/reload is required. Known current-version unknown fields survive a normal update. Saving preferences never writes canonical manuscript JSON, revisions, notes or Undo history.

The generic engine supplies an exact zero-glyph advance for `ch`, optional independent list spacing and an explicit font-size rather than font-metric line-height basis. Standard ligatures are explicitly disabled; Sofia's language-neutral native glyph shaping is tested against explicit `locl=0`. Font source versions, OFL notices and hashes are in `assets/fonts/NOTICE.md` and `manuscript-fonts.json`. Font coverage does not prove IME or shaping acceptance.

## Passive spelling

The macOS host uses the safe existing `objc2-app-kit`0.3.2 `NSSpellChecker` bindings, with process-local automatic language detection and the user's system spelling languages. Interface language is never treated as manuscript language. It does not invoke an external model, change system language preferences or autocorrect author text. The unsafe raw-pointer overload is not used.

Proofing consumes exact measured visible-line ranges from the editor. It analyzes only visible paragraph/heading text windows, preserves Unicode scalar positions across styled runs, masks inline code and nontext atoms with spaces, excludes code blocks and never feeds atom source/captions or private note bodies into the reader-text check. An exact native UTF-16 range must map to scalar boundaries; partial-surrogate or stale results are refused.

Native requests are progressive: at most eight bounded lexical chunks per frame, with a three-millisecond scheduling budget between service calls and a64-entry cache tied to the current semantic revision. OS service latency for one call remains a native qualification concern; the budget is not a false preemption guarantee. Uncommitted IME text is not analyzed. A service failure clears passive results, exposes unavailable state and leaves normal writing and notes usable. Pathological unbroken lexical tokens exceeding the bounded request window are not sent as misleading word fragments.

The generic passive underline channel has no activation identity and does not take pointer focus, replace selection or become canonical formatting. Notes retain their independent authority and interactions. Non-macOS spelling is explicitly unavailable in this checkpoint; Windows/Linux installer success is not proof of a corresponding native spelling service.

## Required remaining evidence

Hosted semantic tests qualify exact line advances, independent list gaps, viewport text ranges, style coverage, source localized-form equivalence, Unicode proofing conversion/exclusions/cache invalidation and preference preservation. Actual compiled native review must verify typography across all families/sizes/spacing choices, eight UI languages, both appearances/palettes, long-document scrolling, OS spelling with real language dictionaries, CJK/emoji fallback and IME. No test or source checkpoint marks these native gates accepted.
