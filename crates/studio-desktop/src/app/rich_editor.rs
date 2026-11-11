//! Isolated engineering preview: canonical rich session, durable drafts and native canvas.
//! This integration does not assert completion of the full-editor-parity release gate.
#[path = "authoring.rs"]
mod authoring;
#[cfg(test)]
#[path = "rich_editor/selection_reveal_tests.rs"]
mod selection_reveal_tests;
use super::*;
use craft_editor_egui::{DocPoint, EditorSession, MarkState, Selection, canvas::CanvasStyle};
use craft_editor_model as tables;
use craft_editor_model::{
    CellRect, CellSelection, Document as RichDocument, Mark, Node, NodePath, NodeRef, ReplayProof,
    Revision, ScalarOffset, TableGrid,
};
use serde_json::{Map, json};

pub(super) struct RichState {
    pub session: EditorSession,
    draft_version: Option<String>,
    pub recovered: bool,
    media: rendering::RenderState,
    pub(super) proofing: proofing::ProofingState,
    pending_proofs: Vec<ReplayProof>,
    read_only: bool,
    recovery_read_only: bool,
    last_diagnostic: Option<String>,
    dialog: Option<authoring::BlockDialog>,
    find_requested: bool,
    capture_scroll_offset: f32,
    measured_scroll_offset: Option<f32>,
    navigation_scroll_offset: Option<f32>,
    measured_viewport: Option<egui::Vec2>,
    capture_text_visible: bool,
    statistics: std::cell::Cell<Option<(Revision, studio_core::DocumentStatistics)>>,
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
        session.set_table_column_limit(Some(63))?;
        session.options.id = egui::Id::new(("rich-manuscript", &document.id));
        Ok(Some(Self {
            session,
            draft_version: draft.map(|draft| draft.version),
            recovered,
            read_only,
            recovery_read_only: false,
            media: rendering::RenderState::default(),
            proofing: proofing::ProofingState::default(),
            pending_proofs: vec![],
            last_diagnostic: None,
            dialog: None,
            find_requested: false,
            capture_scroll_offset: 0.0,
            measured_scroll_offset: None,
            navigation_scroll_offset: None,
            measured_viewport: None,
            capture_text_visible: false,
            statistics: std::cell::Cell::new(None),
        }))
    }

    /// Engineering capture navigation uses the same guarded semantic selection as native UI.
    pub(super) fn select_capture_target(&mut self, id: &str) -> AppResult<()> {
        let path = find_path(self.session.document().root(), id, NodePath::root())
            .ok_or("capture_target_not_found")?;
        let node = self.session.document().node(&path)?;
        if matches!(
            node.node_type.as_str(),
            "paragraph" | "heading" | "codeBlock"
        ) {
            let point = DocPoint {
                block_id: id.into(),
                offset: ScalarOffset(0),
            };
            self.session.set_selection(Selection {
                anchor: point.clone(),
                head: point,
            })?;
        } else {
            let reference = self.session.document().node_ref(&path)?;
            self.session.select_node(reference)?;
        }
        self.session.request_scroll_to_caret();
        Ok(())
    }
    fn manuscript_scroll_area(&mut self, ui: &egui::Ui) -> egui::ScrollArea {
        let salt = ("rich-scroll", self.session.options.id);
        let area = egui::ScrollArea::vertical().id_salt(salt);
        match self.navigation_scroll_offset.take() {
            Some(offset) => {
                // A saved bookmark overrides pending engine reveals AND host-owned animation.
                // Reset only this manuscript's presentation state, leaving its session intact.
                self.session.cancel_scroll_request();
                let mut state = egui::scroll_area::State::default();
                state.offset.y = offset;
                // Match ScrollArea's stored IdSalt layer, rather than a raw tuple's different ID.
                state.store(ui.ctx(), ui.make_persistent_id(egui::IdSalt::new(salt)));
                area.vertical_scroll_offset(offset)
            }
            None => area,
        }
    }
    fn reveal_after_viewport_change(&mut self, extent: egui::Vec2, restoring_scroll: bool) {
        let changed = self
            .measured_viewport
            .replace(extent)
            .is_some_and(|prior| prior != extent);
        if changed
            && !restoring_scroll
            && !self.session.is_composing()
            && self
                .session
                .selection()
                .is_some_and(|selection| selection.anchor != selection.head)
        {
            self.session.request_scroll_to_selection();
        }
    }
    /// Last measured native manuscript scroll offset, never an invented initial bookmark.
    pub(super) fn measured_navigation_scroll_offset(&self) -> Option<f32> {
        self.measured_scroll_offset
    }
    /// Queue presentation restoration after a guarded same-document return.
    pub(super) fn restore_navigation_scroll(&mut self, offset: f32) -> AppResult<()> {
        if !offset.is_finite() || offset < 0.0 {
            return Err("invalid_navigation_scroll_offset".into());
        }
        self.session.cancel_scroll_request();
        self.navigation_scroll_offset = Some(offset);
        Ok(())
    }
    pub(super) fn capture_readiness(
        &self,
        id: Option<&str>,
    ) -> super::native_capture::CaptureReadiness {
        let path =
            id.and_then(|id| find_path(self.session.document().root(), id, NodePath::root()));
        let mut state = self.media.capture_readiness(path.as_ref());
        state.scroll_offset_bits = self.capture_scroll_offset.to_bits();
        if let Some(path) = path.as_ref()
            && let Ok(node) = self.session.document().node(path)
            && matches!(
                node.node_type.as_str(),
                "paragraph" | "heading" | "codeBlock"
            )
        {
            state.target_visible = self.capture_text_visible;
            state.target_is_text = true;
            state.visible_text_targets = usize::from(self.capture_text_visible);
        }
        state
    }
    /// Count the live native tree rather than a lossy autosave/reader projection.
    /// Metadata, selection and repaint changes do not rewalk the manuscript.
    pub(super) fn statistics(&self) -> studio_core::DocumentStatistics {
        let revision = self.session.revision();
        if let Some((known_revision, statistics)) = self.statistics.get()
            && known_revision == revision
        {
            return statistics;
        }
        let statistics = studio_core::rich_document_statistics(self.session.document());
        self.statistics.set(Some((revision, statistics)));
        statistics
    }

    pub(super) fn is_read_only(&self) -> bool {
        self.read_only
    }
    pub(super) fn is_recovery_read_only(&self) -> bool {
        self.recovery_read_only
    }

    /// Mount only the acknowledged canonical tree. A conflicting durable draft
    /// remains intact in the workspace and is never used as attachment authority.
    pub(super) fn open_saved_read_only(document: &Document) -> AppResult<Self> {
        let content = document
            .rich_json
            .as_ref()
            .ok_or("missing_saved_rich_document")?;
        let mut session =
            EditorSession::new(RichDocument::from_json(content.clone())?, Revision(0));
        session.set_table_column_limit(Some(63))?;
        session.options.id = egui::Id::new(("rich-manuscript", &document.id));
        Ok(Self {
            session,
            draft_version: None,
            recovered: false,
            read_only: true,
            recovery_read_only: true,
            media: rendering::RenderState::default(),
            proofing: proofing::ProofingState::default(),
            pending_proofs: vec![],
            last_diagnostic: None,
            dialog: None,
            find_requested: false,
            capture_scroll_offset: 0.0,
            measured_scroll_offset: None,
            navigation_scroll_offset: None,
            measured_viewport: None,
            capture_text_visible: false,
            statistics: std::cell::Cell::new(None),
        })
    }

    pub(super) fn pending_proof_count(&self) -> usize {
        self.pending_proofs.len()
    }

    pub(super) fn dialog_open(&self) -> bool {
        self.dialog.is_some()
    }

    /// Whether the manuscript currently has a whole-object target the source dialog can edit.
    pub(super) fn editable_block_selected(&self) -> bool {
        !self.read_only
            && self.dialog.is_none()
            && authoring::editable_target(&self.session).is_some()
    }

    /// Open the existing source dialog for the current editable object selection.
    pub(super) fn open_selected_block(&mut self) -> AppResult<bool> {
        authoring::open_selected_block(self)
    }

    fn write_draft(&mut self, workspace: &Workspace, document: &Document) -> AppResult<()> {
        if self.read_only {
            return Err(studio_core::StudioError::ReadOnly {
                document_id: document.id.clone(),
            }
            .into());
        }
        for change in self.session.take_pending_changes() {
            self.pending_proofs.push(change.replay_proof());
        }
        let content = self.session.document().to_json()?;
        let draft = if self.recovered {
            // Old unproven snapshots keep conservative recovery; they never
            // receive fabricated replay or attachment authority.
            workspace.save_rich_draft(
                &document.id,
                document.revision,
                self.draft_version.as_deref(),
                content,
            )?
        } else {
            workspace.save_rich_editor_draft(
                &document.id,
                document.revision,
                self.draft_version.as_deref(),
                &self.pending_proofs,
                content,
            )?
        };
        self.draft_version = Some(draft.version);
        Ok(())
    }
}

impl StudioApp {
    pub(super) fn open_saved_recovery(
        &mut self,
        snapshot: &studio_core::RichRecoverySnapshot,
    ) -> AppResult<()> {
        self.commit_title_edit()?;
        self.save()?;
        self.workspace.verify_rich_recovery_snapshot(snapshot)?;
        let document = snapshot.document().clone();
        let projects = self.workspace.list_projects()?;
        let project = projects
            .iter()
            .find(|project| project.id == document.project_id)
            .cloned()
            .ok_or("missing_recovery_project")?;
        let documents = self.workspace.list_documents(&project.id)?;
        let nodes = self.workspace.initialize_project_tree(&project.id)?;
        if !super::project_tree::document_is_active(&nodes, &document.id) {
            return Err(super::project_tree::TrashedDocument.into());
        }
        let outline = super::project_navigation::ProjectOutline::prepare(&project.id, &nodes)?;
        let mut tree_ui = super::project_tree::TreeUi::with_structure(
            nodes,
            self.workspace.project_structure(&project.id)?,
        );
        tree_ui.selected = tree_ui
            .nodes
            .iter()
            .find(|node| node.document_id.as_deref() == Some(document.id.as_str()))
            .map(|node| node.logical_id.clone());
        let proposals = self.workspace.proposals(&document.id)?;
        let mut rich = RichState::open_saved_read_only(&document)?;
        rich.session.set_clipboard_provider(self.clipboard.clone());
        rich.session
            .set_clipboard_mode(craft_editor_egui::clipboard::ClipboardMode::Deferred)?;
        rich.session
            .set_clipboard_intent_gate(self.clipboard_intents.clone())?;
        self.cancel_clipboard_work();
        let notes = self.notes.prepare(&self.workspace, &document);
        self.body = rich.session.document().plain_text();
        self.rich = Some(rich);
        self.document = Some(document);
        self.projects = projects;
        self.project = Some(project);
        self.documents = documents;
        self.project_outline = outline;
        self.tree_ui = tree_ui;
        self.proposals = proposals;
        self.dirty = None;
        self.autosave_paused = false;
        self.library = false;
        self.daily_controls.document_changed();
        self.expected.clear();
        self.replacement.clear();
        self.frozen_selection = None;
        self.status = "Recovery preview · read only".into();
        self.recovery = None;
        self.record_error(notes);
        Ok(())
    }

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

    pub(super) fn ensure_editor_showcase(&mut self, project: &Project) -> AppResult<Document> {
        if let Ok(selector) = std::env::var("WRITERCRAFT_CAPTURE_DOCUMENT")
            && !selector.is_empty()
        {
            if selector != "wide-tables" {
                return Err("unknown_capture_document".into());
            }
            let runner = PathBuf::from(
                std::env::var_os("RUNNER_TEMP")
                    .ok_or("wide_table_capture_requires_hosted_runner")?,
            )
            .canonicalize()?;
            let parent = self
                .database
                .parent()
                .ok_or("wide_table_capture_database_parent_missing")?
                .canonicalize()?;
            if self.capture.is_none()
                || !std::env::args().any(|argument| argument == "--demo")
                || !std::env::args().any(|argument| argument == "--capture")
                || std::env::var("GITHUB_ACTIONS").as_deref() != Ok("true")
                || !parent.starts_with(&runner)
                || !parent.file_name().is_some_and(|name| {
                    name.to_string_lossy()
                        .starts_with("writercraft-startup-workspace-")
                })
            {
                return Err("wide_table_capture_requires_fresh_hosted_synthetic_workspace".into());
            }
            if std::env::var("WRITERCRAFT_CAPTURE_SCENARIO")
                .is_ok_and(|scenario| !scenario.is_empty())
            {
                return Err("capture_document_and_connected_scenario_are_exclusive".into());
            }
            return self.ensure_wide_table_capture(project);
        }
        if let Some(document) = self
            .workspace
            .list_documents(&project.id)?
            .into_iter()
            .find(|document| {
                document
                    .rich_json
                    .as_ref()
                    .and_then(|value| value.get("writerCraftShowcase"))
                    .and_then(serde_json::Value::as_u64)
                    == Some(1)
            })
        {
            return Ok(document);
        }
        let asset = self.workspace.import_image(
            &project.id,
            "lighthouse.png",
            Some("image/png"),
            include_bytes!("../../../../assets/demo/lighthouse.png"),
        )?;
        let mut content: serde_json::Value =
            serde_json::from_str(include_str!("editor_showcase.json"))?;
        fn bind_owned_image(value: &mut serde_json::Value, asset: &studio_core::ImageAsset) {
            if value.get("type").and_then(serde_json::Value::as_str) == Some("imageBlock")
                && value
                    .pointer("/attrs/assetId")
                    .and_then(serde_json::Value::as_str)
                    == Some("showcase-lighthouse")
                && let Some(attrs) = value
                    .get_mut("attrs")
                    .and_then(serde_json::Value::as_object_mut)
            {
                attrs.insert("assetId".into(), json!(asset.id));
                attrs.insert("src".into(), json!(asset.source()));
            }
            if let Some(children) = value
                .get_mut("content")
                .and_then(serde_json::Value::as_array_mut)
            {
                for child in children {
                    bind_owned_image(child, asset);
                }
            }
        }
        bind_owned_image(&mut content, &asset);
        Ok(self
            .workspace
            .create_rich_document(&project.id, "00. All editor blocks", content)?)
    }

    fn ensure_wide_table_capture(&mut self, project: &Project) -> AppResult<Document> {
        if let Some(document) = self
            .workspace
            .list_documents(&project.id)?
            .into_iter()
            .find(|document| {
                document
                    .rich_json
                    .as_ref()
                    .and_then(|value| value.get("writerCraftWideTableCapture"))
                    .and_then(serde_json::Value::as_u64)
                    == Some(1)
            })
        {
            return Ok(document);
        }
        Ok(self.workspace.create_rich_document(
            &project.id,
            "Synthetic wide and nested tables",
            serde_json::from_str(include_str!("editor_wide_tables.json"))?,
        )?)
    }

    pub(super) fn save_rich_editor(&mut self) -> AppResult<()> {
        let document = self.document.as_ref().ok_or("No active document")?;
        let rich = self.rich.as_mut().ok_or("No active rich editor")?;
        if rich.is_read_only() {
            return Err(studio_core::StudioError::ReadOnly {
                document_id: document.id.clone(),
            }
            .into());
        }
        // A pending native preedit belongs to the input method, not canonical storage.
        for change in rich.session.take_pending_changes() {
            rich.pending_proofs.push(change.replay_proof());
        }
        rich.write_draft(&self.workspace, document)?;
        let content = rich.session.document().to_json()?;
        let saved = if !rich.pending_proofs.is_empty() && !rich.recovered {
            self.workspace.save_rich_editor_changes(
                &document.id,
                document.revision,
                &rich.pending_proofs,
                content,
            )?
        } else {
            // Recovered drafts have canonical snapshots but no in-process replay receipts.
            // The core preserves notes conservatively rather than inventing a transform.
            self.workspace
                .save_rich_document(&document.id, document.revision, content)?
        };
        rich.pending_proofs.clear();
        rich.draft_version = None;
        rich.recovered = false;
        self.body.clone_from(&saved.body);
        self.document = Some(saved);
        if let Some(project) = &self.project {
            self.documents = self.workspace.list_documents(&project.id)?;
        }
        self.dirty = None;
        self.status = "All changes saved".into();
        if let Some(document) = &self.document {
            self.notes.prepare(&self.workspace, document)?;
        }
        Ok(())
    }

    pub(super) fn rich_editor(&mut self, ui: &mut egui::Ui, t: Tokens) {
        let l = self.language;
        let Some(mut rich) = self.rich.take() else {
            return;
        };
        let before = rich.session.revision();
        let mut operation_error: Option<Box<dyn std::error::Error + Send + Sync>> = None;
        let preferences = &self.editor_preferences.values;
        let measure = match craft_editor_egui::fonts::zero_advance(
            preferences.font_family.family(),
            "Regular",
            self.font_size,
        ) {
            Ok(zero) => zero * f32::from(preferences.manuscript_width) + 36.0,
            Err(error) => {
                let diagnostic = error.to_string();
                if rich.last_diagnostic.as_ref() != Some(&diagnostic) {
                    rich.last_diagnostic = Some(diagnostic);
                    operation_error = Some(error.into());
                }
                760.0
            }
        };
        let bounds = ui.available_rect_before_wrap();
        let desired_surface = (measure + 160.0).max(920.0);
        let floating = bounds.width() >= desired_surface + 64.0;
        let width = if floating {
            desired_surface
        } else {
            bounds.width()
        };
        let gutter = if floating { 16.0 } else { 0.0 };
        let surface_rect = egui::Rect::from_min_size(
            egui::pos2(bounds.center().x - width * 0.5, bounds.top() + gutter),
            egui::vec2(width, (bounds.height() - gutter * 2.0).max(0.0)),
        );
        ui.painter()
            .rect_filled(bounds, 0, if floating { t.bg } else { t.editor });
        let mut changed = false;
        let mut manuscript_response = None;
        let mut activated_note = None;
        ui.scope_builder(egui::UiBuilder::new().max_rect(surface_rect), |ui| {
            egui::Frame::new()
                .fill(t.editor)
                .corner_radius(if floating { 12 } else { 0 })
                .stroke(Stroke::new(if floating { 1.0 } else { 0.0 }, t.border))
                .show(ui, |ui| {
                    ui.set_min_size(surface_rect.size());
                    ui.set_max_size(surface_rect.size());
                    egui::Panel::top("rich-formatting")
                        .exact_size(40.0)
                        .resizable(false)
                        .frame(
                            egui::Frame::new()
                                .fill(t.editor)
                                .inner_margin(egui::Margin::symmetric(24, 4))
                                .corner_radius(if floating {
                                    egui::CornerRadius {
                                        nw: 12,
                                        ne: 12,
                                        sw: 0,
                                        se: 0,
                                    }
                                } else {
                                    egui::CornerRadius::ZERO
                                }),
                        )
                        .show(ui, |ui| {
                            let boundary = ui.max_rect();
                            ui.painter().hline(
                                (boundary.left() - 24.0)..=(boundary.right() + 24.0),
                                boundary.bottom() + 4.0,
                                Stroke::new(1.0, t.border),
                            );
                            if let Err(error) =
                                authoring::toolbar(ui, &mut rich, l, t, preferences.toolbar_mode)
                            {
                                operation_error = Some(error);
                            }
                        });
                    let media_ready = match rich.media.update(
                        rich.session.document(),
                        rich.session.revision(),
                        rendering::RenderHost {
                            language: l,
                            tokens: t,
                            font_size: self.font_size,
                            pixel_scale: ui.ctx().pixels_per_point(),
                            workspace: &self.workspace,
                            project_id: self
                                .project
                                .as_ref()
                                .map_or("", |project| project.id.as_str()),
                        },
                    ) {
                        Ok(ready) => ready,
                        Err(error) => {
                            let diagnostic = error.to_string();
                            if rich.last_diagnostic.as_ref() != Some(&diagnostic) {
                                rich.last_diagnostic = Some(diagnostic);
                                operation_error = Some(error);
                            }
                            false
                        }
                    };
                    if !media_ready {
                        ui.ctx().request_repaint_after(Duration::from_millis(50));
                    }
                    let restoring_scroll = rich.navigation_scroll_offset.is_some();
                    // Track the host pane, not the internal width that changes when a scrollbar
                    // reappears after bookmark restoration. That must not submit a later reveal.
                    rich.reveal_after_viewport_change(
                        ui.available_rect_before_wrap().size(),
                        restoring_scroll,
                    );
                    rich.manuscript_scroll_area(ui)
                        .auto_shrink([false, false])
                        .show_viewport(ui, |ui, viewport| {
                            // egui advances its animation state after creating this viewport.
                            // A return bookmark must describe what was actually painted.
                            rich.capture_scroll_offset = viewport.min.y;
                            rich.measured_scroll_offset = Some(viewport.min.y);
                            let available = ui.available_width();
                            let width = (available - 56.0).clamp(160.0, measure.max(160.0));
                            ui.horizontal(|ui| {
                                ui.add_space(((available - width) * 0.5).max(0.0));
                                ui.vertical(|ui| {
                                    ui.set_width(width);
                                    ui.add_space(18.0);
                                    if rich.read_only {
                                        ui.label(
                                            RichText::new(l.text(if rich.recovery_read_only {
                                                "Recovery preview · read only"
                                            } else {
                                                "Original format · read only"
                                            }))
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
                                        table_scroll_label: l
                                            .text("Scroll table horizontally")
                                            .into(),
                                        font_size: self.font_size,
                                        font_family: Some(preferences.font_family.family().into()),
                                        line_height: preferences.line_height,
                                        line_height_basis:
                                            craft_editor_egui::canvas::LineHeightBasis::FontSize,
                                        paragraph_spacing: preferences.paragraph_spacing
                                            * self.font_size,
                                        list_item_spacing: Some(
                                            preferences.list_item_spacing * self.font_size,
                                        ),
                                        standard_ligatures: Some(false),
                                        min_height: 500.0,
                                        read_only: rich.read_only || rich.dialog.is_some(),
                                    };
                                    if rich.media.failed() {
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(l.text("Some blocks could not be rendered."));
                                            if ui.button(l.text("Retry rendering")).clicked() {
                                                rich.media.retry();
                                            }
                                        });
                                    }
                                    // Media readiness changes only its opaque atom's native status box. The
                                    // shared prose/table selection and input surface always remains mounted.
                                    let mut note_frame =
                                        self.notes.frame(&rich.pending_proofs, l, t);
                                    let output = {
                                        let mut proof_frame = rich.proofing.frame(
                                            &mut note_frame,
                                            preferences.spellcheck,
                                            l,
                                            t,
                                        );
                                        rich.capture_text_visible = false;
                                        if let Some(target) = self.capture_probe.target() {
                                            let mut observer =
                                                native_capture::VisibleTargetFrame::new(
                                                    &mut proof_frame,
                                                    target,
                                                );
                                            let output = rich.session.show_canvas_with_sources(
                                                ui,
                                                &style,
                                                Some(rich.media.catalog()),
                                                Some(&mut observer),
                                            );
                                            rich.capture_text_visible = observer.visible;
                                            output
                                        } else {
                                            rich.session.show_canvas_with_sources(
                                                ui,
                                                &style,
                                                Some(rich.media.catalog()),
                                                Some(&mut proof_frame),
                                            )
                                        }
                                    };
                                    if rich.proofing.pending() {
                                        ui.ctx().request_repaint_after(Duration::from_millis(50));
                                    }
                                    let note_error = note_frame.take_error();
                                    if let Some(error) = note_error {
                                        operation_error = Some(error);
                                    }
                                    manuscript_response = Some(output.response.clone());
                                    activated_note = output.activated_decorations.first().cloned();
                                    if self.focus_manuscript {
                                        output.response.request_focus();
                                        self.focus_manuscript = false;
                                    }
                                    changed = !output.changes.is_empty();
                                    for change in output.changes {
                                        rich.pending_proofs.push(change.replay_proof());
                                    }
                                    if let Some(error) = output.errors.into_iter().find(|error| {
                                        !matches!(error, craft_editor_egui::Error::DecorationSource)
                                    }) {
                                        if let craft_editor_egui::Error::Clipboard(clipboard) =
                                            &error
                                        {
                                            // Transport backpressure is a retry status, not a modal that
                                            // steals the manuscript selection or composition focus.
                                            self.status = clipboard_host::message(clipboard).into();
                                        } else {
                                            let diagnostic = error.to_string();
                                            if rich.last_diagnostic.as_ref() != Some(&diagnostic) {
                                                rich.last_diagnostic = Some(diagnostic);
                                                operation_error = Some(error.into());
                                            }
                                        }
                                    } else {
                                        rich.last_diagnostic = None;
                                    }
                                    ui.add_space(60.0);
                                });
                            });
                        });
                });
        });
        if let Some(response) = &manuscript_response
            && let Err(error) = authoring::existing_block_gesture(ui.ctx(), &mut rich, response)
            && operation_error.is_none()
        {
            operation_error = Some(error);
        }
        if let Some(project) = &self.project {
            authoring::show_dialog(ui.ctx(), &mut rich, &self.workspace, &project.id, l, t);
        }
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
        let find_requested = std::mem::take(&mut rich.find_requested);
        self.rich = Some(rich);
        if find_requested {
            self.open_document_find();
        }
        if let Some(response) = manuscript_response {
            self.rich_note_selection_menu(&response);
        }
        if let Some(id) = activated_note {
            let result = self.open_mark_note(&id);
            self.record_error(result);
        }
        if let Some(error) = operation_error {
            self.record_error(Err(error));
        }
    }
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
    let kind = if level == 0 {
        craft_editor_egui::TextBlockKind::Paragraph
    } else {
        craft_editor_egui::TextBlockKind::Heading { level }
    };
    session.set_text_block_kind(kind)?;
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
    let id = first_text_identity(&table)
        .ok_or("Missing new cell identity")?
        .to_owned();
    let point = DocPoint {
        block_id: id,
        offset: ScalarOffset(0),
    };
    session.insert_content_with_selection(
        craft_editor_egui::ContentSlice::Blocks(vec![table]),
        Selection {
            anchor: point.clone(),
            head: point,
        },
    )?;
    session.request_scroll_to_caret();
    Ok(())
}

#[derive(Clone, Copy)]
enum TableAction {
    AddRow,
    AddColumn,
    DeleteRow,
    DeleteColumn,
    DeleteTable,
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
        Self::at_path(session, &path)
    }
    fn at_path(session: &EditorSession, path: &NodePath) -> Option<(NodePath, usize, usize)> {
        for length in (0..path.0.len()).rev() {
            let ancestor = NodePath(path.0[..length].to_vec());
            if session.document().node(&ancestor).ok()?.node_type == "table" {
                return Some((ancestor, *path.0.get(length)?, *path.0.get(length + 1)?));
            }
        }
        None
    }
    fn current(session: &EditorSession) -> Option<Self> {
        if let Some(selection) = session.cell_selection() {
            let target = session.document().node_ref(&selection.table_path).ok()?;
            let grid = TableGrid::new(&target.expected).ok()?;
            let cell = grid
                .cell_at(selection.head.row, selection.head.column)?
                .clone();
            let selected = Some(selection.rectangle(session.document()).ok()?);
            return Some(Self {
                target,
                grid,
                cell,
                selected,
            });
        }
        let (path, row, cell_index) = if let Some(reference) = session.node_selection() {
            Self::at_path(session, &reference.path)?
        } else {
            Self::at(session, &session.selection()?.head.block_id)?
        };
        let target = session.document().node_ref(&path).ok()?;
        let grid = TableGrid::new(&target.expected).ok()?;
        let cell = grid
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.cell_index == cell_index)?
            .clone();
        // Only the engine's typed CellSelection is a rectangular table range.
        // A text range keeps the source command's head-cell policy.
        let selected = None;
        Some(Self {
            target,
            grid,
            cell,
            selected,
        })
    }
    fn rectangle(&self) -> CellSelection {
        self.selected.unwrap_or(CellSelection {
            top: self.cell.row,
            left: self.cell.column,
            bottom: self.cell.bottom(),
            right: self.cell.right(),
        })
    }
    fn header_at(&self, row: usize, column: usize) -> bool {
        self.grid.cell_at(row, column).is_some_and(|cell| {
            self.target.expected.children()[cell.row].children()[cell.cell_index].node_type
                == "tableHeader"
        })
    }
    fn allows(&self, action: TableAction) -> bool {
        let selected = self.rectangle();
        match action {
            TableAction::AddColumn => self.grid.columns < 63,
            TableAction::DeleteRow => selected.bottom - selected.top < self.grid.rows,
            TableAction::DeleteColumn => selected.right - selected.left < self.grid.columns,
            TableAction::Merge => self.selected.is_some_and(|selection| {
                (selection.bottom - selection.top) * (selection.right - selection.left)
                    > self.cell.rowspan * self.cell.colspan
            }),
            TableAction::Split => {
                (self.cell.rowspan > 1 || self.cell.colspan > 1)
                    && selected.top == self.cell.row
                    && selected.left == self.cell.column
                    && selected.bottom == self.cell.bottom()
                    && selected.right == self.cell.right()
            }
            TableAction::AddRow | TableAction::DeleteTable => true,
        }
    }
    fn apply(&self, session: &mut EditorSession, action: TableAction) -> AppResult<()> {
        if session.document().node(&self.target.path)? != self.target.expected.as_ref() {
            return Err(craft_editor_model::Error::NodeReferenceConflict.into());
        }
        if !self.allows(action) {
            return Err("Table operation is unavailable".into());
        }
        if matches!(action, TableAction::DeleteTable) {
            session.delete_current_table()?;
            session.request_scroll_to_caret();
            return Ok(());
        }
        let selected = self.rectangle();
        let document = session.document();
        let mut commands = Vec::new();
        let command = match action {
            TableAction::AddRow => {
                let index = selected.bottom;
                let reference =
                    if (0..self.grid.columns).all(|column| self.header_at(index - 1, column)) {
                        (index < self.grid.rows).then_some(index)
                    } else {
                        Some(index - 1)
                    };
                let cells = (0..self.grid.columns)
                    .filter(|column| {
                        !self.grid.cells.iter().any(|cell| {
                            cell.row < index
                                && index < cell.bottom()
                                && cell.column <= *column
                                && *column < cell.right()
                        })
                    })
                    .map(|column| {
                        fresh_cell(reference.is_some_and(|row| self.header_at(row, column)))
                    })
                    .collect();
                let row = Node::block("tableRow", Map::new(), cells);
                Some(tables::insert_table_row(
                    document,
                    &self.target,
                    index,
                    row,
                )?)
            }
            TableAction::AddColumn => {
                let index = selected.right;
                let reference = if (0..self.grid.rows).all(|row| self.header_at(row, index - 1)) {
                    (index < self.grid.columns).then_some(index)
                } else {
                    Some(index - 1)
                };
                let cells = (0..self.grid.rows)
                    .filter(|row| {
                        !self.grid.cells.iter().any(|cell| {
                            cell.row <= *row
                                && *row < cell.bottom()
                                && cell.column < index
                                && index < cell.right()
                        })
                    })
                    .map(|row| {
                        fresh_cell(reference.is_some_and(|column| self.header_at(row, column)))
                    })
                    .collect();
                Some(tables::insert_table_column(
                    document,
                    &self.target,
                    index,
                    cells,
                )?)
            }
            TableAction::DeleteRow | TableAction::DeleteColumn => {
                let mut candidate =
                    craft_editor_model::Editor::new(document.clone(), session.revision());
                let range = if matches!(action, TableAction::DeleteRow) {
                    selected.top..selected.bottom
                } else {
                    selected.left..selected.right
                };
                for index in range.rev() {
                    let target = candidate.document().node_ref(&self.target.path)?;
                    let command = if matches!(action, TableAction::DeleteRow) {
                        tables::delete_table_row(candidate.document(), &target, index)?
                    } else {
                        tables::delete_table_column(candidate.document(), &target, index)?
                    };
                    candidate.apply(craft_editor_model::Transaction {
                        base_revision: candidate.revision(),
                        commands: vec![command.clone()],
                    })?;
                    commands.push(command);
                }
                None
            }
            TableAction::Merge => Some(tables::merge_table_cells(
                document,
                &self.target,
                self.selected.ok_or("No cell selection")?,
            )?),
            TableAction::Split => Some(tables::split_table_cell(
                document,
                &self.target,
                self.cell.row,
                self.cell.cell_index,
                (0..self.cell.rowspan * self.cell.colspan - 1)
                    .map(|_| fresh_cell(false))
                    .collect(),
            )?),
            TableAction::DeleteTable => return Err("Table operation is unavailable".into()),
        };
        commands.extend(command);
        if matches!(action, TableAction::DeleteRow | TableAction::DeleteColumn) {
            let mut candidate =
                craft_editor_model::Editor::new(document.clone(), session.revision());
            candidate.apply(craft_editor_model::Transaction {
                base_revision: candidate.revision(),
                commands: commands.clone(),
            })?;
            let previous = session.selection().map(|selection| &selection.head);
            if let Some(point) =
                previous.filter(|point| candidate.document().block(&point.block_id).is_ok())
            {
                session.apply_commands_with_selection(
                    commands,
                    Selection {
                        anchor: point.clone(),
                        head: point.clone(),
                    },
                )?;
            } else {
                let table = candidate.document().node(&self.target.path)?;
                let grid = TableGrid::new(table)?;
                let row = selected.top.min(grid.rows - 1);
                let column = selected.left.min(grid.columns - 1);
                let cell = grid.cell_at(row, column).ok_or("Missing adjacent cell")?;
                let path = self.target.path.child(cell.row).child(cell.cell_index);
                let node = candidate.document().node(&path)?;
                if let Some(id) = first_text_identity(node) {
                    let point = DocPoint {
                        block_id: id.into(),
                        offset: ScalarOffset(0),
                    };
                    session.apply_commands_with_selection(
                        commands,
                        Selection {
                            anchor: point.clone(),
                            head: point,
                        },
                    )?;
                } else {
                    fn atom(node: &Node, path: NodePath) -> Option<NodePath> {
                        if matches!(
                            node.node_type.as_str(),
                            "imageBlock"
                                | "mermaidBlock"
                                | "mathBlock"
                                | "horizontalRule"
                                | "sceneBreak"
                        ) {
                            Some(path)
                        } else {
                            node.children()
                                .iter()
                                .enumerate()
                                .find_map(|(index, node)| atom(node, path.child(index)))
                        }
                    }
                    let path = atom(node, path).ok_or("Missing adjacent content")?;
                    let target = candidate.document().node_ref(&path)?;
                    session.apply_commands_with_node_selection(commands, target)?;
                }
            }
        } else if let Some(mut cells) = session.cell_selection().cloned() {
            if matches!(action, TableAction::Split) {
                cells.anchor = craft_editor_egui::CellPoint {
                    row: self.cell.row,
                    column: self.cell.column,
                };
                cells.head = craft_editor_egui::CellPoint {
                    row: self.cell.bottom() - 1,
                    column: self.cell.right() - 1,
                };
            } else if matches!(action, TableAction::Merge) {
                cells.anchor = craft_editor_egui::CellPoint {
                    row: selected.top,
                    column: selected.left,
                };
                cells.head = cells.anchor;
            }
            // The guarded table builder replaces its table node. The host knows the exact
            // post-selection; do not let an intentionally conservative path map discard it.
            session.apply_commands_with_cell_selection(commands, cells)?;
        } else {
            session.apply_commands(commands)?;
        }
        session.request_scroll_to_caret();
        Ok(())
    }
}

fn first_text_identity(node: &Node) -> Option<&str> {
    if matches!(
        node.node_type.as_str(),
        "paragraph" | "heading" | "codeBlock"
    ) {
        node.block_id()
    } else {
        node.children().iter().find_map(first_text_identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_editor_model::Command;
    use craft_editor_model::MarkMode;

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
    fn heading_selection_across_cells_is_one_undoable_edit() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let session = &mut app.rich.as_mut().ok_or("Missing session")?.session;
        session.set_selection(Selection {
            anchor: DocPoint {
                block_id: "right".into(),
                offset: ScalarOffset(1),
            },
            head: DocPoint {
                block_id: "bottom".into(),
                offset: ScalarOffset(8),
            },
        })?;
        let before = session.document().to_json()?;
        set_block_kind(session, 2)?;
        for id in ["right", "bottom"] {
            let block = session.document().block(id)?;
            assert_eq!(block.node_type, "heading");
            assert_eq!(
                block.attrs.as_ref().and_then(|attrs| attrs.get("level")),
                Some(&json!(2))
            );
        }
        session.undo()?;
        assert_eq!(session.document().to_json()?, before);
        Ok(())
    }

    #[test]
    fn inserted_table_receives_typing_target_and_redo_restores_it() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let session = &mut app.rich.as_mut().ok_or("Missing session")?.session;
        let point = DocPoint {
            block_id: "intro".into(),
            offset: ScalarOffset(3),
        };
        session.set_selection(Selection {
            anchor: point.clone(),
            head: point,
        })?;
        let before = session.document().to_json()?;
        insert_table(session)?;
        let target = session.selection().ok_or("Missing inserted caret")?.clone();
        assert_ne!(target.head.block_id, "intro");
        assert!(TableContext::current(session).is_some());
        session.undo()?;
        assert_eq!(session.document().to_json()?, before);
        session.redo()?;
        assert_eq!(session.selection(), Some(&target));
        Ok(())
    }

    #[test]
    fn deleting_active_row_keeps_next_typing_in_surviving_table_cell() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let session = &mut app.rich.as_mut().ok_or("Missing session")?.session;
        let point = DocPoint {
            block_id: "corner".into(),
            offset: ScalarOffset(4),
        };
        session.set_selection(Selection {
            anchor: point.clone(),
            head: point,
        })?;
        let before = session.selection().cloned();
        let context = TableContext::current(session).ok_or("Missing table")?;
        context.apply(session, TableAction::DeleteRow)?;
        let after = session.selection().cloned();
        assert_eq!(
            after
                .as_ref()
                .map(|selection| selection.head.block_id.as_str()),
            Some("bottom")
        );
        session.undo()?;
        assert_eq!(session.selection(), before.as_ref());
        session.redo()?;
        assert_eq!(session.selection(), after.as_ref());
        Ok(())
    }

    #[test]
    fn showcase_is_lossless_and_covers_all_original_nodes_and_marks() -> AppResult<()> {
        fn collect(
            node: &Node,
            nodes: &mut std::collections::BTreeSet<String>,
            marks: &mut std::collections::BTreeSet<String>,
        ) {
            nodes.insert(node.node_type.clone());
            for mark in node.marks.as_deref().unwrap_or_default() {
                marks.insert(mark.mark_type.clone());
            }
            for child in node.children() {
                collect(child, nodes, marks);
            }
        }
        let source: serde_json::Value = serde_json::from_str(include_str!("editor_showcase.json"))?;
        let document = RichDocument::from_json(source.clone())?;
        assert_eq!(document.to_json()?, source);
        let mut nodes = std::collections::BTreeSet::new();
        let mut marks = std::collections::BTreeSet::new();
        collect(document.root(), &mut nodes, &mut marks);
        assert_eq!(
            nodes,
            [
                "doc",
                "paragraph",
                "heading",
                "blockquote",
                "bulletList",
                "orderedList",
                "listItem",
                "hardBreak",
                "horizontalRule",
                "sceneBreak",
                "text",
                "codeBlock",
                "table",
                "tableRow",
                "tableHeader",
                "tableCell",
                "imageBlock",
                "mermaidBlock",
                "callout",
                "inlineMath",
                "mathBlock"
            ]
            .map(str::to_owned)
            .into_iter()
            .collect()
        );
        assert_eq!(
            marks,
            ["bold", "italic", "underline", "strike", "link", "code"]
                .map(str::to_owned)
                .into_iter()
                .collect()
        );
        Ok(())
    }

    #[test]
    fn complete_showcase_uses_real_renderers_without_source_fallbacks() -> AppResult<()> {
        let source: serde_json::Value = serde_json::from_str(include_str!("editor_showcase.json"))?;
        let document = RichDocument::from_json(source.clone())?;
        let mut assets = craft_editor_render::AssetMap::default();
        assets.insert(
            "showcase-lighthouse",
            std::sync::Arc::new(include_bytes!("../../../../assets/demo/lighthouse.svg").to_vec()),
        );
        let renderer =
            craft_editor_render::Renderer::new(craft_editor_render::RenderOptions::default())?;
        let catalog = renderer.prepare_document(&document, &assets, 1)?;
        assert_eq!(catalog.len(), 6);
        assert!(
            catalog.failures().is_empty(),
            "Valid showcase source must produce real media: {:?}",
            catalog.failures()
        );
        let geometry = craft_editor_egui::layout::FlowGeometry::with_resolver(
            &document,
            760.0,
            &craft_editor_egui::layout::GeometryStyle::default(),
            &catalog,
        )?;
        assert!(geometry.extent().is_some());
        assert_eq!(document.to_json()?, source);
        Ok(())
    }

    #[test]
    fn wide_table_fixture_retains_final_cell_and_nested_component_widths() -> AppResult<()> {
        let source: serde_json::Value =
            serde_json::from_str(include_str!("editor_wide_tables.json"))?;
        let document = RichDocument::from_json(source.clone())?;
        let wide = find_path(document.root(), "wide-table", NodePath::root())
            .ok_or("missing wide capture table")?;
        let inner = find_path(document.root(), "nested-inner", NodePath::root())
            .ok_or("missing nested capture table")?;
        let grid = TableGrid::new(document.node(&wide)?)?;
        assert_eq!(grid.columns, 63);
        assert_eq!(grid.column_widths.get(&62), Some(&80));
        assert_eq!(TableGrid::new(document.node(&inner)?)?.columns, 3);
        let mut geometry = craft_editor_egui::layout::FlowGeometry::new(&document, 550.0)?;
        for id in ["wide-cell-62", "nested-cell-2"] {
            let target = DocPoint {
                block_id: id.into(),
                offset: ScalarOffset(0),
            };
            geometry.reveal_caret(&target)?;
            assert!(geometry.caret(&target)?.is_some());
        }
        let viewports = geometry.table_viewports();
        assert!(
            viewports
                .iter()
                .any(|viewport| { viewport.table_path == wide && viewport.offset > 0.0 })
        );
        assert!(
            viewports
                .iter()
                .any(|viewport| { viewport.table_path == inner && viewport.offset > 0.0 })
        );
        assert_eq!(document.to_json()?, source);
        Ok(())
    }

    #[test]
    fn showcase_creation_is_idempotent_and_preserves_owner_edits() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let project = app.project.clone().ok_or("Missing project")?;
        let first = app.ensure_editor_showcase(&project)?;
        let owned = RichDocument::from_json(first.rich_json.clone().ok_or("Missing gallery")?)?;
        let snapshot = app.workspace.image_asset_snapshot(&project.id, &owned)?;
        assert!(
            snapshot.failures().is_empty(),
            "New gallery uses real owned image data"
        );
        let mut content = first.rich_json.clone().ok_or("Missing showcase source")?;
        content["content"][1]["content"][0]["text"] =
            serde_json::json!("The owner edited this demonstration.");
        let edited = app
            .workspace
            .save_rich_document(&first.id, first.revision, content)?;
        let again = app.ensure_editor_showcase(&project)?;
        assert_eq!(first.id, again.id);
        assert_eq!(edited.rich_json, again.rich_json);
        assert_eq!(app.workspace.list_documents(&project.id)?.len(), 3);
        Ok(())
    }

    #[test]
    fn autosave_replays_pending_native_edits_and_undo_restores_note_attachment() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let document = app.document.as_ref().ok_or("Missing document")?.clone();
        let anchor = app.workspace.capture_annotation_anchor(
            &document.id,
            document.revision,
            "intro",
            ScalarOffset(7),
            ScalarOffset(13),
            "native",
        )?;
        let note = app.workspace.create_annotation(
            &document.id,
            document.revision,
            anchor,
            "Keep this word.",
        )?;
        app.rich
            .as_mut()
            .ok_or("Missing session")?
            .session
            .apply_commands(vec![Command::ReplaceText {
                range: craft_editor_model::TextRange {
                    block_id: "intro".into(),
                    start: ScalarOffset(0),
                    end: ScalarOffset(0),
                },
                expected: String::new(),
                replacement: "New ".into(),
            }])?;
        app.dirty = Some(Instant::now());
        // Save runs before paint and must drain the native queue itself.
        app.save()?;
        let shifted = app.workspace.annotation(&note.id)?;
        assert_eq!(shifted.anchor.start, ScalarOffset(11));
        assert_eq!(shifted.anchor_status, studio_core::AnchorStatus::Attached);
        assert!(
            app.rich
                .as_ref()
                .ok_or("Missing session")?
                .pending_proofs
                .is_empty()
        );
        app.rich.as_mut().ok_or("Missing session")?.session.undo()?;
        app.dirty = Some(Instant::now());
        app.save()?;
        let restored = app.workspace.annotation(&note.id)?;
        assert_eq!(restored.anchor.start, ScalarOffset(7));
        assert_eq!(restored.note, "Keep this word.");
        assert_eq!(restored.anchor_status, studio_core::AnchorStatus::Attached);
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
    #[test]
    fn verified_draft_restart_recovers_marks_before_a_fresh_canvas_session_is_mounted()
    -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let database = app.database.clone();
        let document = app.document.as_ref().ok_or("Missing document")?.clone();
        let project = app.project.as_ref().ok_or("Missing project")?.clone();
        let anchor = app.workspace.capture_annotation_anchor(
            &document.id,
            document.revision,
            "intro",
            ScalarOffset(7),
            ScalarOffset(13),
            "native",
        )?;
        let note = app.workspace.create_annotation(
            &document.id,
            document.revision,
            anchor,
            "Keep this passage",
        )?;
        let rich = app.rich.as_mut().ok_or("Missing rich session")?;
        let point = DocPoint {
            block_id: "intro".into(),
            offset: ScalarOffset(0),
        };
        rich.session.set_selection(Selection {
            anchor: point.clone(),
            head: point,
        })?;
        rich.session.insert_plain_text("New ")?;
        rich.write_draft(&app.workspace, &document)?;
        let expected = rich.session.document().to_json()?;
        assert!(
            app.workspace
                .rich_draft(&document.id)?
                .ok_or("Missing durable draft")?
                .has_annotation_checkpoint
        );
        assert_eq!(app.workspace.document(&document.id)?, document);
        drop(app); // Simulate a stopped process: no canonical save, history or focus state survives.

        let context = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(context);
        let mut reopened = StudioApp::new(&creation, database, false, None)?;
        reopened.open_project(project)?;
        let current = reopened
            .document
            .as_ref()
            .ok_or("Missing reopened document")?;
        assert_eq!(current.id, document.id);
        assert_eq!(current.revision, document.revision + 1);
        assert_eq!(current.rich_json.as_ref(), Some(&expected));
        let recovered_note = reopened.workspace.annotation(&note.id)?;
        assert_eq!(
            recovered_note.anchor_status,
            studio_core::AnchorStatus::Attached
        );
        assert_eq!(recovered_note.anchor.start, ScalarOffset(11));
        assert_eq!(recovered_note.anchor.exact_text, "native");
        let rich = reopened.rich.as_ref().ok_or("Missing fresh session")?;
        assert!(!rich.recovered);
        assert!(!rich.session.can_undo());
        assert_eq!(rich.pending_proof_count(), 0);
        assert!(reopened.dirty.is_none());
        assert!(reopened.workspace.rich_draft(&document.id)?.is_none());
        assert_eq!(reopened.status, "Unsaved text recovered");
        Ok(())
    }
    #[test]
    fn recovery_conflict_preserves_previous_project_and_saved_view_refuses_writes() -> AppResult<()>
    {
        let (_directory, mut app) = fixture()?;
        let old_document = app
            .document
            .as_ref()
            .ok_or("Missing original document")?
            .clone();
        let old_project = app
            .project
            .as_ref()
            .ok_or("Missing original project")?
            .clone();
        let old_body = app.body.clone();
        let project = app.workspace.create_project("Conflicting recovery")?;
        let target = app.workspace.create_rich_document(
            &project.id,
            "Preserved writing",
            RichDocument::plain("p", "saved passage")?.to_json()?,
        )?;
        let mut editor = craft_editor_model::Editor::new(
            RichDocument::from_json(target.rich_json.clone().ok_or("Missing rich JSON")?)?,
            Revision(0),
        );
        let change = editor.apply(craft_editor_model::Transaction {
            base_revision: Revision(0),
            commands: vec![craft_editor_model::Command::ReplaceText {
                range: craft_editor_model::TextRange {
                    block_id: "p".into(),
                    start: ScalarOffset(0),
                    end: ScalarOffset(0),
                },
                expected: String::new(),
                replacement: "draft ".into(),
            }],
        })?;
        let draft = app.workspace.save_rich_editor_draft(
            &target.id,
            target.revision,
            None,
            &[change.replay_proof()],
            editor.document().to_json()?,
        )?;
        // A later author-created note makes the earlier attachment checkpoint stale.
        let anchor = app.workspace.capture_annotation_anchor(
            &target.id,
            target.revision,
            "p",
            ScalarOffset(6),
            ScalarOffset(13),
            "passage",
        )?;
        let note = app.workspace.create_annotation(
            &target.id,
            target.revision,
            anchor,
            "Keep this later note",
        )?;
        app.open_project(project)?;
        assert!(app.recovery.is_some());
        assert_eq!(
            app.document.as_ref().map(|document| &document.id),
            Some(&old_document.id)
        );
        assert_eq!(
            app.project.as_ref().map(|project| &project.id),
            Some(&old_project.id)
        );
        assert_eq!(app.body, old_body);
        assert_eq!(app.workspace.rich_draft(&target.id)?.as_ref(), Some(&draft));

        let snapshot = app.workspace.rich_recovery_snapshot(&target.id)?;
        app.open_saved_recovery(&snapshot)?;
        assert!(app.is_recovery_read_only());
        assert!(!app.can_edit_manuscript());
        assert_eq!(app.body, target.body);
        assert_eq!(
            app.document.as_ref().map(|document| &document.id),
            Some(&target.id)
        );
        let rich = app.rich.as_mut().ok_or("Missing read-only saved view")?;
        let point = DocPoint {
            block_id: "p".into(),
            offset: ScalarOffset(0),
        };
        rich.session.set_selection(Selection {
            anchor: point.clone(),
            head: point,
        })?;
        // Even a mistaken future dispatcher bypassing the canvas cannot publish a write.
        rich.session.insert_plain_text("must not save ")?;
        app.dirty = Some(Instant::now());
        let error = app
            .save()
            .err()
            .ok_or("A read-only view unexpectedly saved")?;
        assert!(matches!(
            error.downcast_ref::<studio_core::StudioError>(),
            Some(studio_core::StudioError::ReadOnly { .. })
        ));
        let rich = app.rich.as_mut().ok_or("Missing recovery editor")?;
        rich.session.undo()?;
        rich.session.set_selection(Selection {
            anchor: DocPoint {
                block_id: "p".into(),
                offset: ScalarOffset(6),
            },
            head: DocPoint {
                block_id: "p".into(),
                offset: ScalarOffset(13),
            },
        })?;
        app.dirty = None;
        let error = app
            .begin_note()
            .err()
            .ok_or("A read-only view unexpectedly created a note")?;
        assert!(error.is::<recovery::ReadOnlyRecoveryView>());
        assert!(app.recovery.is_none());
        assert_eq!(app.workspace.document(&target.id)?, target);
        assert_eq!(app.workspace.rich_draft(&target.id)?.as_ref(), Some(&draft));
        assert_eq!(app.workspace.annotation(&note.id)?, note);
        Ok(())
    }

    #[test]
    fn rectangular_native_selection_formats_once_and_restores_direction() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let session = &mut app.rich.as_mut().ok_or("Missing session")?.session;
        assert_eq!(session.table_column_limit(), Some(63));
        let path = TableContext::at(session, "right").ok_or("Missing table")?.0;
        let cells = craft_editor_egui::TableSelection {
            table_path: path,
            anchor: craft_editor_egui::CellPoint { row: 2, column: 2 },
            head: craft_editor_egui::CellPoint { row: 0, column: 0 },
        };
        session.set_cell_selection(cells.clone())?;
        let before = session.document().to_json()?;
        set_block_kind(session, 2)?;
        assert_eq!(session.cell_selection(), Some(&cells));
        let context = TableContext::current(session).ok_or("Missing selected table")?;
        assert_eq!(context.selected, Some(cells.rectangle(session.document())?));
        session.undo()?;
        assert_eq!(session.document().to_json()?, before);
        assert_eq!(session.cell_selection(), Some(&cells));
        session.redo()?;
        assert_eq!(session.cell_selection(), Some(&cells));
        Ok(())
    }

    fn rectangular_table_session() -> AppResult<EditorSession> {
        let rows = (0..3)
            .map(|row| {
                Node::block(
                    "tableRow",
                    Map::new(),
                    (0..3)
                        .map(|column| {
                            let mut cell =
                                tables::empty_table_cell(&format!("r{row}c{column}"), row == 0);
                            cell.content = Some(vec![Node::paragraph(
                                format!("r{row}c{column}"),
                                format!("Row {row}, column {column} 😀"),
                            )]);
                            cell.attrs
                                .get_or_insert_default()
                                .insert("preserve".into(), json!({"row":row,"column":column}));
                            cell
                        })
                        .collect(),
                )
            })
            .collect();
        let document = RichDocument::from_root(Node::block(
            "doc",
            Map::from_iter([("schemaVersion".into(), json!(4))]),
            vec![
                Node::paragraph("before", "Before"),
                Node::block(
                    "table",
                    Map::from_iter([("blockId".into(), json!("table"))]),
                    rows,
                ),
                Node::paragraph("after", "After"),
            ],
        ))?;
        let mut session = EditorSession::new(document, Revision(0));
        session.set_table_column_limit(Some(63))?;
        Ok(session)
    }

    #[test]
    fn table_row_and_column_actions_cover_the_entire_reverse_rectangle_atomically() -> AppResult<()>
    {
        for rows in [true, false] {
            let mut session = rectangular_table_session()?;
            let cells = craft_editor_egui::TableSelection {
                table_path: NodePath(vec![1]),
                anchor: craft_editor_egui::CellPoint { row: 2, column: 1 },
                head: craft_editor_egui::CellPoint { row: 1, column: 0 },
            };
            session.set_cell_selection(cells.clone())?;
            let before = session.document().to_json()?;
            let context = TableContext::current(&session).ok_or("Missing range")?;
            context.apply(
                &mut session,
                if rows {
                    TableAction::DeleteRow
                } else {
                    TableAction::DeleteColumn
                },
            )?;
            let after = session.document().to_json()?;
            let grid = TableGrid::new(session.document().node(&NodePath(vec![1]))?)?;
            assert_eq!(
                (grid.rows, grid.columns),
                if rows { (1, 3) } else { (3, 1) }
            );
            if rows {
                assert_eq!(
                    after["content"][1]["content"][0],
                    before["content"][1]["content"][0]
                );
            } else {
                for row in 0..3 {
                    assert_eq!(
                        after["content"][1]["content"][row]["content"][0],
                        before["content"][1]["content"][row]["content"][2]
                    );
                }
            }
            assert_eq!(session.revision(), Revision(1));
            assert!(session.selection().is_some());
            session.undo()?;
            assert_eq!(session.document().to_json()?, before);
            assert_eq!(session.cell_selection(), Some(&cells));
            assert!(!session.can_undo());
            session.redo()?;
            assert_eq!(session.document().to_json()?, after);
        }
        Ok(())
    }

    #[test]
    fn table_insertion_uses_the_rectangle_end_and_source_header_reference() -> AppResult<()> {
        for rows in [true, false] {
            let mut session = rectangular_table_session()?;
            let cells = craft_editor_egui::TableSelection {
                table_path: NodePath(vec![1]),
                anchor: craft_editor_egui::CellPoint { row: 1, column: 1 },
                head: craft_editor_egui::CellPoint { row: 0, column: 0 },
            };
            session.set_cell_selection(cells.clone())?;
            let before = session.document().to_json()?;
            TableContext::current(&session)
                .ok_or("Missing range")?
                .apply(
                    &mut session,
                    if rows {
                        TableAction::AddRow
                    } else {
                        TableAction::AddColumn
                    },
                )?;
            let after = session.document().to_json()?;
            if rows {
                assert_eq!(
                    after["content"][1]["content"][3],
                    before["content"][1]["content"][2]
                );
                assert!(
                    session
                        .document()
                        .node(&NodePath(vec![1, 2]))?
                        .children()
                        .iter()
                        .all(|cell| cell.node_type == "tableCell")
                );
            } else {
                for row in 0..3 {
                    assert_eq!(
                        after["content"][1]["content"][row]["content"][3],
                        before["content"][1]["content"][row]["content"][2]
                    );
                    assert_eq!(
                        after["content"][1]["content"][row]["content"][2]["type"],
                        if row == 0 { "tableHeader" } else { "tableCell" }
                    );
                }
            }
            assert_eq!(session.cell_selection(), Some(&cells));
            session.undo()?;
            assert_eq!(session.document().to_json()?, before);
            assert_eq!(session.cell_selection(), Some(&cells));
        }
        Ok(())
    }

    #[test]
    fn deleting_the_whole_table_is_explicit_and_restores_original_selection_on_undo()
    -> AppResult<()> {
        let mut session = rectangular_table_session()?;
        let cells = craft_editor_egui::TableSelection {
            table_path: NodePath(vec![1]),
            anchor: craft_editor_egui::CellPoint { row: 2, column: 2 },
            head: craft_editor_egui::CellPoint { row: 0, column: 0 },
        };
        session.set_cell_selection(cells.clone())?;
        let before = session.document().to_json()?;
        let context = TableContext::current(&session).ok_or("Missing table")?;
        for action in [TableAction::DeleteRow, TableAction::DeleteColumn] {
            assert!(!context.allows(action));
            assert!(context.apply(&mut session, action).is_err());
            assert_eq!(session.document().to_json()?, before);
            assert!(!session.can_undo());
        }
        context.apply(&mut session, TableAction::DeleteTable)?;
        assert_eq!(session.document().plain_text(), "Before\n\nAfter");
        assert_eq!(
            session
                .selection()
                .map(|selection| selection.head.block_id.as_str()),
            Some("after")
        );
        session.undo()?;
        assert_eq!(session.document().to_json()?, before);
        assert_eq!(session.cell_selection(), Some(&cells));
        assert!(!session.can_undo());
        Ok(())
    }

    #[test]
    fn table_commands_match_the_actual_frozen_source_corpus() -> AppResult<()> {
        fn shape(value: &mut serde_json::Value) {
            match value {
                serde_json::Value::Array(values) => values.iter_mut().for_each(shape),
                serde_json::Value::Object(object) => {
                    if let Some(attrs) = object
                        .get_mut("attrs")
                        .and_then(serde_json::Value::as_object_mut)
                    {
                        attrs.remove("blockId");
                        for key in ["colwidth", "align"] {
                            if attrs.get(key).is_some_and(serde_json::Value::is_null) {
                                attrs.remove(key);
                            }
                        }
                        for key in ["colspan", "rowspan"] {
                            if attrs.get(key) == Some(&json!(1)) {
                                attrs.remove(key);
                            }
                        }
                        if attrs.get("sourceRefs") == Some(&json!([])) {
                            attrs.remove("sourceRefs");
                        }
                    }
                    if object
                        .get("attrs")
                        .and_then(serde_json::Value::as_object)
                        .is_some_and(serde_json::Map::is_empty)
                    {
                        object.remove("attrs");
                    }
                    if object
                        .get("content")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(Vec::is_empty)
                    {
                        object.remove("content");
                    }
                    object.values_mut().for_each(shape);
                }
                _ => {}
            }
        }
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("table_commands_source.json"))?;
        let original = RichDocument::from_json(corpus["input"].clone())?;
        for case in corpus["cases"].as_array().ok_or("Missing table cases")? {
            let mut session = EditorSession::new(original.clone(), Revision(0));
            session.set_table_column_limit(Some(63))?;
            let endpoint = |key: &str| -> AppResult<craft_editor_egui::CellPoint> {
                let id = case[key].as_str().ok_or("Missing source endpoint")?;
                let path = find_path(original.root(), id, NodePath::root())
                    .ok_or("Missing source cell")?;
                let row = *path.0.get(1).ok_or("Missing row")?;
                let column = *path.0.get(2).ok_or("Missing column")?;
                Ok(craft_editor_egui::CellPoint { row, column })
            };
            let selected = craft_editor_egui::TableSelection {
                table_path: NodePath(vec![1]),
                anchor: endpoint("anchor")?,
                head: endpoint("head")?,
            };
            session.set_cell_selection(selected.clone())?;
            let action = match case["action"].as_str().ok_or("Missing source action")? {
                "addRow" => TableAction::AddRow,
                "addColumn" => TableAction::AddColumn,
                "deleteRow" => TableAction::DeleteRow,
                "deleteColumn" => TableAction::DeleteColumn,
                "deleteTable" => TableAction::DeleteTable,
                _ => return Err("Unknown source action".into()),
            };
            let context = TableContext::current(&session).ok_or("Missing source table")?;
            if case["handled"] == false {
                assert!(!context.allows(action));
                assert!(context.apply(&mut session, action).is_err());
                assert_eq!(session.document(), &original);
                assert!(!session.can_undo());
                continue;
            }
            context.apply(&mut session, action)?;
            let exact = session.document().to_json()?;
            let mut actual = exact.clone();
            let mut expected = case["output"].clone();
            shape(&mut actual);
            shape(&mut expected);
            assert_eq!(actual, expected, "{} / {}", case["name"], case["action"]);
            assert_eq!(session.revision(), Revision(1));
            session.undo()?;
            assert_eq!(session.document(), &original);
            assert_eq!(session.cell_selection(), Some(&selected));
            assert!(!session.can_undo());
            session.redo()?;
            assert_eq!(session.document().to_json()?, exact);
        }
        Ok(())
    }

    #[test]
    fn a_text_range_is_not_promoted_to_a_rectangular_table_command_target() -> AppResult<()> {
        let mut session = rectangular_table_session()?;
        session.set_selection(Selection {
            anchor: DocPoint {
                block_id: "r2c2".into(),
                offset: ScalarOffset(3),
            },
            head: DocPoint {
                block_id: "r0c0".into(),
                offset: ScalarOffset(2),
            },
        })?;
        let context = TableContext::current(&session).ok_or("Missing current cell")?;
        assert!(context.selected.is_none());
        assert!(!context.allows(TableAction::Merge));
        let before = session.document().to_json()?;
        context.apply(&mut session, TableAction::AddRow)?;
        assert_eq!(
            session.document().to_json()?["content"][1]["content"][2],
            before["content"][1]["content"][1]
        );
        session.undo()?;
        assert_eq!(session.document().to_json()?, before);
        assert!(session.cell_selection().is_none());
        Ok(())
    }
}
