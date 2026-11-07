//! Native review component.
use super::*;

impl StudioApp {
    pub(super) fn inspector(&mut self, ui: &mut egui::Ui, t: Tokens) {
        let l = self.language;
        egui::Panel::right("inspector")
            .default_size(320.0)
            .min_size(280.0)
            .max_size(460.0)
            .frame(
                egui::Frame::new()
                    .fill(t.panel)
                    .inner_margin(egui::Margin::symmetric(20, 18)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(self.inspector == Inspector::Review, l.text("Review"))
                        .clicked()
                    {
                        self.inspector = Inspector::Review;
                    }
                    if ui
                        .selectable_label(self.inspector == Inspector::Versions, l.text("History"))
                        .clicked()
                    {
                        self.inspector = Inspector::Versions;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::icon_button(ui, Icon::Close, l.text("Hide panel")).clicked() {
                            self.inspector_visible = false;
                        }
                    });
                });
                ui.add_space(14.0);
                ui.separator();
                ui.add_space(16.0);
                egui::ScrollArea::vertical()
                    .id_salt("inspector-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.inspector == Inspector::Versions {
                            self.versions(ui, t);
                        } else {
                            self.review(ui, t);
                        }
                    });
            });
    }

    pub(super) fn review(&mut self, ui: &mut egui::Ui, t: Tokens) {
        let l = self.language;
        ui.horizontal(|ui| {
            icons::icon(ui, Icon::Assistant, 22.0, t.accent);
            ui.label(
                RichText::new(l.text("You are in control"))
                    .size(17.0)
                    .strong(),
            );
        });
        ui.add_space(10.0);
        ui.label(RichText::new(l.text("Review the proposal before changing your manuscript. The original stays unchanged until you apply it.")).size(13.0).color(t.muted));
        ui.add_space(18.0);
        for proposal in self.proposals.clone() {
            match proposal.status {
                ProposalStatus::Staged => {
                    let stale = self.dirty.is_some()
                        || self.document.as_ref().is_none_or(|document| {
                            document.revision != proposal.base_revision
                                || document.body != proposal.before
                        });
                    ui.label(
                        RichText::new(l.text(if stale {
                            "Outdated proposal — the text has changed"
                        } else {
                            "Proposed changes · not applied"
                        }))
                        .size(12.0)
                        .strong()
                        .color(t.warning),
                    );
                    if !proposal.reason.is_empty() {
                        ui.add_space(6.0);
                        ui.label(&proposal.reason);
                    }
                    let (before, after) = proposal_excerpts(&proposal, 340);
                    ui.add_space(12.0);
                    diff_block(ui, l.text("Original text"), &before, t.surface, t);
                    ui.add_space(8.0);
                    diff_block(ui, l.text("Proposal"), &after, t.accent_soft, t);
                    ui.add_space(10.0);
                    if ui.button(l.text("Review all changes")).clicked() {
                        self.full_review = Some(proposal.clone());
                    }
                    ui.add_space(10.0);
                    ui.add_enabled_ui(!stale, |ui| {
                        if primary(ui, l.text("Apply changes"), t).clicked() {
                            let result = self.apply(&proposal.id);
                            self.record_error(result);
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(l.text("Reject")).clicked() {
                            let result = self
                                .workspace
                                .reject_proposal(&proposal.id)
                                .map_err(Into::into)
                                .and_then(|()| self.refresh_proposals());
                            self.record_error(result);
                        }
                        if ui.button(l.text("Copy proposal")).clicked() {
                            ui.ctx().copy_text(proposal.after.clone());
                        }
                    });
                }
                ProposalStatus::Applied => {
                    ui.label(RichText::new(l.text("Changes applied")).color(t.good));
                    if ui.button(l.text("Undo apply")).clicked() {
                        let result = self.undo(&proposal.id);
                        self.record_error(result);
                    }
                }
                ProposalStatus::Rejected => {
                    ui.label(
                        RichText::new(l.text("Proposal rejected"))
                            .size(12.0)
                            .color(t.muted),
                    );
                }
                ProposalStatus::Undone => {
                    ui.label(
                        RichText::new(l.text("Apply undone"))
                            .size(12.0)
                            .color(t.muted),
                    );
                }
            }
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(16.0);
        }
        let editable = self
            .document
            .as_ref()
            .is_some_and(|document| !document.read_only);
        egui::CollapsingHeader::new(l.text("Prepare an exact edit"))
            .id_salt(("proposal-composer", self.proposals.len()))
            .default_open(self.proposals.is_empty())
            .open(self.focus_replacement.then_some(true))
            .show(ui, |ui| {
                ui.add_space(10.0);
                if let Some((selection_start, selection_end)) = self
                    .frozen_selection
                    .as_ref()
                    .map(|scope| (scope.anchor.start, scope.anchor.end))
                {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(l.format(
                                "Selection: {start}–{end}",
                                &[
                                    ("start", selection_start.to_string()),
                                    ("end", selection_end.to_string()),
                                ],
                            ))
                            .size(12.0)
                            .color(t.accent),
                        );
                        if icons::icon_button(ui, Icon::Close, l.text("Clear selection scope"))
                            .clicked()
                        {
                            self.frozen_selection = None;
                        }
                    });
                }
                ui.label(
                    RichText::new(l.text("Original excerpt"))
                        .size(12.0)
                        .color(t.muted),
                );
                let frozen = self.frozen_selection.is_some();
                if frozen {
                    egui::ScrollArea::vertical()
                        .id_salt("frozen-selection-quote")
                        .max_height(120.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            diff_block(ui, "", &self.expected, t.surface, t);
                        });
                } else {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.expected)
                            .desired_rows(3)
                            .desired_width(f32::INFINITY)
                            .hint_text(
                                l.text("Select text in the manuscript or paste an exact excerpt"),
                            ),
                    );
                }
                ui.add_space(10.0);
                ui.label(
                    RichText::new(l.text("Proposed text"))
                        .size(12.0)
                        .color(t.muted),
                );
                let replacement_field = ui.add(
                    egui::TextEdit::multiline(&mut self.replacement)
                        .id_salt("proposal-replacement")
                        .desired_rows(4)
                        .desired_width(f32::INFINITY)
                        .hint_text(l.text("New wording")),
                );
                if self.focus_replacement {
                    replacement_field.request_focus();
                    self.focus_replacement = false;
                }
                ui.add_space(8.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.reason)
                        .desired_width(f32::INFINITY)
                        .hint_text(l.text("Reason for the edit (optional)")),
                );
                ui.add_space(12.0);
                let ready =
                    editable && !self.expected.is_empty() && self.expected != self.replacement;
                if ui
                    .add_enabled(
                        ready,
                        egui::Button::new(l.text("Prepare proposal"))
                            .min_size(Vec2::new(ui.available_width(), 34.0)),
                    )
                    .clicked()
                {
                    let result = self.stage();
                    self.record_error(result);
                }
                ui.add_space(8.0);
                ui.label(
                    RichText::new(l.text("Local editing tool. No AI provider is connected yet."))
                        .size(11.5)
                        .color(t.muted),
                );
            });
    }

    pub(super) fn versions(&self, ui: &mut egui::Ui, t: Tokens) {
        let l = self.language;
        ui.label(
            RichText::new(l.text("Document history"))
                .size(17.0)
                .strong(),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new(l.text("Saved revisions remain in your local project."))
                .size(13.0)
                .color(t.muted),
        );
        ui.add_space(18.0);
        if let Some(document) = &self.document {
            match self.workspace.revisions(&document.id) {
                Ok(revisions) => {
                    for revision in revisions {
                        egui::CollapsingHeader::new(l.format(
                            "Revision {revision} · Words: {count}",
                            &[
                                ("revision", revision.revision.to_string()),
                                (
                                    "count",
                                    revision.body.split_whitespace().count().to_string(),
                                ),
                            ],
                        ))
                        .id_salt(revision.revision)
                        .show(ui, |ui| {
                            ui.label(RichText::new(&revision.body).size(13.0));
                        });
                        ui.add_space(6.0);
                    }
                }
                Err(error) => {
                    ui.colored_label(t.warning, l.text(error_message(&error)));
                    ui.collapsing(l.text("Technical details"), |ui| {
                        ui.label(error.to_string());
                    });
                }
            }
        }
    }

    pub(super) fn full_review_dialog(&mut self, ctx: &egui::Context, t: Tokens) {
        let Some(proposal) = self.full_review.clone() else {
            return;
        };
        let l = self.language;
        let response = modal(
            ctx,
            l,
            "full-review",
            l.text("Review all changes"),
            900.0,
            |ui| {
                let height = (ctx.content_rect().height() - 260.0).clamp(200.0, 560.0);
                ui.columns(2, |columns| {
                    for (index, (title, text)) in [
                        (l.text("Original text"), &proposal.before),
                        (l.text("Proposal"), &proposal.after),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        columns[index].label(RichText::new(title).strong());
                        columns[index].separator();
                        egui::ScrollArea::vertical()
                            .id_salt(("full-diff", index))
                            .max_height(height)
                            .show(&mut columns[index], |ui| {
                                ui.add(egui::Label::new(text).selectable(true));
                            });
                    }
                });
                ui.add_space(16.0);
                let stale = self.dirty.is_some()
                    || self
                        .document
                        .as_ref()
                        .is_none_or(|document| document.revision != proposal.base_revision);
                if stale {
                    ui.colored_label(
                        t.warning,
                        l.text("Outdated proposal — the text has changed"),
                    );
                }
                ui.add_enabled_ui(!stale, |ui| {
                    if primary(ui, l.text("Apply changes"), t).clicked() {
                        let result = self.apply(&proposal.id);
                        if result.is_ok() {
                            self.full_review = None;
                        }
                        self.record_error(result);
                    }
                });
            },
        );
        if response.should_close() {
            self.full_review = None;
        }
    }
}
