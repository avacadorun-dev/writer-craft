# Original Canvas authoring baseline

Issue [#17](https://github.com/avacadorun-dev/writer-craft/issues/17), alpha.3. Audited against native `408a9c91d233207d610ebd76be314f2768fcc4fd` on 2026-11-09. This is the initial source inventory, **not an exhaustive-inventory sign-off or native acceptance**. The independent audit must split rows further if a grouped behavior hides missing operations. Source working-tree byte hashes are recorded in `PARITY_SCOPE.json`; a clean source commit is not assumed.

Source locations: `src/widgets/canvas-board/ui/canvas-board.tsx` (GFM346, views1347, creation1489, clipboard1674, layout1801, types/promotion2022, output2445, tools3124–3375); `src/pages/canvas/ui/canvas-page.tsx:221` (management); `src/entities/canvas/model/templates.ts:27` (templates); `src/features/canvas-schema-editor/ui/canvas-schema-editor.tsx:36` (custom schemas). Line numbers are navigation aids; file hashes identify the inspected baseline. Known source bugs are corrected, not copied.

| ID | Required original behavior | Complete native acceptance |
|---|---|---|
| B01 | Direct card text creation, input and rendered GFM | Pending |
| B02 | Document cards, exact target opening and promotion to document | Pending |
| B03 | URL cards and preview/navigation | Pending |
| B04 | Image creation, display and original assets | Pending |
| B05 | PDF creation, page navigation and display | Pending |
| B06 | Audio/video creation and playback | Pending |
| B07 | Document, URL and file drag-and-drop | Pending |
| B08 | Pen, highlighter and eraser | Pending |
| B09 | Rectangle, ellipse, diamond, line and arrow | Pending |
| B10 | Groups and presentation frames | Pending |
| B11 | Selection, movement, sizing and rotation | Pending |
| B12 | Structured Copy/Cut/Paste and duplicate | Pending |
| B13 | Colors, locking and layering | Pending |
| B14 | Alignment, distribution and tidy | Pending |
| B15 | Connections and labels | Pending |
| B16 | Card/relation types and project-wide schema editing | Pending |
| B17 | All fifteen existing starting templates | Pending |
| B18 | Free, mind-map, timeline, swimlane, dependency and filtered views | Pending |
| B19 | Automatic-layout preview and Apply | Pending |
| B20 | Object navigator, search and diagnostics | Pending |
| B21 | Dialogue, quest and state playthrough | Pending |
| B22 | Frame presentation and focus timer | Pending |
| B23 | Board rename, duplicate, trash and restore | Pending |
| B24 | .canvas import and editable JSON Canvas export | Pending |
| B25 | PNG, SVG and PDF output | Pending |
| B26 | Undo/Redo, save/reopen, pending/failure/cancel and recovery | Pending |

Every applicable row needs a reachable native entry point and create → configure/edit → render → delete → Undo/Redo → save/reopen evidence. Include keyboard/IME/clipboard, selection, zoom/clipping, stale/failure/cancel/recovery, old/custom schemas, IDs and unknown-field/media preservation. An imported card or a model-only command cannot pass authoring. A PNG/PDF output is not an editable backup.

#15 remains the first bounded implementation slice (B01/B26); #10 covers whole-project compatibility and the private reference fixture; #14 covers researched additional relationships and sizing. Existing document references, promotion and custom types are baseline obligations, not newly researched features. Do not close #17 after #15 alone.
