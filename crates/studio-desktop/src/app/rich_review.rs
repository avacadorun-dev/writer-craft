//! Actual selection attachments and durable structured review; local tools are never called AI.
use super::*;
use craft_editor_egui::{DocPoint, EditorSession, Selection, canvas::CanvasStyle};
use craft_editor_model::{Document as RichTree, Revision, TextPoint};
use studio_core::{AssistantSelectionAttachment, RichProposal, RichProposalSummary};

#[derive(Default)]
pub(super) struct RichReviewState {
    attachment: Option<AssistantSelectionAttachment>,
    replacements: Vec<String>,
    reason: String,
    document: Option<(String, String)>,
    records: Vec<RichProposalSummary>,
    next_cursor: Option<i64>,
    selected: Option<std::sync::Arc<RichProposal>>,
    preview: Option<RichPreview>,
    excerpts: Option<(String, String)>,
    pub(super) full_open: bool,
    focus: bool,
}
struct RichPreview {
    before: EditorSession,
    after: EditorSession,
    before_media: rendering::RenderState,
    after_media: rendering::RenderState,
}
impl RichPreview {
    fn new(proposal: &RichProposal) -> AppResult<Self> {
        let mut before =
            EditorSession::new(RichTree::from_json(proposal.before.clone())?, Revision(0));
        let mut after =
            EditorSession::new(RichTree::from_json(proposal.after.clone())?, Revision(0));
        before.options.id = egui::Id::new(("rich-review-before", &proposal.summary.id));
        after.options.id = egui::Id::new(("rich-review-after", &proposal.summary.id));
        Ok(Self {
            before,
            after,
            before_media: rendering::RenderState::default(),
            after_media: rendering::RenderState::default(),
        })
    }
    fn show(&mut self, ui: &mut egui::Ui, host: rendering::RenderHost<'_>) -> AppResult<()> {
        let rendering::RenderHost {
            language,
            tokens,
            font_size,
            workspace,
            project_id: project,
            ..
        } = host;
        ui.columns(2, |columns| -> AppResult<()> {
            for (column, (label, session, media)) in columns.iter_mut().zip([
                ("Original text", &mut self.before, &mut self.before_media),
                ("Proposal", &mut self.after, &mut self.after_media),
            ]) {
                column.label(RichText::new(language.text(label)).strong());
                column.separator();
                if !media.update(
                    session.document(),
                    session.revision(),
                    rendering::RenderHost {
                        language,
                        tokens,
                        font_size,
                        pixel_scale: column.ctx().pixels_per_point(),
                        workspace,
                        project_id: project,
                    },
                )? {
                    column
                        .ctx()
                        .request_repaint_after(Duration::from_millis(50));
                }
                egui::ScrollArea::both()
                    .id_salt(session.options.id)
                    .max_height(420.0)
                    .auto_shrink([false, false])
                    .show(column, |ui| -> AppResult<()> {
                        let style = CanvasStyle {
                            foreground: tokens.text,
                            background: tokens.editor,
                            selection: tokens.accent,
                            caret: tokens.accent,
                            table_border: tokens.border,
                            table_header: tokens.panel,
                            font_size,
                            read_only: true,
                            ..CanvasStyle::default()
                        };
                        let output = session.show_canvas_with_sources(
                            ui,
                            &style,
                            Some(media.catalog()),
                            None,
                        );
                        if let Some(error) = output.errors.into_iter().next() {
                            return Err(error.into());
                        }
                        Ok(())
                    })
                    .inner?;
            }
            Ok(())
        })
    }
}
impl StudioApp {
    pub(super) fn capture_review_selection(&mut self) -> AppResult<()> {
        self.commit_title_edit()?;
        let selection = self
            .rich
            .as_ref()
            .and_then(|rich| rich.session.selection().cloned())
            .ok_or("Select a passage for review.")?;
        self.save()?;
        let document = self
            .document
            .as_ref()
            .ok_or("Open a document for review.")?;
        let point = |point: DocPoint| TextPoint {
            block_id: point.block_id,
            offset: point.offset,
        };
        let attachment = self.workspace.capture_assistant_selection(
            &document.project_id,
            &document.id,
            document.revision,
            &point(selection.anchor),
            &point(selection.head),
        )?;
        self.rich_review.replacements = attachment
            .ranges
            .iter()
            .map(|range| range.expected_text.clone())
            .collect();
        self.rich_review.attachment = Some(attachment);
        self.rich_review.focus = true;
        self.inspector = Inspector::Review;
        self.inspector_visible = true;
        self.prefer_inspector = true;
        self.focus_mode = false;
        Ok(())
    }
    fn reveal_review_selection(&mut self) -> AppResult<()> {
        let attachment = self
            .rich_review
            .attachment
            .clone()
            .ok_or("Select a passage for review.")?;
        let project = self
            .project
            .as_ref()
            .ok_or("Open the selection's project before revealing it.")?;
        let (from, to) = self
            .workspace
            .assistant_selection_boundaries(&project.id, &attachment)?;
        if self
            .document
            .as_ref()
            .is_none_or(|document| document.id != attachment.document_id)
        {
            self.open_document(self.workspace.document(&attachment.document_id)?)?;
        }
        let session = &mut self
            .rich
            .as_mut()
            .ok_or("Open a structured document for review.")?
            .session;
        let point = |point: TextPoint| DocPoint {
            block_id: point.block_id,
            offset: point.offset,
        };
        session.set_selection(Selection {
            anchor: point(from),
            head: point(to),
        })?;
        session.request_scroll_to_caret();
        self.focus_manuscript = true;
        Ok(())
    }
    fn load_rich_proposal(&mut self, id: &str) -> AppResult<()> {
        let project = self.project.as_ref().ok_or("Open a project for review.")?;
        let proposal = self.workspace.rich_proposal(&project.id, id)?;
        let preview = RichPreview::new(&proposal)?;
        self.rich_review.excerpts = Some((
            excerpt(&preview.before.document().plain_text(), 340),
            excerpt(&preview.after.document().plain_text(), 340),
        ));
        self.rich_review.selected = Some(std::sync::Arc::new(proposal));
        self.rich_review.preview = Some(preview);
        Ok(())
    }
    fn refresh_rich_proposals(&mut self) -> AppResult<()> {
        let document = self
            .document
            .as_ref()
            .ok_or("Open a document for review.")?;
        let rows = self
            .workspace
            .rich_proposals(&document.project_id, &document.id, None, 16)?;
        self.rich_review.document = Some((document.project_id.clone(), document.id.clone()));
        self.rich_review.next_cursor = (rows.len() == 16)
            .then(|| rows.last().map(|row| row.cursor))
            .flatten();
        self.rich_review.records = rows;
        Ok(())
    }
    fn stage_rich_selected_edit(&mut self) -> AppResult<()> {
        self.save()?;
        let project = self.project.as_ref().ok_or("Open a project for review.")?;
        let attachment = self
            .rich_review
            .attachment
            .as_ref()
            .ok_or("Select a passage for review.")?;
        let reason = if self.rich_review.reason.trim().is_empty() {
            self.language.text("Local author edit")
        } else {
            &self.rich_review.reason
        };
        let proposal = self.workspace.stage_assistant_selection_replacements(
            &project.id,
            attachment,
            &self.rich_review.replacements,
            reason,
        )?;
        self.refresh_rich_proposals()?;
        self.load_rich_proposal(&proposal.summary.id)?;
        self.status = "Proposal prepared; manuscript unchanged".into();
        Ok(())
    }
    fn act_rich_proposal(&mut self, id: &str, action: ProposalStatus) -> AppResult<()> {
        self.commit_title_edit()?;
        self.save()?;
        let project = self
            .project
            .as_ref()
            .ok_or("Open a project for review.")?
            .id
            .clone();
        match action {
            ProposalStatus::Applied => {
                let document = self.workspace.apply_rich_proposal(&project, id)?;
                self.open_document(document)?;
                self.status = "Changes applied; undo is available".into();
            }
            ProposalStatus::Undone => {
                let document = self.workspace.undo_rich_proposal(&project, id)?;
                self.open_document(document)?;
                self.status = "Apply undone".into();
            }
            ProposalStatus::Rejected => {
                self.workspace.reject_rich_proposal(&project, id)?;
                self.status = "Proposal rejected".into();
            }
            ProposalStatus::Staged => return Err("Choose an explicit review action".into()),
        }
        self.refresh_rich_proposals()?;
        self.load_rich_proposal(id)?;
        Ok(())
    }
    pub(super) fn rich_review_panel(&mut self, ui: &mut egui::Ui, t: Tokens) {
        let l = self.language;
        let current = self
            .document
            .as_ref()
            .map(|document| (document.project_id.clone(), document.id.clone()));
        if self.rich_review.document != current {
            self.rich_review.selected = None;
            self.rich_review.preview = None;
            let result = self.refresh_rich_proposals();
            self.record_error(result);
        }
        ui.label(
            RichText::new(l.text("You are in control"))
                .size(17.0)
                .strong(),
        );
        ui.label(RichText::new(l.text("Review the proposal before changing your manuscript. The original stays unchanged until you apply it.")).size(13.0).color(t.muted));
        ui.add_space(12.0);
        if components::action(
            ui,
            l.text("Prepare an edit from selection"),
            self.rich.is_some(),
            None,
        )
        .clicked()
        {
            let result = self.capture_review_selection();
            self.record_error(result);
        }
        if let Some(attachment) = self.rich_review.attachment.clone() {
            let stale = self.dirty.is_some()
                || self.document.as_ref().is_none_or(|document| {
                    document.id != attachment.document_id
                        || document.revision != attachment.base_revision
                        || document.project_id != attachment.project_id
                });
            ui.add_space(12.0);
            egui::Frame::new()
                .fill(t.accent_soft)
                .inner_margin(12)
                .corner_radius(8)
                .show(ui, |ui| {
                    if let Ok(document) = self.workspace.document(&attachment.document_id) {
                        ui.label(RichText::new(document.title).strong());
                    }
                    ui.label(l.format(
                        "Revision {revision}",
                        &[("revision", attachment.base_revision.to_string())],
                    ));
                    ui.label(RichText::new(excerpt(&attachment.quote, 340)).size(13.0));
                    if stale {
                        ui.colored_label(
                            t.warning,
                            l.text("Outdated proposal — the text has changed"),
                        );
                    }
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .add_enabled(!stale, egui::Button::new(l.text("Reveal selection")))
                            .clicked()
                        {
                            let result = self.reveal_review_selection();
                            self.record_error(result);
                        }
                        if icons::icon_button(ui, Icon::Close, l.text("Clear selection scope"))
                            .clicked()
                        {
                            self.rich_review.attachment = None;
                            self.rich_review.replacements.clear();
                        }
                    });
                });
            if self.rich_review.attachment.is_some() {
                for (index, (range, replacement)) in attachment
                    .ranges
                    .iter()
                    .zip(&mut self.rich_review.replacements)
                    .enumerate()
                {
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(l.text("Original excerpt"))
                            .size(12.0)
                            .strong(),
                    );
                    diff_block(ui, "", &range.expected_text, t.surface, t);
                    ui.label(l.text("Proposed text"));
                    let response = ui.add(
                        egui::TextEdit::multiline(replacement)
                            .id_salt(("rich-proposal-wording", index))
                            .desired_rows(3)
                            .desired_width(f32::INFINITY)
                            .margin(12),
                    );
                    if self.rich_review.focus && index == 0 {
                        response.request_focus();
                        self.rich_review.focus = false;
                    }
                }
                ui.add_space(10.0);
                components::singleline(
                    ui,
                    &mut self.rich_review.reason,
                    l.text("Reason for the edit (optional)"),
                );
                let changed = attachment
                    .ranges
                    .iter()
                    .zip(&self.rich_review.replacements)
                    .any(|(range, text)| range.expected_text != *text);
                ui.add_space(12.0);
                if components::action(ui, l.text("Prepare proposal"), !stale && changed, Some(t))
                    .clicked()
                {
                    let result = self.stage_rich_selected_edit();
                    self.record_error(result);
                }
            }
        }
        ui.add_space(10.0);
        ui.label(
            RichText::new(l.text("Local editing tool. No AI provider is connected yet."))
                .size(11.5)
                .color(t.muted),
        );
        ui.add_space(18.0);
        ui.separator();
        ui.add_space(12.0);
        for row in self.rich_review.records.clone() {
            let selected = self
                .rich_review
                .selected
                .as_ref()
                .is_some_and(|proposal| proposal.summary.id == row.id);
            let label = if row.reason.is_empty() {
                l.text("Proposal").into()
            } else {
                row.reason.clone()
            };
            if ui.selectable_label(selected, label).clicked() {
                let result = self.load_rich_proposal(&row.id);
                self.record_error(result);
            }
        }
        if let Some(cursor) = self.rich_review.next_cursor
            && ui.button(l.text("Load more proposals")).clicked()
        {
            let result = (|| -> AppResult<()> {
                let document = self
                    .document
                    .as_ref()
                    .ok_or("Open a document for review.")?;
                let rows = self.workspace.rich_proposals(
                    &document.project_id,
                    &document.id,
                    Some(cursor),
                    16,
                )?;
                self.rich_review.next_cursor = if rows.len() == 16 {
                    rows.last().map(|row| row.cursor)
                } else {
                    None
                };
                self.rich_review.records.extend(rows);
                Ok(())
            })();
            self.record_error(result);
        }
        if let Some(proposal) = self.rich_review.selected.clone() {
            ui.add_space(12.0);
            if let Some((before, after)) = &self.rich_review.excerpts {
                diff_block(ui, l.text("Original text"), before, t.surface, t);
                ui.add_space(8.0);
                diff_block(ui, l.text("Proposal"), after, t.accent_soft, t);
            }
            ui.add_space(10.0);
            if components::action(ui, l.text("Review all changes"), true, None).clicked() {
                self.rich_review.full_open = true;
            }
            self.rich_proposal_actions(ui, &proposal, t);
        }
    }
    fn rich_proposal_actions(&mut self, ui: &mut egui::Ui, proposal: &RichProposal, t: Tokens) {
        let l = self.language;
        let same = self.dirty.is_none()
            && self.document.as_ref().is_some_and(|document| {
                document.id == proposal.summary.document_id
                    && document.rich_json.as_ref()
                        == Some(if proposal.summary.status == ProposalStatus::Staged {
                            &proposal.before
                        } else {
                            &proposal.after
                        })
                    && document.revision
                        == if proposal.summary.status == ProposalStatus::Staged {
                            proposal.summary.base_revision
                        } else {
                            proposal.summary.applied_revision.unwrap_or(-1)
                        }
            });
        ui.add_space(10.0);
        match proposal.summary.status {
            ProposalStatus::Staged => {
                if !same {
                    ui.colored_label(
                        t.warning,
                        l.text("Outdated proposal — the text has changed"),
                    );
                }
                if components::action(ui, l.text("Apply changes"), same, Some(t)).clicked() {
                    let result =
                        self.act_rich_proposal(&proposal.summary.id, ProposalStatus::Applied);
                    self.record_error(result);
                }
                if components::action(ui, l.text("Reject"), true, None).clicked() {
                    let result =
                        self.act_rich_proposal(&proposal.summary.id, ProposalStatus::Rejected);
                    self.record_error(result);
                }
            }
            ProposalStatus::Applied => {
                ui.colored_label(t.good, l.text("Changes applied"));
                if components::action(ui, l.text("Undo apply"), same, None).clicked() {
                    let result =
                        self.act_rich_proposal(&proposal.summary.id, ProposalStatus::Undone);
                    self.record_error(result);
                }
            }
            ProposalStatus::Rejected => {
                ui.label(l.text("Proposal rejected"));
            }
            ProposalStatus::Undone => {
                ui.label(l.text("Apply undone"));
            }
        }
    }
    pub(super) fn rich_review_dialog(&mut self, ctx: &egui::Context, t: Tokens) {
        if !self.rich_review.full_open {
            return;
        }
        let Some(proposal) = self.rich_review.selected.clone() else {
            self.rich_review.full_open = false;
            return;
        };
        let mut preview = self.rich_review.preview.take();
        let response = modal(
            ctx,
            self.language,
            "rich-full-review",
            self.language.text("Review all changes"),
            940.0,
            |ui| {
                if let Some(preview) = &mut preview {
                    let result = preview.show(
                        ui,
                        rendering::RenderHost {
                            workspace: &self.workspace,
                            project_id: self
                                .project
                                .as_ref()
                                .map_or("", |project| project.id.as_str()),
                            language: self.language,
                            tokens: t,
                            font_size: self.font_size,
                            pixel_scale: ui.ctx().pixels_per_point(),
                        },
                    );
                    self.record_error(result);
                }
                self.rich_proposal_actions(ui, &proposal, t);
            },
        );
        if self.rich_review.preview.is_none() {
            self.rich_review.preview = preview;
        }
        if response.should_close() {
            self.rich_review.full_open = false;
        }
    }
}
fn excerpt(text: &str, limit: usize) -> String {
    let mut chars = text.chars();
    let mut output: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() {
        output.push('…');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_editor_model::ScalarOffset;

    fn fixture() -> AppResult<(tempfile::TempDir, StudioApp, Document, Document)> {
        let directory = tempfile::tempdir()?;
        let creation = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = StudioApp::new(
            &creation,
            directory.path().join("review.sqlite"),
            false,
            None,
        )?;
        let project = app.workspace.create_project("Synthetic review")?;
        let first = app.workspace.create_rich_document(
            &project.id,
            "Repeated passage",
            RichTree::plain("p", "Я😀 echo echo")?.to_json()?,
        )?;
        let second = app.workspace.create_rich_document(
            &project.id,
            "Another document",
            RichTree::plain("q", "Different writing")?.to_json()?,
        )?;
        app.open_project(project)?;
        app.open_document(first.clone())?;
        app.rich
            .as_mut()
            .ok_or("missing rich session")?
            .session
            .set_selection(Selection {
                anchor: DocPoint {
                    block_id: "p".into(),
                    offset: ScalarOffset(8),
                },
                head: DocPoint {
                    block_id: "p".into(),
                    offset: ScalarOffset(12),
                },
            })?;
        Ok((directory, app, first, second))
    }

    #[test]
    fn frozen_review_selection_should_reveal_its_exact_document_after_navigation() -> AppResult<()>
    {
        let (_directory, mut app, first, second) = fixture()?;
        app.capture_review_selection()?;
        let attachment = app
            .rich_review
            .attachment
            .clone()
            .ok_or("missing attachment")?;
        assert_eq!(attachment.ranges[0].start, ScalarOffset(8));
        app.open_document(second)?;
        assert_eq!(app.rich_review.attachment.as_ref(), Some(&attachment));
        app.reveal_review_selection()?;
        assert_eq!(
            app.document.as_ref().map(|document| document.id.as_str()),
            Some(first.id.as_str())
        );
        let selection = app
            .rich
            .as_ref()
            .and_then(|rich| rich.session.selection())
            .ok_or("missing revealed selection")?;
        assert_eq!(
            (selection.anchor.offset, selection.head.offset),
            (ScalarOffset(8), ScalarOffset(12))
        );
        assert_eq!(app.workspace.document(&first.id)?, first);
        Ok(())
    }

    #[test]
    fn native_review_controller_should_stage_apply_and_undo_without_flattening_or_early_save()
    -> AppResult<()> {
        let (_directory, mut app, first, _) = fixture()?;
        app.capture_review_selection()?;
        app.rich_review.replacements[0] = "changed".into();
        app.stage_rich_selected_edit()?;
        let proposal = app.rich_review.selected.clone().ok_or("missing proposal")?;
        assert_eq!(app.workspace.document(&first.id)?, first);
        app.act_rich_proposal(&proposal.summary.id, ProposalStatus::Applied)?;
        assert_eq!(app.body, "Я😀 echo changed");
        assert_eq!(
            app.rich
                .as_ref()
                .ok_or("missing applied native session")?
                .session
                .document()
                .to_json()?,
            app.document
                .as_ref()
                .and_then(|document| document.rich_json.clone())
                .ok_or("missing applied canonical tree")?
        );
        app.act_rich_proposal(&proposal.summary.id, ProposalStatus::Undone)?;
        assert_eq!(
            app.document
                .as_ref()
                .and_then(|document| document.rich_json.as_ref()),
            first.rich_json.as_ref()
        );
        assert_eq!(app.workspace.rich_revisions(&first.id)?.len(), 3);
        Ok(())
    }

    #[test]
    fn stale_selection_should_refuse_after_a_formatting_only_save_and_keep_proposed_wording()
    -> AppResult<()> {
        let (_directory, mut app, first, _) = fixture()?;
        app.capture_review_selection()?;
        app.rich_review.replacements[0] = "Preserved proposed wording".into();
        let mut changed = first.rich_json.clone().ok_or("missing original tree")?;
        changed["content"][0]["content"][0]["marks"] = serde_json::json!([{"type":"italic"}]);
        let formatted = app.workspace.save_rich_document(&first.id, 1, changed)?;
        app.open_document(formatted.clone())?;
        assert!(app.stage_rich_selected_edit().is_err());
        assert_eq!(
            app.rich_review.replacements[0],
            "Preserved proposed wording"
        );
        assert_eq!(app.workspace.document(&first.id)?, formatted);
        assert!(
            app.workspace
                .rich_proposals(&first.project_id, &first.id, None, 16)?
                .is_empty()
        );
        Ok(())
    }
}
