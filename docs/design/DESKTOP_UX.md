# Desktop UX contract

WriterCraft is a local Rust application with one coherent writing workspace. The visual direction uses the compact charcoal chrome, restrained blue emphasis and controlled panes seen in the user's SolidCraft references. Writing remains the central activity. CAD symbols and workflows are not copied.

## Composition

- Keep operating-system window controls and a single shared application command bar. Do not draw a second titlebar or a rounded container around the whole application.
- Toolbar: 46–52 logical px, project/document identity on the left and utility commands on the right. No address, browser history, reload or server connection chrome.
- Navigator: resizable, 244 px default and approximately 200–340 px limits. Show a quiet project tree with compact rows, document hierarchy, notes, dictionary, research and boards when those views are implemented. Do not create a card for every item.
- Editor: continuous writing with centered text measure around 680–760 px where space permits and comfortable line spacing. Per owner correction F-34, a distinct bounded document surface with exterior gutters appears when center width permits; under panel constraint it joins the pane structure. Focus retains the readable maximum measure. This is a document boundary, never another application frame or forced print pagination. See `WORKSPACE_EDITOR_REDESIGN.md` for the proposed adaptive geometry awaiting image review.
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

All utility icons come from one coherent, licensed upstream family selected after the owner reviews an actual same-size comparison (F-25). Lucide, Tabler and Phosphor Regular are candidates; Lucide was an example, not a mandatory choice. Retain unmodified upstream geometry, source/version provenance and license notices for the chosen subset. Render through the native icon adapter, independently of installed symbol fonts; generation-time tessellation into static native meshes is permitted. Qualify the actual 16/20/24px output rather than substituting hand-drawn approximations. Never use emoji or Unicode font symbols as fallback UI icons. A microphone, search, document, folder and settings icon have ordinary functional meanings; there are no decorative sparkles or mascot controls.

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

## Structured desktop preferences — owner correction F-20

Preferences are a single owned desktop surface with a stable category sidebar, a bounded content pane and a footer. The previous small all-in-one form is superseded. The categories below reorganize verified existing Writers Studio capabilities and the explicitly requested native appearance/localization/shortcut behavior; they are not a new feature backlog.

| Category | Working in the foundation | Existing parity scope placed here | Legacy source evidence |
|---|---|---|---|
| General | Interface language, with existing draft/content preserved | Application-level preferences where required by an existing capability | Explicit owner localization requirement; preference ownership in original settings |
| Appearance | Independent Light/Dark/System and seven palettes | Existing theme behavior; keep brightness independent from palette | `src/pages/settings/ui/settings-page.tsx:505`, `editor-theme-picker.tsx` |
| Editor | Manuscript text size | Manuscript font, line/paragraph/list spacing, measure, formatting-toolbar mode, word goal, spelling and source autoplay | Original `settings-page.tsx:395–582` |
| AI & providers | Informational scaffold only | Provider/model/endpoint configuration, actual connection/capability checks, assistant speech checkpoint and existing project step limit | Original `settings-page.tsx:242–315`, `src/pages/assistant-providers/ui/assistant-providers-page.tsx` |
| Agents & MCP | Informational scaffold only | Existing MCP server/client connection and agent configuration | Original `src/pages/mcp-settings/ui/mcp-settings-page.tsx`, linked from `settings-page.tsx:308` |
| Voice | Informational scaffold only | Microphone, transcription language, command prefix, voice-command and live-transcript choices | Original `settings-page.tsx:316–394` |
| Files & recovery | Honest information about current local persistence; no invented backup or repair action | Existing local working data, recovery, backup and audio-retention behavior adapted to local ownership | Original `settings-page.tsx:583–678`; native recovery contract |
| Keyboard shortcuts | Readable table of actual current commands | Existing command bindings as their native workflows become available | Native command contract; original editor command behavior |

The original browser's quota protection and server-account operations cannot be copied literally into a local application. Preserve their actual data/recovery purpose through explicit migration contracts; do not invent an operational native account/delete/cloud control merely to fill a category. Project-specific settings must retain an explicit project scope when they are implemented.

### Geometry and spacing

- Preferred maximum outer size: 860×620 logical pixels, clamped to the viewport with at least 32px clearance on each side. At the 1000×680 minimum workspace, the settings surface must fit rather than rely on off-window controls.
- Header 44px, footer 56px; fixed category region approximately 196px wide. A single 1px divider aligns the sidebar and content boundary. Header/footer remain outside the content scroll area.
- Content padding 24px; category heading 21px; concise supporting text separated by 8px; group gap 24px; field-row gap 16px; label/hint gap 8px; compact subordinate gap 12px. Use the shared 8/12/16/24 scale consistently instead of unrelated arbitrary whitespace.
- Category rows 36–40px with coherent 18–20px outline icons, 12px horizontal inset, visible selected/focus state and complete accessible labels. Truncate a long category label within its allocated width and reveal the complete label; never draw it into content.
- Per owner correction F-35, ordinary single-line fields and dialog actions use a consistent 40px minimum height, full available form width, 12px horizontal/8px vertical field padding and vertically centered text. Segmented choices retain deliberate usable geometry. Align label and input columns when the content width permits; use a deliberate stacked form when a long translated label needs it. Do not create many decorative cards around simple fields.
- Palette choices use a responsive grid of miniature-window preview cards with names. Selection must be visible in both appearances; the grid belongs to Appearance rather than displacing all other preferences.
- The manuscript font-size preference changes manuscript text only. It must not enlarge the settings shell or invalidate category/control geometry.
- Done remains visible at the footer's trailing edge. Working appearance/interface preferences preview and persist immediately; do not display a fake Apply/Save action. Failed persistence remains actionable. Escape closes the innermost transient and returns the prior editor focus.

### Functional settings versus scaffold

Working controls must change actual persisted state. General, Appearance, Editor and the current shortcut table are functional foundation sections. Unimplemented parity sections remain selectable for orientation, but show a plain, short `Not available in this alpha` message and a source-backed summary of the existing capability being migrated. They do not contain editable fake endpoint fields, disabled Connect/Test buttons or switches with no effect. Scaffold visibility is explicitly requested by F-20 and must not imply feature completion.

No search field is required until real category/setting search is implemented. A decorative search box or fabricated connection success is unacceptable. As each category becomes functional, give its real success/error/cancel states their own OpenSpec scenario and independent UX acceptance.

### Settings acceptance after F-20

Review General, Appearance, Editor and at least one informational parity category in actual compiled screenshots. Exercise the sidebar and Done by keyboard/pointer, Escape, language switching, mode/palette choice and persistence while an unfinished draft exists. At minimum/common sizes and German/CJK languages, verify clear hierarchy, aligned controls, visible category state, regular vertical spacing and a reachable footer. Catalog completeness and token contrast alone cannot close the owner's spacing rejection.

### Settings reference refinements

The selected shell uses a left-aligned category hierarchy with a coherent licensed utility icon family. Final family selection follows the comparison and owner participation recorded in F-25; no Lucide preference is assumed. Group labels follow the same inset grid as navigation content. The normal header and footer have explicit heights; content allocation cannot stretch either row. Appearance uses miniature-window mode/palette cards with a selected accent outline and accessible labels. These refinements supersede earlier plain swatch-button presentation. The approved lion writer supersedes the monk/book crest in the command bar and app icon (#16, F-44). The header uses a 32 px visible rounded tile and an 8 px gap to the wordmark; OS exports have their own safe-area padding. Preserve the supplied artwork and original blue texture in both appearance modes.

### Final utility family choice

The owner selected Lucide after the official-SVG comparison and independent designer recommendation (F-26). Use its exact source geometry consistently at 20px for primary utility controls and 16–18px for secondary rows. Retain upstream licenses and immutable source provenance. Native generated geometry must preserve path curves, round caps/joins, consistent weight and accessible command labels. The comparison artifacts informed the choice; actual compiled rendering remains the acceptance evidence.


## Shared form/modal correction — F-35

Search, creation and contextual block dialogs share the field helpers rather than inheriting shallow default TextEdit margins. A modal uses one 24px inset for title/body/footer, a 44px header with aligned title/close center and a separated 40px action row. New document/project actions sit at the trailing edge, with explicit Cancel/Escape behavior. Existing source-preserving failure and focus-return contracts remain unchanged. Native inspection of minimum/common viewports, long localized values, font scaling and every field state is required; source dimensions alone are not acceptance.
