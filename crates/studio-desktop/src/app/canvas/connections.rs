//! Native connection controls over the existing guarded board authority.
use super::*;
use craft_canvas_model::{LabelTarget, Selection};

pub(super) struct ConnectionEditor {
    edge_id: String,
    base: ContentIdentity,
    durable_revision: u64,
    expected: String,
    label: String,
    focus: bool,
    submitted: bool,
    acknowledged: bool,
    visible: bool,
}

impl ConnectionEditor {
    pub(super) fn is_open(&self) -> bool {
        self.visible
    }
}

impl CanvasState {
    pub(super) fn connection_tools(
        &mut self,
        ui: &mut egui::Ui,
        database: &Path,
        language: Language,
    ) {
        let l = language;
        if let Some(editor) = &mut self.connection_editor {
            if !editor.visible && ui.button(l.text("Resume connection label")).clicked() {
                editor.visible = true;
                editor.focus = true;
            }
            return;
        }
        let mut selection = None;
        let mut edit = None;
        let mut delete = None;
        let enabled =
            self.editable() && self.text_editor.is_none() && self.connection_editor.is_none();
        ui.add_enabled_ui(enabled, |ui| {
            let menu = ui.menu_button(l.text("Board connections"), |ui| {
                ui.set_min_width(260.0);
                ui.set_max_width(340.0);
                if self.session.document().edges().is_empty() {
                    ui.label(l.text("No connections yet"));
                } else {
                    egui::ScrollArea::vertical()
                        .id_salt("canvas-connection-list")
                        .max_height(220.0)
                        .show(ui, |ui| {
                            for edge in self.session.document().edges() {
                                let endpoints = format!(
                                    "{} → {}",
                                    endpoint_caption(
                                        self.session.document().node(edge.from_node()),
                                        l
                                    ),
                                    endpoint_caption(
                                        self.session.document().node(edge.to_node()),
                                        l
                                    ),
                                );
                                let title = if edge.label().is_empty() {
                                    endpoints.as_str()
                                } else {
                                    edge.label()
                                };
                                let selected = self.view.selection().edges.contains(edge.id());
                                let row = ui.add(
                                    egui::Button::selectable(selected, title)
                                        .truncate()
                                        .min_size(egui::vec2(ui.available_width(), 30.0)),
                                );
                                super::super::capture_connected_writing::control(
                                    ui.ctx(),
                                    &format!("canvas.connection.select.{}", edge.id()),
                                    row.rect,
                                );
                                let row = row.on_hover_ui(|ui| {
                                    ui.label(&endpoints);
                                    if !edge.label().is_empty() {
                                        ui.separator();
                                        ui.label(edge.label());
                                    }
                                });
                                if row.clicked() {
                                    selection = Some(edge.id().to_owned());
                                }
                            }
                        });
                }
                ui.separator();
                let selected = selection.as_ref().or_else(|| {
                    (self.view.selection().edges.len() == 1)
                        .then(|| self.view.selection().edges.iter().next())
                        .flatten()
                });
                let label = ui.add_enabled(
                    selected.is_some(),
                    egui::Button::new(l.text("Edit connection label…")),
                );
                super::super::capture_connected_writing::control(
                    ui.ctx(),
                    "canvas.connection.edit",
                    label.rect,
                );
                if label.clicked() {
                    edit = selected.cloned();
                    ui.close();
                }
                let remove = ui.add_enabled(
                    selected.is_some(),
                    egui::Button::new(l.text("Delete connection")),
                );
                super::super::capture_connected_writing::control(
                    ui.ctx(),
                    "canvas.connection.delete",
                    remove.rect,
                );
                if remove.clicked() {
                    delete = selected.cloned();
                    ui.close();
                }
            });
            super::super::capture_connected_writing::control(
                ui.ctx(),
                "canvas.connections",
                menu.response.rect,
            );
        });
        if let Some(id) = selection {
            self.set_tool(BoardTool::Select);
            self.view.set_selection(edge_selection(id));
        }
        if let Some(id) = edit {
            let result = self.open_connection_label(&id);
            self.set_failure(result);
        }
        if let Some(id) = delete {
            self.set_tool(BoardTool::Select);
            let result = self.command(
                database,
                vec![CanvasCommand::RemoveSelection(edge_selection(id))],
            );
            self.set_failure(result);
        }
    }

    fn open_connection_label(&mut self, id: &str) -> AppResult<()> {
        if !self.editable() || self.text_editor.is_some() || self.connection_editor.is_some() {
            return Err(PendingCanvas.into());
        }
        let edge = self
            .session
            .document()
            .edge(id)
            .ok_or_else(|| CanvasError::NotFound(id.into()))?;
        let label = edge.label().to_owned();
        let editor = ConnectionEditor {
            edge_id: id.into(),
            base: self.session.identity(),
            durable_revision: self.snapshot.revision.info.revision,
            expected: label.clone(),
            label,
            focus: true,
            submitted: false,
            acknowledged: false,
            visible: true,
        };
        self.set_tool(BoardTool::Select);
        self.view.set_selection(edge_selection(id.into()));
        self.connection_editor = Some(editor);
        Ok(())
    }

    /// Called only after the exact prepared transition has been durably saved and applied.
    pub(super) fn connection_acknowledged(&mut self, base: ContentIdentity) -> AppResult<()> {
        let Some(editor) = &mut self.connection_editor else {
            return Ok(());
        };
        if !editor.submitted {
            return Ok(());
        }
        let saved = &self.snapshot.revision;
        let exact_edge = self
            .session
            .document()
            .edge(&editor.edge_id)
            .is_some_and(|edge| edge.label() == normalized_label(&editor.label));
        let saved_document: serde_json::Value =
            serde_json::from_str(self.session.document().raw_json())?;
        if base != editor.base
            || editor.durable_revision.checked_add(1) != Some(saved.info.revision)
            || !exact_edge
            || saved_document != saved.content
        {
            return Err(CanvasError::StaleBase.into());
        }
        editor.acknowledged = true;
        Ok(())
    }

    fn submit_connection_label(
        &mut self,
        database: &Path,
        editor: &ConnectionEditor,
    ) -> AppResult<()> {
        if editor.base != self.session.identity() {
            return Err(CanvasError::StaleBase.into());
        }
        let edge = self
            .session
            .document()
            .edge(&editor.edge_id)
            .ok_or_else(|| CanvasError::NotFound(editor.edge_id.clone()))?;
        if edge.label() != editor.expected {
            return Err(CanvasError::StaleBase.into());
        }
        self.begin(
            database,
            PendingOperation::Transaction(CanvasTransaction::new(
                editor.base,
                vec![CanvasCommand::EditLabel {
                    target: LabelTarget::Edge(editor.edge_id.clone()),
                    expected: editor.expected.clone(),
                    label: editor.label.clone(),
                }],
            )),
        )
    }

    pub(super) fn connection_dialog(
        &mut self,
        ctx: &egui::Context,
        database: &Path,
        language: Language,
        tokens: Tokens,
    ) {
        let Some(mut editor) = self.connection_editor.take() else {
            return;
        };
        if editor.acknowledged {
            return;
        }
        if !editor.visible {
            self.connection_editor = Some(editor);
            return;
        }
        if editor.submitted
            && self.pending.is_none()
            && self.failed.is_none()
            && (self.failure.is_some() || editor.base != self.session.identity())
        {
            editor.submitted = false;
        }
        let l = language;
        let stale = editor.base != self.session.identity();
        let pending = self.pending.is_some();
        let retryable = editor.submitted && self.failed.is_some() && !pending;
        let writable = !editor.submitted && self.editable();
        let mut save = false;
        let mut retry = false;
        let mut close = false;
        let response = components::modal(
            ctx,
            l,
            "canvas-connection-label",
            l.text("Connection label"),
            480.0,
            |ui| {
                ui.label(l.text("Connection label"));
                let field = if writable {
                    components::singleline(ui, &mut editor.label, l.text("Connection label"))
                } else {
                    // An immutable native buffer keeps selection/copy available while saving.
                    let mut text = editor.label.as_str();
                    ui.add(
                        egui::TextEdit::singleline(&mut text)
                            .desired_width(f32::INFINITY)
                            .min_size(egui::vec2(0.0, components::FIELD_HEIGHT))
                            .margin(egui::Margin::symmetric(12, 8))
                            .vertical_align(Align::Center),
                    )
                };
                super::super::capture_connected_writing::control(
                    ctx,
                    "canvas.connection.label",
                    field.rect,
                );
                if editor.focus {
                    field.request_focus();
                    editor.focus = false;
                }
                ui.label(
                    RichText::new(l.text("Leave the label empty to remove it."))
                        .small()
                        .color(tokens.muted),
                );
                let within_limit = normalized_label(&editor.label).chars().count() <= 500;
                if !within_limit {
                    ui.colored_label(
                        tokens.warning,
                        l.text("Connection labels can contain up to 500 characters."),
                    );
                }
                if stale {
                    ui.colored_label(tokens.warning, l.text("The board changed. Copy your label or cancel and reopen the connection."));
                }
                if pending {
                    ui.label(l.text("Saving…"));
                } else if let Some(failure) = &self.failure {
                    ui.colored_label(
                        tokens.warning,
                        l.text("The board could not be saved. Pending changes are preserved."),
                    );
                    ui.collapsing(l.text("Technical details"), |ui| {
                        ui.label(failure);
                    });
                }
                components::footer(ui, |ui| {
                    let primary = components::action(
                        ui,
                        l.text(if retryable { "Retry save" } else { "Save" }),
                        retryable || (writable && !stale && within_limit),
                        Some(tokens),
                    );
                    super::super::capture_connected_writing::control(
                        ctx,
                        "canvas.connection.save",
                        primary.rect,
                    );
                    if primary.clicked() {
                        retry = retryable;
                        save = !retryable;
                    }
                    let cancel = components::action(
                        ui,
                        l.text(if editor.submitted {
                            "Return to board"
                        } else {
                            "Cancel"
                        }),
                        !pending,
                        None,
                    );
                    super::super::capture_connected_writing::control(
                        ctx,
                        "canvas.connection.cancel",
                        cancel.rect,
                    );
                    close = cancel.clicked();
                });
            },
        );
        super::super::capture_connected_writing::observe(
            ctx,
            "canvas_connection",
            serde_json::json!({
                "edge_id":editor.edge_id, "label":editor.label, "expected":editor.expected,
                "submitted":editor.submitted, "pending":pending, "stale":stale,
            }),
        );
        if save {
            if normalized_label(&editor.label) == editor.expected {
                return;
            }
            match self.submit_connection_label(database, &editor) {
                Ok(()) => editor.submitted = true,
                Err(error) => self.failure = Some(error.to_string()),
            }
        } else if retry {
            let result = self.retry(database);
            self.set_failure(result);
        } else if !pending && (close || response.should_close()) {
            if !editor.submitted {
                return;
            }
            // Recovery controls remain reachable without losing the raw label draft.
            editor.visible = false;
        }
        self.connection_editor = Some(editor);
    }

    pub(super) fn paint_connections(&self, ui: &egui::Ui, viewport: egui::Rect, tokens: Tokens) {
        let painter = ui
            .painter()
            .with_clip_rect(viewport.intersect(ui.clip_rect()));
        let screen = |point| {
            let local = self.view.camera().world_to_view(
                point,
                WorldPoint {
                    x: f64::from(viewport.width()),
                    y: f64::from(viewport.height()),
                },
            );
            egui::pos2(
                viewport.left() + local.x as f32,
                viewport.top() + local.y as f32,
            )
        };
        if !self.view.has_gesture() {
            for edge in self.session.document().edges() {
                if edge.label().is_empty() {
                    continue;
                }
                let (Some(from), Some(to)) = (
                    self.session.document().node(edge.from_node()),
                    self.session.document().node(edge.to_node()),
                ) else {
                    continue;
                };
                let a = edge
                    .from_side()
                    .map_or_else(|| from.bounds().center(), |side| side.point(from.bounds()));
                let b = edge
                    .to_side()
                    .map_or_else(|| to.bounds().center(), |side| side.point(to.bounds()));
                let midpoint = screen(a).lerp(screen(b), 0.5);
                let mut job = egui::text::LayoutJob::simple(
                    edge.label().into(),
                    egui::FontId::proportional(12.0),
                    tokens.text,
                    180.0,
                );
                job.wrap.max_rows = 1;
                job.wrap.break_anywhere = true;
                let galley = painter.layout_job(job);
                let label_rect =
                    egui::Rect::from_center_size(midpoint, galley.size() + egui::vec2(12.0, 8.0));
                if viewport.intersects(label_rect) {
                    super::super::capture_connected_writing::control(
                        ui.ctx(),
                        &format!("canvas.connection.painted.{}", edge.id()),
                        label_rect.intersect(viewport).intersect(ui.clip_rect()),
                    );
                    painter.rect_filled(label_rect, 4.0, tokens.surface);
                    painter.rect_stroke(
                        label_rect,
                        4.0,
                        egui::Stroke::new(1.0, tokens.border),
                        egui::StrokeKind::Inside,
                    );
                    painter.galley(label_rect.min + egui::vec2(6.0, 4.0), galley, tokens.text);
                    ui.interact(
                        label_rect.intersect(viewport),
                        ui.id().with(("connection-label", edge.id())),
                        egui::Sense::hover(),
                    )
                    .on_hover_text(edge.label());
                }
            }
        }
        if self.tool == BoardTool::Connect
            && self.editable()
            && !self.modal_open()
            && ui.rect_contains_pointer(viewport)
            && let Some(source) = self
                .connect_source
                .as_ref()
                .and_then(|id| self.session.document().node(id))
            && let Some(pointer) = ui.input(|input| input.pointer.hover_pos())
        {
            painter.arrow(
                screen(source.bounds().center()),
                pointer - screen(source.bounds().center()),
                egui::Stroke::new(1.5, tokens.accent),
            );
        }
    }
}

fn edge_selection(id: String) -> Selection {
    Selection {
        nodes: BTreeSet::new(),
        edges: BTreeSet::from([id]),
    }
}

fn endpoint_caption(node: Option<&NodeRecord>, language: Language) -> String {
    let text = node
        .and_then(|node| node.body().lines().find(|line| !line.trim().is_empty()))
        .unwrap_or_else(|| language.text("Untitled card"));
    let mut characters = text.chars();
    let mut caption: String = characters.by_ref().take(48).collect();
    if characters.next().is_some() {
        caption.push('…');
    }
    caption
}

// Match the existing engine/source ECMAScript trim contract, including U+FEFF.
fn normalized_label(label: &str) -> &str {
    label.trim_matches(|ch| matches!(ch, '\u{0009}'..='\u{000d}'|'\u{0020}'|'\u{00a0}'|'\u{1680}'|'\u{2000}'..='\u{200a}'|'\u{2028}'|'\u{2029}'|'\u{202f}'|'\u{205f}'|'\u{3000}'|'\u{feff}'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reload_should_keep_a_copyable_label_draft_and_refuse_its_stale_base() -> AppResult<()> {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("stale-label.sqlite");
        let workspace = Workspace::open(&database)?;
        let project = workspace.create_project("Synthetic connection label")?;
        let content = json!({
            "nodes":[
                {"id":"a","type":"text","x":0,"y":0,"width":100,"height":100,"text":"A"},
                {"id":"b","type":"text","x":200,"y":0,"width":100,"height":100,"text":"B"}
            ],
            "edges":[{"id":"edge","fromNode":"a","toNode":"b","label":"Original", "future":{"keep":true}}]
        });
        let board = workspace.create_project_canvas(&project.id, "Board", content.clone())?;
        let mut state = CanvasState::from_snapshot(&project.id, board)?;
        state.open_connection_label("edge")?;
        let editor = state.connection_editor.as_mut().ok_or("label draft")?;
        editor.label = "  Later 日本語 🪶  ".into();
        editor.visible = false;
        // Use the actual guarded reload path: an equal graph still has a new authority.
        state.failed_preserved = true;
        state.reload_saved(&database)?;
        assert!(state.blocks_navigation());
        assert!(!state.modal_open());
        let editor = state
            .connection_editor
            .take()
            .ok_or("retained label draft")?;
        let result = state.submit_connection_label(&database, &editor);
        state.connection_editor = Some(editor);
        assert!(result.is_err());
        assert!(state.pending.is_none());
        assert_eq!(
            state
                .connection_editor
                .as_ref()
                .map(|draft| draft.label.as_str()),
            Some("  Later 日本語 🪶  ")
        );
        let saved = workspace.project_canvas(&project.id, &state.snapshot.canvas.id)?;
        assert_eq!(saved.revision.info.revision, 1);
        assert_eq!(saved.revision.content, content);
        Ok(())
    }
}
