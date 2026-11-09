//! Actual selection attachments and durable structured review; local tools are never called AI.
use super::*;
use craft_editor_egui::{DocPoint, EditorSession, Selection, canvas::CanvasStyle};
use craft_editor_model::{Document as RichTree, Revision, TextPoint};
use studio_core::{AssistantSelectionAttachment, RichProposal, RichProposalSummary};

#[cfg(test)]
mod input_tests;

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
    markup_id: Option<String>,
    markup_revision: Option<i64>,
    queue: Option<MarkupQueue>,
    parked: Vec<ReviewDraft>,
    prepared: Option<PreparedEdit>,
    paused: bool,
}
#[derive(Clone)]
struct PreparedEdit {
    attachment: AssistantSelectionAttachment,
    replacements: Vec<String>,
    reason: String,
    proposal: std::sync::Arc<RichProposal>,
}
struct ReviewDraft {
    attachment: AssistantSelectionAttachment,
    replacements: Vec<String>,
    reason: String,
    markup_id: Option<String>,
    markup_revision: Option<i64>,
    prepared: Option<PreparedEdit>,
}
#[derive(Clone)]
struct MarkupQueue {
    project_id: String,
    ids: Vec<String>,
    index: usize,
    unavailable: std::collections::HashSet<String>,
    skipped: usize,
}
impl MarkupQueue {
    fn has_neighbor(&self, forward: bool) -> bool {
        self.ids.iter().enumerate().any(|(index, id)| {
            (if forward {
                index > self.index
            } else {
                index < self.index
            }) && !self.unavailable.contains(id)
        })
    }
}
impl RichReviewState {
    pub(super) fn has_unsaved_drafts(&self) -> bool {
        self.attachment.as_ref().is_some_and(|attachment| {
            review_draft_needs_save(
                attachment,
                &self.replacements,
                &self.reason,
                self.prepared.as_ref(),
            )
        }) || self.parked.iter().any(|draft| {
            review_draft_needs_save(
                &draft.attachment,
                &draft.replacements,
                &draft.reason,
                draft.prepared.as_ref(),
            )
        })
    }
    fn changed(&self) -> bool {
        self.attachment.as_ref().is_some_and(|attachment| {
            attachment
                .ranges
                .iter()
                .zip(&self.replacements)
                .any(|(range, text)| &range.expected_text != text)
        })
    }
    fn park(&mut self) {
        let retain = self.changed() || !self.reason.is_empty() || self.prepared.is_some();
        if let Some(attachment) = self.attachment.take() {
            let draft = ReviewDraft {
                attachment,
                replacements: std::mem::take(&mut self.replacements),
                reason: std::mem::take(&mut self.reason),
                markup_id: self.markup_id.take(),
                markup_revision: self.markup_revision.take(),
                prepared: self.prepared.take(),
            };
            if retain {
                self.parked.push(draft);
            }
        }
        self.paused = false;
    }
    fn install(
        &mut self,
        attachment: AssistantSelectionAttachment,
        markup_id: Option<String>,
        markup_revision: Option<i64>,
    ) {
        if self.attachment.as_ref() == Some(&attachment)
            && self.markup_id == markup_id
            && self.markup_revision == markup_revision
        {
            self.paused = false;
            self.focus = true;
            return;
        }
        self.park();
        let saved = self.parked.iter().rposition(|draft| {
            if let Some(id) = &markup_id {
                draft.markup_id.as_ref() == Some(id)
                    && draft.attachment.project_id == attachment.project_id
                    && draft.attachment.document_id == attachment.document_id
            } else {
                draft.markup_id.is_none() && draft.attachment == attachment
            }
        });
        if let Some(index) = saved {
            self.resume(index);
        } else {
            self.replacements = attachment
                .ranges
                .iter()
                .map(|range| range.expected_text.clone())
                .collect();
            self.attachment = Some(attachment);
            self.markup_id = markup_id;
            self.markup_revision = markup_revision;
            self.prepared = None;
        }
        self.focus = true;
    }
    fn resume(&mut self, index: usize) {
        let draft = self.parked.remove(index);
        self.attachment = Some(draft.attachment);
        self.replacements = draft.replacements;
        self.reason = draft.reason;
        self.markup_id = draft.markup_id;
        self.markup_revision = draft.markup_revision;
        self.prepared = draft.prepared;
        self.paused = false;
        self.focus = true;
    }
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
                            table_scroll_label: language.text("Scroll table horizontally").into(),
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
    pub(super) fn save_review_drafts(&mut self) -> AppResult<()> {
        self.save()?;
        self.rich_review.park();
        for draft in &mut self.rich_review.parked {
            if !review_draft_needs_save(
                &draft.attachment,
                &draft.replacements,
                &draft.reason,
                draft.prepared.as_ref(),
            ) {
                continue;
            }
            if !draft
                .attachment
                .ranges
                .iter()
                .zip(&draft.replacements)
                .any(|(range, text)| &range.expected_text != text)
            {
                return Err("Write a replacement before saving this draft, or discard it.".into());
            }
            let reason = if draft.reason.trim().is_empty() {
                self.language.text("Local author edit")
            } else {
                &draft.reason
            };
            let proposal = self.workspace.stage_assistant_selection_replacements(
                &draft.attachment.project_id,
                &draft.attachment,
                &draft.replacements,
                reason,
            )?;
            draft.prepared = Some(PreparedEdit {
                attachment: draft.attachment.clone(),
                replacements: draft.replacements.clone(),
                reason: draft.reason.clone(),
                proposal: std::sync::Arc::new(proposal),
            });
        }
        Ok(())
    }
    pub(super) fn discard_review_drafts(&mut self) {
        self.rich_review.attachment = None;
        self.rich_review.replacements.clear();
        self.rich_review.reason.clear();
        self.rich_review.markup_id = None;
        self.rich_review.markup_revision = None;
        self.rich_review.prepared = None;
        self.rich_review.parked.clear();
        self.rich_review.paused = false;
    }
    fn captured_review_selection(&mut self) -> AppResult<AssistantSelectionAttachment> {
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
        Ok(attachment)
    }
    pub(super) fn capture_review_selection(&mut self) -> AppResult<()> {
        let attachment = self.captured_review_selection()?;
        self.rich_review.install(attachment, None, None);
        self.rich_review.queue = None;
        self.show_inline_review();
        Ok(())
    }
    fn show_inline_review(&mut self) {
        self.inspector = Inspector::Review;
        self.inspector_visible = true;
        self.prefer_inspector = true;
        self.focus_mode = false;
        self.focus_manuscript = false;
    }
    pub(super) fn begin_markup_review(
        &mut self,
        ids: Vec<String>,
        selected_id: &str,
    ) -> AppResult<()> {
        let project_id = self
            .project
            .as_ref()
            .ok_or("Open a project for review.")?
            .id
            .clone();
        let index = ids
            .iter()
            .position(|id| id == selected_id)
            .ok_or("This marked passage is unavailable. Restore or reattach it first.")?;
        let queue = MarkupQueue {
            project_id,
            ids,
            index,
            unavailable: Default::default(),
            skipped: 0,
        };
        self.open_markup_review_target(&queue)?;
        self.rich_review.queue = Some(queue);
        Ok(())
    }
    fn open_markup_review_target(&mut self, queue: &MarkupQueue) -> AppResult<()> {
        if self
            .project
            .as_ref()
            .is_none_or(|project| project.id != queue.project_id)
        {
            return Err("Open the selection's project before revealing it.".into());
        }
        self.save()?;
        let id = queue
            .ids
            .get(queue.index)
            .ok_or("This marked passage is unavailable. Restore or reattach it first.")?;
        let row = self
            .workspace
            .project_annotations(&queue.project_id, false)?
            .into_iter()
            .find(|row| &row.annotation.id == id)
            .ok_or("This marked passage is unavailable. Restore or reattach it first.")?;
        if row.annotation.anchor_status != studio_core::AnchorStatus::Attached
            || !project_tree::document_is_active(
                &self.workspace.project_tree(&queue.project_id)?,
                &row.annotation.document_id,
            )
        {
            return Err("This marked passage is unavailable. Restore or reattach it first.".into());
        }
        if self
            .document
            .as_ref()
            .is_none_or(|document| document.id != row.annotation.document_id)
        {
            self.begin_reference_excursion(self.workspace.document(&row.annotation.document_id)?)?;
        }
        if self.is_recovery_read_only()
            || self
                .document
                .as_ref()
                .is_none_or(|document| document.id != row.annotation.document_id)
        {
            return Err("This marked passage is unavailable. Restore or reattach it first.".into());
        }
        let note = self.select_markup_for_review(id)?;
        let attachment = self.captured_review_selection()?;
        if attachment.document_id != note.document_id
            || self.workspace.annotation(id)?.revision != note.revision
        {
            return Err(
                "This marked passage changed. Refresh the review before applying it.".into(),
            );
        }
        self.rich_review
            .install(attachment, Some(id.clone()), Some(note.revision));
        self.show_inline_review();
        Ok(())
    }
    fn step_markup_review(&mut self, forward: bool) -> AppResult<()> {
        let mut queue = self
            .rich_review
            .queue
            .clone()
            .ok_or("Select a passage for review.")?;
        if self
            .project
            .as_ref()
            .is_none_or(|project| project.id != queue.project_id)
        {
            return Err("Open the selection's project before revealing it.".into());
        }
        self.save()?;
        let records = self
            .workspace
            .project_annotations(&queue.project_id, false)?;
        let tree = self.workspace.project_tree(&queue.project_id)?;
        let mut position = queue.index;
        queue.skipped = 0;
        loop {
            let next = if forward {
                position.checked_add(1)
            } else {
                position.checked_sub(1)
            };
            let Some(index) = next.filter(|index| *index < queue.ids.len()) else {
                self.rich_review.queue = Some(queue);
                return Ok(());
            };
            position = index;
            let id = &queue.ids[index];
            let available = records.iter().any(|row| {
                &row.annotation.id == id
                    && row.annotation.anchor_status == studio_core::AnchorStatus::Attached
                    && project_tree::document_is_active(&tree, &row.annotation.document_id)
            });
            if !available {
                queue.unavailable.insert(id.clone());
                queue.skipped += 1;
                continue;
            }
            queue.unavailable.remove(id);
            queue.index = index;
            self.open_markup_review_target(&queue)?;
            self.rich_review.queue = Some(queue);
            return Ok(());
        }
    }
    fn reveal_review_selection(&mut self) -> AppResult<()> {
        let attachment = self
            .rich_review
            .attachment
            .clone()
            .ok_or("Select a passage for review.")?;
        self.commit_title_edit()?;
        self.save()?;
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
        if self.is_recovery_read_only()
            || self.dirty.is_some()
            || self.document.as_ref().is_none_or(|document| {
                document.id != attachment.document_id
                    || document.revision != attachment.base_revision
            })
        {
            return Err("This passage cannot be revealed safely. Save or restore the document, then select it again.".into());
        }
        let session = &mut self
            .rich
            .as_mut()
            .ok_or("Open a structured document for review.")?
            .session;
        if self
            .document
            .as_ref()
            .and_then(|document| document.rich_json.as_ref())
            != Some(&session.document().to_json()?)
        {
            return Err("This passage cannot be revealed safely. Save or restore the document, then select it again.".into());
        }
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
        self.rich_review.excerpts = review_excerpts(&proposal);
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
        if let Some(prepared) = &self.rich_review.prepared
            && prepared.attachment == *attachment
            && prepared.replacements == self.rich_review.replacements
            && prepared.reason == self.rich_review.reason
        {
            self.rich_review.selected = Some(prepared.proposal.clone());
            return Ok(());
        }
        let reason = if self.rich_review.reason.trim().is_empty() {
            self.language.text("Local author edit")
        } else {
            &self.rich_review.reason
        };
        let proposal = std::sync::Arc::new(self.workspace.stage_assistant_selection_replacements(
            &project.id,
            attachment,
            &self.rich_review.replacements,
            reason,
        )?);
        // The durable receipt precedes any fallible presentation refresh, so retry cannot stage twice.
        self.rich_review.prepared = Some(PreparedEdit {
            attachment: attachment.clone(),
            replacements: self.rich_review.replacements.clone(),
            reason: self.rich_review.reason.clone(),
            proposal: proposal.clone(),
        });
        self.rich_review.selected = Some(proposal.clone());
        self.rich_review.excerpts = review_excerpts(&proposal);
        self.rich_review.preview = Some(RichPreview::new(&proposal)?);
        self.refresh_rich_proposals()?;
        self.status = "Proposal prepared; manuscript unchanged".into();
        Ok(())
    }
    fn apply_inline_review(&mut self, advance: bool) -> AppResult<()> {
        if let Some(id) = &self.rich_review.markup_id
            && self.rich_review.markup_revision != Some(self.workspace.annotation(id)?.revision)
        {
            return Err(
                "This marked passage changed. Refresh the review before applying it.".into(),
            );
        }
        self.stage_rich_selected_edit()?;
        let id = self
            .rich_review
            .prepared
            .as_ref()
            .ok_or("Select a passage for review.")?
            .proposal
            .summary
            .id
            .clone();
        self.act_rich_proposal(&id, ProposalStatus::Applied)?;
        if advance {
            self.step_markup_review(true)?;
        }
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
        let mut receipt = self.workspace.rich_proposal(&project, id)?;
        let document = match action {
            ProposalStatus::Applied => {
                let document = self.workspace.apply_rich_proposal(&project, id)?;
                receipt.summary.applied_revision = Some(document.revision);
                self.status = "Changes applied; undo is available".into();
                Some(document)
            }
            ProposalStatus::Undone => {
                let document = self.workspace.undo_rich_proposal(&project, id)?;
                receipt.summary.undone_revision = Some(document.revision);
                self.status = "Apply undone".into();
                Some(document)
            }
            ProposalStatus::Rejected => {
                self.workspace.reject_rich_proposal(&project, id)?;
                self.status = "Proposal rejected".into();
                None
            }
            ProposalStatus::Staged => return Err("Choose an explicit review action".into()),
        };
        receipt.summary.status = action;
        let reveal = proposal_selection(&receipt, action);
        self.rich_review.selected = Some(std::sync::Arc::new(receipt));
        self.rich_review.parked.retain_mut(|draft| {
            let Some(prepared) = &draft.prepared else {
                return true;
            };
            if prepared.proposal.summary.id != id {
                return true;
            }
            let fulfilled = action == ProposalStatus::Applied
                && prepared.attachment == draft.attachment
                && prepared.replacements == draft.replacements
                && prepared.reason == draft.reason;
            draft.prepared = None;
            !fulfilled
        });
        if self
            .rich_review
            .prepared
            .as_ref()
            .is_some_and(|prepared| prepared.proposal.summary.id == id)
        {
            let fulfilled = self.rich_review.prepared.as_ref().is_some_and(|prepared| {
                self.rich_review.attachment.as_ref() == Some(&prepared.attachment)
                    && self.rich_review.replacements == prepared.replacements
                    && self.rich_review.reason == prepared.reason
            });
            self.rich_review.prepared = None;
            if action == ProposalStatus::Applied && fulfilled {
                self.rich_review.attachment = None;
                self.rich_review.replacements.clear();
                self.rich_review.reason.clear();
                self.rich_review.markup_id = None;
                self.rich_review.markup_revision = None;
                self.rich_review.paused = false;
            }
        }
        if let Some(document) = document
            && self.open_document(document).is_err()
        {
            return Err("The edit was saved, but the panel could not refresh. Reopen the document before continuing.".into());
        }
        if let Some(selection) = reveal {
            let receipt = self
                .rich_review
                .selected
                .as_ref()
                .ok_or("Open a document for review.")?;
            let (revision, tree) = if action == ProposalStatus::Applied {
                (receipt.summary.applied_revision, &receipt.after)
            } else {
                (receipt.summary.undone_revision, &receipt.before)
            };
            if self.document.as_ref().is_none_or(|document| {
                document.id != receipt.summary.document_id
                    || Some(document.revision) != revision
                    || document.rich_json.as_ref() != Some(tree)
            }) {
                return Err("The edit was saved, but the panel could not refresh. Reopen the document before continuing.".into());
            }
            let rich = self
                .rich
                .as_mut()
                .ok_or("Open a structured document for review.")?;
            rich.session.set_selection(selection)?;
            rich.session.request_scroll_to_caret();
        }
        if self.refresh_rich_proposals().is_err() || self.load_rich_proposal(id).is_err() {
            return Err("The edit was saved, but the panel could not refresh. Reopen the document before continuing.".into());
        }
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
        if let Some(queue) = self.rich_review.queue.clone() {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        queue.has_neighbor(false),
                        egui::Button::new(l.text("Previous")),
                    )
                    .clicked()
                {
                    let result = self.step_markup_review(false);
                    self.record_error(result);
                }
                ui.label(format!("{} / {}", queue.index + 1, queue.ids.len()));
                if ui
                    .add_enabled(queue.has_neighbor(true), egui::Button::new(l.text("Next")))
                    .clicked()
                {
                    let result = self.step_markup_review(true);
                    self.record_error(result);
                }
            });
            if queue.skipped > 0 {
                ui.label(
                    RichText::new(l.format(
                        "Skipped {count} unavailable passages",
                        &[("count", queue.skipped.to_string())],
                    ))
                    .size(11.0)
                    .color(t.muted),
                );
            }
            if let Some(document) = &self.document {
                ui.add(
                    egui::Label::new(RichText::new(&document.title).size(12.0).color(t.muted))
                        .truncate(),
                );
            }
            let note = queue
                .ids
                .get(queue.index)
                .and_then(|id| self.workspace.annotation(id).ok())
                .filter(|note| {
                    self.document
                        .as_ref()
                        .is_some_and(|document| document.id == note.document_id)
                });
            if let Some(note) = note {
                if ui
                    .add_enabled_ui(self.can_edit_manuscript(), |ui| {
                        icons::labeled_button(ui, Icon::Trash, l.text("Delete markup"), false)
                    })
                    .inner
                    .clicked()
                {
                    let result = self.quick_remove_note(&note);
                    if result.is_ok() {
                        self.rich_review.paused = true;
                    }
                    self.record_error(result);
                }
            }
            self.quick_undo_control(ui, t);
            ui.add_space(6.0);
        }
        let mut capture = false;
        let mut resume = None;
        ui.horizontal_wrapped(|ui| {
            let enabled = self.can_edit_manuscript()
                && self
                    .rich
                    .as_ref()
                    .and_then(|rich| rich.session.selection())
                    .is_some_and(|selection| selection.anchor != selection.head);
            capture = ui
                .add_enabled(enabled, egui::Button::new(l.text("Edit selection")))
                .clicked();
            if self.rich_review.paused && ui.button(l.text("Resume edit")).clicked() {
                self.rich_review.paused = false;
                self.rich_review.focus = true;
            }
            if !self.rich_review.parked.is_empty() {
                ui.menu_button(l.text("Paused edits"), |ui| {
                    for (index, draft) in self.rich_review.parked.iter().enumerate() {
                        let title = self
                            .workspace
                            .document(&draft.attachment.document_id)
                            .map(|document| document.title)
                            .unwrap_or_else(|_| l.text("Document").into());
                        if ui
                            .button(format!(
                                "{} · {}",
                                title,
                                excerpt(&draft.attachment.quote, 36)
                            ))
                            .clicked()
                        {
                            resume = Some(index);
                            ui.close();
                        }
                    }
                });
            }
        });
        if capture {
            let result = self.capture_review_selection();
            self.record_error(result);
        } else if let Some(index) = resume {
            self.rich_review.park();
            self.rich_review.resume(index);
            self.rich_review.queue = None;
            let result = self.reveal_review_selection();
            self.record_error(result);
            self.focus_manuscript = false;
        }
        let mut apply = false;
        let mut advance = false;
        if let Some(attachment) = self
            .rich_review
            .attachment
            .clone()
            .filter(|_| !self.rich_review.paused)
        {
            let stale = self.dirty.is_some()
                || self.document.as_ref().is_none_or(|document| {
                    document.id != attachment.document_id
                        || document.revision != attachment.base_revision
                        || document.project_id != attachment.project_id
                });
            let allowed = !stale && self.rich_review.changed() && self.can_edit_manuscript();
            ui.add_space(8.0);
            // Primary actions stay above the locally scrolling wording, even for long passages.
            ui.horizontal_wrapped(|ui| {
                apply = components::action(ui, l.text("Apply changes"), allowed, Some(t)).clicked();
                if self
                    .rich_review
                    .queue
                    .as_ref()
                    .is_some_and(|queue| queue.has_neighbor(true))
                {
                    advance =
                        components::action(ui, l.text("Apply and next"), allowed, None).clicked();
                }
                if icons::icon_button(ui, Icon::Close, l.text("Pause edit")).clicked() {
                    self.rich_review.paused = true;
                }
            });
            if stale {
                ui.colored_label(
                    t.warning,
                    l.text("Outdated proposal — the text has changed"),
                );
            } else {
                ui.label(
                    RichText::new(l.text("Local author edit · not applied"))
                        .size(11.0)
                        .color(t.muted),
                );
            }
            egui::ScrollArea::vertical()
                .id_salt("inline-review-wording")
                .max_height((ui.available_height() - 56.0).max(80.0))
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for (index, (range, replacement)) in attachment
                        .ranges
                        .iter()
                        .zip(&mut self.rich_review.replacements)
                        .enumerate()
                    {
                        ui.add_space(8.0);
                        ui.collapsing(
                            l.format(
                                "Original excerpt {index}",
                                &[("index", (index + 1).to_string())],
                            ),
                            |ui| {
                                ui.label(&range.expected_text);
                            },
                        );
                        ui.add(
                            egui::Label::new(
                                RichText::new(excerpt(&range.expected_text, 120))
                                    .size(12.0)
                                    .color(t.muted),
                            )
                            .wrap(),
                        );
                        ui.label(RichText::new(l.text("Proposed text")).size(12.0).strong());
                        let response = ui.add(
                            egui::TextEdit::multiline(replacement)
                                .id_salt((
                                    "rich-proposal-wording",
                                    &attachment.document_id,
                                    attachment.base_revision,
                                    index,
                                ))
                                .desired_rows(4)
                                .desired_width(f32::INFINITY)
                                .margin(8),
                        );
                        if self.rich_review.focus && index == 0 {
                            response.request_focus();
                            self.rich_review.focus = false;
                        }
                        if response.has_focus()
                            && allowed
                            && ui.input_mut(|input| {
                                input.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter)
                            })
                        {
                            apply = true;
                        }
                    }
                    ui.collapsing(l.text("Edit details"), |ui| {
                        components::singleline(
                            ui,
                            &mut self.rich_review.reason,
                            l.text("Reason for the edit (optional)"),
                        );
                    });
                });
        } else if let Some(proposal) = self.rich_review.selected.clone() {
            self.rich_proposal_actions(ui, &proposal, t);
            if ui.button(l.text("Review all changes")).clicked() {
                self.rich_review.full_open = true;
            }
            egui::ScrollArea::vertical()
                .id_salt("selected-review-fragments")
                .max_height((ui.available_height() - 56.0).max(80.0))
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    if let Some((before, after)) = &self.rich_review.excerpts {
                        diff_block(ui, l.text("Original excerpt"), before, t.surface, t);
                        ui.add_space(6.0);
                        diff_block(ui, l.text("Proposed text"), after, t.accent_soft, t);
                    } else {
                        ui.label(l.text(
                            "This edit changes structure or formatting. Open the full comparison.",
                        ));
                    }
                });
        } else {
            ui.add_space(8.0);
            ui.label(l.text(if self.rich_review.paused {
                "Your replacement is kept. Resume when ready."
            } else {
                "Select text to write a replacement, or review marked passages from Notes."
            }));
        }
        if apply || advance {
            let result = self.apply_inline_review(advance);
            self.record_error(result);
        }
        ui.add_space(8.0);
        let mut load = None;
        egui::CollapsingHeader::new(l.text("Saved edits"))
            .id_salt("saved-review-edits")
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(140.0)
                    .show(ui, |ui| {
                        for row in &self.rich_review.records {
                            let label = if row.reason.is_empty() {
                                l.text("Local author edit")
                            } else {
                                &row.reason
                            };
                            if ui
                                .selectable_label(
                                    self.rich_review
                                        .selected
                                        .as_ref()
                                        .is_some_and(|proposal| proposal.summary.id == row.id),
                                    label,
                                )
                                .clicked()
                            {
                                load = Some(row.id.clone());
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
                    });
            });
        if let Some(id) = load {
            self.rich_review.park();
            let result = self.load_rich_proposal(&id);
            self.record_error(result);
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
fn review_draft_needs_save(
    attachment: &AssistantSelectionAttachment,
    replacements: &[String],
    reason: &str,
    prepared: Option<&PreparedEdit>,
) -> bool {
    let authored = !reason.is_empty()
        || attachment
            .ranges
            .iter()
            .zip(replacements)
            .any(|(range, text)| &range.expected_text != text);
    authored
        && !prepared.is_some_and(|prepared| {
            prepared.attachment == *attachment
                && prepared.replacements == replacements
                && prepared.reason == reason
        })
}
fn proposal_selection(proposal: &RichProposal, status: ProposalStatus) -> Option<Selection> {
    if !matches!(status, ProposalStatus::Applied | ProposalStatus::Undone)
        || (status == ProposalStatus::Undone && proposal.command.commands.len() != 1)
        || proposal
            .command
            .commands
            .iter()
            .any(|command| !matches!(command, craft_editor_model::Command::ReplaceText { .. }))
    {
        return None;
    }
    let craft_editor_model::Command::ReplaceText {
        range,
        expected,
        replacement,
    } = proposal.command.commands.last()?
    else {
        return None;
    };
    let text = if status == ProposalStatus::Applied {
        replacement
    } else {
        expected
    };
    Some(Selection {
        anchor: DocPoint {
            block_id: range.block_id.clone(),
            offset: range.start,
        },
        head: DocPoint {
            block_id: range.block_id.clone(),
            offset: craft_editor_model::ScalarOffset(
                range.start.0.checked_add(text.chars().count())?,
            ),
        },
    })
}
fn review_excerpts(proposal: &RichProposal) -> Option<(String, String)> {
    let mut before = Vec::new();
    let mut after = Vec::new();
    for command in &proposal.command.commands {
        if let craft_editor_model::Command::ReplaceText {
            expected,
            replacement,
            ..
        } = command
        {
            before.push(expected.clone());
            after.push(replacement.clone());
        } else {
            return None;
        }
    }
    if before.is_empty() {
        return None;
    }
    // Selection staging records ranges in reverse order to keep scalar positions stable.
    before.reverse();
    after.reverse();
    Some((before.join("\n\n"), after.join("\n\n")))
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
    #[test]
    fn saving_review_drafts_for_close_is_durable_noncanonical_and_idempotent() -> AppResult<()> {
        let (_directory, mut app, first, _) = fixture()?;
        app.capture_review_selection()?;
        app.rich_review.replacements[0] = "Новая версия 😀".into();
        assert!(app.rich_review.has_unsaved_drafts());
        app.save_review_drafts()?;
        assert!(!app.rich_review.has_unsaved_drafts());
        app.save_review_drafts()?;
        let reopened = Workspace::open(&app.database)?;
        assert_eq!(reopened.document(&first.id)?, first);
        let rows = reopened.rich_proposals(&first.project_id, &first.id, None, 16)?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, ProposalStatus::Staged);
        let proposal = reopened.rich_proposal(&first.project_id, &rows[0].id)?;
        assert_eq!(
            review_excerpts(&proposal),
            Some(("echo".into(), "Новая версия 😀".into()))
        );
        assert_eq!(reopened.rich_revisions(&first.id)?.len(), 1);
        Ok(())
    }

    #[test]
    fn a_changed_reason_gets_its_own_receipt_and_applying_history_clears_only_fulfilled_drafts()
    -> AppResult<()> {
        let (_directory, mut app, first, _) = fixture()?;
        app.capture_review_selection()?;
        app.rich_review.replacements[0] = "changed".into();
        app.stage_rich_selected_edit()?;
        let prior = app
            .rich_review
            .prepared
            .as_ref()
            .ok_or("prepared edit")?
            .proposal
            .summary
            .id
            .clone();
        app.rich_review.reason = "Clarify the second signal".into();
        app.stage_rich_selected_edit()?;
        let current = app
            .rich_review
            .prepared
            .as_ref()
            .ok_or("prepared edit")?
            .proposal
            .summary
            .id
            .clone();
        assert_ne!(prior, current);
        assert_eq!(
            app.workspace
                .rich_proposal(&first.project_id, &current)?
                .summary
                .reason,
            "Clarify the second signal"
        );
        app.rich_review.park();
        assert_eq!(app.rich_review.parked.len(), 1);
        app.act_rich_proposal(&current, ProposalStatus::Applied)?;
        assert!(app.rich_review.parked.is_empty());
        let selection = app
            .rich
            .as_ref()
            .and_then(|rich| rich.session.selection())
            .ok_or("changed occurrence")?;
        assert_eq!(
            (selection.anchor.offset, selection.head.offset),
            (ScalarOffset(8), ScalarOffset(15))
        );
        assert_eq!(app.body, "Я😀 echo changed");
        Ok(())
    }

    #[test]
    fn failed_close_save_retains_stale_wording_until_explicit_discard() -> AppResult<()> {
        let (_directory, mut app, first, _) = fixture()?;
        app.capture_review_selection()?;
        app.rich_review.replacements[0] = "Keep this unsubmitted wording".into();
        let mut changed = first.rich_json.clone().ok_or("rich text")?;
        changed["content"][0]["content"][0]["marks"] = serde_json::json!([{"type":"italic"}]);
        let current = app
            .workspace
            .save_rich_document(&first.id, first.revision, changed)?;
        app.open_document(current.clone())?;
        assert!(app.save_review_drafts().is_err());
        assert!(app.rich_review.has_unsaved_drafts());
        assert_eq!(
            app.rich_review.parked[0].replacements,
            ["Keep this unsubmitted wording"]
        );
        assert_eq!(app.workspace.document(&first.id)?, current);
        assert!(
            app.workspace
                .rich_proposals(&first.project_id, &first.id, None, 16)?
                .is_empty()
        );
        app.discard_review_drafts();
        assert!(!app.rich_review.has_unsaved_drafts());
        assert_eq!(app.workspace.document(&first.id)?, current);
        Ok(())
    }

    #[test]
    fn applying_an_older_receipt_preserves_changed_active_wording_and_reason() -> AppResult<()> {
        let (_directory, mut app, _, _) = fixture()?;
        app.capture_review_selection()?;
        app.rich_review.replacements[0] = "Saved proposal".into();
        app.stage_rich_selected_edit()?;
        let id = app
            .rich_review
            .prepared
            .as_ref()
            .ok_or("receipt")?
            .proposal
            .summary
            .id
            .clone();
        app.rich_review.replacements[0] = "Keep this newer wording".into();
        app.rich_review.reason = "A more precise motivation".into();
        app.rich_review.paused = true;
        app.act_rich_proposal(&id, ProposalStatus::Applied)?;
        assert_eq!(app.body, "Я😀 echo Saved proposal");
        assert_eq!(app.rich_review.replacements, ["Keep this newer wording"]);
        assert_eq!(app.rich_review.reason, "A more precise motivation");
        assert!(app.rich_review.attachment.is_some());
        assert!(app.rich_review.prepared.is_none());
        assert!(app.rich_review.has_unsaved_drafts());
        assert!(app.reveal_review_selection().is_err());
        assert_eq!(app.rich_review.replacements, ["Keep this newer wording"]);
        Ok(())
    }

    #[test]
    fn revealing_a_draft_refuses_a_live_tree_that_differs_from_its_canonical_revision()
    -> AppResult<()> {
        let (_directory, mut app, first, _) = fixture()?;
        app.capture_review_selection()?;
        app.rich_review.replacements[0] = "Keep proposed wording".into();
        let session = &mut app.rich.as_mut().ok_or("rich session")?.session;
        session.apply_commands(vec![craft_editor_model::Command::ReplaceText {
            range: craft_editor_model::TextRange {
                block_id: "p".into(),
                start: ScalarOffset(0),
                end: ScalarOffset(0),
            },
            expected: String::new(),
            replacement: "Local prefix ".into(),
        }])?;
        let selection = session.selection().cloned();
        let live_tree = session.document().to_json()?;
        // A native frame may not yet have synchronized its dirty flag after input.
        assert!(app.dirty.is_none());
        assert!(app.reveal_review_selection().is_err());
        let session = &app.rich.as_ref().ok_or("rich session")?.session;
        assert_eq!(session.selection(), selection.as_ref());
        assert_eq!(session.document().to_json()?, live_tree);
        assert_eq!(app.workspace.document(&first.id)?, first);
        assert_eq!(app.rich_review.replacements, ["Keep proposed wording"]);
        Ok(())
    }
}
