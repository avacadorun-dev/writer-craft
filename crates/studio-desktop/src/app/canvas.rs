//! Native board projection over one project-scoped durable authority.
use super::native_media::{MediaCache, MediaViewer};
use super::*;
use craft_canvas_egui::{
    CanvasIntent, CanvasView, CanvasViewConfig, CanvasViewHost, CanvasViewLabels, InlineTextOutput,
    NodeBodyResponse, TextCardEditor,
};
use craft_canvas_model::{
    CanvasCommand, CanvasDialect, CanvasDocument, CanvasError, CanvasLimits, CanvasSession,
    CanvasTransaction, ContentIdentity, GroupBehavior, IdMapping, NodeKind, NodePolicy, NodeRecord,
    PreparedChange, WorkControl, WorldPoint, WorldRect,
};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::Arc;
use studio_core::{ProjectCanvasRevision, ProjectCanvasSnapshot};

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
    checkpoint_version: Option<String>,
}

/// The view is disposable; this fixed project/board/session owns every acknowledged transition.
pub(super) struct CanvasState {
    pub(super) project_id: String,
    pub(super) snapshot: ProjectCanvasSnapshot,
    session: Arc<CanvasSession>,
    view: CanvasView,
    dialect: SourceDialect,
    text_editor: Option<TextCardEditor>,
    text_changed_at: Instant,
    text_done: bool,
    text_cancel: bool,
    text_composing: bool,
    text_save_error: bool,
    edit_after_insert: Option<String>,
    checkpoint_version: Option<String>,
    pending: Option<mpsc::Receiver<CommitCompletion>>,
    failed: Option<PreparedChange>,
    failure: Option<String>,
    failed_preserved: bool,
    draft_preserved: bool,
    fit_on_open: bool,
    viewport_size: Vec2,
    resource: Option<CanvasResource>,
    media: Option<MediaCache>,
    media_viewer: Option<MediaViewer>,
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
            text_changed_at: Instant::now(),
            text_done: false,
            text_cancel: false,
            text_composing: false,
            text_save_error: false,
            edit_after_insert: None,
            checkpoint_version: None,
            pending: None,
            failed: None,
            failure: None,
            failed_preserved: false,
            draft_preserved: false,
            fit_on_open: true,
            viewport_size: Vec2::new(800.0, 600.0),
            resource: None,
            media: None,
            media_viewer: None,
        })
    }
    pub(super) fn modal_open(&self) -> bool {
        self.media_viewer.is_some()
    }
    /// Dirty or unacknowledged native input cannot silently disappear through navigation.
    pub(super) fn blocks_navigation(&self) -> bool {
        self.pending.is_some()
            || self.failed.is_some()
            || self.text_save_error
            || self.text_composing
            || self
                .text_editor
                .as_ref()
                .is_some_and(|editor| editor.dirty() || editor.has_submission())
    }
    /// One acknowledgement-aware status shared by all visible save indicators.
    pub(super) fn durability_status(&self) -> &'static str {
        if self.failed.is_some() || self.failure.is_some() || self.text_save_error {
            "Attention needed"
        } else if self.pending.is_some() {
            "Saving…"
        } else if self.blocks_navigation() {
            "Unsaved board changes"
        } else {
            "All changes saved"
        }
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
        self.text_save_error = false;
        self.checkpoint_version = None;
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
        let checkpoint_version = self.checkpoint_version.clone();
        std::thread::spawn(move || {
            let completion = persist_candidate_with_checkpoint(
                &path,
                &project,
                &board,
                revision,
                prepared,
                checkpoint_version.as_deref(),
            );
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
                self.text_save_error = self.text_editor.is_some();
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
                self.checkpoint_version = None;
                self.text_save_error = false;
                if let Some(editor) = &mut self.text_editor
                    && editor.has_submission()
                {
                    editor.acknowledge(session)?;
                }
                if let Some(id) = self.edit_after_insert.take() {
                    self.text_editor = Some(TextCardEditor::new(session, &id)?);
                    self.text_changed_at = Instant::now();
                }
                if self.text_cancel
                    || (self.text_done
                        && self
                            .text_editor
                            .as_ref()
                            .is_some_and(|editor| !editor.dirty()))
                {
                    self.finish_inline();
                }
                Ok(())
            }
            Err(error) => {
                self.failed = completion.prepared;
                self.checkpoint_version = completion.checkpoint_version;
                self.text_save_error = self
                    .text_editor
                    .as_ref()
                    .is_some_and(TextCardEditor::has_submission);
                if self.failed.is_none()
                    && let Some(editor) = &mut self.text_editor
                {
                    editor.submission_not_started();
                }
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
        let id = node.id().to_owned();
        self.command(
            database,
            vec![CanvasCommand::InsertNode {
                node,
                index: self.session.document().nodes().len(),
            }],
        )?;
        self.edit_after_insert = Some(id);
        Ok(())
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
                let node=self.session.document().node(&node_id).ok_or_else(||CanvasError::NotFound(node_id.clone()))?;
                if !self.dialect.node_policy(node).movable { return Err(CanvasError::NotEditable(node_id).into()); }
                self.view.cancel_transient();
                self.text_editor = Some(TextCardEditor::new(&self.session, &node_id)?);
                self.text_changed_at=Instant::now(); self.text_done=false; self.text_cancel=false; self.text_composing=false; self.text_save_error=false;
                Ok(())
            }
            CanvasIntent::Duplicate { base, node_ids } => self.duplicate(database, base, node_ids),
            CanvasIntent::OpenNode { base, node_id } => {
                if base != self.session.identity() { return Err(CanvasError::StaleBase.into()); }
                let node = self.session.document().node(&node_id).ok_or_else(|| CanvasError::NotFound(node_id.clone()))?;
                if matches!(node.kind(), NodeKind::Link) {
                    self.resource=Some(CanvasResource {project:self.project_id.clone(),kind:"url".into(),logical_id:node.body().into()});
                    return Ok(());
                }
                let extension = node.field("x-writers-studio").map(|raw| serde_json::from_str::<serde_json::Value>(raw.get())).transpose()?;
                let resource = extension.as_ref().and_then(|value| value.get("resource"));
                let kind = resource.and_then(|value| value["kind"].as_str());
                let id = resource.and_then(|value| value["id"].as_str());
                if let (Some("media"), Some(id)) = (kind, id) {
                    self.view.cancel_transient();
                    self.media_viewer = Some(MediaViewer::new(id.into()));
                    Ok(())
                } else if let (Some(kind @ ("document" | "canvas")), Some(id)) = (kind, id) {
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
                            RichText::new(l.text(self.durability_status()))
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
                    ui.add_enabled_ui(
                        self.editable()
                            && !blocked
                            && !self.modal_open()
                            && self.text_editor.is_none(),
                        |ui| {
                            if icons::labeled_button(ui, Icon::Plus, l.text("Text card"), false)
                                .clicked()
                            {
                                let result = self.add_text(database, self.view.camera().center());
                                self.set_failure(result);
                            }
                            if ui
                                .add_enabled(
                                    self.session.history_usage().undo_steps > 0,
                                    egui::Button::new(l.text("Undo")),
                                )
                                .clicked()
                            {
                                let result = self.begin(
                                    database,
                                    PendingOperation::Undo(self.session.identity()),
                                );
                                self.set_failure(result);
                            }
                            if ui
                                .add_enabled(
                                    self.session.history_usage().redo_steps > 0,
                                    egui::Button::new(l.text("Redo")),
                                )
                                .clicked()
                            {
                                let result = self.begin(
                                    database,
                                    PendingOperation::Redo(self.session.identity()),
                                );
                                self.set_failure(result);
                            }
                        },
                    );
                    if self.text_editor.is_some() {
                        ui.separator();
                        ui.label(l.text("Editing text card"));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .add_enabled(
                                    !self.text_composing,
                                    egui::Button::new(l.text("Done")),
                                )
                                .clicked()
                            {
                                self.text_done = true;
                            }
                            if ui
                                .add_enabled(
                                    !self.text_composing,
                                    egui::Button::new(l.text("Cancel")),
                                )
                                .clicked()
                            {
                                self.text_cancel = true;
                            }
                        });
                    }
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
                        l.text(if self.failed.is_some() || self.text_save_error {
                            "The board could not be saved. Pending changes are preserved."
                        } else {
                            "The board operation failed. Your saved content is unchanged."
                        }),
                    );
                    if (self.failed.is_some() || self.text_save_error) && self.pending.is_none() {
                        ui.horizontal_wrapped(|ui| {
                            if ui.button(l.text("Retry save")).clicked() {
                                let result = if self.failed.is_some() {
                                    self.retry(database)
                                } else {
                                    self.submit_inline(database)
                                };
                                self.set_failure(result);
                            }
                            if (self.failed.is_some() || self.text_editor.is_some())
                                && ui.button(l.text("Preserve pending board…")).clicked()
                            {
                                let result = self.preserve_pending(l);
                                self.set_failure(result);
                            }
                            if self.text_editor.is_some()
                                && ui.button(l.text("Preserve card text…")).clicked()
                            {
                                let result = self.preserve_card_text(l);
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
        config.interactive =
            self.editable() && !blocked && !self.modal_open() && self.text_editor.is_none();
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
        let media = self
            .media
            .get_or_insert_with(|| MediaCache::new(database.to_owned()));
        media.poll(ui.ctx());
        let mut inline_output = None;
        let mut host = CardBody {
            language: l,
            tokens,
            project: &self.project_id,
            media,
            editor: self.text_editor.as_mut(),
            inline_output: &mut inline_output,
            blocked: blocked || self.media_viewer.is_some(),
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
        if let Some(output) = inline_output {
            self.text_composing = output.composing;
            if output.changed {
                self.text_changed_at = Instant::now();
                self.failed_preserved = false;
            }
            self.text_done |= output.done_requested;
            self.text_cancel |= output.cancelled;
        }
        if self.text_editor.is_some() {
            let result = self.drive_inline(database, ui.ctx());
            self.set_failure(result);
        }
    }
    fn finish_inline(&mut self) {
        self.text_editor = None;
        self.text_done = false;
        self.text_cancel = false;
        self.text_composing = false;
        self.text_save_error = false;
        self.view.cancel_transient();
    }
    fn submit_inline(&mut self, database: &Path) -> AppResult<()> {
        let transaction = self
            .text_editor
            .as_mut()
            .ok_or(CanvasError::NoHistory)?
            .submit()?;
        if let Err(error) = self.begin(database, PendingOperation::Transaction(transaction)) {
            if let Some(editor) = &mut self.text_editor {
                editor.submission_not_started();
            }
            self.text_save_error = true;
            return Err(error);
        }
        Ok(())
    }
    /// Flush the active card through its normal guarded save, retaining direct input focus.
    /// An already pending write remains in progress; this is not an early durability claim.
    pub(super) fn save_now(&mut self, database: &Path) -> AppResult<()> {
        if self.pending.is_some() {
            return Ok(());
        }
        if self.failed.is_some() {
            return self.retry(database);
        }
        if self.text_composing {
            return Err(PendingCanvas.into());
        }
        if self.text_editor.as_ref().is_some_and(TextCardEditor::dirty) {
            return self.submit_inline(database);
        }
        Ok(())
    }
    fn drive_inline(&mut self, database: &Path, ctx: &egui::Context) -> AppResult<()> {
        if self.pending.is_some()
            || self.failed.is_some()
            || self.text_save_error
            || self.text_composing
        {
            return Ok(());
        }
        let Some(editor) = &self.text_editor else {
            return Ok(());
        };
        if self.text_cancel || (self.text_done && !editor.dirty()) {
            self.finish_inline();
            return Ok(());
        }
        if editor.dirty() {
            let debounce = Duration::from_millis(600);
            let elapsed = self.text_changed_at.elapsed();
            if self.text_done || elapsed >= debounce {
                self.submit_inline(database)?;
            } else {
                ctx.request_repaint_after(debounce - elapsed);
            }
        }
        Ok(())
    }
    pub(super) fn dialogs(
        &mut self,
        ctx: &egui::Context,
        _database: &Path,
        language: Language,
        tokens: Tokens,
    ) {
        if let (Some(media), Some(viewer)) = (&mut self.media, &mut self.media_viewer)
            && media.viewer(ctx, &self.project_id, viewer, language, tokens)
        {
            self.media_viewer = None;
        }
    }
    fn preserve_card_text(&mut self, language: Language) -> AppResult<()> {
        let text = self
            .text_editor
            .as_ref()
            .ok_or(CanvasError::NoHistory)?
            .text();
        let Some(path) = rfd::FileDialog::new()
            .set_title(language.text("Preserve card text…"))
            .set_file_name("pending-card.md")
            .add_filter("Markdown", &["md"])
            .save_file()
        else {
            return Ok(());
        };
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        file.write_all(text.as_bytes())?;
        file.flush()?;
        file.sync_all()?;
        Ok(())
    }
    fn preserve_pending(&mut self, language: Language) -> AppResult<()> {
        let latest = if let Some(editor) = &self.text_editor {
            Some(self.session.prepare(editor.transaction(), &self.dialect)?)
        } else {
            None
        };
        let prepared = latest
            .as_ref()
            .or(self.failed.as_ref())
            .ok_or(CanvasError::NoHistory)?;
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
            checkpoint_version: None,
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
    persist_candidate_with_checkpoint(database, project, board, revision, prepared, None)
}
fn persist_candidate_with_checkpoint(
    database: &Path,
    project: &str,
    board: &str,
    revision: ProjectCanvasRevision,
    prepared: PreparedChange,
    expected_checkpoint: Option<&str>,
) -> CommitCompletion {
    let mut checkpoint_version = expected_checkpoint.map(str::to_owned);
    let result = (|| -> AppResult<ProjectCanvasRevision> {
        if !prepared.changed() {
            return Ok(revision);
        }
        let workspace = Workspace::open(database)?;
        let content: serde_json::Value = serde_json::from_str(prepared.document().raw_json())?;
        let draft = workspace.save_project_canvas_draft(
            project,
            board,
            revision.info.revision,
            expected_checkpoint,
            content.clone(),
        )?;
        checkpoint_version = Some(draft.version);
        Ok(workspace.save_project_canvas(project, board, revision.info.revision, content)?)
    })();
    CommitCompletion {
        prepared: Some(prepared),
        result,
        checkpoint_version,
    }
}

struct CardBody<'a> {
    language: Language,
    tokens: Tokens,
    project: &'a str,
    media: &'a mut MediaCache,
    editor: Option<&'a mut TextCardEditor>,
    inline_output: &'a mut Option<InlineTextOutput>,
    blocked: bool,
}
impl CanvasViewHost for CardBody<'_> {
    fn paint_body(
        &mut self,
        ui: &mut egui::Ui,
        node: &NodeRecord,
        rect: egui::Rect,
        zoom: f64,
    ) -> NodeBodyResponse {
        if let Some(editor) = &mut self.editor
            && editor.node_id() == node.id()
        {
            let output = ui
                .add_enabled_ui(!self.blocked, |ui| editor.show_inline(ui, rect, zoom))
                .inner;
            let input = output.input;
            *self.inline_output = Some(output);
            return input;
        }
        if let Some(id) = media_resource(node) {
            return self.media.card(
                ui,
                self.project,
                &id,
                rect.size(),
                self.language,
                self.tokens,
            );
        }
        let color = self.tokens.text;
        let font = egui::FontId::proportional((14.0 * zoom as f32).clamp(8.0, 42.0));
        let text = match node.kind() {
            NodeKind::Text if node.body().trim().is_empty() => {
                self.language.text("Double-click to write a note…")
            }
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

/// Read only a source project logical reference; never turn the raw file body into a path.
fn media_resource(node: &NodeRecord) -> Option<String> {
    let extension: serde_json::Value =
        serde_json::from_str(node.field("x-writers-studio")?.get()).ok()?;
    let resource = extension.get("resource")?;
    (resource["kind"].as_str()? == "media")
        .then(|| resource["id"].as_str().map(str::to_owned))
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
    #[test]
    fn media_reference_should_require_project_metadata_instead_of_trusting_a_raw_file_path()
    -> TestResult {
        let limits = CanvasLimits::default();
        let json = r#"{"nodes":[{"id":"raw","type":"file","x":0,"y":0,"width":300,"height":180,"file":"/private/original.png"},{"id":"scoped","type":"file","x":320,"y":0,"width":300,"height":180,"file":"inert/path","x-writers-studio":{"resource":{"kind":"media","id":"original-media"},"future":{"keep":true}}}],"edges":[]}"#;
        let document = CanvasDocument::parse(json, &limits)?;
        assert_eq!(document.node("raw").and_then(media_resource), None);
        assert_eq!(
            document.node("scoped").and_then(media_resource).as_deref(),
            Some("original-media")
        );
        assert_eq!(document.raw_json(), json);
        Ok(())
    }
    fn deliver(state: &mut CanvasState, completion: CommitCompletion) -> AppResult<()> {
        let (sender, receiver) = mpsc::channel();
        sender
            .send(completion)
            .map_err(|_| "synthetic completion channel closed")?;
        state.pending = Some(receiver);
        state.poll(&egui::Context::default())
    }
    #[test]
    fn inline_save_acknowledgement_should_keep_later_typing_and_append_durable_text_history()
    -> TestResult {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("inline.sqlite");
        let workspace = Workspace::open(&database)?;
        let project = workspace.create_project("Synthetic inline card")?;
        let board = workspace.create_project_canvas(&project.id, "Board", graph())?;
        let mut state = CanvasState::from_snapshot(&project.id, board)?;
        state.text_editor = Some(TextCardEditor::new(&state.session, "card")?);
        let editor = state.text_editor.as_mut().ok_or("editor")?;
        *editor.text_mut() = "Submitted 日本語".into();
        let transaction = editor.submit()?;
        let prepared = state.session.prepare(transaction, &state.dialect)?;
        *state.text_editor.as_mut().ok_or("editor")?.text_mut() =
            "Submitted 日本語; typed during save 🪶".into();
        let completion = persist_candidate(
            &database,
            &project.id,
            &state.snapshot.canvas.id,
            state.snapshot.revision.clone(),
            prepared,
        );
        assert_eq!(
            state.session.document().node("card").ok_or("card")?.body(),
            "Old 😀 text"
        );
        assert!(completion.result.is_ok());
        deliver(&mut state, completion)?;
        let editor = state.text_editor.as_mut().ok_or("editor")?;
        assert_eq!(editor.text(), "Submitted 日本語; typed during save 🪶");
        assert!(editor.dirty());
        let transaction = editor.submit()?;
        let prepared = state.session.prepare(transaction, &state.dialect)?;
        let completion = persist_candidate(
            &database,
            &project.id,
            &state.snapshot.canvas.id,
            state.snapshot.revision.clone(),
            prepared,
        );
        deliver(&mut state, completion)?;
        assert!(!state.text_editor.as_ref().ok_or("editor")?.dirty());
        let reopened = workspace.project_canvas(&project.id, &state.snapshot.canvas.id)?;
        assert_eq!(reopened.revision.info.revision, 3);
        assert!(reopened.draft.is_none());
        assert_eq!(
            reopened.revision.content["nodes"][0]["text"],
            "Submitted 日本語; typed during save 🪶"
        );
        assert_eq!(
            reopened.revision.content["nodes"][0]["x-writers-studio"]["future"],
            graph()["nodes"][0]["x-writers-studio"]["future"]
        );
        state.finish_inline();
        let prepared = state.session.prepare_undo()?;
        let completion = persist_candidate(
            &database,
            &project.id,
            &state.snapshot.canvas.id,
            state.snapshot.revision.clone(),
            prepared,
        );
        deliver(&mut state, completion)?;
        assert_eq!(
            workspace
                .project_canvas(&project.id, &state.snapshot.canvas.id)?
                .revision
                .content["nodes"][0]["text"],
            "Submitted 日本語"
        );
        Ok(())
    }
    #[test]
    fn failed_inline_save_should_keep_checkpoint_token_latest_draft_and_retry_without_adopting_other_drafts()
    -> TestResult {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("failure.sqlite");
        let workspace = Workspace::open(&database)?;
        let project = workspace.create_project("Synthetic failed text")?;
        let board = workspace.create_project_canvas(&project.id, "Board", graph())?;
        let mut state = CanvasState::from_snapshot(&project.id, board)?;
        let mut editor = TextCardEditor::new(&state.session, "card")?;
        *editor.text_mut() = "Exact pending source".into();
        let prepared = state.session.prepare(editor.submit()?, &state.dialect)?;
        let content = serde_json::from_str(prepared.document().raw_json())?;
        let checkpoint = workspace.save_project_canvas_draft(
            &project.id,
            &state.snapshot.canvas.id,
            1,
            None,
            content,
        )?;
        *editor.text_mut() = "Later text retained after failure".into();
        state.text_editor = Some(editor);
        assert!(
            deliver(
                &mut state,
                CommitCompletion {
                    prepared: Some(prepared),
                    result: Err("synthetic failure after checkpoint".into()),
                    checkpoint_version: Some(checkpoint.version.clone())
                }
            )
            .is_err()
        );
        assert!(state.blocks_navigation());
        assert!(state.text_save_error);
        assert_eq!(
            state.text_editor.as_ref().ok_or("editor")?.text(),
            "Later text retained after failure"
        );
        let candidate = state.failed.take().ok_or("candidate")?;
        let completion = persist_candidate_with_checkpoint(
            &database,
            &project.id,
            &state.snapshot.canvas.id,
            state.snapshot.revision.clone(),
            candidate,
            state.checkpoint_version.as_deref(),
        );
        assert!(completion.result.is_ok());
        deliver(&mut state, completion)?;
        assert_eq!(
            state.text_editor.as_ref().ok_or("editor")?.text(),
            "Later text retained after failure"
        );
        assert!(state.text_editor.as_ref().ok_or("editor")?.dirty());
        assert!(
            workspace
                .project_canvas(&project.id, &state.snapshot.canvas.id)?
                .draft
                .is_none()
        );
        Ok(())
    }
    #[test]
    fn a_foreign_checkpoint_should_refuse_the_inline_save_and_preserve_both_working_texts()
    -> TestResult {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("foreign-draft.sqlite");
        let workspace = Workspace::open(&database)?;
        let project = workspace.create_project("Synthetic checkpoint conflict")?;
        let board = workspace.create_project_canvas(&project.id, "Board", graph())?;
        let mut state = CanvasState::from_snapshot(&project.id, board)?;
        let mut editor = TextCardEditor::new(&state.session, "card")?;
        *editor.text_mut() = "My unsaved text".into();
        let prepared = state.session.prepare(editor.submit()?, &state.dialect)?;
        state.text_editor = Some(editor);
        let mut other = graph();
        other["nodes"][0]["text"] = json!("Another draft");
        let foreign = workspace.save_project_canvas_draft(
            &project.id,
            &state.snapshot.canvas.id,
            1,
            None,
            other.clone(),
        )?;
        let completion = persist_candidate(
            &database,
            &project.id,
            &state.snapshot.canvas.id,
            state.snapshot.revision.clone(),
            prepared,
        );
        assert!(deliver(&mut state, completion).is_err());
        assert!(state.failed.is_some());
        assert!(state.blocks_navigation());
        assert_eq!(
            state.text_editor.as_ref().ok_or("editor")?.text(),
            "My unsaved text"
        );
        let actual = workspace.project_canvas(&project.id, &state.snapshot.canvas.id)?;
        assert_eq!(actual.revision.content, graph());
        assert_eq!(
            actual.draft.as_ref().ok_or("draft")?.version,
            foreign.version
        );
        assert_eq!(actual.draft.ok_or("draft")?.content, other);
        Ok(())
    }
    #[test]
    fn new_card_should_activate_direct_editor_only_after_durable_creation_acknowledgement()
    -> TestResult {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("new-card.sqlite");
        let workspace = Workspace::open(&database)?;
        let project = workspace.create_project("Synthetic new card")?;
        let board = workspace.create_project_canvas(&project.id, "Board", graph())?;
        let mut state = CanvasState::from_snapshot(&project.id, board)?;
        state.add_text(&database, WorldPoint { x: 320., y: 0. })?;
        assert!(state.text_editor.is_none());
        assert!(state.pending.is_some());
        let deadline = Instant::now() + Duration::from_secs(5);
        while state.pending.is_some() && Instant::now() < deadline {
            state.poll(&egui::Context::default())?;
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(state.pending.is_none());
        let editor = state.text_editor.as_ref().ok_or("direct editor")?;
        assert!(editor.text().is_empty());
        assert!(state.view.selection().nodes.contains(editor.node_id()));
        assert!(!state.blocks_navigation());
        assert_eq!(
            workspace
                .project_canvas(&project.id, &state.snapshot.canvas.id)?
                .revision
                .info
                .revision,
            2
        );
        Ok(())
    }
    #[test]
    fn composing_card_should_not_autosave_and_explicit_cancel_should_keep_canonical_source()
    -> TestResult {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("composition.sqlite");
        let workspace = Workspace::open(&database)?;
        let project = workspace.create_project("Synthetic composition")?;
        let board = workspace.create_project_canvas(&project.id, "Board", graph())?;
        let mut state = CanvasState::from_snapshot(&project.id, board)?;
        state.text_editor = Some(TextCardEditor::new(&state.session, "card")?);
        state.text_composing = true;
        assert!(
            state.blocks_navigation(),
            "preedit ownership blocks navigation even before text changes"
        );
        let mut editor = state.text_editor.take().ok_or("editor")?;
        *editor.text_mut() = "Uncommitted composition".into();
        state.text_editor = Some(editor);
        state.text_composing = true;
        state.text_done = true;
        state.drive_inline(&database, &egui::Context::default())?;
        assert!(state.pending.is_none());
        assert!(state.blocks_navigation());
        state.text_composing = false;
        state.text_cancel = true;
        state.drive_inline(&database, &egui::Context::default())?;
        assert!(state.text_editor.is_none());
        assert!(!state.blocks_navigation());
        assert_eq!(
            workspace
                .project_canvas(&project.id, &state.snapshot.canvas.id)?
                .revision
                .content,
            graph()
        );
        Ok(())
    }
    fn show_card_frame(
        state: &mut CanvasState,
        database: &Path,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        time: f64,
    ) {
        let tokens = theme::resolve(&ThemeSettings::default(), false);
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1000., 680.),
                )),
                events,
                time: Some(time),
                focused: true,
                ..Default::default()
            },
            |root| {
                egui::CentralPanel::default().show(root, |ui| {
                    state.show(ui, database, Language::English, tokens, false);
                });
            },
        );
        output.textures_delta.clear();
        output.platform_output.commands.clear();
    }
    #[test]
    fn actual_host_card_input_should_accept_native_paste_and_done_without_opening_a_source_modal()
    -> TestResult {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("native-card.sqlite");
        let workspace = Workspace::open(&database)?;
        let project = workspace.create_project("Synthetic native card input")?;
        let board = workspace.create_project_canvas(&project.id, "Board", graph())?;
        let mut state = CanvasState::from_snapshot(&project.id, board)?;
        state.intent(
            &database,
            CanvasIntent::EditText {
                base: state.session.identity(),
                node_id: "card".into(),
            },
        )?;
        assert!(!state.modal_open());
        let ctx = egui::Context::default();
        show_card_frame(&mut state, &database, &ctx, vec![], 0.);
        let key = |key, modifiers| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        show_card_frame(
            &mut state,
            &database,
            &ctx,
            vec![
                key(egui::Key::A, egui::Modifiers::COMMAND),
                egui::Event::Paste("Native in-card 日本語 🪶".into()),
            ],
            1.,
        );
        assert_eq!(
            state.text_editor.as_ref().ok_or("direct input")?.text(),
            "Native in-card 日本語 🪶"
        );
        assert!(state.blocks_navigation());
        assert!(state.pending.is_none());
        assert_eq!(
            state.session.document().node("card").ok_or("card")?.body(),
            "Old 😀 text"
        );
        state.save_now(&database)?;
        let deadline = Instant::now() + Duration::from_secs(5);
        while state.pending.is_some() && Instant::now() < deadline {
            state.poll(&ctx)?;
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(state.pending.is_none());
        assert!(state.text_editor.is_some());
        assert!(!state.blocks_navigation());
        show_card_frame(
            &mut state,
            &database,
            &ctx,
            vec![key(egui::Key::Z, egui::Modifiers::COMMAND)],
            2.,
        );
        assert_eq!(
            state.text_editor.as_ref().ok_or("direct input")?.text(),
            "Old 😀 text"
        );
        assert!(state.blocks_navigation());
        show_card_frame(
            &mut state,
            &database,
            &ctx,
            vec![key(
                egui::Key::Z,
                egui::Modifiers {
                    shift: true,
                    ..egui::Modifiers::COMMAND
                },
            )],
            3.,
        );
        assert_eq!(
            state.text_editor.as_ref().ok_or("direct input")?.text(),
            "Native in-card 日本語 🪶"
        );
        assert!(!state.blocks_navigation());
        show_card_frame(
            &mut state,
            &database,
            &ctx,
            vec![key(egui::Key::Enter, egui::Modifiers::COMMAND)],
            4.,
        );
        assert!(state.text_editor.is_none());
        assert!(!state.modal_open());
        assert_eq!(
            workspace
                .project_canvas(&project.id, &state.snapshot.canvas.id)?
                .revision
                .content["nodes"][0]["text"],
            "Native in-card 日本語 🪶"
        );
        Ok(())
    }
    #[test]
    fn global_app_save_should_flush_the_open_card_without_saving_an_unrelated_document()
    -> TestResult {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("global-save.sqlite");
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = StudioApp::new(&creation, database.clone(), false, None)?;
        let project = app.workspace.create_project("Synthetic global card save")?;
        let document = app.workspace.create_document(
            &project.id,
            "Unrelated document",
            "Immutable sentinel",
        )?;
        let board = app
            .workspace
            .create_project_canvas(&project.id, "Board", graph())?;
        app.open_project(project.clone())?;
        app.open_canvas(&board.canvas.id)?;
        let state = app.canvas.as_mut().ok_or("canvas")?;
        state.intent(
            &database,
            CanvasIntent::EditText {
                base: state.session.identity(),
                node_id: "card".into(),
            },
        )?;
        *state.text_editor.as_mut().ok_or("editor")?.text_mut() =
            "Global Save routes to this card".into();
        app.save_explicit()?;
        assert!(app.canvas.as_ref().ok_or("canvas")?.pending.is_some());
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.canvas.as_ref().ok_or("canvas")?.pending.is_some() && Instant::now() < deadline {
            app.canvas.as_mut().ok_or("canvas")?.poll(&ctx)?;
            std::thread::sleep(Duration::from_millis(5));
        }
        let state = app.canvas.as_ref().ok_or("canvas")?;
        assert!(state.pending.is_none());
        assert!(state.text_editor.is_some());
        assert_eq!(
            app.workspace
                .project_canvas(&project.id, &board.canvas.id)?
                .revision
                .content["nodes"][0]["text"],
            "Global Save routes to this card"
        );
        assert_eq!(app.workspace.document(&document.id)?, document);
        Ok(())
    }
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

/// Navigation refusal is distinct from a malformed input or canonical save failure.
#[derive(Debug)]
pub(super) struct PendingCanvas;
impl std::fmt::Display for PendingCanvas {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Finish or preserve the pending board operation before leaving.")
    }
}
impl std::error::Error for PendingCanvas {}

impl StudioApp {
    pub(super) fn ensure_canvas_navigation(&self) -> AppResult<()> {
        if self
            .canvas
            .as_ref()
            .is_some_and(CanvasState::blocks_navigation)
        {
            return Err(PendingCanvas.into());
        }
        Ok(())
    }
    pub(super) fn open_canvas(&mut self, id: &str) -> AppResult<()> {
        self.ensure_canvas_navigation()?;
        self.commit_title_edit()?;
        self.cancel_clipboard_work();
        self.save()?;
        let project = self.project.as_ref().ok_or_else(|| {
            studio_core::StudioError::Validation("Choose a project before opening a board".into())
        })?;
        let state = CanvasState::open(&self.workspace, &project.id, id)?;
        self.canvas = Some(state);
        self.document = None;
        self.rich = None;
        self.body.clear();
        self.notes.clear();
        self.daily_controls.document_changed();
        self.inspector_visible = false;
        self.library = false;
        Ok(())
    }
    fn create_board(&mut self) -> AppResult<()> {
        self.ensure_canvas_navigation()?;
        self.commit_title_edit()?;
        self.save()?;
        let project = self.project.as_ref().ok_or_else(|| {
            studio_core::StudioError::Validation("Choose a project before creating a board".into())
        })?;
        let snapshot = self.workspace.create_project_canvas(
            &project.id,
            self.language.text("New board"),
            serde_json::json!({"nodes":[],"edges":[],"x-writers-studio":{"schemaVersion":2}}),
        )?;
        self.open_canvas(&snapshot.canvas.id)
    }
    pub(super) fn canvas_navigation(&mut self, ui: &mut egui::Ui, tokens: Tokens) {
        let Some(project) = self.project.as_ref() else {
            return;
        };
        let boards = match self.workspace.project_canvases(&project.id) {
            Ok(boards) => boards,
            Err(error) => {
                self.record_error(Err(error.into()));
                return;
            }
        };
        let l = self.language;
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            icons::icon(ui, Icon::Board, 20.0, tokens.muted);
            ui.label(RichText::new(l.text("Boards")).size(13.0).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icons::icon_button(ui, Icon::Plus, l.text("Add board")).clicked() {
                    let result = self.create_board();
                    self.record_error(result);
                }
            });
        });
        let mut selected = None;
        egui::ScrollArea::vertical()
            .id_salt("canvas-navigation")
            .max_height(180.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for board in boards {
                    let active = self
                        .canvas
                        .as_ref()
                        .is_some_and(|state| state.snapshot.canvas.id == board.id);
                    if nav_row(ui, Icon::Board, &board.title, active, tokens).clicked() {
                        selected = Some(board.id);
                    }
                }
            });
        if let Some(id) = selected {
            let result = self.open_canvas(&id);
            self.record_error(result);
        }
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);
    }
    pub(super) fn poll_canvas(&mut self, ctx: &egui::Context) {
        let Some(state) = &mut self.canvas else {
            return;
        };
        let result = state.poll(ctx);
        self.status = state.durability_status().into();
        let resource = state.take_resource();
        self.record_error(result);
        if let Some(resource) = resource {
            let result = self.open_canvas_resource(resource, ctx);
            self.record_error(result);
        }
    }
    fn open_canvas_resource(
        &mut self,
        resource: CanvasResource,
        ctx: &egui::Context,
    ) -> AppResult<()> {
        if self.project.as_ref().map(|project| project.id.as_str())
            != Some(resource.project.as_str())
        {
            return Err(CanvasError::StaleBase.into());
        }
        match resource.kind.as_str() {
            "document" => {
                let id=match self.workspace.resolve_project_resource(&resource.project,"node",&resource.logical_id) {
                    Ok(id)=>id,
                    Err(studio_core::StudioError::NotFound{..})=>resource.logical_id.clone(),
                    Err(error)=>return Err(error.into()),
                };
                let document = self.workspace.document(&id)?;
                if document.project_id!=resource.project {return Err(CanvasError::StaleBase.into());}
                self.open_document(document)
            }
            "canvas" => self.open_canvas(&resource.logical_id),
            "url" if resource.logical_id.starts_with("https://") || resource.logical_id.starts_with("http://") => {
                ctx.open_url(egui::OpenUrl::new_tab(resource.logical_id));Ok(())
            }
            _ => Err(CanvasError::Invalid("This project resource preview is not available in the current integration checkpoint; its original bytes remain preserved".into()).into()),
        }
    }
}
