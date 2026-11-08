//! Isolated engineering preview: canonical rich session, durable drafts and native canvas.
//! This integration does not assert completion of the full-editor-parity release gate.
use super::*;
use craft_editor_egui::{EditorSession, MarkState, canvas::CanvasStyle};
use craft_editor_model::{
    Command, Document as RichDocument, Mark, Node, NodePath, NodeRef, Revision,
    tables::{self, CellRect, CellSelection, TableGrid},
};
use serde_json::{Map, json};

pub(super) struct RichState {
    pub session: EditorSession,
    draft_version: Option<String>,
    pub recovered: bool,
    read_only: bool,
    last_diagnostic: Option<String>,
}

impl RichState {
    pub fn open(workspace: &Workspace, document: &Document) -> AppResult<Option<Self>> {
        let Some(content) = &document.rich_json else {
            return Ok(None);
        };
        let draft = workspace.rich_draft(&document.id)?;
        if draft
            .as_ref()
            .is_some_and(|draft| draft.base_revision != document.revision)
        {
            return Err(studio_core::StudioError::DraftConflict {
                draft_revision: draft.as_ref().map_or(0, |draft| draft.base_revision),
                current_revision: document.revision,
            }
            .into());
        }
        let tree = RichDocument::from_json(
            draft
                .as_ref()
                .map_or(content, |draft| &draft.content)
                .clone(),
        )?;
        let read_only = document.read_only || wider_than_native_limit(tree.root());
        let recovered = draft
            .as_ref()
            .is_some_and(|draft| &draft.content != content);
        let mut session = EditorSession::new(tree, Revision(0));
        session.options.id = egui::Id::new(("rich-manuscript", &document.id));
        Ok(Some(Self {
            session,
            draft_version: draft.map(|draft| draft.version),
            recovered,
            read_only,
            last_diagnostic: None,
        }))
    }

    fn write_draft(&mut self, workspace: &Workspace, document: &Document) -> AppResult<()> {
        let draft = workspace.save_rich_draft(
            &document.id,
            document.revision,
            self.draft_version.as_deref(),
            self.session.document().to_json()?,
        )?;
        self.draft_version = Some(draft.version);
        Ok(())
    }
}

impl StudioApp {
    pub(super) fn create_editor_example(&mut self) -> AppResult<()> {
        let project = self.workspace.create_project("Editor Preview")?;
        self.workspace.create_rich_document(
            &project.id,
            "01. Native tables and formatting",
            serde_json::from_str(include_str!("editor_fixture.json"))?,
        )?;
        self.workspace.create_rich_document(
            &project.id,
            "02. A blank manuscript",
            RichDocument::plain(uuid::Uuid::new_v4().to_string(), "")?.to_json()?,
        )?;
        self.projects = self.workspace.list_projects()?;
        Ok(())
    }

    pub(super) fn save_rich_editor(&mut self) -> AppResult<()> {
        let document = self.document.as_ref().ok_or("No active document")?;
        let rich = self.rich.as_mut().ok_or("No active rich editor")?;
        // A pending native preedit belongs to the input method, not canonical storage.
        rich.write_draft(&self.workspace, document)?;
        let saved = self.workspace.save_rich_document(
            &document.id,
            document.revision,
            rich.session.document().to_json()?,
        )?;
        rich.draft_version = None;
        rich.recovered = false;
        self.body.clone_from(&saved.body);
        self.document = Some(saved);
        if let Some(project) = &self.project {
            self.documents = self.workspace.list_documents(&project.id)?;
        }
        self.dirty = None;
        self.status = "All changes saved".into();
        Ok(())
    }

    pub(super) fn rich_editor(&mut self, ui: &mut egui::Ui, t: Tokens) {
        let l = self.language;
        let Some(mut rich) = self.rich.take() else {
            return;
        };
        let before = rich.session.revision();
        let mut operation_error: Option<Box<dyn std::error::Error + Send + Sync>> = None;
        egui::Panel::top("rich-formatting")
            .exact_size(44.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(t.editor)
                    .inner_margin(egui::Margin::symmetric(24, 5))
                    .stroke(Stroke::new(1.0, t.border)),
            )
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    ui.add_enabled_ui(!rich.read_only && !rich.session.is_composing(), |ui| {
                        let kind = current_block(&rich.session)
                            .map_or("paragraph", |node| node.node_type.as_str());
                        let label = if kind == "heading" {
                            "Heading"
                        } else {
                            "Paragraph"
                        };
                        ui.menu_button(l.text(label), |ui| {
                            for (msgid, level) in [
                                ("Paragraph", 0),
                                ("Heading 1", 1),
                                ("Heading 2", 2),
                                ("Heading 3", 3),
                            ] {
                                if ui.button(l.text(msgid)).clicked() {
                                    if let Err(error) = set_block_kind(&mut rich.session, level) {
                                        operation_error = Some(error);
                                    }
                                    rich.session.request_canvas_focus(ui.ctx());
                                    ui.close();
                                }
                            }
                        });
                        ui.separator();
                        let can_format = rich.session.selection().is_some();
                        for (mark, glyph, msgid) in [
                            ("bold", "B", "Bold"),
                            ("italic", "I", "Italic"),
                            ("underline", "U", "Underline"),
                        ] {
                            let state = rich.session.mark_state(mark).unwrap_or(MarkState::Off);
                            let active = state == MarkState::On;
                            let mut text = RichText::new(glyph).size(16.0);
                            if mark == "bold" {
                                text = text.strong();
                            }
                            if mark == "italic" {
                                text = text.italics();
                            }
                            if mark == "underline" {
                                text = text.underline();
                            }
                            if ui
                                .add_enabled(
                                    can_format,
                                    egui::Button::new(text)
                                        .selected(active)
                                        .stroke(Stroke::new(
                                            if state == MarkState::Mixed { 1.0 } else { 0.0 },
                                            t.accent,
                                        ))
                                        .min_size(egui::vec2(30.0, 30.0)),
                                )
                                .on_hover_text(if state == MarkState::Mixed {
                                    format!("{} · {}", l.text(msgid), l.text("Mixed formatting"))
                                } else {
                                    l.text(msgid).into()
                                })
                                .clicked()
                            {
                                if let Err(error) = rich.session.toggle_mark(Mark::new(mark)) {
                                    operation_error = Some(error.into());
                                }
                                rich.session.request_canvas_focus(ui.ctx());
                            }
                        }
                        ui.separator();
                        ui.menu_button(l.text("Table"), |ui| {
                            if ui.button(l.text("Insert table · 3 × 3")).clicked() {
                                if let Err(error) = insert_table(&mut rich.session) {
                                    operation_error = Some(error);
                                }
                                rich.session.request_canvas_focus(ui.ctx());
                                ui.close();
                            }
                            let context = TableContext::current(&rich.session);
                            ui.add_enabled_ui(context.is_some(), |ui| {
                                for (msgid, action) in [
                                    ("Add row below", TableAction::AddRow),
                                    ("Add column right", TableAction::AddColumn),
                                    ("Delete row", TableAction::DeleteRow),
                                    ("Delete column", TableAction::DeleteColumn),
                                    ("Merge selected cells", TableAction::Merge),
                                    ("Split cell", TableAction::Split),
                                ] {
                                    let allowed = context
                                        .as_ref()
                                        .is_some_and(|context| context.allows(action));
                                    if ui
                                        .add_enabled(allowed, egui::Button::new(l.text(msgid)))
                                        .clicked()
                                    {
                                        if let Some(context) = &context
                                            && let Err(error) =
                                                context.apply(&mut rich.session, action)
                                        {
                                            operation_error = Some(error);
                                        }
                                        rich.session.request_canvas_focus(ui.ctx());
                                        ui.close();
                                    }
                                }
                            });
                        });
                        ui.separator();
                        if ui
                            .add_enabled_ui(rich.session.can_undo(), |ui| {
                                icons::icon_button(ui, Icon::Undo, l.text("Undo"))
                            })
                            .inner
                            .clicked()
                        {
                            if let Err(error) = rich.session.undo() {
                                operation_error = Some(error.into());
                            }
                            rich.session.request_canvas_focus(ui.ctx());
                        }
                        if ui
                            .add_enabled_ui(rich.session.can_redo(), |ui| {
                                icons::icon_button(ui, Icon::Redo, l.text("Redo"))
                            })
                            .inner
                            .clicked()
                        {
                            if let Err(error) = rich.session.redo() {
                                operation_error = Some(error.into());
                            }
                            rich.session.request_canvas_focus(ui.ctx());
                        }
                        let more = icons::icon_button(ui, Icon::More, l.text("More formatting"));
                        egui::Popup::menu(&more).show(|ui| {
                            for (mark, msgid) in
                                [("strike", "Strikethrough"), ("code", "Inline code")]
                            {
                                if ui
                                    .add_enabled(can_format, egui::Button::new(l.text(msgid)))
                                    .clicked()
                                {
                                    if let Err(error) = rich.session.toggle_mark(Mark::new(mark)) {
                                        operation_error = Some(error.into());
                                    }
                                    rich.session.request_canvas_focus(ui.ctx());
                                    ui.close();
                                }
                            }
                        });
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(l.text("Editor Preview"))
                                .small()
                                .color(t.muted),
                        );
                    });
                });
            });
        let mut changed = false;
        egui::ScrollArea::vertical()
            .id_salt(("rich-scroll", rich.session.options.id))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let available = ui.available_width();
                let width = (available - 48.0).clamp(160.0, 820.0);
                ui.horizontal(|ui| {
                    ui.add_space(((available - width) * 0.5).max(0.0));
                    ui.vertical(|ui| {
                        ui.set_width(width);
                        ui.add_space(24.0);
                        if rich.read_only {
                            ui.label(
                                RichText::new(l.text("Original format · read only"))
                                    .color(t.warning),
                            );
                        }
                        let style = CanvasStyle {
                            foreground: t.text,
                            background: t.editor,
                            selection: t.accent,
                            caret: t.accent,
                            table_border: t.border,
                            table_header: t.panel,
                            font_size: self.font_size,
                            font_family: Some("Source Serif 4".into()),
                            line_height: 1.65,
                            paragraph_spacing: 14.0,
                            min_height: 500.0,
                            read_only: rich.read_only,
                        };
                        let output = rich.session.show_canvas(ui, &style);
                        if self.focus_manuscript {
                            output.response.request_focus();
                            self.focus_manuscript = false;
                        }
                        changed = !output.changes.is_empty();
                        if let Some(error) = output.errors.into_iter().next() {
                            let diagnostic = error.to_string();
                            if rich.last_diagnostic.as_ref() != Some(&diagnostic) {
                                rich.last_diagnostic = Some(diagnostic);
                                operation_error = Some(error.into());
                            }
                        } else {
                            rich.last_diagnostic = None;
                        }
                        ui.add_space(60.0);
                    });
                });
            });
        if changed || rich.session.revision() != before {
            self.body = rich.session.document().plain_text();
            self.dirty = Some(Instant::now());
            self.status = "Saving…".into();
            if let Some(document) = &self.document {
                let result = rich.write_draft(&self.workspace, document);
                self.autosave_paused = result.is_err();
                self.record_error(result);
            }
        }
        self.rich = Some(rich);
        if let Some(error) = operation_error {
            self.record_error(Err(error));
        }
    }
}

fn current_block(session: &EditorSession) -> Option<&Node> {
    session
        .document()
        .block(&session.selection()?.head.block_id)
        .ok()
}

fn find_path(node: &Node, id: &str, path: NodePath) -> Option<NodePath> {
    if node.block_id() == Some(id) {
        return Some(path);
    }
    node.children()
        .iter()
        .enumerate()
        .find_map(|(index, child)| find_path(child, id, path.child(index)))
}

fn set_block_kind(session: &mut EditorSession, level: u8) -> AppResult<()> {
    let selection = session.selection().ok_or("No selection")?;
    let path = find_path(
        session.document().root(),
        &selection.head.block_id,
        NodePath::root(),
    )
    .ok_or("Missing text block")?;
    let target = session.document().node_ref(&path)?;
    let mut replacement = target.expected.as_ref().clone();
    replacement.node_type = if level == 0 { "paragraph" } else { "heading" }.into();
    if level == 0 {
        if let Some(attrs) = &mut replacement.attrs {
            attrs.remove("level");
        }
    } else {
        replacement
            .attrs
            .get_or_insert_default()
            .insert("level".into(), json!(level));
    }
    let command = session.document().replace_node(&target, replacement)?;
    session.apply_commands(vec![command])?;
    Ok(())
}

fn wider_than_native_limit(node: &Node) -> bool {
    (node.node_type == "table" && TableGrid::new(node).is_ok_and(|grid| grid.columns > 63))
        || node.children().iter().any(wider_than_native_limit)
}

fn fresh_cell(header: bool) -> Node {
    tables::empty_table_cell(&uuid::Uuid::new_v4().to_string(), header)
}

fn insert_table(session: &mut EditorSession) -> AppResult<()> {
    let index = session
        .selection()
        .and_then(|selection| {
            find_path(
                session.document().root(),
                &selection.head.block_id,
                NodePath::root(),
            )
        })
        .and_then(|path| path.0.first().copied())
        .map_or(session.document().blocks().len(), |index| index + 1);
    let rows = (0..3)
        .map(|row| {
            Node::block(
                "tableRow",
                Map::new(),
                (0..3).map(|_| fresh_cell(row == 0)).collect(),
            )
        })
        .collect();
    let table = Node::block(
        "table",
        Map::from_iter([("blockId".into(), json!(uuid::Uuid::new_v4().to_string()))]),
        rows,
    );
    let parent = session.document().node_ref(&NodePath::root())?;
    session.apply_commands(vec![Command::InsertChildren {
        parent,
        index,
        children: vec![table],
    }])?;
    Ok(())
}

#[derive(Clone, Copy)]
enum TableAction {
    AddRow,
    AddColumn,
    DeleteRow,
    DeleteColumn,
    Merge,
    Split,
}
struct TableContext {
    target: NodeRef,
    grid: TableGrid,
    cell: CellRect,
    selected: Option<CellSelection>,
}
impl TableContext {
    fn at(session: &EditorSession, block_id: &str) -> Option<(NodePath, usize, usize)> {
        let path = find_path(session.document().root(), block_id, NodePath::root())?;
        for length in (0..path.0.len()).rev() {
            let ancestor = NodePath(path.0[..length].to_vec());
            if session.document().node(&ancestor).ok()?.node_type == "table" {
                return Some((ancestor, *path.0.get(length)?, *path.0.get(length + 1)?));
            }
        }
        None
    }
    fn current(session: &EditorSession) -> Option<Self> {
        let selection = session.selection()?;
        let (path, row, cell_index) = Self::at(session, &selection.head.block_id)?;
        let target = session.document().node_ref(&path).ok()?;
        let grid = TableGrid::new(&target.expected).ok()?;
        let cell = grid
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.cell_index == cell_index)?
            .clone();
        let selected =
            Self::at(session, &selection.anchor.block_id).and_then(|(anchor, row, index)| {
                if anchor != path {
                    return None;
                }
                let first = grid
                    .cells
                    .iter()
                    .find(|cell| cell.row == row && cell.cell_index == index)?;
                Some(CellSelection {
                    top: first.row.min(cell.row),
                    left: first.column.min(cell.column),
                    bottom: first.bottom().max(cell.bottom()),
                    right: first.right().max(cell.right()),
                })
            });
        Some(Self {
            target,
            grid,
            cell,
            selected,
        })
    }
    fn allows(&self, action: TableAction) -> bool {
        match action {
            TableAction::AddColumn => self.grid.columns < 63,
            TableAction::DeleteRow => self.grid.rows > 1,
            TableAction::DeleteColumn => self.grid.columns > 1,
            TableAction::Merge => self.selected.is_some_and(|selection| {
                (selection.bottom - selection.top) * (selection.right - selection.left)
                    > self.cell.rowspan * self.cell.colspan
            }),
            TableAction::Split => self.cell.rowspan > 1 || self.cell.colspan > 1,
            TableAction::AddRow => true,
        }
    }
    fn apply(&self, session: &mut EditorSession, action: TableAction) -> AppResult<()> {
        if !self.allows(action) {
            return Err("Table operation is unavailable".into());
        }
        let document = session.document();
        let command = match action {
            TableAction::AddRow => {
                let index = self.cell.bottom();
                let covered: usize = self
                    .grid
                    .cells
                    .iter()
                    .filter(|cell| cell.row < index && index < cell.bottom())
                    .map(|cell| cell.colspan)
                    .sum();
                let row = Node::block(
                    "tableRow",
                    Map::new(),
                    (covered..self.grid.columns)
                        .map(|_| fresh_cell(false))
                        .collect(),
                );
                tables::insert_table_row(document, &self.target, index, row)?
            }
            TableAction::AddColumn => {
                let index = self.cell.right();
                let cells = (0..self.grid.rows)
                    .filter(|row| {
                        !self.grid.cells.iter().any(|cell| {
                            cell.row <= *row
                                && *row < cell.bottom()
                                && cell.column < index
                                && index < cell.right()
                        })
                    })
                    .map(|row| fresh_cell(row == 0))
                    .collect();
                tables::insert_table_column(document, &self.target, index, cells)?
            }
            TableAction::DeleteRow => {
                tables::delete_table_row(document, &self.target, self.cell.row)?
            }
            TableAction::DeleteColumn => {
                tables::delete_table_column(document, &self.target, self.cell.column)?
            }
            TableAction::Merge => tables::merge_table_cells(
                document,
                &self.target,
                self.selected.ok_or("No cell selection")?,
            )?,
            TableAction::Split => {
                let count = self.cell.rowspan * self.cell.colspan - 1;
                tables::split_table_cell(
                    document,
                    &self.target,
                    self.cell.row,
                    self.cell.cell_index,
                    (0..count).map(|_| fresh_cell(false)).collect(),
                )?
            }
        };
        session.apply_commands(vec![command])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_editor_egui::{DocPoint, Selection};
    use craft_editor_model::{MarkMode, ScalarOffset};

    fn fixture() -> AppResult<(tempfile::TempDir, StudioApp)> {
        let directory = tempfile::tempdir()?;
        let context = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(context);
        let mut app = StudioApp::new(&creation, directory.path().join("test.sqlite"), false, None)?;
        app.create_editor_example()?;
        let project = app.projects.first().ok_or("Missing project")?.clone();
        app.open_project(project)?;
        Ok((directory, app))
    }

    fn mark_intro(app: &mut StudioApp) -> AppResult<()> {
        let rich = app.rich.as_mut().ok_or("Missing session")?;
        rich.session.set_selection(Selection {
            anchor: DocPoint {
                block_id: "intro".into(),
                offset: ScalarOffset(2),
            },
            head: DocPoint {
                block_id: "intro".into(),
                offset: ScalarOffset(8),
            },
        })?;
        rich.session
            .format_selection(Mark::new("underline"), MarkMode::Add)?;
        app.dirty = Some(Instant::now());
        Ok(())
    }

    #[test]
    fn formatting_save_reopen_preserves_exact_tree_and_live_history() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let original = app.document.as_ref().ok_or("Missing document")?.clone();
        mark_intro(&mut app)?;
        let rich = app.rich.as_ref().ok_or("Missing session")?;
        let expected = rich.session.document().to_json()?;
        let selection = rich.session.selection().cloned();
        app.save()?;
        let saved = app.document.as_ref().ok_or("Missing document")?.clone();
        assert_eq!(saved.body, original.body);
        assert_eq!(saved.revision, original.revision + 1);
        assert_eq!(saved.rich_json.as_ref(), Some(&expected));
        let live = app.rich.as_ref().ok_or("Missing session")?;
        assert!(live.session.can_undo());
        assert_eq!(live.session.selection(), selection.as_ref());
        app.open_document(saved)?;
        assert_eq!(
            app.rich
                .as_ref()
                .ok_or("Missing session")?
                .session
                .document()
                .to_json()?,
            expected
        );
        Ok(())
    }

    #[test]
    fn rich_draft_is_recovered_without_flattening_merged_table() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        mark_intro(&mut app)?;
        let document = app.document.as_ref().ok_or("Missing document")?;
        let rich = app.rich.as_mut().ok_or("Missing session")?;
        rich.write_draft(&app.workspace, document)?;
        let expected = rich.session.document().to_json()?;
        let recovered = RichState::open(&app.workspace, document)?.ok_or("Missing recovery")?;
        assert!(recovered.recovered);
        assert_eq!(recovered.session.document().to_json()?, expected);
        Ok(())
    }

    #[test]
    fn second_rich_writer_cannot_overwrite_acknowledged_draft() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let document = app.document.as_ref().ok_or("Missing document")?;
        let mut second =
            RichState::open(&app.workspace, document)?.ok_or("Missing second session")?;
        app.rich
            .as_mut()
            .ok_or("Missing session")?
            .write_draft(&app.workspace, document)?;
        let result = second.write_draft(&app.workspace, document);
        assert!(result.is_err());
        assert!(app.workspace.rich_draft(&document.id)?.is_some());
        Ok(())
    }

    #[test]
    fn table_changes_use_same_undo_history_as_inline_marks() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let rich = app.rich.as_mut().ok_or("Missing session")?;
        rich.session.set_selection(Selection {
            anchor: DocPoint {
                block_id: "right".into(),
                offset: ScalarOffset(0),
            },
            head: DocPoint {
                block_id: "right".into(),
                offset: ScalarOffset(0),
            },
        })?;
        let before = rich.session.document().to_json()?;
        let context = TableContext::current(&rich.session).ok_or("Missing table target")?;
        context.apply(&mut rich.session, TableAction::AddColumn)?;
        assert_eq!(
            TableContext::current(&rich.session)
                .ok_or("Missing table")?
                .grid
                .columns,
            4
        );
        rich.session.undo()?;
        assert_eq!(rich.session.document().to_json()?, before);
        Ok(())
    }

    #[test]
    fn native_table_limit_blocks_widening_without_changing_canonical_tree() -> AppResult<()> {
        let mut table = Node::block(
            "table",
            Map::from_iter([("blockId".into(), json!("wide-grid"))]),
            vec![Node::block(
                "tableRow",
                Map::new(),
                (0..63).map(|_| fresh_cell(false)).collect(),
            )],
        );
        let paragraph_id = table.children()[0].children()[0].children()[0]
            .block_id()
            .ok_or("Missing cell identity")?
            .to_owned();
        let document = RichDocument::from_root(Node::block(
            "doc",
            Map::from_iter([("schemaVersion".into(), json!(4))]),
            vec![table.clone()],
        ))?;
        let mut session = EditorSession::new(document, Revision(0));
        let point = DocPoint {
            block_id: paragraph_id,
            offset: ScalarOffset(0),
        };
        session.set_selection(Selection {
            anchor: point.clone(),
            head: point,
        })?;
        let context = TableContext::current(&session).ok_or("Missing table")?;
        let before = session.document().to_json()?;
        assert!(!context.allows(TableAction::AddColumn));
        assert!(context.apply(&mut session, TableAction::AddColumn).is_err());
        assert_eq!(session.document().to_json()?, before);
        table.content.as_mut().ok_or("Missing rows")?[0]
            .content
            .as_mut()
            .ok_or("Missing cells")?
            .push(fresh_cell(false));
        assert!(wider_than_native_limit(&table));
        Ok(())
    }
}
