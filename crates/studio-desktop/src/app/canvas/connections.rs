//! Native connection controls over the existing guarded board authority.
use super::*;
use craft_canvas_model::{ConnectorEndpointKind, LabelTarget, Selection};

pub(super) struct ConnectionEditor {
    edge_id: String,
    base: ContentIdentity,
    durable_revision: u64,
    expected: String,
    label: String,
    expected_from: ConnectorEndpoint,
    expected_to: ConnectorEndpoint,
    from: ConnectorEndpoint,
    to: ConnectorEndpoint,
    focus: bool,
    submitted: bool,
    acknowledged: bool,
    visible: bool,
    preview_node: Option<String>,
}

impl ConnectionEditor {
    pub(super) fn is_open(&self) -> bool {
        self.visible
    }
    fn endpoints_changed(&self) -> bool {
        self.from != self.expected_from || self.to != self.expected_to
    }
    fn saved_label(&self) -> &str {
        if self.label == self.expected {
            &self.expected
        } else {
            normalized_label(&self.label)
        }
    }
    fn invalid_loop(&self) -> bool {
        self.endpoints_changed()
            && self.from.node_id == self.to.node_id
            && (self.from.side.is_none()
                || self.to.side.is_none()
                || self.from.side == self.to.side)
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
            if !editor.visible && ui.button(l.text("Resume connection editing")).clicked() {
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
                                super::super::capture_connected_writing::actionable_control(
                                    ui,
                                    &format!("canvas.connection.select.{}", edge.id()),
                                    &row,
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
                    egui::Button::new(l.text("Edit connection…")),
                );
                super::super::capture_connected_writing::actionable_control(
                    ui,
                    "canvas.connection.edit",
                    &label,
                );
                if label.clicked() {
                    edit = selected.cloned();
                    ui.close();
                }
                let remove = ui.add_enabled(
                    selected.is_some(),
                    egui::Button::new(l.text("Delete connection")),
                );
                super::super::capture_connected_writing::actionable_control(
                    ui,
                    "canvas.connection.delete",
                    &remove,
                );
                if remove.clicked() {
                    delete = selected.cloned();
                    ui.close();
                }
            });
            super::super::capture_connected_writing::actionable_control(
                ui,
                "canvas.connections",
                &menu.response,
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
        let from = edge_endpoint(edge, ConnectorEndpointKind::From);
        let to = edge_endpoint(edge, ConnectorEndpointKind::To);
        let editor = ConnectionEditor {
            edge_id: id.into(),
            base: self.session.identity(),
            durable_revision: self.snapshot.revision.info.revision,
            expected: label.clone(),
            label,
            expected_from: from.clone(),
            expected_to: to.clone(),
            from,
            to,
            focus: true,
            submitted: false,
            acknowledged: false,
            visible: true,
            preview_node: None,
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
            .is_some_and(|edge| {
                edge.label() == editor.saved_label()
                    && edge_endpoint(edge, ConnectorEndpointKind::From) == editor.from
                    && edge_endpoint(edge, ConnectorEndpointKind::To) == editor.to
            });
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
        if edge.label() != editor.expected
            || edge_endpoint(edge, ConnectorEndpointKind::From) != editor.expected_from
            || edge_endpoint(edge, ConnectorEndpointKind::To) != editor.expected_to
        {
            return Err(CanvasError::StaleBase.into());
        }
        if editor.invalid_loop() {
            return Err(CanvasError::Invalid(
                "Choose different sides for a connection on the same card".into(),
            )
            .into());
        }
        let mut commands = Vec::new();
        if editor.saved_label() != editor.expected {
            commands.push(CanvasCommand::EditLabel {
                target: LabelTarget::Edge(editor.edge_id.clone()),
                expected: editor.expected.clone(),
                label: editor.label.clone(),
            });
        }
        for (endpoint, expected, replacement) in [
            (
                ConnectorEndpointKind::From,
                &editor.expected_from,
                &editor.from,
            ),
            (ConnectorEndpointKind::To, &editor.expected_to, &editor.to),
        ] {
            if expected != replacement {
                commands.push(CanvasCommand::ReconnectEdge {
                    edge_id: editor.edge_id.clone(),
                    endpoint,
                    expected: expected.clone(),
                    replacement: replacement.clone(),
                });
            }
        }
        self.begin(
            database,
            PendingOperation::Transaction(CanvasTransaction::new(editor.base, commands)),
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
        let mut reveal = None;
        let response = components::modal(
            ctx,
            l,
            "canvas-connection-label",
            l.text("Connection properties"),
            480.0,
            |ui| {
                ui.add_enabled_ui(writable, |ui| {
                    if endpoint_selector(
                        ui,
                        "connection-from",
                        l.text("From card"),
                        &mut editor.from,
                        self.session.document(),
                        &self.dialect,
                        l,
                    ) {
                        reveal = Some(editor.from.node_id.clone());
                    }
                    if endpoint_selector(
                        ui,
                        "connection-to",
                        l.text("To card"),
                        &mut editor.to,
                        self.session.document(),
                        &self.dialect,
                        l,
                    ) {
                        reveal = Some(editor.to.node_id.clone());
                    }
                });
                ui.add_space(8.0);
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
                super::super::capture_connected_writing::actionable_control(
                    ui,
                    "canvas.connection.label",
                    &field,
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
                if editor.invalid_loop() {
                    ui.colored_label(
                        tokens.warning,
                        l.text("Choose different sides for a connection on the same card."),
                    );
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
                        retryable || (writable && !stale && within_limit && !editor.invalid_loop()),
                        Some(tokens),
                    );
                    super::super::capture_connected_writing::actionable_control(
                        ui,
                        "canvas.connection.save",
                        &primary,
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
                    super::super::capture_connected_writing::actionable_control(
                        ui,
                        "canvas.connection.cancel",
                        &cancel,
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
                "from":{"node_id":editor.from.node_id,"side":editor.from.side.map(Side::as_str)},
                "to":{"node_id":editor.to.node_id,"side":editor.to.side.map(Side::as_str)},
                "submitted":editor.submitted, "pending":pending, "stale":stale,
            }),
        );
        if let Some(id) = reveal {
            // A deliberate preview changes camera only; the guarded draft remains
            // resumable, and the actual edge selection stays unchanged.
            let selection = self.view.selection().clone();
            self.view.set_selection(Selection {
                nodes: BTreeSet::from([id.clone()]),
                edges: BTreeSet::new(),
            });
            let result = self
                .view
                .fit(&self.session, self.viewport_size, true)
                .map_err(Into::into);
            self.view.set_selection(selection);
            self.set_failure(result);
            editor.preview_node = Some(id);
            editor.visible = false;
        } else if save {
            if editor.saved_label() == editor.expected && !editor.endpoints_changed() {
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

    /// Returns whether the painted connection label owns this frame's pointer action.
    pub(super) fn paint_connections(
        &mut self,
        ui: &egui::Ui,
        output: &CanvasViewOutput,
        tokens: Tokens,
        language: Language,
        interactive: bool,
    ) -> bool {
        let viewport = output.viewport;
        let nodes = &output.nodes;
        let board_clip = viewport.intersect(ui.clip_rect());
        let painter = ui.painter().with_clip_rect(board_clip);
        let camera = self.view.camera();
        let screen = |point| {
            let local = camera.world_to_view(
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
        let mut owns_pointer = false;
        let mut select = None;
        let mut open = None;
        let pan_input = ui.input(|input| {
            input.key_down(egui::Key::Space)
                || input.key_pressed(egui::Key::Space)
                || input.key_released(egui::Key::Space)
                || input.pointer.button_down(egui::PointerButton::Middle)
                || input.pointer.button_released(egui::PointerButton::Middle)
        });
        if let Some(id) = self
            .connection_editor
            .as_ref()
            .and_then(|editor| editor.preview_node.as_ref())
            && let Some(node) = nodes.iter().find(|node| &node.node_id == id)
        {
            painter.rect_stroke(
                node.reserved.expand(3.0),
                6.0,
                egui::Stroke::new(2.0, tokens.accent),
                egui::StrokeKind::Outside,
            );
        }
        if !self.view.has_gesture() || ui.input(|input| input.pointer.primary_pressed()) {
            for geometry in &output.edges {
                let Some(edge) = self.session.document().edge(&geometry.edge_id) else {
                    continue;
                };
                if edge.label().is_empty() {
                    continue;
                }
                let midpoint = geometry.label_anchor;
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
                // A covered label cannot steal a card's input. The anchor is shared
                // with the exact routed line painted and hit-tested by the engine.
                if !board_clip.intersects(label_rect)
                    || nodes
                        .iter()
                        .any(|node| node.reserved.intersects(label_rect))
                {
                    continue;
                }
                let clipped = label_rect.intersect(board_clip);
                super::super::capture_connected_writing::control(
                    ui.ctx(),
                    &format!("canvas.connection.painted.{}", edge.id()),
                    clipped,
                );
                painter.rect_filled(label_rect, 4.0, tokens.surface);
                painter.rect_stroke(
                    label_rect,
                    4.0,
                    egui::Stroke::new(1.0, tokens.border),
                    egui::StrokeKind::Inside,
                );
                painter.galley(label_rect.min + egui::vec2(6.0, 4.0), galley, tokens.text);
                let selectable = interactive
                    && !pan_input
                    && self.tool == BoardTool::Select
                    && self.connection_editor.is_none();
                let response = ui
                    .interact(
                        clipped,
                        ui.id().with(("connection-label", edge.id())),
                        if selectable {
                            egui::Sense::click()
                        } else {
                            egui::Sense::hover()
                        },
                    )
                    .on_hover_text(edge.label());
                if selectable
                    && response.contains_pointer()
                    && ui.input(|input| {
                        input.pointer.primary_pressed()
                            || input.pointer.primary_down()
                            || input.pointer.primary_released()
                    })
                {
                    owns_pointer = true;
                }
                if selectable && response.clicked() {
                    select = Some(edge.id().to_owned());
                }
                if selectable && response.double_clicked() {
                    open = Some(edge.id().to_owned());
                }
            }
        }
        if self.tool == BoardTool::Connect && interactive {
            for (index, geometry) in nodes.iter().enumerate() {
                let Some(node) = self.session.document().node(&geometry.node_id) else {
                    continue;
                };
                if !self.dialect.can_connect(node) {
                    continue;
                }
                for side in Side::ALL {
                    let point = handle_point(geometry.reserved, side);
                    if !handle_visible(nodes, index, point, board_clip) {
                        continue;
                    }
                    let active = self.connect_source.as_deref() == Some(node.id())
                        && self.connect_source_side == Some(side);
                    painter.circle_filled(point, if active { 6.0 } else { 4.5 }, tokens.accent);
                    painter.circle_stroke(
                        point,
                        6.0,
                        egui::Stroke::new(if active { 2.0 } else { 1.0 }, tokens.surface),
                    );
                    let rect = egui::Rect::from_center_size(
                        point,
                        egui::Vec2::splat(HANDLE_HIT_RADIUS * 2.0),
                    )
                    .intersect(board_clip);
                    let key = format!("canvas.handle.{}.{}", node.id(), side.as_str());
                    super::super::capture_connected_writing::control(ui.ctx(), &key, rect);
                    ui.interact(rect, ui.id().with(key), egui::Sense::hover())
                        .on_hover_text(language.text(side_label(Some(side))));
                }
            }
            if ui.rect_contains_pointer(board_clip)
                && let Some(source) = self
                    .connect_source
                    .as_ref()
                    .and_then(|id| self.session.document().node(id))
                && let Some(pointer) = ui.input(|input| input.pointer.hover_pos())
            {
                let start = screen(self.connect_source_side.map_or_else(
                    || source.bounds().center(),
                    |side| side.point(source.bounds()),
                ));
                painter.arrow(
                    start,
                    pointer - start,
                    egui::Stroke::new(1.5, tokens.accent),
                );
            }
        }
        if owns_pointer {
            self.press_origin = None;
            self.view.cancel_transient();
        }
        if let Some(id) = select {
            self.view.set_selection(edge_selection(id));
        }
        if let Some(id) = open {
            let result = self.open_connection_label(&id);
            self.set_failure(result);
        }
        owns_pointer
    }

    pub(super) fn connection_endpoint_at(
        &self,
        nodes: &[VisibleNodeGeometry],
        point: egui::Pos2,
        clip: egui::Rect,
    ) -> Option<ConnectorEndpoint> {
        if !clip.contains(point) {
            return None;
        }
        for (index, geometry) in nodes.iter().enumerate().rev() {
            let Some(node) = self.session.document().node(&geometry.node_id) else {
                continue;
            };
            if self.dialect.can_connect(node) {
                for side in Side::ALL {
                    let handle = handle_point(geometry.reserved, side);
                    if handle_visible(nodes, index, handle, clip)
                        && handle.distance(point) <= HANDLE_HIT_RADIUS
                    {
                        return Some(ConnectorEndpoint {
                            node_id: node.id().into(),
                            side: Some(side),
                        });
                    }
                }
            }
            if geometry.reserved.contains(point) {
                return Some(ConnectorEndpoint {
                    node_id: node.id().into(),
                    side: None,
                });
            }
        }
        None
    }
}

fn edge_selection(id: String) -> Selection {
    Selection {
        nodes: BTreeSet::new(),
        edges: BTreeSet::from([id]),
    }
}

const HANDLE_HIT_RADIUS: f32 = 9.0;

fn handle_point(rect: egui::Rect, side: Side) -> egui::Pos2 {
    match side {
        Side::Top => rect.center_top(),
        Side::Right => rect.right_center(),
        Side::Bottom => rect.center_bottom(),
        Side::Left => rect.left_center(),
    }
}

fn handle_visible(
    nodes: &[VisibleNodeGeometry],
    index: usize,
    point: egui::Pos2,
    clip: egui::Rect,
) -> bool {
    let painted = egui::Rect::from_center_size(point, egui::Vec2::splat(12.0));
    clip.contains(point)
        && !nodes[index + 1..]
            .iter()
            .any(|node| node.reserved.intersects(painted))
}

fn edge_endpoint(edge: &EdgeRecord, kind: ConnectorEndpointKind) -> ConnectorEndpoint {
    match kind {
        ConnectorEndpointKind::From => ConnectorEndpoint {
            node_id: edge.from_node().into(),
            side: edge.from_side(),
        },
        ConnectorEndpointKind::To => ConnectorEndpoint {
            node_id: edge.to_node().into(),
            side: edge.to_side(),
        },
    }
}

fn side_label(side: Option<Side>) -> &'static str {
    match side {
        None => "Automatic side",
        Some(Side::Top) => "Top",
        Some(Side::Right) => "Right",
        Some(Side::Bottom) => "Bottom",
        Some(Side::Left) => "Left",
    }
}

fn endpoint_selector(
    ui: &mut egui::Ui,
    id: &str,
    label: &str,
    endpoint: &mut ConnectorEndpoint,
    document: &CanvasDocument,
    dialect: &SourceDialect,
    language: Language,
) -> bool {
    let mut reveal = false;
    ui.label(label);
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.y = components::FIELD_HEIGHT;
        ui.horizontal(|ui| {
            let available = ui.available_width();
            egui::ComboBox::from_id_salt((id, "node"))
                .width((available - 180.0).max(80.0))
                .height(220.0)
                .wrap_mode(egui::TextWrapMode::Truncate)
                .selected_text(
                    document
                        .nodes()
                        .iter()
                        .enumerate()
                        .find(|(_, node)| node.id() == endpoint.node_id)
                        .map(|(index, node)| {
                            format!("{}. {}", index + 1, endpoint_caption(Some(node), language))
                        })
                        .unwrap_or_else(|| language.text("Unavailable card").into()),
                )
                .show_ui(ui, |ui| {
                    ui.set_max_width(300.0);
                    for (index, node) in document
                        .nodes()
                        .iter()
                        .enumerate()
                        .filter(|(_, node)| dialect.can_connect(node))
                    {
                        if ui
                            .selectable_label(
                                endpoint.node_id == node.id(),
                                format!(
                                    "{}. {}",
                                    index + 1,
                                    endpoint_caption(Some(node), language)
                                ),
                            )
                            .clicked()
                        {
                            endpoint.node_id = node.id().to_owned();
                        }
                    }
                });
            egui::ComboBox::from_id_salt((id, "side"))
                .width(120.0)
                .wrap_mode(egui::TextWrapMode::Truncate)
                .selected_text(language.text(side_label(endpoint.side)))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut endpoint.side, None, language.text(side_label(None)));
                    for side in Side::ALL {
                        ui.selectable_value(
                            &mut endpoint.side,
                            Some(side),
                            language.text(side_label(Some(side))),
                        );
                    }
                });
            reveal =
                icons::icon_button(ui, Icon::Focus, language.text("Show card on board")).clicked();
        });
    });
    reveal
}

fn endpoint_caption(node: Option<&NodeRecord>, language: Language) -> String {
    let resource = node.and_then(|node| {
        ["file", "url"]
            .into_iter()
            .filter_map(|key| node.field(key))
            .find_map(|raw| serde_json::from_str::<String>(raw.get()).ok())
    });
    let text = node
        .and_then(|node| node.body().lines().find(|line| !line.trim().is_empty()))
        .or(resource.as_deref())
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
    fn endpoint_only_edit_and_undo_should_keep_an_untouched_imported_label() -> AppResult<()> {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("untouched-label.sqlite");
        let workspace = Workspace::open(&database)?;
        let project = workspace.create_project("Synthetic unchanged label")?;
        let original = json!({
            "nodes":[
                {"id":"a","type":"text","x":0,"y":0,"width":100,"height":100,"text":"A"},
                {"id":"b","type":"text","x":200,"y":0,"width":100,"height":100,"text":"B"}
            ],
            "edges":[{"id":"edge","fromNode":"a","toNode":"b","label":"  kept 日本語  ","future":{"keep":true}}]
        });
        let board = workspace.create_project_canvas(&project.id, "Board", original.clone())?;
        let mut state = CanvasState::from_snapshot(&project.id, board)?;
        state.open_connection_label("edge")?;
        let mut editor = state.connection_editor.take().ok_or("connection draft")?;
        editor.from.side = Some(Side::Right);
        state.submit_connection_label(&database, &editor)?;
        editor.submitted = true;
        state.connection_editor = Some(editor);
        let ctx = egui::Context::default();
        let settle = |state: &mut CanvasState| -> AppResult<()> {
            let deadline = Instant::now() + Duration::from_secs(5);
            while state.pending.is_some() && Instant::now() < deadline {
                state.poll(&ctx)?;
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(state.pending.is_none());
            assert!(state.failure.is_none(), "{:?}", state.failure);
            Ok(())
        };
        settle(&mut state)?;
        assert!(
            state
                .connection_editor
                .as_ref()
                .is_some_and(|editor| editor.acknowledged)
        );
        let mut expected = original.clone();
        expected["edges"][0]["fromSide"] = json!("right");
        assert_eq!(
            workspace
                .project_canvas(&project.id, &state.snapshot.canvas.id)?
                .revision
                .content,
            expected
        );
        // The actual acknowledgement close path discards only the acknowledged draft.
        state.connection_dialog(
            &ctx,
            &database,
            Language::English,
            theme::resolve(&ThemeSettings::default(), false),
        );
        assert!(state.connection_editor.is_none());
        state.begin(&database, PendingOperation::Undo(state.session.identity()))?;
        settle(&mut state)?;
        let restored = workspace.project_canvas(&project.id, &state.snapshot.canvas.id)?;
        assert_eq!(restored.revision.info.revision, 3);
        assert_eq!(restored.revision.content, original);
        Ok(())
    }

    #[test]
    fn endpoint_edit_should_refuse_same_side_then_preserve_metadata_in_a_durable_loop()
    -> AppResult<()> {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("reconnect.sqlite");
        let workspace = Workspace::open(&database)?;
        let project = workspace.create_project("Synthetic endpoint edit")?;
        let mut expected = json!({
            "nodes":[
                {"id":"a","type":"text","x":0,"y":0,"width":100,"height":100,"text":"A"},
                {"id":"b","type":"text","x":200,"y":0,"width":100,"height":100,"text":"B"}
            ],
            "edges":[{"id":"edge","fromNode":"a","toNode":"b","label":"Original", "future":{"keep":true}, "toEnd":"arrow"}]
        });
        let board = workspace.create_project_canvas(&project.id, "Board", expected.clone())?;
        let mut state = CanvasState::from_snapshot(&project.id, board)?;
        state.open_connection_label("edge")?;
        let mut editor = state.connection_editor.take().ok_or("connection draft")?;
        editor.from.side = Some(Side::Right);
        editor.to = ConnectorEndpoint {
            node_id: "a".into(),
            side: Some(Side::Right),
        };
        assert!(state.submit_connection_label(&database, &editor).is_err());
        assert!(
            state.pending.is_none(),
            "invalid loop must not start a save"
        );
        editor.to.side = Some(Side::Top);
        editor.label = "  Kept 日本語 🪶  ".into();
        state.submit_connection_label(&database, &editor)?;
        editor.submitted = true;
        state.connection_editor = Some(editor);
        let ctx = egui::Context::default();
        let deadline = Instant::now() + Duration::from_secs(5);
        while state.pending.is_some() && Instant::now() < deadline {
            state.poll(&ctx)?;
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(state.pending.is_none());
        assert!(state.failure.is_none(), "{:?}", state.failure);
        assert!(
            state
                .connection_editor
                .as_ref()
                .is_some_and(|editor| editor.acknowledged)
        );
        expected["edges"][0]["fromSide"] = json!("right");
        expected["edges"][0]["toNode"] = json!("a");
        expected["edges"][0]["toSide"] = json!("top");
        expected["edges"][0]["label"] = json!("Kept 日本語 🪶");
        let saved = workspace.project_canvas(&project.id, &state.snapshot.canvas.id)?;
        assert_eq!(saved.revision.info.revision, 2);
        assert_eq!(saved.revision.content, expected);
        Ok(())
    }

    #[test]
    fn reload_should_keep_connection_drafts_and_refuse_their_stale_base() -> AppResult<()> {
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
        editor.from.side = Some(Side::Right);
        editor.to = ConnectorEndpoint {
            node_id: "a".into(),
            side: Some(Side::Top),
        };
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
        let retained = state
            .connection_editor
            .as_ref()
            .ok_or("retained endpoint draft")?;
        assert_eq!(retained.from.side, Some(Side::Right));
        assert_eq!(retained.to.node_id, "a");
        assert_eq!(retained.to.side, Some(Side::Top));
        assert_eq!(
            retained.expected_from.side, None,
            "reload cannot rebase missing imported sides"
        );
        assert_eq!(retained.expected_to.node_id, "b");
        let saved = workspace.project_canvas(&project.id, &state.snapshot.canvas.id)?;
        assert_eq!(saved.revision.info.revision, 1);
        assert_eq!(saved.revision.content, content);
        Ok(())
    }
}
