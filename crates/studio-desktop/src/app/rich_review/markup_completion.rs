//! Exact originating-note completion, separate from the durable manuscript Apply receipt.
use super::*;
use studio_core::{AnchorStatus, AnnotationStatus, PassageAnnotation};

#[derive(Clone)]
pub(super) struct AppliedMarkup {
    pub(super) proposal_id: String,
    note: PassageAnnotation,
}

impl AppliedMarkup {
    pub(super) fn note_id(&self) -> &str {
        &self.note.id
    }
}

impl StudioApp {
    /// Keep context only for a real current queue target containing this exact scalar selection.
    pub(super) fn current_review_markup_for_selection(
        &self,
        attachment: &AssistantSelectionAttachment,
    ) -> AppResult<Option<PassageAnnotation>> {
        let Some(queue) = &self.rich_review.queue else {
            return Ok(None);
        };
        let Some(id) = queue.ids.get(queue.index) else {
            return Ok(None);
        };
        let Some(expected) = self
            .rich_review
            .markup_id
            .as_ref()
            .filter(|active| *active == id)
            .and(self.rich_review.markup_revision)
            .or_else(|| {
                self.rich_review
                    .applied_markup
                    .as_ref()
                    .filter(|mark| &mark.note.id == id)
                    .map(|mark| mark.note.revision)
            })
        else {
            return Ok(None);
        };
        let note = self.workspace.annotation(id)?;
        if note.revision != expected
            || note.deleted
            || note.status == AnnotationStatus::Resolved
            || note.anchor_status != AnchorStatus::Attached
            || note.document_id != attachment.document_id
            || note.base_revision != attachment.base_revision
            || queue.project_id != attachment.project_id
        {
            return Ok(None);
        }
        let Some(rich) = &self.rich else {
            return Ok(None);
        };
        let Some(selection) = rich.session.selection() else {
            return Ok(None);
        };
        let point = |point: &DocPoint| TextPoint {
            block_id: point.block_id.clone(),
            offset: point.offset,
        };
        let Ok(selected) = studio_core::AnnotationAnchor::from_selection(
            rich.session.document(),
            &point(&selection.anchor),
            &point(&selection.head),
        ) else {
            // A cross-block standalone correction cannot inherit this single-note attachment.
            return Ok(None);
        };
        Ok((selected.block_id == note.anchor.block_id
            && selected.start >= note.anchor.start
            && selected.end <= note.anchor.end)
            .then_some(note))
    }

    pub(super) fn markup_origin_for_apply(
        &self,
        proposal_id: &str,
    ) -> AppResult<Option<PassageAnnotation>> {
        let Some(prepared) = self.rich_review.prepared.as_ref().filter(|prepared| {
            prepared.proposal.summary.id == proposal_id
                && self.rich_review.attachment.as_ref() == Some(&prepared.attachment)
                && self.rich_review.replacements == prepared.replacements
                && self.rich_review.reason == prepared.reason
        }) else {
            return Ok(None);
        };
        let Some(id) = &self.rich_review.markup_id else {
            return Ok(None);
        };
        let note = self.workspace.annotation(id)?;
        if note.deleted
            || note.status == AnnotationStatus::Resolved
            || Some(note.revision) != self.rich_review.markup_revision
            || note.document_id != prepared.attachment.document_id
            || note.base_revision != prepared.attachment.base_revision
        {
            return Err(
                "This marked passage changed. Refresh the review before applying it.".into(),
            );
        }
        Ok(Some(note))
    }

    pub(super) fn remember_applied_markup(
        &mut self,
        id: &str,
        before: Option<PassageAnnotation>,
    ) -> AppResult<()> {
        self.rich_review.applied_markup = None;
        let Some(before) = before else { return Ok(()) };
        let after = self.workspace.annotation(&before.id)?;
        if before.revision.checked_add(1) != Some(after.revision)
            || before.base_revision.checked_add(1) != Some(after.base_revision)
            || after.deleted
            || after.note != before.note
            || after.tags != before.tags
            || after.status != before.status
            || after.document_id != before.document_id
        {
            return Err("The edit was applied, but its note changed. Open Notes to review it before resolving it.".into());
        }
        self.rich_review.applied_markup = Some(AppliedMarkup {
            proposal_id: id.into(),
            note: after,
        });
        Ok(())
    }

    pub(super) fn resolve_applied_review_note(&mut self, advance: bool) -> AppResult<()> {
        self.change_applied_review_note_status(AnnotationStatus::Resolved)?;
        if advance && let Some(queue) = &self.rich_review.queue {
            let index = queue.index;
            self.step_markup_review(true)?;
            self.rich_review.queue_finished = self
                .rich_review
                .queue
                .as_ref()
                .is_some_and(|queue| queue.index == index);
        }
        Ok(())
    }

    fn change_applied_review_note_status(&mut self, status: AnnotationStatus) -> AppResult<()> {
        let mark = self
            .rich_review
            .applied_markup
            .clone()
            .ok_or("Select a passage for review.")?;
        let receipt = self
            .rich_review
            .selected
            .as_ref()
            .ok_or("Open a document for review.")?;
        if receipt.summary.id != mark.proposal_id
            || receipt.summary.status != ProposalStatus::Applied
            || self.dirty.is_some()
            || self.document.as_ref().is_none_or(|document| {
                document.id != mark.note.document_id
                    || Some(document.revision) != receipt.summary.applied_revision
                    || document.rich_json.as_ref() != Some(&receipt.after)
            })
        {
            return Err(
                "This note or manuscript changed. Review it in Notes before resolving it.".into(),
            );
        }
        let after = self.update_review_note_status(&mark.note, status)?;
        // Acknowledge before the fallible panel refresh; retry must never repeat a committed action.
        self.rich_review.applied_markup = Some(AppliedMarkup {
            note: after.clone(),
            ..mark
        });
        self.rich_review.queue_finished = false;
        if let Some(queue) = &mut self.rich_review.queue {
            if status == AnnotationStatus::Resolved {
                queue.unavailable.insert(after.id.clone());
            } else {
                queue.unavailable.remove(&after.id);
            }
        }
        self.status = if status == AnnotationStatus::Resolved {
            "Note resolved"
        } else {
            "Note reopened"
        }
        .into();
        self.refresh_notes().map_err(|_| "The note was updated, but the panel could not refresh. Reopen Notes before continuing.".into())
    }

    pub(super) fn review_markup_completion_actions(
        &mut self,
        ui: &mut egui::Ui,
        proposal: &RichProposal,
        t: Tokens,
    ) {
        let Some(mark) = self
            .rich_review
            .applied_markup
            .as_ref()
            .filter(|mark| mark.proposal_id == proposal.summary.id)
            .cloned()
        else {
            return;
        };
        let l = self.language;
        if mark.note.status == AnnotationStatus::Resolved {
            ui.colored_label(
                t.muted,
                l.text(if self.rich_review.queue_finished {
                    "End of review list"
                } else {
                    "Note resolved"
                }),
            );
            if ui.button(l.text("Reopen note")).clicked() {
                let result = self.change_applied_review_note_status(AnnotationStatus::Open);
                self.record_error(result);
            }
        } else {
            let has_next = self
                .rich_review
                .queue
                .as_ref()
                .is_some_and(|queue| queue.has_neighbor(true));
            let label = if has_next {
                "Resolve note and next"
            } else {
                "Resolve note"
            };
            if components::action(ui, l.text(label), self.can_edit_manuscript(), Some(t)).clicked()
            {
                let result = self.resolve_applied_review_note(true);
                self.record_error(result);
            }
        }
    }
}
