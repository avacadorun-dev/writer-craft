# Desktop UX contract

WriterCraft is a local Rust application with one coherent writing workspace. The visual direction uses the compact charcoal chrome, restrained blue emphasis and controlled panes seen in the user's SolidCraft references. Writing remains the central activity. CAD symbols and workflows are not copied.

## Composition

- Keep operating-system window controls and a single shared application command bar. Do not draw a second titlebar or a rounded container around the whole application.
- Toolbar: 46–52 logical px, project/document identity on the left and utility commands on the right. No address, browser history, reload or server connection chrome.
- Navigator: resizable, 244 px default and approximately 200–340 px limits. Show a quiet project tree with compact rows, document hierarchy, notes, dictionary, research and boards when those views are implemented. Do not create a card for every item.
- Editor: uninterrupted surface, centered text measure around 680–760 px where space permits, body text around 20 px with comfortable line spacing. Document whitespace provides separation. The editor is not enclosed in another floating application frame.
- Inspector: optional, resizable, 350 px default. Assistant, Notes and Versions are peer views. One-pixel structural dividers connect panes; local scroll areas contain each pane's content.
- Status strip: 24–28 px, word count and local save state. Saving failures remain visible and actionable.
- At smaller window sizes collapse the inspector before the navigator. The current writing surface retains a useful width. Persist panel sizes and chosen view when persistence supports them.

## Theme tokens

Appearance and palette are independent settings. Light, Dark and System modes exist from the first release. The initial default is Dark with Graphite and Blue. `theme.rs` resolves a semantic `Tokens` value; every component uses those colors rather than fixed global constants. System mode follows current OS appearance, including later changes. Settings persistence is owned by the application.

Retain these six current Writers Studio palettes in both modes: Кедр и медь (`cedar-and-copper`), Сухая роза и вино (`dried-rose-and-wine`), Туман и графит (`fog-graphite`), Лён и глина (`linen-clay`), Пыльная лаванда (`dusty-lavender`), Фарфор и чернила (`porcelain-ink`). Add Графит и синий (`solid-craft-blue`) for the new reference direction. Exclude the green Тихая редакция (`quiet`) and Олива и чернила (`olive-ink`) palettes. Obsolete `pine-copper` and `moss-wine` CSS aliases are not additional selectable palettes. Success indicators may remain semantically green.

Source of truth: the final overrides in the original `src/shared/styles/globals.css`, and selectable names in `src/shared/lib/preferences.ts` / `src/pages/settings/ui/editor-theme-picker.tsx`. Original light/dark background and accent colors are retained. Secondary text is moved minimally toward primary text if needed to keep at least 4.5:1 contrast over normal, raised and selected surfaces. Solid accent commands use `on_accent`, chosen for readable contrast. The following values are the **default dark blue sample**, not colors to hard-code in views.

| Token | RGB / hex | Role |
| --- | --- | --- |
| `bg` | `25,28,33` / `#191C21` | Main command chrome and deep input background |
| `panel` | `33,37,43` / `#21252B` | Navigator and inspector |
| `editor` | `28,31,36` / `#1C1F24` | Writing surface |
| `surface` | `43,49,58` / `#2B313A` | Raised controls, inactive button fill |
| `text` | `232,236,243` / `#E8ECF3` | Main text |
| `muted` | `157,168,185` / `#9DA8B9` | Supporting text |
| `border` | `55,63,75` / `#373F4B` | Structural separators |
| `accent` | `109,164,255` / `#6DA4FF` | Primary action, focus, links |
| `accent_soft` | `38,61,96` / `#263D60` | Selected row and quiet accent fill |
| `good` | `122,202,166` / `#7ACAA6` | Confirmed successful state |
| `warning` | `235,190,112` / `#EBBE70` | Actionable nonfatal issue |

Use UI body 14.5 px, button labels 14 px, metadata 12 px, headings 21 px. Main writing font is a named serif family with Cyrillic coverage; UI uses a readable proportional family. Load bundled fonts before applying named families. Follow a 4/8/12/16/24 spacing scale. Controls have 6 px corners; structural panes have square boundaries. Use 1 px dividers and visible keyboard focus. Do not place low-contrast text on similarly colored blue surfaces.

Settings should feel like a tool's preferences: stable navigation/categories and aligned labelled fields, restrained descriptions, explicit connection-test results, and a clear save/apply action where required. Avoid a dashboard of decorative cards or a full-page marketing layout. Unsupported options must not resemble working settings.

## Theme organization and Craft comparison

Keep the serializable `ThemeSettings` (appearance/palette) independent of egui visuals. A pure `resolve` function serves previews and tests; `apply` installs both light and dark egui styles; `tokens` follows the current system appearance. Contrast tests cover all 14 explicit mode/palette combinations and both System resolutions, including selected text and accent button foregrounds. This verifies semantic colors; actual rendering, focus and screenshots remain required.

[craft-ui](https://github.com/storytold/craft-libs/blob/main/crates/ui/README.md) supplies theme mechanisms while each application supplies its palette, fonts and icon catalog. WriterCraft follows that boundary locally rather than importing a suite-wide visual identity. Direct dependency is deferred because the inspected Craft helper stack targets egui 0.37 plus suite patches while this MVP uses published 0.36.2. See `../research/ARTCRAFT.md` for the inspected source and dependency decisions.

## Icon contract

All utility icons come from `icons.rs`: native egui painter paths normalized to a 20 px square, drawn at 18–20 px with a nominal 1.6 px stroke. This system is independent of installed symbol fonts. Never use emoji or Unicode font symbols as fallback UI icons. A microphone, search, document, folder and settings icon have ordinary functional meanings; there are no decorative sparkles or mascot controls.

`icon` allocates a decorative symbol. `icon_button` uses an actual egui button, preserves keyboard activation, paints hover/focus, and supplies an accessible name plus tooltip. Icon-only primary actions need nearby text or a familiar context. Disabled states must not conceal why an action is unavailable.

## Commands and focus

UI, keyboard, command palette and future CLI/MCP surfaces must dispatch the same core operation. The UI does not duplicate mutation logic.

| Intent | Expected behavior |
| --- | --- |
| New/open project | Native file dialog or explicit local project creation; no account requirement |
| Save | Durable local operation with visible result; no invented success after an error |
| Search | Focus document search, or project search when explicitly selected |
| Assistant | Toggle inspector, preserving editor caret/selection |
| Apply proposal | Validate source revision/text, then commit through the core |
| Undo application | Restore only when the post-apply guard is still valid |
| Stop | Cancel active generation and prevent late events from applying edits |
| Export | Show supported formats only; errors preserve the current project |

Use Cmd on macOS and Ctrl elsewhere. Keep conventional Open/Save/Find/Undo/Redo shortcuts, and do not intercept them while a text field or modal owns the event. Escape dismisses the innermost transient surface first. Panel toggles restore previous pane widths. Source links navigate to the source node and select the referenced quote without discarding the current draft.

## Assistant presentation

Show the actual provider, current scope and active status. Local deterministic checks are labelled `Локальная проверка`; they must not be presented as an LLM response. Before any mutation show `Предлагаемые изменения · не применены`, compare `Исходный текст` with `Предложение`, and expose `Применить изменения` / `Отклонить`. Applied state offers `Отменить применение`. Stale or incomplete output cannot silently overwrite the manuscript. See `../research/ASSISTANT.md`.

## Verification

Inspect real native screenshots at multiple window widths, with and without the inspector. Verify Cyrillic text, keyboard selection, paste, undo/redo, focus visibility, scroll containment and icon coverage. Check empty, loading, no-provider, cancelled, stale-proposal and save-error states. A successful compiler run does not establish visual quality or rich-editor parity.

This egui UI is native-rendered custom application chrome. egui explicitly does not aim to produce system-native-looking controls: [official README](https://github.com/emilk/egui#non-goals). AppKit-style integration and accessibility behavior require their own verification.
