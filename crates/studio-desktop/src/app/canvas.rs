//! Native board projection over one project-scoped durable authority.
use super::*;
use craft_canvas_egui::{
    CanvasIntent, CanvasView, CanvasViewConfig, CanvasViewHost, CanvasViewLabels, NodeBodyResponse,
    TextCardEditor,
};
use craft_canvas_model::{
    CanvasCommand, CanvasDialect, CanvasDocument, CanvasError, CanvasLimits, CanvasSession,
    CanvasTransaction, ContentIdentity, GroupBehavior, IdMapping, NodeKind, NodePolicy, NodeRecord,
    PreparedChange, WorkControl, WorldPoint, WorldRect,
};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::Arc;
use studio_core::{ProjectCanvasDraft, ProjectCanvasRevision, ProjectCanvasSnapshot};

/// Explicit project-relative navigation. It never authorizes an ambient file read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CanvasResource {
    pub(super) project: String,
    pub(super) kind: String,
    pub(super) logical_id: String,
}

struct SourceDialect {
    policies: HashMap<String, NodePolicy>,
    strokes: BTreeSet<String>,
}
impl SourceDialect {
    fn new(document: &CanvasDocument) -> AppResult<Self> {
        let mut policies = HashMap::new();
        let mut strokes = BTreeSet::new();
        for node in document.nodes() {
            let extension = node
                .field("x-writers-studio")
                .map(|raw| serde_json::from_str::<serde_json::Value>(raw.get()))
                .transpose()?;
            let locked = extension
                .as_ref()
                .is_some_and(|value| value["locked"] == true);
            let element = extension.as_ref().and_then(|value| value.get("element"));
            let kind = element.and_then(|value| value["kind"].as_str());
            let known = !matches!(node.kind(), NodeKind::Unknown(_));
            if kind == Some("stroke") {
                strokes.insert(node.id().to_owned());
            }
            policies.insert(
                node.id().to_owned(),
                NodePolicy {
                    movable: known && !locked,
                    // Stroke scaling needs source point/pressure mapping, rather than only a box edit.
                    resizable: known && !locked && kind != Some("stroke"),
                    group: if kind == Some("frame") {
                        GroupBehavior::Frame
                    } else if matches!(node.kind(), NodeKind::Group) && element.is_none() {
                        GroupBehavior::Spatial
                    } else {
                        GroupBehavior::Independent
                    },
                },
            );
        }
        Ok(Self { policies, strokes })
    }
}
impl CanvasDialect for SourceDialect {
    fn node_policy(&self, node: &NodeRecord) -> NodePolicy {
        self.policies.get(node.id()).copied().unwrap_or(NodePolicy {
            movable: false,
            resizable: false,
            group: GroupBehavior::Independent,
        })
    }
    fn can_connect(&self, node: &NodeRecord) -> bool {
        !self.strokes.contains(node.id()) && !matches!(node.kind(), NodeKind::Unknown(_))
    }
}

enum PendingOperation {
    Transaction(CanvasTransaction),
    Undo(ContentIdentity),
    Redo(ContentIdentity),
}
struct CommitCompletion {
    prepared: Option<PreparedChange>,
    result: AppResult<ProjectCanvasRevision>,
}

/// The view is disposable; this fixed project/board/session owns every acknowledged transition.
pub(super) struct CanvasState {
    pub(super) project_id: String,
    pub(super) snapshot: ProjectCanvasSnapshot,
    session: Arc<CanvasSession>,
    view: CanvasView,
    dialect: SourceDialect,
    text_editor: Option<TextCardEditor>,
    pending: Option<mpsc::Receiver<CommitCompletion>>,
    failed: Option<PreparedChange>,
    failure: Option<String>,
    failed_preserved: bool,
    draft_preserved: bool,
    fit_on_open: bool,
    viewport_size: Vec2,
    resource: Option<CanvasResource>,
}
impl CanvasState {
    pub(super) fn open(workspace: &Workspace, project: &str, id: &str) -> AppResult<Self> {
        let snapshot = workspace.project_canvas(project, id)?;
        Self::from_snapshot(project, snapshot)
    }
    fn from_snapshot(project: &str, snapshot: ProjectCanvasSnapshot) -> AppResult<Self> {
        let limits = CanvasLimits::default();
        let document =
            CanvasDocument::parse(&serde_json::to_string(&snapshot.revision.content)?, &limits)?;
        let dialect = SourceDialect::new(&document)?;
        let session = Arc::new(CanvasSession::new(document, limits)?);
        Ok(Self {
            project_id: project.into(),
            snapshot,
            session,
            view: CanvasView::default(),
            dialect,
            text_editor: None,
            pending: None,
            failed: None,
            failure: None,
            failed_preserved: false,
            draft_preserved: false,
            fit_on_open: true,
            viewport_size: Vec2::new(800.0, 600.0),
            resource: None,
        })
    }
    pub(super) fn modal_open(&self) -> bool {
        self.text_editor.is_some()
    }
    /// Navigation must wait for acknowledgement; a source dialog must be deliberately saved/cancelled.
    pub(super) fn blocks_navigation(&self) -> bool {
        self.pending.is_some() || self.failed.is_some() || self.text_editor.is_some()
    }
    fn editable(&self) -> bool {
        self.pending.is_none() && self.failed.is_none() && self.snapshot.draft.is_none()
    }
    pub(super) fn take_resource(&mut self) -> Option<CanvasResource> {
        self.resource.take()
    }
    fn begin(&mut self, database: &Path, operation: PendingOperation) -> AppResult<()> {
        if !self.editable() {
            return Err(CanvasError::Invalid(
                "Resolve the pending board operation or recovery draft first".into(),
            )
            .into());
        }
        let session = Arc::clone(&self.session);
        let path = database.to_owned();
        let project = self.project_id.clone();
        let board = self.snapshot.canvas.id.clone();
        let revision = self.snapshot.revision.clone();
        let (sender, receiver) = mpsc::channel();
        self.view.cancel_transient();
        self.failure = None;
        std::thread::spawn(move || {
            let completion =
                prepare_and_save(session, &path, &project, &board, revision, operation);
            let _ = sender.send(completion);
        });
        self.pending = Some(receiver);
        Ok(())
    }
    fn retry(&mut self, database: &Path) -> AppResult<()> {
        if self.pending.is_some() {
            return Err(CanvasError::Invalid("A board save is already pending".into()).into());
        }
        let prepared = self.failed.take().ok_or(CanvasError::NoHistory)?;
        if prepared.base() != self.session.identity() {
            self.failed = Some(prepared);
            return Err(CanvasError::StaleBase.into());
        }
        let path = database.to_owned();
        let project = self.project_id.clone();
        let board = self.snapshot.canvas.id.clone();
        let revision = self.snapshot.revision.clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let completion = persist_candidate(&path, &project, &board, revision, prepared);
            let _ = sender.send(completion);
        });
        self.pending = Some(receiver);
        self.failure = None;
        Ok(())
    }
    pub(super) fn poll(&mut self, ctx: &egui::Context) -> AppResult<()> {
        let Some(receiver) = &self.pending else {
            return Ok(());
        };
        let completion = match receiver.try_recv() {
            Ok(completion) => completion,
            Err(mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(Duration::from_millis(30));
                return Ok(());
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending = None;
                self.failure = Some("The board worker stopped before acknowledgement; reload saved history before continuing".into());
                return Err(CanvasError::Invalid(
                    "Board worker disconnected before acknowledgement".into(),
                )
                .into());
            }
        };
        self.pending = None;
        match completion.result {
            Ok(saved) => {
                let prepared = completion
                    .prepared
                    .ok_or_else(|| CanvasError::Invalid("Missing board acknowledgement".into()))?;
                let dialect = SourceDialect::new(prepared.document())?;
                let session = Arc::get_mut(&mut self.session).ok_or_else(|| {
                    CanvasError::Invalid(
                        "Board preparation still owns the acknowledged session".into(),
                    )
                })?;
                let mapping = session.apply(prepared)?;
                let mut selection = self.view.selection().clone();
                if !mapping.added_nodes.is_empty() {
                    selection.nodes = mapping.added_nodes;
                    selection.edges = mapping.added_edges;
                }
                selection.retain_existing(session.document());
                self.view.set_selection(selection);
                self.dialect = dialect;
                self.snapshot.canvas.current_revision = saved.info.revision;
                self.snapshot.revision = saved;
                self.failure = None;
                self.failed = None;
                self.text_editor = None;
                Ok(())
            }
            Err(error) => {
                self.failed = completion.prepared;
                self.failed_preserved = false;
                self.failure = Some(error.to_string());
                Err(error)
            }
        }
    }
    fn set_failure(&mut self, result: AppResult<()>) {
        if let Err(error) = result {
            self.failure = Some(error.to_string());
        }
    }
    fn command(&mut self, database: &Path, commands: Vec<CanvasCommand>) -> AppResult<()> {
        self.begin(
            database,
            PendingOperation::Transaction(CanvasTransaction::new(
                self.session.identity(),
                commands,
            )),
        )
    }
    fn add_text(&mut self, database: &Path, at: WorldPoint) -> AppResult<()> {
        let node = NodeRecord::text(
            uuid::Uuid::new_v4().to_string(),
            WorldRect {
                x: at.x.round(),
                y: at.y.round(),
                width: 300.0,
                height: 180.0,
            },
            String::new(),
            self.session.limits(),
        )?;
        self.command(
            database,
            vec![CanvasCommand::InsertNode {
                node,
                index: self.session.document().nodes().len(),
            }],
        )
    }
    fn duplicate(
        &mut self,
        database: &Path,
        base: ContentIdentity,
        node_ids: Vec<String>,
    ) -> AppResult<()> {
        if base != self.session.identity() {
            return Err(CanvasError::StaleBase.into());
        }
        let mut work = WorkControl::new(self.session.limits().max_work_units, None);
        let nodes =
            self.session
                .document()
                .expand_move_selection(&node_ids, &self.dialect, &mut work)?;
        let mut ids = IdMapping::default();
        for id in &nodes {
            ids.nodes
                .insert(id.clone(), uuid::Uuid::new_v4().to_string());
        }
        for edge in self.session.document().edges() {
            if nodes.contains(edge.from_node()) && nodes.contains(edge.to_node()) {
                ids.edges
                    .insert(edge.id().into(), uuid::Uuid::new_v4().to_string());
            }
        }
        self.command(
            database,
            vec![CanvasCommand::Duplicate {
                node_ids,
                ids,
                delta: WorldPoint { x: 24.0, y: 24.0 },
            }],
        )
    }
    fn intent(&mut self, database: &Path, intent: CanvasIntent) -> AppResult<()> {
        match intent {
            CanvasIntent::Transaction(transaction) => self.begin(database, PendingOperation::Transaction(transaction)),
            CanvasIntent::Undo(base) => self.begin(database, PendingOperation::Undo(base)),
            CanvasIntent::Redo(base) => self.begin(database, PendingOperation::Redo(base)),
            CanvasIntent::CreateText { base, position } => {
                if base != self.session.identity() { return Err(CanvasError::StaleBase.into()); }
                self.add_text(database, position)
            }
            CanvasIntent::EditText { base, node_id } => {
                if base != self.session.identity() { return Err(CanvasError::StaleBase.into()); }
                self.view.cancel_transient();
                self.text_editor = Some(TextCardEditor::new(&self.session, &node_id)?);
                Ok(())
            }
            CanvasIntent::Duplicate { base, node_ids } => self.duplicate(database, base, node_ids),
            CanvasIntent::OpenNode { base, node_id } => {
                if base != self.session.identity() { return Err(CanvasError::StaleBase.into()); }
                let node = self.session.document().node(&node_id).ok_or_else(|| CanvasError::NotFound(node_id.clone()))?;
                let extension = node.field("x-writers-studio").map(|raw| serde_json::from_str::<serde_json::Value>(raw.get())).transpose()?;
                let resource = extension.as_ref().and_then(|value| value.get("resource"));
                let kind = resource.and_then(|value| value["kind"].as_str());
                let id = resource.and_then(|value| value["id"].as_str());
                if let (Some(kind @ ("document" | "canvas" | "media")), Some(id)) = (kind, id) {
                    self.resource = Some(CanvasResource { project: self.project_id.clone(), kind: kind.into(), logical_id: id.into() });
                    Ok(())
                } else {
                    Err(CanvasError::Invalid("This card has no supported project resource reference".into()).into())
                }
            }
            CanvasIntent::Paste { base, position, text: Some(text) } => {
                if base != self.session.identity() { return Err(CanvasError::StaleBase.into()); }
                let node = NodeRecord::text(uuid::Uuid::new_v4().to_string(), WorldRect {
                    x: position.x.round(), y: position.y.round(), width: 300.0, height: 180.0,
                }, text, self.session.limits())?;
                self.command(database, vec![CanvasCommand::InsertNode { node, index: self.session.document().nodes().len() }])
            }
            CanvasIntent::Copy { .. } | CanvasIntent::Cut { .. } | CanvasIntent::Paste { text: None, .. } =>
                Err(CanvasError::Invalid("Structured board clipboard transfer is not available in this integration checkpoint".into()).into()),
        }
    }
    pub(super) fn show(
        &mut self,
        ui: &mut egui::Ui,
        database: &Path,
        language: Language,
        tokens: Tokens,
        blocked: bool,
    ) {
        let l = language;
        egui::Panel::top("canvas-heading")
            .exact_size(56.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(tokens.editor)
                    .inner_margin(egui::Margin::symmetric(24, 8)),
            )
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    icons::icon(ui, Icon::Board, 20.0, tokens.accent);
                    ui.add(
                        egui::Label::new(
                            RichText::new(&self.snapshot.canvas.title)
                                .size(17.0)
                                .strong(),
                        )
                        .truncate(),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let fitting =
                            icons::icon_button(ui, Icon::Focus, l.text("Fit board")).clicked();
                        if fitting {
                            let result = self
                                .view
                                .fit(&self.session, self.viewport_size, false)
                                .map_err(Into::into);
                            self.set_failure(result);
                        }
                        ui.label(
                            RichText::new(l.text(if self.pending.is_some() {
                                "Saving…"
                            } else {
                                "All changes saved"
                            }))
                            .size(12.0)
                            .color(tokens.muted),
                        );
                    });
                });
            });
        egui::Panel::top("canvas-tools")
            .exact_size(40.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(tokens.editor)
                    .inner_margin(egui::Margin::symmetric(16, 4)),
            )
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_enabled_ui(self.editable() && !blocked && !self.modal_open(), |ui| {
                        if icons::labeled_button(ui, Icon::Plus, l.text("Text card"), false)
                            .clicked()
                        {
                            let result = self.add_text(database, WorldPoint { x: 0.0, y: 0.0 });
                            self.set_failure(result);
                        }
                        if ui
                            .add_enabled(
                                self.session.history_usage().undo_steps > 0,
                                egui::Button::new(l.text("Undo")),
                            )
                            .clicked()
                        {
                            let result = self
                                .begin(database, PendingOperation::Undo(self.session.identity()));
                            self.set_failure(result);
                        }
                        if ui
                            .add_enabled(
                                self.session.history_usage().redo_steps > 0,
                                egui::Button::new(l.text("Redo")),
                            )
                            .clicked()
                        {
                            let result = self
                                .begin(database, PendingOperation::Redo(self.session.identity()));
                            self.set_failure(result);
                        }
                    });
                });
            });
        if self.snapshot.draft.is_some() {
            ui.vertical(|ui| {
                ui.colored_label(
                    tokens.warning,
                    l.text("This board has a preserved recovery draft. Review it before editing."),
                );
                ui.horizontal_wrapped(|ui| {
                    let current = self.snapshot.draft.as_ref().is_some_and(|draft| {
                        draft.base_revision == self.snapshot.revision.info.revision
                    });
                    if ui
                        .add_enabled(current, egui::Button::new(l.text("Recover board draft")))
                        .clicked()
                    {
                        let result = Workspace::open(database)
                            .map_err(Into::into)
                            .and_then(|workspace| self.recover(&workspace));
                        self.set_failure(result);
                    }
                    if ui.button(l.text("Preserve recovery board…")).clicked() {
                        let result = self.preserve_recovery(l);
                        self.set_failure(result);
                    }
                    if ui
                        .add_enabled(
                            self.draft_preserved,
                            egui::Button::new(l.text("Keep saved board")),
                        )
                        .clicked()
                    {
                        let result = self.keep_saved(database);
                        self.set_failure(result);
                    }
                });
            });
        }
        if let Some(failure) = self.failure.clone() {
            egui::Frame::new()
                .fill(tokens.surface)
                .inner_margin(12)
                .show(ui, |ui| {
                    ui.colored_label(
                        tokens.warning,
                        l.text(if self.failed.is_some() {
                            "The board could not be saved. Pending changes are preserved."
                        } else {
                            "The board operation failed. Your saved content is unchanged."
                        }),
                    );
                    if self.failed.is_some() && self.pending.is_none() {
                        ui.horizontal_wrapped(|ui| {
                            if ui.button(l.text("Retry save")).clicked() {
                                let result = self.retry(database);
                                self.set_failure(result);
                            }
                            if ui.button(l.text("Preserve pending board…")).clicked() {
                                let result = self.preserve_pending(l);
                                self.set_failure(result);
                            }
                            if ui
                                .add_enabled(
                                    self.failed_preserved,
                                    egui::Button::new(l.text("Reload saved board")),
                                )
                                .clicked()
                            {
                                let result = self.reload_saved(database);
                                self.set_failure(result);
                            }
                        });
                    }
                    ui.collapsing(l.text("Technical details"), |ui| {
                        ui.label(failure);
                    });
                });
        }
        let mut config = CanvasViewConfig::new(
            egui::Id::new(("project-canvas", &self.project_id, &self.snapshot.canvas.id)),
            ui,
        );
        config.labels = labels(l);
        config.style.background = tokens.editor;
        config.style.card = tokens.surface;
        config.style.border = tokens.border;
        config.style.text = tokens.text;
        config.style.selection = tokens.accent;
        config.style.error = tokens.warning;
        config.interactive = self.editable() && !blocked && !self.modal_open();
        config.shortcuts_blocked = !config.interactive;
        self.viewport_size = ui.available_size_before_wrap();
        if self.fit_on_open && self.viewport_size.x > 0.0 && self.viewport_size.y > 0.0 {
            let result = self
                .view
                .fit(&self.session, self.viewport_size, false)
                .map_err(Into::into);
            self.fit_on_open = false;
            self.set_failure(result);
        }
        let mut host = CardBody {
            language: l,
            tokens,
        };
        let output = self
            .view
            .show(ui, &self.session, &self.dialect, &mut host, &config);
        for error in output.errors {
            self.failure = Some(error.to_string());
        }
        for intent in output.intents {
            // The first completed mutation freezes this session until its durable acknowledgement.
            let result = self.intent(database, intent);
            self.set_failure(result);
        }
    }
    pub(super) fn dialogs(
        &mut self,
        ctx: &egui::Context,
        database: &Path,
        language: Language,
        tokens: Tokens,
    ) {
        let Some(editor) = &mut self.text_editor else {
            return;
        };
        let pending = self.pending.is_some();
        let failed = self.failed.is_some();
        let failure = self.failure.clone();
        let mut retry = false;
        let mut preserve = false;
        let mut request = None;
        let mut cancel = false;
        let response = components::modal(
            ctx,
            language,
            "canvas-source",
            language.text("Edit text card"),
            680.0,
            |ui| {
                ui.add_enabled_ui(!pending && !failed, |ui| {
                    let result = editor.show(ui, &labels(language));
                    request = result.transaction;
                    cancel = result.cancelled;
                });
                if pending {
                    ui.label(language.text("Saving…"));
                }
                if failed && !pending {
                    ui.colored_label(
                        tokens.warning,
                        language
                            .text("The board could not be saved. Pending changes are preserved."),
                    );
                    ui.horizontal_wrapped(|ui| {
                        retry = ui.button(language.text("Retry save")).clicked();
                        preserve = ui
                            .button(language.text("Preserve pending board…"))
                            .clicked();
                    });
                    if let Some(failure) = &failure {
                        ui.collapsing(language.text("Technical details"), |ui| {
                            ui.label(failure);
                        });
                    }
                }
            },
        );
        if (response.should_close() || cancel) && !pending {
            self.text_editor = None;
            self.view.cancel_transient();
        } else if let Some(transaction) = request {
            let result = self.begin(database, PendingOperation::Transaction(transaction));
            self.set_failure(result);
        }
        if retry {
            let result = self.retry(database);
            self.set_failure(result);
        }
        if preserve {
            let result = self.preserve_pending(language);
            self.set_failure(result);
        }
    }
    fn preserve_pending(&mut self, language: Language) -> AppResult<()> {
        let prepared = self.failed.as_ref().ok_or(CanvasError::NoHistory)?;
        if write_board_copy(
            language,
            "pending-board.canvas",
            prepared.document().raw_json(),
        )? {
            self.failed_preserved = true;
        }
        Ok(())
    }
    fn preserve_recovery(&mut self, language: Language) -> AppResult<()> {
        let draft = self.snapshot.draft.as_ref().ok_or(CanvasError::NoHistory)?;
        if write_board_copy(
            language,
            "recovery-board.canvas",
            &serde_json::to_string_pretty(&draft.content)?,
        )? {
            self.draft_preserved = true;
        }
        Ok(())
    }
    fn reload_saved(&mut self, database: &Path) -> AppResult<()> {
        if !self.failed_preserved || self.pending.is_some() {
            return Err(CanvasError::StaleBase.into());
        }
        *self = Self::open(
            &Workspace::open(database)?,
            &self.project_id,
            &self.snapshot.canvas.id,
        )?;
        Ok(())
    }
    fn keep_saved(&mut self, database: &Path) -> AppResult<()> {
        if !self.draft_preserved || self.pending.is_some() {
            return Err(CanvasError::StaleBase.into());
        }
        let workspace = Workspace::open(database)?;
        let draft = self.snapshot.draft.as_ref().ok_or(CanvasError::NoHistory)?;
        workspace.discard_project_canvas_draft(
            &self.project_id,
            &self.snapshot.canvas.id,
            &draft.version,
        )?;
        *self = Self::open(&workspace, &self.project_id, &self.snapshot.canvas.id)?;
        Ok(())
    }
    /// Recover only the exact persisted native draft, never an imported checkpoint envelope.
    pub(super) fn recover(&mut self, workspace: &Workspace) -> AppResult<()> {
        if self.pending.is_some() || self.failed.is_some() {
            return Err(CanvasError::StaleBase.into());
        }
        let draft = self.snapshot.draft.as_ref().ok_or(CanvasError::NoHistory)?;
        workspace.recover_project_canvas_draft(
            &self.project_id,
            &self.snapshot.canvas.id,
            self.snapshot.revision.info.revision,
            &draft.version,
        )?;
        *self = Self::open(workspace, &self.project_id, &self.snapshot.canvas.id)?;
        Ok(())
    }
    pub(super) fn preserved_draft(&self) -> Option<&ProjectCanvasDraft> {
        self.snapshot.draft.as_ref()
    }
}

fn write_board_copy(language: Language, filename: &str, content: &str) -> AppResult<bool> {
    let Some(path) = rfd::FileDialog::new()
        .set_title(language.text("Preserve board JSON"))
        .set_file_name(filename)
        .add_filter("JSON Canvas", &["canvas"])
        .save_file()
    else {
        return Ok(false);
    };
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(content.as_bytes())?;
    file.flush()?;
    file.sync_all()?;
    Ok(true)
}

fn labels(language: Language) -> CanvasViewLabels {
    CanvasViewLabels {
        canvas: language.text("Canvas").into(),
        save: language.text("Save").into(),
        cancel: language.text("Cancel").into(),
        text_source: language.text("Markdown source").into(),
    }
}

fn prepare_and_save(
    session: Arc<CanvasSession>,
    database: &Path,
    project: &str,
    board: &str,
    revision: ProjectCanvasRevision,
    operation: PendingOperation,
) -> CommitCompletion {
    let prepared = (|| -> AppResult<PreparedChange> {
        let dialect = SourceDialect::new(session.document())?;
        let prepared = match operation {
            PendingOperation::Transaction(transaction) => session.prepare(transaction, &dialect)?,
            PendingOperation::Undo(base) => {
                if base != session.identity() {
                    return Err(CanvasError::StaleBase.into());
                }
                session.prepare_undo()?
            }
            PendingOperation::Redo(base) => {
                if base != session.identity() {
                    return Err(CanvasError::StaleBase.into());
                }
                session.prepare_redo()?
            }
        };
        Ok(prepared)
    })();
    // Acknowledgement on the render thread requires unique ownership, never a worker borrow.
    drop(session);
    match prepared {
        Ok(prepared) => persist_candidate(database, project, board, revision, prepared),
        Err(error) => CommitCompletion {
            prepared: None,
            result: Err(error),
        },
    }
}
fn persist_candidate(
    database: &Path,
    project: &str,
    board: &str,
    revision: ProjectCanvasRevision,
    prepared: PreparedChange,
) -> CommitCompletion {
    let result = (|| -> AppResult<ProjectCanvasRevision> {
        if !prepared.changed() {
            return Ok(revision);
        }
        let workspace = Workspace::open(database)?;
        let content = serde_json::from_str(prepared.document().raw_json())?;
        Ok(workspace.save_project_canvas(project, board, revision.info.revision, content)?)
    })();
    CommitCompletion {
        prepared: Some(prepared),
        result,
    }
}

struct CardBody {
    language: Language,
    tokens: Tokens,
}
impl CanvasViewHost for CardBody {
    fn paint_body(
        &mut self,
        ui: &mut egui::Ui,
        node: &NodeRecord,
        rect: egui::Rect,
        zoom: f64,
    ) -> NodeBodyResponse {
        let color = self.tokens.text;
        let font = egui::FontId::proportional((14.0 * zoom as f32).clamp(8.0, 42.0));
        let text = match node.kind() {
            NodeKind::Text | NodeKind::Group => node.body(),
            NodeKind::File => node.body(),
            NodeKind::Link => node.body(),
            NodeKind::Unknown(_) => self
                .language
                .text("Unsupported board content is preserved."),
        };
        let galley = ui.painter().layout(text.into(), font, color, rect.width());
        ui.painter().galley(rect.left_top(), galley, color);
        NodeBodyResponse::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn graph() -> serde_json::Value {
        json!({"nodes":[{"id":"card","type":"text","x":0,"y":0,"width":300,"height":180,"text":"Old 😀 text","x-writers-studio":{"card":{"typeId":"unknown","fields":{"opaque":[1,null]}},"future":{"keep":true}}}],"edges":[],"x-writers-studio":{"schemaVersion":2,"vendor":{"keep":true}}})
    }
    #[test]
    fn a_prepared_board_edit_is_persisted_before_session_acknowledgement_and_undo_appends()
    -> TestResult {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("board.sqlite");
        let workspace = Workspace::open(&path)?;
        let project = workspace.create_project("Synthetic board")?;
        let board = workspace.create_project_canvas(&project.id, "Board", graph())?;
        let mut state = CanvasState::from_snapshot(&project.id, board)?;
        let old = state.session.document().raw_json().to_owned();
        let prepared = state.session.prepare(
            CanvasTransaction::new(
                state.session.identity(),
                vec![CanvasCommand::EditText {
                    node_id: "card".into(),
                    expected: "Old 😀 text".into(),
                    text: "New 日本語 text".into(),
                }],
            ),
            &state.dialect,
        )?;
        let completed = persist_candidate(
            &path,
            &project.id,
            &state.snapshot.canvas.id,
            state.snapshot.revision.clone(),
            prepared,
        );
        let saved = completed.result?;
        assert_eq!(state.session.document().raw_json(), old);
        assert_eq!(saved.info.revision, 2);
        let prepared = completed.prepared.ok_or("prepared")?;
        Arc::get_mut(&mut state.session)
            .ok_or("session")?
            .apply(prepared)?;
        assert_eq!(
            state.session.document().node("card").ok_or("card")?.body(),
            "New 日本語 text"
        );
        let undo = state.session.prepare_undo()?;
        let completed =
            persist_candidate(&path, &project.id, &state.snapshot.canvas.id, saved, undo);
        assert_eq!(completed.result?.info.revision, 3);
        Arc::get_mut(&mut state.session)
            .ok_or("session")?
            .apply(completed.prepared.ok_or("prepared")?)?;
        assert_eq!(state.session.document().raw_json(), old);
        let reopened =
            Workspace::open(&path)?.project_canvas(&project.id, &state.snapshot.canvas.id)?;
        assert_eq!(reopened.revision.content, graph());
        assert_eq!(reopened.revision.info.revision, 3);
        Ok(())
    }
    #[test]
    fn stale_durable_save_preserves_the_candidate_without_advancing_its_local_session() -> TestResult
    {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("stale.sqlite");
        let workspace = Workspace::open(&path)?;
        let project = workspace.create_project("Synthetic stale board")?;
        let board = workspace.create_project_canvas(&project.id, "Board", graph())?;
        let state = CanvasState::from_snapshot(&project.id, board)?;
        let prepared = state.session.prepare(
            CanvasTransaction::new(
                state.session.identity(),
                vec![CanvasCommand::Translate {
                    node_ids: vec!["card".into()],
                    delta: WorldPoint { x: 10.0, y: 20.0 },
                }],
            ),
            &state.dialect,
        )?;
        let mut newer = graph();
        newer["nodes"][0]["text"] = json!("Other writer");
        workspace.save_project_canvas(&project.id, &state.snapshot.canvas.id, 1, newer.clone())?;
        let completion = persist_candidate(
            &path,
            &project.id,
            &state.snapshot.canvas.id,
            state.snapshot.revision.clone(),
            prepared,
        );
        assert!(completion.result.is_err());
        assert!(completion.prepared.is_some());
        assert_eq!(
            state
                .session
                .document()
                .node("card")
                .ok_or("card")?
                .bounds()
                .x,
            0.0
        );
        assert_eq!(
            workspace
                .project_canvas(&project.id, &state.snapshot.canvas.id)?
                .revision
                .content,
            newer
        );
        Ok(())
    }
    #[test]
    fn source_dialect_retains_lock_and_frame_semantics_without_spatially_moving_frame_children()
    -> TestResult {
        let limits = CanvasLimits::default();
        let source = json!({"nodes":[{"id":"locked","type":"text","x":0,"y":0,"width":300,"height":180,"text":"Locked","x-writers-studio":{"locked":true}},{"id":"frame","type":"group","x":0,"y":0,"width":500,"height":500,"label":"Frame","x-writers-studio":{"element":{"kind":"frame","order":0}}}],"edges":[]});
        let document = CanvasDocument::parse(&serde_json::to_string(&source)?, &limits)?;
        let dialect = SourceDialect::new(&document)?;
        assert!(
            !dialect
                .node_policy(document.node("locked").ok_or("locked")?)
                .movable
        );
        assert_eq!(
            dialect
                .node_policy(document.node("frame").ok_or("frame")?)
                .group,
            GroupBehavior::Frame
        );
        Ok(())
    }
}
