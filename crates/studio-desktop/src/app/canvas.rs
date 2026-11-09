//! Native board projection over one project-scoped durable authority.
use super::native_media::{MediaCache, MediaViewer};
use super::*;
use craft_canvas_egui::{
    CanvasIntent, CanvasView, CanvasViewConfig, CanvasViewHost, CanvasViewLabels, InlineTextOutput,
    NodeBodyResponse, TextCardEditor,
};
use craft_canvas_model::{
    CanvasCommand, CanvasDialect, CanvasDocument, CanvasError, CanvasLimits, CanvasSession,
    CanvasTransaction, ContentIdentity, EdgeRecord, GroupBehavior, IdMapping, NodeKind, NodePolicy,
    NodeRecord, PreparedChange, WorkControl, WorldPoint, WorldRect,
};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::Arc;
use studio_core::{ProjectCanvasRevision, ProjectCanvasSnapshot};

#[cfg(test)]
mod input_tests;

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
/// One exclusive board tool at a time; arming one disarms the other.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum BoardTool {
    /// Select, move and edit cards with the existing gestures.
    #[default]
    Select,
    /// Click empty board space to place a text card.
    Text,
    /// Click a source card and then a different target card to connect them.
    Connect,
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
    checkpoint_version: Option<String>,
    pending: Option<mpsc::Receiver<CommitCompletion>>,
    failed: Option<PreparedChange>,
    failure: Option<String>,
    failed_preserved: bool,
    reload_requested: bool,
    draft_preserved: bool,
    fit_on_open: bool,
    tool: BoardTool,
    connect_source: Option<String>,
    press_origin: Option<egui::Pos2>,
    /// Board tool active when the primary press landed; toolbar clicks in the
    /// same frame must not retro-activate placement for an earlier press.
    press_tool: BoardTool,
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
            checkpoint_version: None,
            pending: None,
            failed: None,
            failure: None,
            failed_preserved: false,
            reload_requested: false,
            draft_preserved: false,
            fit_on_open: true,
            tool: BoardTool::default(),
            connect_source: None,
            press_origin: None,
            press_tool: BoardTool::default(),
            viewport_size: Vec2::new(800.0, 600.0),
            resource: None,
            media: None,
            media_viewer: None,
        })
    }
    /// Bounded camera/selection bookmark, without retaining decoded media or draft buffers.
    pub(super) fn navigation_view(
        &self,
    ) -> AppResult<(craft_canvas_model::Camera, craft_canvas_model::Selection)> {
        if self.blocks_navigation() {
            return Err(PendingCanvas.into());
        }
        Ok((self.view.camera(), self.view.selection().clone()))
    }
    /// Restore view-only state after the host has verified the exact project/board revision.
    pub(super) fn restore_navigation_view(
        &mut self,
        camera: craft_canvas_model::Camera,
        mut selection: craft_canvas_model::Selection,
    ) -> AppResult<()> {
        if self.blocks_navigation() {
            return Err(PendingCanvas.into());
        }
        selection.retain_existing(self.session.document());
        self.view.set_camera(camera);
        self.view.set_selection(selection);
        self.fit_on_open = false;
        Ok(())
    }
    /// Set presentation only for an explicitly armed hosted synthetic capture.
    pub(super) fn set_capture_camera(
        &mut self,
        ctx: &egui::Context,
        center: WorldPoint,
        zoom: f64,
    ) -> AppResult<()> {
        if !super::capture_connected_writing::enabled(ctx) {
            return Err("capture_camera_requires_armed_synthetic_fixture".into());
        }
        self.view
            .set_camera(craft_canvas_model::Camera::new(center, zoom)?);
        self.fit_on_open = false;
        Ok(())
    }
    pub(super) fn modal_open(&self) -> bool {
        self.media_viewer.is_some()
    }
    /// Run before egui focus navigation, with the same modal boundary as the board view.
    pub(super) fn prepare_input(
        &mut self,
        ctx: &egui::Context,
        input: &mut egui::RawInput,
        blocked: bool,
    ) {
        let blocked = blocked || self.media_viewer.is_some() || self.text_cancel;
        if let Some(editor) = &mut self.text_editor {
            editor.prepare_inline_input(ctx, input, blocked);
        }
    }
    /// Keep an active input mountable after a resize without changing its data or zoom.
    fn reveal_inline_input(&mut self) -> AppResult<()> {
        let Some(editor) = self.text_editor.as_ref() else {
            return Ok(());
        };
        if self.viewport_size.x <= 0.0 || self.viewport_size.y <= 0.0 {
            return Ok(());
        }
        let pending = editor.pending_node(&self.session);
        let node = pending
            .as_deref()
            .or_else(|| self.session.document().node(editor.node_id()))
            .ok_or_else(|| CanvasError::NotFound(editor.node_id().into()))?;
        let center = node.bounds().center();
        let camera = self.view.camera();
        let size = WorldPoint {
            x: f64::from(self.viewport_size.x),
            y: f64::from(self.viewport_size.y),
        };
        let minimum = camera.view_to_world(WorldPoint { x: 0.0, y: 0.0 }, size);
        let maximum = camera.view_to_world(size, size);
        let visible_world = WorldRect {
            x: minimum.x,
            y: minimum.y,
            width: maximum.x - minimum.x,
            height: maximum.y - minimum.y,
        };
        // Match CanvasView's culling boundary; partially visible cards keep their camera.
        if !node.bounds().intersects(visible_world) {
            self.view
                .set_camera(craft_canvas_model::Camera::new(center, camera.zoom())?);
        }
        Ok(())
    }
    /// Dirty or unacknowledged native input cannot silently disappear through navigation.
    pub(super) fn blocks_navigation(&self) -> bool {
        self.pending.is_some()
            || self.failed.is_some()
            || self.text_save_error
            || self.text_composing
            || self.text_editor.as_ref().is_some_and(|editor| {
                editor.dirty() || editor.has_submission() || editor.awaits_insertion()
            })
    }
    /// One acknowledgement-aware status shared by all visible save indicators.
    pub(super) fn durability_status(&self) -> &'static str {
        if self.failed.is_some() || self.failure.is_some() || self.text_save_error {
            "Attention needed"
        } else if self.text_cancel && self.pending.is_some() {
            "Finishing cancellation…"
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
        if self.failed.is_none()
            && self
                .text_editor
                .as_ref()
                .is_some_and(TextCardEditor::awaits_insertion)
        {
            // A preparation failure has no candidate. Recreate only the exact original
            // insertion at its unchanged base, never an EditText against an absent node.
            let transaction = self.insertion_transaction()?;
            return self.begin(database, PendingOperation::Transaction(transaction));
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
                if let Some(editor) = &mut self.text_editor {
                    if editor.awaits_insertion() {
                        editor.acknowledge_insertion(session)?;
                    } else if editor.has_submission() {
                        editor.acknowledge(session)?;
                    }
                }
                // Cancel already froze input. Done remains live until this frame's
                // native input has been processed by drive_inline; polling happens first.
                if self.text_cancel {
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
                    .is_some_and(|editor| editor.has_submission() || editor.awaits_insertion());
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
    /// Arm exactly one board tool; switching tools cancels a pending connection source.
    fn set_tool(&mut self, tool: BoardTool) {
        self.tool = tool;
        self.press_origin = None;
        self.view.cancel_transient();
        self.cancel_connect_selection();
    }
    /// Cancel a pending connection source selection without any mutation.
    fn cancel_connect_selection(&mut self) {
        self.connect_source = None;
    }
    /// Board-click handler for a completed card press: select the source, then connect.
    fn board_card_click(&mut self, database: &Path, ctx: &egui::Context, node_id: &str) {
        if self.tool != BoardTool::Connect {
            return;
        }
        let source = self.connect_source.clone();
        match source.as_deref() {
            // A second press on the source card is not a self-connection.
            Some(source) if source == node_id => {}
            Some(source) => {
                self.cancel_connect_selection();
                let result = self.connect_nodes(database, ctx, source, node_id);
                self.set_failure(result);
            }
            None => self.connect_source = Some(node_id.to_owned()),
        }
    }
    /// Build and durably insert one connector between two existing cards through the
    /// same prepare → worker draft → canonical save → acknowledgement flow as text edits.
    fn connect_nodes(
        &mut self,
        database: &Path,
        ctx: &egui::Context,
        from: &str,
        to: &str,
    ) -> AppResult<()> {
        if from == to {
            return Err(CanvasError::Invalid("A card cannot be connected to itself".into()).into());
        }
        let document = self.session.document();
        let from_node = document
            .node(from)
            .ok_or_else(|| CanvasError::NotFound(from.into()))?;
        let to_node = document
            .node(to)
            .ok_or_else(|| CanvasError::NotFound(to.into()))?;
        if !self.dialect.can_connect(from_node) {
            return Err(CanvasError::NotEditable(from.into()).into());
        }
        if !self.dialect.can_connect(to_node) {
            return Err(CanvasError::NotEditable(to.into()).into());
        }
        let edge = EdgeRecord::parse(
            &serde_json::to_string(&serde_json::json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "fromNode": from,
                "toNode": to,
            }))?,
            self.session.limits(),
        )?;
        let index = self.session.document().edges().len();
        self.command(database, vec![CanvasCommand::InsertEdge { edge, index }])?;
        ctx.request_repaint();
        Ok(())
    }
    /// Place a new text card at the clicked empty board position.
    fn place_text_at(
        &mut self,
        database: &Path,
        ctx: &egui::Context,
        screen: egui::Pos2,
        viewport: egui::Rect,
    ) -> AppResult<()> {
        let size = WorldPoint {
            x: f64::from(viewport.width()),
            y: f64::from(viewport.height()),
        };
        let world = self.view.camera().view_to_world(
            WorldPoint {
                x: f64::from(screen.x - viewport.left()),
                y: f64::from(screen.y - viewport.top()),
            },
            size,
        );
        self.add_text(database, ctx, world)
    }
    fn add_text(&mut self, database: &Path, ctx: &egui::Context, at: WorldPoint) -> AppResult<()> {
        if !self.editable() || self.text_editor.is_some() {
            return Err(PendingCanvas.into());
        }
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
        let mut editor = TextCardEditor::pending_insertion(&self.session, node)?;
        editor.begin_inline(ctx)?;
        self.text_editor = Some(editor);
        self.text_changed_at = Instant::now();
        self.text_done = false;
        self.text_cancel = false;
        self.text_composing = false;
        self.text_save_error = false;
        let result = self.insertion_transaction().and_then(|transaction| {
            self.begin(database, PendingOperation::Transaction(transaction))
        });
        if result.is_err() {
            self.text_save_error = true;
        }
        result
    }
    /// Retry the original insertion without changing its raw fields, index or base.
    fn insertion_transaction(&self) -> AppResult<CanvasTransaction> {
        let editor = self.text_editor.as_ref().ok_or(CanvasError::NoHistory)?;
        let node = editor
            .pending_node(&self.session)
            .ok_or(CanvasError::StaleBase)?;
        Ok(CanvasTransaction::new(
            self.session.identity(),
            vec![CanvasCommand::InsertNode {
                node: NodeRecord::parse(node.raw_json(), self.session.limits())?,
                index: self.session.document().nodes().len(),
            }],
        ))
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
    fn intent(
        &mut self,
        database: &Path,
        ctx: &egui::Context,
        intent: CanvasIntent,
    ) -> AppResult<()> {
        match intent {
            CanvasIntent::Transaction(transaction) => self.begin(database, PendingOperation::Transaction(transaction)),
            CanvasIntent::Undo(base) => self.begin(database, PendingOperation::Undo(base)),
            CanvasIntent::Redo(base) => self.begin(database, PendingOperation::Redo(base)),
            CanvasIntent::CreateText { base, position } => {
                if base != self.session.identity() { return Err(CanvasError::StaleBase.into()); }
                self.add_text(database, ctx, position)
            }
            CanvasIntent::EditText { base, node_id } => {
                if base != self.session.identity() { return Err(CanvasError::StaleBase.into()); }
                let node=self.session.document().node(&node_id).ok_or_else(||CanvasError::NotFound(node_id.clone()))?;
                if !self.dialect.node_policy(node).movable { return Err(CanvasError::NotEditable(node_id).into()); }
                if !self.editable() || self.text_editor.is_some() { return Err(PendingCanvas.into()); }
                let mut editor = TextCardEditor::new(&self.session, &node_id)?;
                editor.begin_inline(ctx)?;
                self.view.cancel_transient();
                self.text_editor = Some(editor);
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
        let tool_at_frame_start = self.tool;
        // An export permission covers the previous input pass only. Even if clicking
        // Reload moves focus first, same-batch native text must not disappear unnoticed.
        let text_input_arrived = self.text_editor.is_some()
            && ui.input(|input| {
                input.events.iter().any(|event| {
                    matches!(
                        event,
                        egui::Event::Text(_) | egui::Event::Paste(_) | egui::Event::Ime(_)
                    )
                })
            });
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
                            let text = icons::labeled_button(
                                ui,
                                Icon::Plus,
                                l.text("Text card"),
                                self.tool == BoardTool::Text,
                            )
                            .on_hover_text(l.text("Click the board to place a text card"));
                            super::capture_connected_writing::control(
                                ui.ctx(),
                                "canvas.add_text",
                                text.rect,
                            );
                            if text.clicked() {
                                let tool = if self.tool == BoardTool::Text {
                                    BoardTool::Select
                                } else {
                                    BoardTool::Text
                                };
                                self.set_tool(tool);
                            }
                            let connection = icons::labeled_button(
                                ui,
                                Icon::ArrowRight,
                                l.text("Connection"),
                                self.tool == BoardTool::Connect,
                            )
                            .on_hover_text(
                                l.text("Click one card, then another, to create a connection"),
                            );
                            super::capture_connected_writing::control(
                                ui.ctx(),
                                "canvas.connect",
                                connection.rect,
                            );
                            if connection.clicked() {
                                let tool = if self.tool == BoardTool::Connect {
                                    BoardTool::Select
                                } else {
                                    BoardTool::Connect
                                };
                                self.set_tool(tool);
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
                        ui.label(l.text(if self.text_cancel {
                            "Editing paused"
                        } else {
                            "Editing text card"
                        }));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let done = ui.add_enabled(
                                !self.text_composing && !self.text_cancel,
                                egui::Button::new(l.text("Done")),
                            );
                            super::capture_connected_writing::control(
                                ui.ctx(),
                                "canvas.done",
                                done.rect,
                            );
                            if done.clicked() {
                                self.text_done = true;
                            }
                            let cancel = ui
                                .add_enabled(
                                    !self.text_composing && !self.text_cancel,
                                    egui::Button::new(l.text("Cancel")),
                                )
                                .on_hover_text(l.text("Already saved changes are kept."));
                            super::capture_connected_writing::control(
                                ui.ctx(),
                                "canvas.cancel",
                                cancel.rect,
                            );
                            if cancel.clicked() {
                                self.text_cancel = true;
                            }
                        });
                    } else {
                        match self.tool {
                            BoardTool::Connect => {
                                ui.separator();
                                ui.label(l.text(if self.connect_source.is_some() {
                                    "Select target card"
                                } else {
                                    "Select source card"
                                }));
                            }
                            BoardTool::Text => {
                                ui.separator();
                                ui.label(l.text("Click the board to place a text card"));
                            }
                            BoardTool::Select => {}
                        }
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
                            if ui
                                .add_enabled(
                                    !self.text_cancel
                                        || self.failed.is_some()
                                        || self
                                            .text_editor
                                            .as_ref()
                                            .is_some_and(TextCardEditor::awaits_insertion),
                                    egui::Button::new(l.text("Retry save")),
                                )
                                .clicked()
                            {
                                let result = if self.failed.is_some()
                                    || self
                                        .text_editor
                                        .as_ref()
                                        .is_some_and(TextCardEditor::awaits_insertion)
                                {
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
                            if self.text_editor.is_some()
                                && ui.button(l.text("Preserve full card draft…"))
                                    .on_hover_text(l.text("Recovery JSON preserves the board and exact draft. Manual recovery is required; it is not an importable project."))
                                    .clicked()
                            {
                                let result = self.preserve_full_card_draft(l);
                                self.set_failure(result);
                            }
                            if ui
                                .add_enabled(
                                    self.failed_preserved,
                                    egui::Button::new(l.text("Reload saved board")),
                                )
                                .clicked()
                            {
                                self.reload_requested = true;
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
        // The pre-input hook can grant focus before this pass's viewport is known.
        // Reveal an offscreen active card now so resizing cannot omit its real widget.
        let reveal = self.reveal_inline_input();
        self.set_failure(reveal);
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
            blocked: blocked || self.media_viewer.is_some() || self.text_cancel,
        };
        let output = self
            .view
            .show(ui, &self.session, &self.dialect, &mut host, &config);
        for error in output.errors {
            self.failure = Some(error.to_string());
        }
        let board_rect = output.viewport.intersect(ui.clip_rect());
        let board_owns_pointer = ui.rect_contains_pointer(board_rect);
        let pan_input = ui.input(|input| {
            input.key_down(egui::Key::Space)
                || input.key_pressed(egui::Key::Space)
                || input.key_released(egui::Key::Space)
                || input.pointer.button_down(egui::PointerButton::Middle)
                || input.pointer.button_released(egui::PointerButton::Middle)
        });
        // Tool clicks are scoped to the board. Preserve the Select tool's native
        // double-click creation without reinterpreting toolbar clicks as placement.
        let drag_occurred = output
            .intents
            .iter()
            .any(|intent| matches!(intent, CanvasIntent::Transaction(_)));
        for intent in output.intents {
            let filtered = match &intent {
                CanvasIntent::CreateText { .. } => {
                    self.tool != BoardTool::Select
                        || self.tool != tool_at_frame_start
                        || !board_owns_pointer
                        || pan_input
                        || !output
                            .response
                            .double_clicked_by(egui::PointerButton::Primary)
                }
                CanvasIntent::EditText { .. } | CanvasIntent::OpenNode { .. } => {
                    self.tool == BoardTool::Connect
                }
                _ => false,
            };
            if filtered {
                continue;
            }
            // The first completed mutation freezes this session until its durable acknowledgement.
            let result = self.intent(database, ui.ctx(), intent);
            self.set_failure(result);
        }
        if config.interactive
            && self.editable()
            && self.text_editor.is_none()
            && ui.input(|input| input.focused)
            && self.tool == tool_at_frame_start
            && !pan_input
        {
            // A pending connection source must never dangle after its card disappeared.
            if self
                .connect_source
                .as_ref()
                .is_some_and(|id| self.session.document().node(id).is_none())
            {
                self.cancel_connect_selection();
            }
            if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                self.press_origin = None;
                if self.connect_source.is_some() {
                    self.cancel_connect_selection();
                } else {
                    self.set_tool(BoardTool::Select);
                }
            }
            let mut release_at = None;
            ui.input(|input| {
                for event in &input.events {
                    if matches!(event, egui::Event::PointerGone) {
                        self.press_origin = None;
                    }
                    if let egui::Event::PointerButton {
                        button: egui::PointerButton::Primary,
                        pressed,
                        pos,
                        ..
                    } = event
                    {
                        if *pressed {
                            self.press_origin =
                                (board_owns_pointer && board_rect.contains(*pos)).then_some(*pos);
                            self.press_tool = tool_at_frame_start;
                        } else {
                            release_at = Some(*pos);
                        }
                    }
                }
            });
            // Always consume the press on release, including a drag or an outside
            // release. egui's click classification also rejects out-and-back drags.
            let origin = release_at.and_then(|_| self.press_origin.take());
            if !drag_occurred
                && board_owns_pointer
                && self.press_tool == self.tool
                && ui.input(|input| input.pointer.primary_clicked())
                && let (Some(origin), Some(pos)) = (origin, release_at)
                && board_rect.contains(origin)
                && board_rect.contains(pos)
            {
                let node_at = |point: egui::Pos2| {
                    output
                        .nodes
                        .iter()
                        .rev()
                        .find(|node| node.reserved.contains(point))
                        .map(|node| node.node_id.as_str())
                };
                match (node_at(origin), node_at(pos)) {
                    (Some(pressed), Some(released))
                        if pressed == released && self.press_tool == BoardTool::Connect =>
                    {
                        self.board_card_click(database, ui.ctx(), released);
                    }
                    (None, None) if self.press_tool == BoardTool::Text => {
                        let result = self.place_text_at(database, ui.ctx(), pos, output.viewport);
                        self.set_failure(result);
                    }
                    _ => {}
                }
            }
        } else {
            self.press_origin = None;
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
        if std::mem::take(&mut self.reload_requested) {
            let result = if text_input_arrived || self.text_composing {
                self.failed_preserved = false;
                Err(PendingCanvas.into())
            } else {
                self.reload_saved(database)
            };
            self.set_failure(result);
            ui.ctx().request_repaint();
        }
        if super::capture_connected_writing::enabled(ui.ctx()) {
            let rect = |rect: egui::Rect| [rect.min.x, rect.min.y, rect.max.x, rect.max.y];
            let camera = self.view.camera();
            super::capture_connected_writing::observe(
                ui.ctx(),
                "canvas",
                serde_json::json!({
                    "project_id":self.project_id, "board_id":self.snapshot.canvas.id,
                    "revision":self.snapshot.revision.info.revision, "canonical":self.snapshot.revision.content,
                    "pending":self.pending.is_some(), "failed":self.failed.is_some() || self.failure.is_some() || self.text_save_error,
                    "recovery":self.snapshot.draft.is_some(), "navigation_blocked":self.blocks_navigation(), "status":self.durability_status(),
                    "editor":self.text_editor.as_ref().map(|editor| serde_json::json!({"node_id":editor.node_id(), "text":editor.text(), "dirty":editor.dirty(), "submitted":editor.has_submission(), "awaiting_insertion":editor.awaits_insertion(), "composing":self.text_composing})),
                    "selected_nodes":self.view.selection().nodes,
                    "tool":match self.tool { BoardTool::Select => "select", BoardTool::Text => "text", BoardTool::Connect => "connect" },
                    "connect_source":self.connect_source,
                    "camera":{"center":[camera.center().x,camera.center().y],"zoom":camera.zoom()},
                    "viewport":rect(output.viewport),
                    "visible_nodes":output.nodes.iter().map(|node|serde_json::json!({"node_id":node.node_id,"reserved":rect(node.reserved),"body_clip":rect(node.body_clip)})).collect::<Vec<_>>()
                }),
            );
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
        if self.text_cancel {
            return Err(PendingCanvas.into());
        }
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
        if self.text_cancel {
            // Cancellation must not turn global Save/backup into a new write of discarded text.
            return Err(PendingCanvas.into());
        }
        if self.failed.is_some()
            || self
                .text_editor
                .as_ref()
                .is_some_and(TextCardEditor::awaits_insertion)
        {
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
        if editor.awaits_insertion() {
            // No autosave or focus handoff may certify an unacknowledged empty card.
            return Ok(());
        }
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
    /// A lossless diagnostic fallback even when a candidate exceeds Canvas limits.
    /// Raw JSON stays a string; this record grants no replay authority or import guarantee.
    fn card_recovery_record(&self) -> AppResult<String> {
        let editor = self.text_editor.as_ref().ok_or(CanvasError::NoHistory)?;
        let pending_node = editor.pending_node(&self.session);
        let node = pending_node
            .as_deref()
            .or_else(|| self.session.document().node(editor.node_id()))
            .ok_or_else(|| CanvasError::NotFound(editor.node_id().into()))?;
        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "format": "writercraft-card-recovery", "version": 1,
            "project_id": self.project_id, "board_id": self.snapshot.canvas.id,
            "acknowledged_board_revision": self.snapshot.revision.info.revision,
            "commit_outcome": if self.pending.is_some() || self.failure.is_some() || self.text_save_error {
                "unacknowledged"
            } else { "local-draft" },
            "awaiting_insertion": editor.awaits_insertion(),
            "base_board_json": self.session.document().raw_json(),
            "original_card_json": node.raw_json(),
            "latest_text": editor.text(),
            "submitted_candidate_json": self.failed.as_ref().map(|candidate| candidate.document().raw_json()),
            "recovery": "Manual recovery only. This is not an importable project or an acknowledged save. Separate project media is not bundled."
        }))?)
    }
    fn write_full_card_draft(&mut self, path: &Path) -> AppResult<()> {
        let record = self.card_recovery_record()?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        file.write_all(record.as_bytes())?;
        file.flush()?;
        file.sync_all()?;
        // The next native draft change invalidates this permission in show().
        self.failed_preserved = true;
        Ok(())
    }
    fn preserve_full_card_draft(&mut self, language: Language) -> AppResult<()> {
        let Some(path) = rfd::FileDialog::new()
            .set_title(language.text("Preserve full card draft…"))
            .set_file_name("card-recovery.json")
            .add_filter("JSON", &["json"])
            .save_file()
        else {
            return Ok(());
        };
        self.write_full_card_draft(&path)
    }
    /// Recover an absent card and its latest text atomically while preserving raw metadata.
    /// This is an export candidate, never acknowledgement of the original insertion.
    fn latest_text_candidate(&self) -> AppResult<Option<PreparedChange>> {
        let Some(editor) = &self.text_editor else {
            return Ok(None);
        };
        let transaction = if editor.awaits_insertion() {
            let node = editor
                .pending_node(&self.session)
                .ok_or(CanvasError::StaleBase)?;
            CanvasTransaction::new(
                self.session.identity(),
                vec![
                    CanvasCommand::InsertNode {
                        node: NodeRecord::parse(node.raw_json(), self.session.limits())?,
                        index: self.session.document().nodes().len(),
                    },
                    CanvasCommand::EditText {
                        node_id: node.id().into(),
                        expected: node.body().into(),
                        text: editor.text().into(),
                    },
                ],
            )
        } else {
            editor.transaction()
        };
        Ok(Some(self.session.prepare(transaction, &self.dialect)?))
    }
    fn preserve_pending(&mut self, language: Language) -> AppResult<()> {
        let latest = self.latest_text_candidate()?;
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
        if !self.failed_preserved || self.pending.is_some() || self.text_composing {
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
        edit_text_hint: language.text("Double-click or press Enter to edit").into(),
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
    fn pending_text_editor(&self) -> Option<&TextCardEditor> {
        self.editor.as_deref()
    }
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
        assert_eq!(state.durability_status(), "Unsaved board changes");
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
    fn externally_advanced_durable_head_should_refuse_live_inline_save_and_retry_without_losing_either_version()
    -> TestResult {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("inline-stale-head.sqlite");
        let workspace = Workspace::open(&database)?;
        let project = workspace.create_project("Synthetic live inline revision conflict")?;
        let mut original = graph();
        original["future"] =
            json!({"exact":9007199254740993123u64,"opaque":[null,true,{"keep":"board"}]});
        original["nodes"][0]["future"] =
            json!({"exact":9007199254740993u64,"opaque":[{"keep":"card"},null]});
        let board = workspace.create_project_canvas(&project.id, "Board", original.clone())?;
        let mut state = CanvasState::from_snapshot(&project.id, board)?;
        let base = state.session.identity();
        let base_raw = state.session.document().raw_json().to_owned();
        let ctx = egui::Context::default();
        activate_card(&mut state, &database, &ctx)?;
        let submitted = "My local 日本語 🪶";
        show_card_frame(
            &mut state,
            &database,
            &ctx,
            vec![
                egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                },
                egui::Event::Paste(submitted.into()),
            ],
            0.1,
        );
        assert_eq!(
            state.text_editor.as_ref().ok_or("inline editor")?.text(),
            submitted
        );
        assert!(state.pending.is_none());

        // Advance the saved graph through an independent workspace, not a foreign draft token.
        let other_workspace = Workspace::open(&database)?;
        let mut external = original.clone();
        external["nodes"][0]["text"] = json!("Externally saved text");
        external["future"]["external"] =
            json!({"exact":9007199254740995u64,"keep":["external",null]});
        external["nodes"][0]["future"]["external"] = json!({"keep":true});
        let advanced = other_workspace.save_project_canvas(
            &project.id,
            &state.snapshot.canvas.id,
            1,
            external.clone(),
        )?;
        assert_eq!(advanced.info.revision, 2);
        let mut submitted_graph = original.clone();
        submitted_graph["nodes"][0]["text"] = json!(submitted);
        let latest = "My local 日本語 🪶; typed during stale save e\u{301}";

        for retry in [false, true] {
            if retry {
                state.retry(&database)?;
            } else {
                state.save_now(&database)?;
                // Completion has not been polled: the same live native editor still accepts input.
                show_card_frame(
                    &mut state,
                    &database,
                    &ctx,
                    vec![egui::Event::Text(
                        "; typed during stale save e\u{301}".into(),
                    )],
                    0.2,
                );
            }
            assert!(state.pending.is_some());
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut refusal = None;
            while state.pending.is_some() && Instant::now() < deadline {
                if let Err(error) = state.poll(&ctx) {
                    refusal = Some(error);
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            let refusal = refusal.ok_or("stale inline save did not report a conflict")?;
            assert!(matches!(
                refusal.downcast_ref::<studio_core::StudioError>(),
                Some(studio_core::StudioError::RevisionConflict {
                    expected: 1,
                    actual: 2
                })
            ));
            assert!(state.pending.is_none());
            assert!(state.blocks_navigation() && state.text_save_error);
            assert_eq!(state.durability_status(), "Attention needed");
            assert_eq!(state.session.identity(), base);
            assert_eq!(state.session.document().raw_json(), base_raw);
            assert_eq!(state.snapshot.revision.info.revision, 1);
            assert_eq!(state.snapshot.revision.content, original);
            let editor = state.text_editor.as_ref().ok_or("retained inline editor")?;
            assert_eq!(editor.text(), latest);
            assert!(editor.dirty() && editor.has_submission());
            let failed = state
                .failed
                .as_ref()
                .ok_or("retained submitted candidate")?;
            assert_eq!(failed.base(), base);
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(failed.document().raw_json())?,
                submitted_graph,
            );
            let reopened = Workspace::open(&database)?
                .project_canvas(&project.id, &state.snapshot.canvas.id)?;
            assert_eq!(reopened.revision.info.revision, 2);
            assert_eq!(reopened.revision.content, external);
            assert!(reopened.draft.is_none());
            assert_eq!(
                workspace
                    .project_canvas_history(&project.id, &state.snapshot.canvas.id, None, 10)?
                    .len(),
                2,
            );
        }
        let preservation = state
            .latest_text_candidate()?
            .ok_or("latest draft preservation")?;
        let mut latest_graph = original;
        latest_graph["nodes"][0]["text"] = json!(latest);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(preservation.document().raw_json())?,
            latest_graph,
        );
        Ok(())
    }

    #[test]
    fn new_card_should_accept_native_input_before_durable_creation_and_keep_it_after_acknowledgement()
    -> TestResult {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("new-card.sqlite");
        let workspace = Workspace::open(&database)?;
        let project = workspace.create_project("Synthetic new card")?;
        let board = workspace.create_project_canvas(&project.id, "Board", graph())?;
        let mut state = CanvasState::from_snapshot(&project.id, board)?;
        let center = WorldPoint {
            x: 4_096.0,
            y: -8_192.0,
        };
        state
            .view
            .set_camera(craft_canvas_model::Camera::new(center, 1.5)?);
        state.fit_on_open = false;
        let ctx = egui::Context::default();
        let mut created = Ok(());
        // The real activation pass has not mounted a TextEdit yet. The next pass
        // carries the author's input; no blank editor frame may preinstall its filter.
        let mut activation_output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let center = state.view.camera().center();
            created = state.add_text(
                &database,
                ui.ctx(),
                WorldPoint {
                    x: center.x - 150.0,
                    y: center.y - 90.0,
                },
            );
        });
        activation_output.textures_delta.clear();
        activation_output.platform_output.commands.clear();
        created?;
        assert!(state.pending.is_some());
        assert!(
            state
                .text_editor
                .as_ref()
                .ok_or("editor")?
                .awaits_insertion()
        );
        assert!(
            state.blocks_navigation(),
            "even an empty absent card is unacknowledged"
        );
        assert_eq!(state.session.document().nodes().len(), 1);
        show_card_frame(
            &mut state,
            &database,
            &ctx,
            vec![egui::Event::Text("First input 日本語 🪶".into())],
            0.1,
        );
        assert_eq!(
            state.text_editor.as_ref().ok_or("editor")?.text(),
            "First input 日本語 🪶"
        );
        show_card_frame(
            &mut state,
            &database,
            &ctx,
            vec![egui::Event::Paste(" — still saving".into())],
            0.2,
        );
        let latest = "First input 日本語 🪶 — still saving";
        assert_eq!(state.text_editor.as_ref().ok_or("editor")?.text(), latest);
        assert_eq!(state.session.document().nodes().len(), 1);
        let deadline = Instant::now() + Duration::from_secs(5);
        while state.pending.is_some() && Instant::now() < deadline {
            state.poll(&ctx)?;
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(state.pending.is_none());
        let editor = state.text_editor.as_ref().ok_or("direct editor")?;
        assert!(!editor.awaits_insertion());
        assert_eq!(editor.text(), latest);
        assert_eq!(
            state
                .session
                .document()
                .node(editor.node_id())
                .ok_or("card")?
                .bounds()
                .center(),
            center
        );
        assert!(state.view.selection().nodes.contains(editor.node_id()));
        assert!(
            state.blocks_navigation(),
            "latest input has not been saved by the insertion ack"
        );
        assert_eq!(
            workspace
                .project_canvas(&project.id, &state.snapshot.canvas.id)?
                .revision
                .info
                .revision,
            2
        );
        state.save_now(&database)?;
        let deadline = Instant::now() + Duration::from_secs(5);
        while state.pending.is_some() && Instant::now() < deadline {
            state.poll(&ctx)?;
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(state.pending.is_none());
        assert!(!state.blocks_navigation());
        let reopened = CanvasState::open(&workspace, &project.id, &state.snapshot.canvas.id)?;
        assert_eq!(reopened.session.document().nodes()[1].body(), latest);
        assert_eq!(reopened.snapshot.revision.info.revision, 3);
        Ok(())
    }
    #[test]
    fn resized_board_should_mount_armed_input_and_preserve_zoom_and_first_text() -> TestResult {
        for provisional in [false, true] {
            let directory = tempfile::tempdir()?;
            let database = directory.path().join("resized-input.sqlite");
            let workspace = Workspace::open(&database)?;
            let project = workspace.create_project("Synthetic resized input")?;
            let mut original = graph();
            original["nodes"][0]["x"] = json!(900);
            let board = workspace.create_project_canvas(&project.id, "Board", original.clone())?;
            let mut state = CanvasState::from_snapshot(&project.id, board)?;
            state.view.set_camera(craft_canvas_model::Camera::new(
                WorldPoint { x: 0.0, y: 0.0 },
                1.5,
            )?);
            state.fit_on_open = false;
            // This card was visible on the wide board, but the next native pass is narrow.
            state.viewport_size = Vec2::new(4000.0, 1000.0);
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            let mut activation = Ok(());
            let mut full = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.label("Board already painted");
                activation = (|| -> AppResult<()> {
                    if provisional {
                        // Exercise the host's pending projection before durable submission.
                        let mut raw = original["nodes"][0].clone();
                        raw["id"] = json!("provisional");
                        let node = NodeRecord::parse(&raw.to_string(), state.session.limits())?;
                        let mut editor = TextCardEditor::pending_insertion(&state.session, node)?;
                        editor.begin_inline(ui.ctx())?;
                        state.text_editor = Some(editor);
                        Ok(())
                    } else {
                        state.intent(
                            &database,
                            ui.ctx(),
                            CanvasIntent::EditText {
                                base: state.session.identity(),
                                node_id: "card".into(),
                            },
                        )
                    }
                })();
            });
            full.textures_delta.clear();
            full.platform_output.commands.clear();
            activation?;
            let before = state.session.document().raw_json().to_owned();
            let full = show_card_frame(
                &mut state,
                &database,
                &ctx,
                vec![egui::Event::Text(" — retained first input".into())],
                1.0,
            );
            assert!(
                state
                    .text_editor
                    .as_ref()
                    .ok_or("editor")?
                    .text()
                    .contains("retained first input")
            );
            assert_eq!(state.view.camera().zoom(), 1.5);
            assert_eq!(
                state.view.camera().center(),
                state
                    .session
                    .document()
                    .node("card")
                    .ok_or("card")?
                    .bounds()
                    .center()
            );
            assert_eq!(state.session.document().raw_json(), before);
            assert_eq!(
                workspace
                    .project_canvas(&project.id, &state.snapshot.canvas.id)?
                    .revision
                    .content,
                original
            );
            let update = full
                .platform_output
                .accesskit_update
                .as_ref()
                .ok_or("native accessibility update")?;
            assert!(update.nodes.iter().any(|(id, node)| *id == update.focus
                && node.role() == egui::accesskit::Role::MultilineTextInput));
        }
        Ok(())
    }

    #[test]
    fn partially_clipped_active_card_should_keep_its_camera_position() -> TestResult {
        let directory = tempfile::tempdir()?;
        let workspace = Workspace::open(directory.path().join("clipped-input.sqlite"))?;
        let project = workspace.create_project("Synthetic clipped input")?;
        let board = workspace.create_project_canvas(&project.id, "Board", graph())?;
        let mut state = CanvasState::from_snapshot(&project.id, board)?;
        state.text_editor = Some(TextCardEditor::new(&state.session, "card")?);
        state.viewport_size = Vec2::new(736.0, 500.0);
        for center_x in [40.0, -50.0] {
            let camera = craft_canvas_model::Camera::new(
                WorldPoint {
                    x: center_x,
                    y: 90.0,
                },
                2.0,
            )?;
            state.view.set_camera(camera);
            state.reveal_inline_input()?;
            assert_eq!(state.view.camera(), camera);
        }
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
    ) -> egui::FullOutput {
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(1000., 680.),
            )),
            events,
            time: Some(time),
            focused: true,
            ..Default::default()
        };
        state.prepare_input(ctx, &mut input, false);
        paint_card_input(state, database, ctx, input)
    }
    fn paint_card_input(
        state: &mut CanvasState,
        database: &Path,
        ctx: &egui::Context,
        input: egui::RawInput,
    ) -> egui::FullOutput {
        let tokens = theme::resolve(&ThemeSettings::default(), false);
        let mut output = ctx.run_ui(input, |root| {
            egui::CentralPanel::default().show(root, |ui| {
                state.show(ui, database, Language::English, tokens, false);
            });
        });
        output.textures_delta.clear();
        output.platform_output.commands.clear();
        output
    }
    #[test]
    fn failed_absent_card_should_preserve_latest_text_and_unknown_fields_then_retry_exact_insertion()
    -> TestResult {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("absent-card.sqlite");
        let workspace = Workspace::open(&database)?;
        let project = workspace.create_project("Synthetic absent card recovery")?;
        let board = workspace.create_project_canvas(&project.id, "Board", graph())?;
        let mut state = CanvasState::from_snapshot(&project.id, board)?;
        let raw = r#"{"id":"new-card","type":"text","x":0,"y":0,"width":300,"height":180,"text":"","future":{"number":9007199254740993123,"opaque":[true,null]}}"#;
        let node = NodeRecord::parse(raw, state.session.limits())?;
        state.text_editor = Some(TextCardEditor::pending_insertion(&state.session, node)?);
        let original = state.insertion_transaction()?;
        let original_prepared = state.session.prepare(original, &state.dialect)?;
        // Fail before producing a durable candidate; even an empty draft still blocks navigation.
        assert!(
            deliver(
                &mut state,
                CommitCompletion {
                    prepared: None,
                    result: Err("synthetic preparation failure".into()),
                    checkpoint_version: None,
                }
            )
            .is_err()
        );
        assert!(state.failed.is_none());
        assert!(state.blocks_navigation());
        *state.text_editor.as_mut().ok_or("editor")?.text_mut() = "Latest draft 日本語 🪶".into();
        let candidate = state.latest_text_candidate()?.ok_or("recovery candidate")?;
        let restored =
            CanvasDocument::parse(candidate.document().raw_json(), state.session.limits())?;
        let restored_node = restored.node("new-card").ok_or("restored card")?;
        assert_eq!(restored_node.body(), "Latest draft 日本語 🪶");
        assert_eq!(
            restored_node
                .field("future")
                .ok_or("unknown extension")?
                .get(),
            r#"{"number":9007199254740993123,"opaque":[true,null]}"#
        );
        assert_eq!(
            state.session.document().nodes().len(),
            1,
            "recovery is not canonical acknowledgement"
        );
        assert!(
            state
                .text_editor
                .as_ref()
                .ok_or("editor")?
                .awaits_insertion()
        );
        state.retry(&database)?;
        let ctx = egui::Context::default();
        let deadline = Instant::now() + Duration::from_secs(5);
        while state.pending.is_some() && Instant::now() < deadline {
            state.poll(&ctx)?;
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(state.pending.is_none());
        assert_eq!(
            state.session.document().raw_json(),
            original_prepared.document().raw_json()
        );
        assert_eq!(
            state
                .session
                .document()
                .node("new-card")
                .ok_or("saved card")?
                .body(),
            ""
        );
        assert_eq!(
            state.text_editor.as_ref().ok_or("editor")?.text(),
            "Latest draft 日本語 🪶"
        );
        assert!(
            state.blocks_navigation(),
            "only the original insertion was retried"
        );
        Ok(())
    }
    #[test]
    fn capacity_refused_card_should_export_lossless_recovery_before_reloading_saved_board()
    -> TestResult {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("capacity.sqlite");
        let workspace = Workspace::open(&database)?;
        let project = workspace.create_project("Synthetic capacity recovery")?;
        let board = workspace.create_project_canvas(&project.id, "Board", graph())?;
        let mut state = CanvasState::from_snapshot(&project.id, board)?;
        let limits = CanvasLimits {
            max_nodes: 1,
            ..CanvasLimits::default()
        };
        state.session = Arc::new(CanvasSession::new(
            CanvasDocument::parse(state.session.document().raw_json(), &limits)?,
            limits,
        )?);
        let raw = r#"{"id":"over-capacity","type":"text","x":0,"y":0,"width":300,"height":180,"text":"","unknown":{"value":9007199254740993123}}"#;
        state.text_editor = Some(TextCardEditor::pending_insertion(
            &state.session,
            NodeRecord::parse(raw, state.session.limits())?,
        )?);
        *state.text_editor.as_mut().ok_or("editor")?.text_mut() =
            "Never lose 日本語 e\u{301} 🪶".into();
        let transaction = state.insertion_transaction()?;
        let completion = prepare_and_save(
            Arc::clone(&state.session),
            &database,
            &project.id,
            &state.snapshot.canvas.id,
            state.snapshot.revision.clone(),
            PendingOperation::Transaction(transaction),
        );
        assert!(
            completion.prepared.is_none(),
            "actual model capacity refusal"
        );
        assert!(deliver(&mut state, completion).is_err());
        assert!(state.latest_text_candidate().is_err());
        assert!(state.reload_saved(&database).is_err());
        let path = directory.path().join("card-recovery.json");
        std::fs::write(&path, "Existing file must remain unchanged")?;
        assert!(state.write_full_card_draft(&path).is_err());
        assert!(!state.failed_preserved);
        assert_eq!(
            std::fs::read_to_string(&path)?,
            "Existing file must remain unchanged"
        );
        let path = directory.path().join("complete-card-recovery.json");
        state.write_full_card_draft(&path)?;
        let restored: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        assert_eq!(restored["format"], "writercraft-card-recovery");
        assert_eq!(restored["original_card_json"], raw);
        assert_eq!(restored["latest_text"], "Never lose 日本語 e\u{301} 🪶");
        assert_eq!(
            restored["base_board_json"],
            state.session.document().raw_json()
        );
        assert_eq!(restored["commit_outcome"], "unacknowledged");
        assert_eq!(state.session.document().nodes().len(), 1);
        assert!(state.failed_preserved);
        // A successful export is no longer complete after actual subsequent native input.
        let ctx = egui::Context::default();
        show_card_frame(&mut state, &database, &ctx, vec![], 0.);
        show_card_frame(
            &mut state,
            &database,
            &ctx,
            vec![egui::Event::Text(" later".into())],
            0.1,
        );
        assert!(!state.failed_preserved);
        assert!(state.reload_saved(&database).is_err());
        let path = directory.path().join("latest-card-recovery.json");
        state.write_full_card_draft(&path)?;
        state.reload_saved(&database)?;
        assert!(state.text_editor.is_none());
        assert!(!state.blocks_navigation());
        assert_eq!(state.snapshot.revision.content, graph());
        Ok(())
    }
    #[test]
    fn reload_after_export_should_wait_for_same_frame_native_input_and_recheck_preservation()
    -> TestResult {
        for event in [
            egui::Event::Text(" later".into()),
            egui::Event::Paste(" later".into()),
            egui::Event::Ime(egui::ImeEvent::Preedit {
                text: "にほん".into(),
                active_range_chars: Some(0..3),
            }),
        ] {
            let directory = tempfile::tempdir()?;
            let database = directory.path().join("reload-order.sqlite");
            let workspace = Workspace::open(&database)?;
            let project = workspace.create_project("Synthetic reload ordering")?;
            let board = workspace.create_project_canvas(&project.id, "Board", graph())?;
            let mut state = CanvasState::from_snapshot(&project.id, board)?;
            let ctx = egui::Context::default();
            activate_card(&mut state, &database, &ctx)?;
            show_card_frame(&mut state, &database, &ctx, vec![], 0.);
            state.text_save_error = true;
            state.failure = Some("synthetic refused save".into());
            state.write_full_card_draft(&directory.path().join("recovery.json"))?;
            assert!(state.failed_preserved);
            // The toolbar queues this same request. Painting must process input before resolution.
            state.reload_requested = true;
            show_card_frame(&mut state, &database, &ctx, vec![event], 0.1);
            assert!(state.text_editor.is_some());
            assert!(!state.failed_preserved);
            assert!(!state.reload_requested);
            assert!(state.blocks_navigation());
            assert_eq!(state.snapshot.revision.content, graph());
        }
        Ok(())
    }
    #[test]
    fn existing_card_recovery_should_keep_submitted_candidate_and_later_text_separate() -> TestResult
    {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("existing-recovery.sqlite");
        let workspace = Workspace::open(&database)?;
        let project = workspace.create_project("Synthetic existing card recovery")?;
        let board = workspace.create_project_canvas(&project.id, "Board", graph())?;
        let mut state = CanvasState::from_snapshot(&project.id, board)?;
        let mut editor = TextCardEditor::new(&state.session, "card")?;
        *editor.text_mut() = "Submitted source".into();
        let prepared = state.session.prepare(editor.submit()?, &state.dialect)?;
        let submitted_raw = prepared.document().raw_json().to_owned();
        *editor.text_mut() = "Later source 日本語".into();
        state.text_editor = Some(editor);
        assert!(
            deliver(
                &mut state,
                CommitCompletion {
                    prepared: Some(prepared),
                    result: Err("synthetic persistence failure".into()),
                    checkpoint_version: None,
                }
            )
            .is_err()
        );
        let path = directory.path().join("existing-card-recovery.json");
        state.write_full_card_draft(&path)?;
        let restored: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        assert_eq!(restored["latest_text"], "Later source 日本語");
        assert_eq!(restored["submitted_candidate_json"], submitted_raw);
        assert_eq!(
            restored["base_board_json"],
            state.session.document().raw_json()
        );
        assert_eq!(restored["awaiting_insertion"], false);
        assert!(state.failed_preserved);
        Ok(())
    }
    fn activate_card(
        state: &mut CanvasState,
        database: &Path,
        ctx: &egui::Context,
    ) -> AppResult<()> {
        let mut result = Ok(());
        let mut activation_output = ctx.run_ui(egui::RawInput::default(), |ui| {
            result = state.intent(
                database,
                ui.ctx(),
                CanvasIntent::EditText {
                    base: state.session.identity(),
                    node_id: "card".into(),
                },
            );
        });
        activation_output.textures_delta.clear();
        activation_output.platform_output.commands.clear();
        result
    }
    #[test]
    fn done_acknowledgement_should_not_drop_input_arriving_before_the_same_frames_editor_pass()
    -> TestResult {
        for creating in [false, true] {
            for event in [
                egui::Event::Text(" later".into()),
                egui::Event::Paste(" later".into()),
                egui::Event::Ime(egui::ImeEvent::Preedit {
                    text: "にほん".into(),
                    active_range_chars: Some(0..3),
                }),
                egui::Event::Ime(egui::ImeEvent::Commit("日本語".into())),
            ] {
                let directory = tempfile::tempdir()?;
                let database = directory.path().join("done-ack.sqlite");
                let workspace = Workspace::open(&database)?;
                let project = workspace.create_project("Synthetic Done acknowledgement race")?;
                let board = workspace.create_project_canvas(&project.id, "Board", graph())?;
                let mut state = CanvasState::from_snapshot(&project.id, board)?;
                let ctx = egui::Context::default();
                let transaction = if creating {
                    let node = NodeRecord::text(
                        "pending-card".into(),
                        WorldRect {
                            x: 0.0,
                            y: 0.0,
                            width: 300.0,
                            height: 180.0,
                        },
                        String::new(),
                        state.session.limits(),
                    )?;
                    let mut editor = TextCardEditor::pending_insertion(&state.session, node)?;
                    let mut activated = Ok(());
                    let mut activation_output = ctx.run_ui(egui::RawInput::default(), |ui| {
                        activated = editor.begin_inline(ui.ctx());
                    });
                    activation_output.textures_delta.clear();
                    activation_output.platform_output.commands.clear();
                    activated?;
                    state.text_editor = Some(editor);
                    state.insertion_transaction()?
                } else {
                    activate_card(&mut state, &database, &ctx)?;
                    show_card_frame(&mut state, &database, &ctx, vec![], 0.);
                    let editor = state.text_editor.as_mut().ok_or("editor")?;
                    *editor.text_mut() = "Saved".into();
                    editor.submit()?
                };
                let prepared = state.session.prepare(transaction, &state.dialect)?;
                state.text_done = true;
                let completion = persist_candidate(
                    &database,
                    &project.id,
                    &state.snapshot.canvas.id,
                    state.snapshot.revision.clone(),
                    prepared,
                );
                // Exact app order: raw hook -> poll persistence -> native card paint.
                let ime = matches!(event, egui::Event::Ime(_));
                let commit = matches!(event, egui::Event::Ime(egui::ImeEvent::Commit(_)));
                let events = if commit {
                    vec![
                        egui::Event::Ime(egui::ImeEvent::Preedit {
                            text: "にほん".into(),
                            active_range_chars: Some(0..3),
                        }),
                        event,
                    ]
                } else {
                    vec![event]
                };
                let mut input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(1000., 680.),
                    )),
                    events,
                    time: Some(0.1),
                    focused: true,
                    ..Default::default()
                };
                state.prepare_input(&ctx, &mut input, false);
                let (sender, receiver) = mpsc::channel();
                sender
                    .send(completion)
                    .map_err(|_| "completion receiver closed")?;
                state.pending = Some(receiver);
                state.poll(&ctx)?;
                assert!(state.text_editor.is_some(), "creating={creating}");
                paint_card_input(&mut state, &database, &ctx, input);
                let expected_prefix = if creating { "" } else { "Saved" };
                let editor = state.text_editor.as_ref().ok_or("retained editor")?;
                assert!(editor.text().starts_with(expected_prefix));
                assert!(editor.text().ends_with(if commit {
                    "日本語"
                } else if ime {
                    "にほん"
                } else {
                    " later"
                }));
                if ime {
                    assert!(state.text_composing);
                    assert!(
                        state.pending.is_none(),
                        "preedit must not be saved as committed text"
                    );
                    if !commit {
                        show_card_frame(
                            &mut state,
                            &database,
                            &ctx,
                            vec![egui::Event::Ime(egui::ImeEvent::Commit("日本語".into()))],
                            0.2,
                        );
                    }
                    assert!(
                        state
                            .text_editor
                            .as_ref()
                            .ok_or("editor")?
                            .text()
                            .ends_with("日本語")
                    );
                    show_card_frame(&mut state, &database, &ctx, vec![], 0.3);
                }
                assert!(
                    state.pending.is_some(),
                    "Done saves the latest committed input"
                );
                let latest = state
                    .text_editor
                    .as_ref()
                    .ok_or("editor")?
                    .text()
                    .to_owned();
                let node_id = state
                    .text_editor
                    .as_ref()
                    .ok_or("editor")?
                    .node_id()
                    .to_owned();
                let deadline = Instant::now() + Duration::from_secs(5);
                while state.pending.is_some() && Instant::now() < deadline {
                    state.poll(&ctx)?;
                    std::thread::sleep(Duration::from_millis(5));
                }
                assert!(state.pending.is_none());
                // Completion is considered only after a native frame with no new input.
                assert!(state.text_editor.is_some());
                show_card_frame(&mut state, &database, &ctx, vec![], 0.4);
                assert!(state.text_editor.is_none());
                let reopened =
                    CanvasState::open(&workspace, &project.id, &state.snapshot.canvas.id)?;
                assert_eq!(
                    reopened
                        .session
                        .document()
                        .node(&node_id)
                        .ok_or("saved card")?
                        .body(),
                    latest
                );
            }
        }
        Ok(())
    }
    #[test]
    fn pending_cancel_should_freeze_native_input_until_the_original_save_acknowledges() -> TestResult
    {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("cancel-pending.sqlite");
        let workspace = Workspace::open(&database)?;
        let project = workspace.create_project("Synthetic cancelled card")?;
        let board = workspace.create_project_canvas(&project.id, "Board", graph())?;
        let mut state = CanvasState::from_snapshot(&project.id, board)?;
        state.text_editor = Some(TextCardEditor::new(&state.session, "card")?);
        let ctx = egui::Context::default();
        show_card_frame(&mut state, &database, &ctx, vec![], 0.);
        let editor = state.text_editor.as_mut().ok_or("editor")?;
        *editor.text_mut() = "Submitted source 日本語".into();
        let transaction = editor.submit()?;
        let prepared = state.session.prepare(transaction, &state.dialect)?;
        *state.text_editor.as_mut().ok_or("editor")?.text_mut() =
            "Later draft to discard 🪶".into();
        let (sender, receiver) = mpsc::channel();
        state.pending = Some(receiver);
        state.text_cancel = true;
        assert_eq!(state.durability_status(), "Finishing cancellation…");
        show_card_frame(
            &mut state,
            &database,
            &ctx,
            vec![
                egui::Event::Text("Must not be accepted".into()),
                egui::Event::Paste("Nor pasted".into()),
                egui::Event::Ime(egui::ImeEvent::Commit("禁止".into())),
            ],
            1.,
        );
        assert_eq!(
            state.text_editor.as_ref().ok_or("editor")?.text(),
            "Later draft to discard 🪶"
        );
        assert!(state.blocks_navigation());
        sender
            .send(persist_candidate(
                &database,
                &project.id,
                &state.snapshot.canvas.id,
                state.snapshot.revision.clone(),
                prepared,
            ))
            .map_err(|_| "completion")?;
        state.poll(&ctx)?;
        assert!(state.text_editor.is_none());
        let reopened = workspace.project_canvas(&project.id, &state.snapshot.canvas.id)?;
        assert_eq!(
            reopened.revision.content["nodes"][0]["text"],
            "Submitted source 日本語"
        );
        assert!(!state.blocks_navigation());
        Ok(())
    }

    #[test]
    fn failed_cancel_should_retain_exact_draft_and_retry_only_the_original_candidate() -> TestResult
    {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("cancel-failure.sqlite");
        let workspace = Workspace::open(&database)?;
        let project = workspace.create_project("Synthetic cancellation failure")?;
        let board = workspace.create_project_canvas(&project.id, "Board", graph())?;
        let mut state = CanvasState::from_snapshot(&project.id, board)?;
        state.text_editor = Some(TextCardEditor::new(&state.session, "card")?);
        let ctx = egui::Context::default();
        show_card_frame(&mut state, &database, &ctx, vec![], 0.);
        let editor = state.text_editor.as_mut().ok_or("editor")?;
        *editor.text_mut() = "Original submitted request".into();
        let transaction = editor.submit()?;
        let prepared = state.session.prepare(transaction, &state.dialect)?;
        *state.text_editor.as_mut().ok_or("editor")?.text_mut() =
            "Retained recovery draft 🪶".into();
        state.text_cancel = true;
        assert!(
            deliver(
                &mut state,
                CommitCompletion {
                    prepared: Some(prepared),
                    result: Err(std::io::Error::other("Synthetic save failure").into()),
                    checkpoint_version: None
                }
            )
            .is_err()
        );
        assert!(state.save_now(&database).is_err());
        assert!(state.pending.is_none());
        show_card_frame(
            &mut state,
            &database,
            &ctx,
            vec![egui::Event::Text("Rejected later input".into())],
            1.,
        );
        assert_eq!(
            state.text_editor.as_ref().ok_or("editor")?.text(),
            "Retained recovery draft 🪶"
        );
        assert_eq!(state.durability_status(), "Attention needed");
        assert!(state.blocks_navigation());
        assert_eq!(
            workspace
                .project_canvas(&project.id, &state.snapshot.canvas.id)?
                .revision
                .content,
            graph()
        );
        state.retry(&database)?;
        let deadline = Instant::now() + Duration::from_secs(5);
        while state.pending.is_some() && Instant::now() < deadline {
            state.poll(&ctx)?;
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(state.pending.is_none());
        assert!(state.text_editor.is_none());
        assert_eq!(
            workspace
                .project_canvas(&project.id, &state.snapshot.canvas.id)?
                .revision
                .content["nodes"][0]["text"],
            "Original submitted request"
        );
        Ok(())
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
        let ctx = egui::Context::default();
        activate_card(&mut state, &database, &ctx)?;
        assert!(!state.modal_open());
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
        activate_card(state, &database, &ctx)?;
        *state.text_editor.as_mut().ok_or("editor")?.text_mut() =
            "Global Save routes to this card".into();
        app.poll_canvas(&ctx);
        assert_eq!(app.status, "Unsaved board changes");
        assert_eq!(app.save_status_msgid(false), "Unsaved board changes");
        app.save_explicit()?;
        assert_eq!(app.save_status_msgid(false), "Saving…");
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
        app.poll_canvas(&ctx);
        assert_eq!(app.save_status_msgid(false), "All changes saved");
        let state = app.canvas.as_mut().ok_or("canvas")?;
        state.text_composing = true;
        assert_eq!(app.save_status_msgid(false), "Unsaved board changes");
        let state = app.canvas.as_mut().ok_or("canvas")?;
        state.text_composing = false;
        state.text_save_error = true;
        assert_eq!(app.save_status_msgid(false), "Attention needed");
        assert_eq!(app.workspace.document(&document.id)?, document);
        Ok(())
    }
    fn backup_fixture() -> AppResult<(tempfile::TempDir, StudioApp, String)> {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("board-backup.sqlite");
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = StudioApp::new(&creation, database, false, None)?;
        let project = app.workspace.create_project("Board backup prerequisite")?;
        app.workspace
            .create_document(&project.id, "Sentinel", "Unrelated writing")?;
        let board = app
            .workspace
            .create_project_canvas(&project.id, "Board", graph())?;
        app.open_project(project.clone())?;
        app.open_canvas(&board.canvas.id)?;
        Ok((directory, app, project.id))
    }
    #[test]
    fn backup_should_wait_for_exact_board_acknowledgement_and_later_typing() -> TestResult {
        let (_directory, mut app, project) = backup_fixture()?;
        let database = app.database.clone();
        let state = app.canvas.as_mut().ok_or("canvas")?;
        let board = state.snapshot.canvas.id.clone();
        let revision = state.snapshot.revision.clone();
        let mut editor = TextCardEditor::new(&state.session, "card")?;
        *editor.text_mut() = "First requested writing".into();
        let prepared = state.session.prepare(editor.submit()?, &state.dialect)?;
        state.text_editor = Some(editor);
        let (sender, receiver) = mpsc::channel();
        state.pending = Some(receiver);
        assert!(!app.prepare_workspace_backup()?);
        assert!(app.storage.modal_open());
        assert!(!app.storage.busy());
        *app.canvas
            .as_mut()
            .ok_or("canvas")?
            .text_editor
            .as_mut()
            .ok_or("editor")?
            .text_mut() = "Later exact writing 日本語 😀".into();
        sender
            .send(persist_candidate(
                &database, &project, &board, revision, prepared,
            ))
            .map_err(|_| "completion")?;
        let ctx = egui::Context::default();
        app.poll_canvas(&ctx);
        assert_eq!(
            app.workspace
                .project_canvas(&project, &board)?
                .revision
                .content["nodes"][0]["text"],
            "First requested writing"
        );
        assert!(
            !app.prepare_workspace_backup()?,
            "a first acknowledgement is not a receipt for later typing"
        );
        assert!(!app.storage.busy());
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.canvas.as_ref().ok_or("canvas")?.pending.is_some() && Instant::now() < deadline {
            app.poll_canvas(&ctx);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(app.prepare_workspace_backup()?);
        assert!(!app.storage.modal_open());
        assert!(
            !app.storage.busy(),
            "preparation alone never starts a backup worker"
        );
        assert_eq!(
            app.workspace
                .project_canvas(&project, &board)?
                .revision
                .content["nodes"][0]["text"],
            "Later exact writing 日本語 😀"
        );
        assert_eq!(
            app.workspace.list_documents(&project)?[0].body,
            "Unrelated writing"
        );
        Ok(())
    }
    #[test]
    fn backup_failure_or_composition_should_preserve_the_card_and_start_no_snapshot_job()
    -> TestResult {
        let (_directory, mut app, project) = backup_fixture()?;
        let state = app.canvas.as_mut().ok_or("canvas")?;
        let board = state.snapshot.canvas.id.clone();
        let mut editor = TextCardEditor::new(&state.session, "card")?;
        *editor.text_mut() = "Retained unsaved source 😀".into();
        state.text_editor = Some(editor);
        state.text_composing = true;
        assert!(app.prepare_workspace_backup().is_err());
        assert!(!app.storage.busy());
        let state = app.canvas.as_mut().ok_or("canvas")?;
        state.text_composing = false;
        let prepared = state.session.prepare(
            state.text_editor.as_mut().ok_or("editor")?.submit()?,
            &state.dialect,
        )?;
        let (sender, receiver) = mpsc::channel();
        state.pending = Some(receiver);
        assert!(!app.prepare_workspace_backup()?);
        sender
            .send(CommitCompletion {
                prepared: Some(prepared),
                result: Err(std::io::Error::other("Synthetic persistence failure").into()),
                checkpoint_version: None,
            })
            .map_err(|_| "completion")?;
        app.poll_canvas(&egui::Context::default());
        assert!(app.canvas.as_ref().ok_or("canvas")?.failed.is_some());
        assert!(!app.storage.busy());
        assert!(app.storage.modal_open());
        app.cancel_workspace_backup_preflight();
        assert!(!app.storage.modal_open());
        let state = app.canvas.as_ref().ok_or("canvas")?;
        assert!(state.blocks_navigation());
        assert_eq!(
            state.text_editor.as_ref().ok_or("editor")?.text(),
            "Retained unsaved source 😀"
        );
        assert_eq!(
            app.workspace
                .project_canvas(&project, &board)?
                .revision
                .content,
            graph()
        );
        Ok(())
    }

    #[test]
    fn creation_should_refuse_before_mutation_for_every_unacknowledged_card_state() -> TestResult {
        for blocked in ["dirty", "composition", "pending", "failed"] {
            let (_directory, mut app, project) = backup_fixture()?;
            app.project_name = "Retained project form".into();
            app.document_name = "Retained document form".into();
            let before_projects = app.workspace.list_projects()?;
            let before_documents = app.workspace.list_documents(&project)?;
            let state = app.canvas.as_mut().ok_or("canvas")?;
            let mut editor = TextCardEditor::new(&state.session, "card")?;
            if blocked == "dirty" {
                *editor.text_mut() = "Local unsaved source 😀".into();
            }
            state.text_editor = Some(editor);
            let (_sender, receiver) = mpsc::channel();
            match blocked {
                "composition" => state.text_composing = true,
                "pending" => state.pending = Some(receiver),
                "failed" => {
                    state.failed = Some(state.session.prepare(
                        CanvasTransaction::new(
                            state.session.identity(),
                            vec![CanvasCommand::EditText {
                                node_id: "card".into(),
                                expected: "Old 😀 text".into(),
                                text: "Unacknowledged source".into(),
                            }],
                        ),
                        &state.dialect,
                    )?)
                }
                _ => {}
            }
            assert!(app.create_project_from_form().is_err(), "{blocked}");
            assert!(app.create_document_from_form().is_err(), "{blocked}");
            assert_eq!(app.project_name, "Retained project form");
            assert_eq!(app.document_name, "Retained document form");
            assert_eq!(app.workspace.list_projects()?, before_projects);
            assert_eq!(app.workspace.list_documents(&project)?, before_documents);
            let state = app.canvas.as_mut().ok_or("canvas")?;
            assert!(state.text_editor.is_some());
            state.text_editor = None;
            state.text_composing = false;
            state.pending = None;
            state.failed = None;
            let created = app.create_project_from_form()?;
            assert_eq!(
                app.workspace.list_projects()?.len(),
                before_projects.len() + 1
            );
            assert_eq!(app.workspace.list_documents(&created.id)?.len(), 1);
            let document = app.create_document_from_form()?;
            assert_eq!(document.project_id, project);
            assert_eq!(
                app.workspace.list_documents(&project)?.len(),
                before_documents.len() + 1
            );
        }
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
    fn two_card_graph() -> serde_json::Value {
        json!({"nodes":[{"id":"card","type":"text","x":0,"y":0,"width":300,"height":180,"text":"Old 😀 text","x-writers-studio":{"card":{"typeId":"unknown","fields":{"opaque":[1,null]}},"future":{"keep":true}}},{"id":"other","type":"text","x":420,"y":40,"width":300,"height":180,"text":"Second 日本語 card","future":{"keep":"edge-test"}}],"edges":[],"x-writers-studio":{"schemaVersion":2,"vendor":{"keep":true}}})
    }
    fn settle_pending_write(state: &mut CanvasState, ctx: &egui::Context) -> TestResult {
        let deadline = Instant::now() + Duration::from_secs(5);
        while state.pending.is_some() && Instant::now() < deadline {
            state.poll(ctx)?;
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            state.pending.is_none(),
            "synthetic save did not acknowledge"
        );
        Ok(())
    }
    #[test]
    fn connection_mode_should_create_a_durable_compensated_edge_between_two_cards() -> TestResult {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("connection.sqlite");
        let workspace = Workspace::open(&database)?;
        let project = workspace.create_project("Synthetic board connection")?;
        let source = two_card_graph();
        let board = workspace.create_project_canvas(&project.id, "Board", source.clone())?;
        let mut state = CanvasState::from_snapshot(&project.id, board)?;
        let ctx = egui::Context::default();
        assert_eq!(state.tool, BoardTool::Select);
        state.set_tool(BoardTool::Connect);
        // The same controller entry points the board view drives for completed card clicks.
        state.board_card_click(&database, &ctx, "card");
        assert_eq!(state.connect_source.as_deref(), Some("card"));
        state.board_card_click(&database, &ctx, "other");
        assert!(state.connect_source.is_none());
        assert!(state.pending.is_some());
        assert!(
            state.blocks_navigation(),
            "the unacknowledged edge write blocks navigation like a text save"
        );
        settle_pending_write(&mut state, &ctx)?;
        assert!(!state.blocks_navigation());
        assert_eq!(state.session.document().edges().len(), 1);
        let edge_id = state.session.document().edges()[0].id().to_owned();
        let saved = workspace.project_canvas(&project.id, &state.snapshot.canvas.id)?;
        assert_eq!(saved.revision.info.revision, 2);
        assert!(saved.draft.is_none());
        let edges = saved.revision.content["edges"]
            .as_array()
            .ok_or("durable edges")?;
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0]["id"], json!(edge_id));
        assert_eq!(edges[0]["fromNode"], json!("card"));
        assert_eq!(edges[0]["toNode"], json!("other"));
        // Every canonical record the connector never touched stays exactly preserved.
        assert_eq!(saved.revision.content["nodes"], source["nodes"]);
        assert_eq!(
            saved.revision.content["x-writers-studio"],
            source["x-writers-studio"]
        );
        // Reopening the board shows the acknowledged edge.
        let reopened = CanvasState::open(&workspace, &project.id, &state.snapshot.canvas.id)?;
        let reopened_edge = reopened
            .session
            .document()
            .edge(&edge_id)
            .ok_or("reopened edge")?;
        assert_eq!(reopened_edge.from_node(), "card");
        assert_eq!(reopened_edge.to_node(), "other");
        // Undo compensates the insertion through the guarded durable path.
        state.begin(&database, PendingOperation::Undo(state.session.identity()))?;
        settle_pending_write(&mut state, &ctx)?;
        assert!(state.session.document().edges().is_empty());
        let saved = workspace.project_canvas(&project.id, &state.snapshot.canvas.id)?;
        assert_eq!(saved.revision.info.revision, 3);
        assert_eq!(saved.revision.content["edges"], json!([]));
        // Redo restores the same connector durably.
        state.begin(&database, PendingOperation::Redo(state.session.identity()))?;
        settle_pending_write(&mut state, &ctx)?;
        assert_eq!(state.session.document().edges().len(), 1);
        assert_eq!(state.session.document().edges()[0].id(), edge_id);
        let saved = workspace.project_canvas(&project.id, &state.snapshot.canvas.id)?;
        assert_eq!(saved.revision.info.revision, 4);
        let edges = saved.revision.content["edges"]
            .as_array()
            .ok_or("redone edges")?;
        assert_eq!(edges[0]["id"], json!(edge_id));
        assert_eq!(edges[0]["fromNode"], json!("card"));
        assert_eq!(edges[0]["toNode"], json!("other"));
        Ok(())
    }
    #[test]
    fn connection_escape_cancel_and_self_click_should_refuse_without_any_mutation() -> TestResult {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("connection-refusal.sqlite");
        let workspace = Workspace::open(&database)?;
        let project = workspace.create_project("Synthetic refused connections")?;
        let source = two_card_graph();
        let board = workspace.create_project_canvas(&project.id, "Board", source.clone())?;
        let mut state = CanvasState::from_snapshot(&project.id, board)?;
        let ctx = egui::Context::default();
        let original = state.session.document().raw_json().to_owned();
        state.set_tool(BoardTool::Connect);
        state.board_card_click(&database, &ctx, "card");
        assert_eq!(state.connect_source.as_deref(), Some("card"));
        // Escape cancels only the pending source selection; no write ever starts.
        state.cancel_connect_selection();
        assert!(state.connect_source.is_none());
        assert!(state.pending.is_none());
        assert!(!state.blocks_navigation());
        // Two presses on the same card never become a self-loop.
        state.board_card_click(&database, &ctx, "card");
        state.board_card_click(&database, &ctx, "card");
        assert_eq!(state.connect_source.as_deref(), Some("card"));
        assert!(state.pending.is_none());
        let self_connect = state.connect_nodes(&database, &ctx, "card", "card");
        assert!(self_connect.is_err());
        state.cancel_connect_selection();
        let unknown_target = state.connect_nodes(&database, &ctx, "card", "absent");
        assert!(unknown_target.is_err());
        assert_eq!(state.session.document().raw_json(), original);
        assert!(state.session.document().edges().is_empty());
        let saved = workspace.project_canvas(&project.id, &state.snapshot.canvas.id)?;
        assert_eq!(saved.revision.info.revision, 1);
        assert_eq!(saved.revision.content, source);
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
    /// Start or continue saving only the matching active board; true means acknowledged clean.
    /// This never starts a transfer or treats a queued worker as a durability receipt.
    pub(super) fn flush_canvas_for_transfer(&mut self, project: Option<&str>) -> AppResult<bool> {
        let Some(state) = self
            .canvas
            .as_mut()
            .filter(|state| project.is_none_or(|project| project == state.project_id))
        else {
            return Ok(true);
        };
        state.save_now(&self.database)?;
        Ok(!state.blocks_navigation())
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
