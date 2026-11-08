//! Persistent passage-note presentation over guarded core commands and ephemeral native ranges.
use super::*;
use craft_editor_egui::{
    DocPoint, Selection,
    decorations::{DecorationSource, TextDecoration},
};
use craft_editor_model::{
    BlockTextProjection, Document as RichTree, InlineAtomText, ReplayProof, Revision, ScalarOffset,
    TextPoint, TextProjectionOptions, scalar_to_byte,
};
use std::{collections::HashMap, sync::Arc};
use studio_core::{
    AnchorStatus, AnnotationAnchor, AnnotationPatch, AnnotationPreviewSnapshot, AnnotationStatus,
    PassageAnnotation,
};

#[derive(Clone)]
struct NoteDraft {
    original: PassageAnnotation,
    body: String,
}
struct NoteComposer {
    id: String,
    document_id: String,
    base_revision: i64,
    anchor: AnnotationAnchor,
    body: String,
    focus: bool,
}

#[derive(Default)]
pub(super) struct NotesState {
    snapshot: Option<Arc<AnnotationPreviewSnapshot>>,
    preview: Vec<PassageAnnotation>,
    preview_key: Option<(Revision, u64)>,
    epoch: u64,
    invalid: bool,
    selected: Option<String>,
    drafts: HashMap<String, NoteDraft>,
    composer: Option<NoteComposer>,
    paused_composers: HashMap<String, NoteComposer>,
    failed_context: Option<(String, i64)>,
    close_confirmation: bool,
    delete_confirmation: Option<PassageAnnotation>,
    reveal_row: bool,
    last_error: Option<Box<dyn std::error::Error + Send + Sync>>,
    reported_error_epoch: Option<u64>,
}
impl NotesState {
    pub(super) fn clear(&mut self) {
        let mut paused_composers = std::mem::take(&mut self.paused_composers);
        if let Some(composer) = self.composer.take() {
            paused_composers.insert(composer.document_id.clone(), composer);
        }
        let mut drafts = std::mem::take(&mut self.drafts);
        drafts.retain(|_, draft| draft.body.trim() != draft.original.note.trim());
        *self = Self {
            drafts,
            paused_composers,
            ..Self::default()
        };
    }
    pub(super) fn modal_open(&self) -> bool {
        self.composer.is_some() || self.delete_confirmation.is_some() || self.close_confirmation
    }
    pub(super) fn has_unsaved_drafts(&self) -> bool {
        self.drafts
            .values()
            .any(|draft| draft.body.trim() != draft.original.note.trim())
            || self
                .composer
                .as_ref()
                .is_some_and(|composer| !composer.body.trim().is_empty())
            || self
                .paused_composers
                .values()
                .any(|composer| !composer.body.trim().is_empty())
    }
    pub(super) fn request_close_confirmation(&mut self) {
        self.close_confirmation = true;
    }
    pub(super) fn close_confirmation_open(&self) -> bool {
        self.close_confirmation
    }
    pub(super) fn invalidate(&mut self) {
        self.invalid = true;
        self.failed_context = None;
        self.snapshot = None;
        self.preview_key = None;
    }

    /// Call outside canvas/input resolution after open, acknowledged save or note mutation.
    pub(super) fn prepare(&mut self, workspace: &Workspace, document: &Document) -> AppResult<()> {
        if document.rich_json.is_none() {
            self.clear();
            return Ok(());
        }
        if self.snapshot.as_ref().is_some_and(|snapshot| {
            snapshot.document_id() == document.id && snapshot.base_revision() == document.revision
        }) {
            return Ok(());
        }
        if self
            .failed_context
            .as_ref()
            .is_some_and(|(id, revision)| id == &document.id && *revision == document.revision)
        {
            return Ok(());
        }
        let prior_revision = self
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.base_revision());
        let prior_id = self
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.document_id().to_owned());
        if prior_id.as_deref().is_some_and(|id| id != document.id)
            || self
                .preview
                .iter()
                .any(|note| note.document_id != document.id)
        {
            self.selected = None;
            self.preview.clear();
            self.preview_key = None;
            if let Some(composer) = self.composer.take() {
                self.paused_composers
                    .insert(composer.document_id.clone(), composer);
            }
            self.delete_confirmation = None;
        }
        let snapshot = match workspace.annotation_preview_snapshot(&document.id, document.revision)
        {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.invalid = true;
                self.snapshot = None;
                self.failed_context = Some((document.id.clone(), document.revision));
                return Err(error.into());
            }
        };
        if self.composer.is_none() {
            self.composer = self.paused_composers.remove(&document.id);
        }
        // A guarded local canonical save advances every note once. Rebase the
        // editor's metadata guard only when authored fields stayed identical.
        if prior_revision.is_some_and(|revision| revision.checked_add(1) == Some(document.revision))
        {
            for note in snapshot.stored_notes() {
                if let Some(draft) = self.drafts.get_mut(&note.id)
                    && draft.original.revision.checked_add(1) == Some(note.revision)
                    && draft.original.note == note.note
                    && draft.original.status == note.status
                {
                    draft.original = note.clone();
                }
            }
        }
        self.preview = snapshot.stored_notes().to_vec();
        self.snapshot = Some(Arc::new(snapshot));
        self.epoch = self
            .epoch
            .checked_add(1)
            .ok_or("note_preview_epoch_overflow")?;
        self.preview_key = None;
        self.invalid = false;
        self.failed_context = None;
        Ok(())
    }
    pub(super) fn frame<'a>(
        &'a mut self,
        proofs: &'a [ReplayProof],
        language: Language,
        tokens: Tokens,
    ) -> NotesFrame<'a> {
        NotesFrame {
            snapshot: self.snapshot.clone(),
            state: self,
            prior: proofs,
            language,
            tokens,
        }
    }
    pub(super) fn activate(&mut self, id: &str) {
        self.selected = Some(id.into());
        self.reveal_row = true;
    }
    fn acknowledge(&mut self, note: &PassageAnnotation) {
        if let Some(draft) = self.drafts.get_mut(&note.id)
            && draft.original.note == note.note
        {
            draft.original = note.clone();
        }
        self.invalidate();
    }
}

pub(super) struct NotesFrame<'a> {
    state: &'a mut NotesState,
    snapshot: Option<Arc<AnnotationPreviewSnapshot>>,
    prior: &'a [ReplayProof],
    language: Language,
    tokens: Tokens,
}
impl NotesFrame<'_> {
    pub(super) fn take_error(&mut self) -> Option<Box<dyn std::error::Error + Send + Sync>> {
        self.state.last_error.take()
    }
}
impl DecorationSource for NotesFrame<'_> {
    fn decorations(
        &mut self,
        document: &RichTree,
        revision: Revision,
        pending: &[ReplayProof],
    ) -> craft_editor_egui::Result<Vec<TextDecoration>> {
        let key = (revision, self.state.epoch);
        if self.snapshot.is_none() {
            // Preparation already reported this context's acquisition failure.
            // Typing must not create a fresh modal for every new input revision.
            self.state.invalid = true;
            return Err(craft_editor_egui::Error::DecorationSource);
        }
        if self.state.preview_key != Some(key) {
            self.state.preview_key = Some(key);
            let result = (|| -> AppResult<Vec<PassageAnnotation>> {
                let snapshot = self.snapshot.as_ref().ok_or("note_preview_not_prepared")?;
                if snapshot.stored_notes().is_empty() {
                    return Ok(vec![]);
                }
                let proofs: Vec<_> = self.prior.iter().chain(pending).cloned().collect();
                Ok(snapshot.preview(&proofs, document.to_json()?)?)
            })();
            match result {
                Ok(notes) => {
                    self.state.preview = notes;
                    self.state.invalid = false;
                    self.state.reported_error_epoch = None;
                }
                Err(error) => {
                    self.state.invalid = true;
                    if self.state.reported_error_epoch != Some(self.state.epoch) {
                        self.state.last_error = Some(error);
                        self.state.reported_error_epoch = Some(self.state.epoch);
                    }
                }
            }
        }
        if self.state.invalid {
            return Err(craft_editor_egui::Error::DecorationSource);
        }
        self.state
            .preview
            .iter()
            .filter(|note| {
                note.anchor_status == AnchorStatus::Attached
                    && note.status != AnnotationStatus::Resolved
            })
            .map(|note| {
                Ok(TextDecoration {
                    id: note.id.clone(),
                    range: selection_for(note, document)
                        .map_err(|_| craft_editor_egui::Error::InvalidDecoration)?,
                    color: self.tokens.warning,
                    label: self
                        .language
                        .format("Author note: {note}", &[("note", excerpt(&note.note, 240))]),
                })
            })
            .collect::<craft_editor_egui::Result<Vec<_>>>()
    }
}

fn selection_for(note: &PassageAnnotation, tree: &RichTree) -> AppResult<Selection> {
    let (first, last) = note.anchor.selection(tree)?;
    Ok(Selection {
        anchor: DocPoint {
            block_id: first.block_id,
            offset: first.offset,
        },
        head: DocPoint {
            block_id: last.block_id,
            offset: last.offset,
        },
    })
}
fn note_input(
    ui: &mut egui::Ui,
    body: &mut String,
    id: egui::Id,
    l: Language,
    t: Tokens,
    read_only: bool,
) -> egui::Response {
    let response = if read_only {
        let mut text: &str = body;
        ui.add(
            egui::TextEdit::multiline(&mut text)
                .id(id)
                .desired_width(f32::INFINITY)
                .desired_rows(6)
                .margin(12),
        )
    } else {
        ui.add(
            egui::TextEdit::multiline(body)
                .id(id)
                .desired_width(f32::INFINITY)
                .desired_rows(6)
                .margin(12),
        )
    };
    note_length(ui, body, l, t);
    response
}
fn note_row(
    ui: &mut egui::Ui,
    note: &PassageAnnotation,
    quote: &str,
    selected: bool,
    l: Language,
    t: Tokens,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 44.0), egui::Sense::click());
    if selected || response.hovered() {
        ui.painter()
            .rect_filled(rect, 5.0, if selected { t.accent_soft } else { t.surface });
    }
    let attention = note.anchor_status == AnchorStatus::Detached
        || note.status == AnnotationStatus::NeedsAttention;
    let icon = if attention {
        Icon::Warning
    } else if note.status == AnnotationStatus::Resolved {
        Icon::Check
    } else {
        Icon::Assistant
    };
    let color = if attention {
        t.warning
    } else if selected {
        t.accent
    } else {
        t.muted
    };
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(egui::vec2(8.0, 3.0)))
            .layout(Layout::left_to_right(Align::Center)),
        |ui| {
            icons::icon(ui, icon, 18.0, color);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                ui.add(
                    egui::Label::new(RichText::new(excerpt(quote, 80)).size(13.0).color(t.text))
                        .truncate(),
                );
                let body = if note.status == AnnotationStatus::Resolved {
                    format!("{} · {}", l.text("Resolved"), excerpt(&note.note, 80))
                } else {
                    excerpt(&note.note, 80)
                };
                ui.add(egui::Label::new(RichText::new(body).size(12.0).color(t.muted)).truncate());
            });
        },
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            format!("{}: {}", quote, note.note),
        )
    });
    response.on_hover_text(format!("{}\n{}", quote, note.note))
}
fn visible_menu_anchor(visible: egui::Rect, caret: Option<egui::Rect>) -> egui::Pos2 {
    let inset = 4.0_f32
        .min(visible.width() * 0.25)
        .min(visible.height() * 0.25);
    visible.shrink(inset).clamp(caret.map_or_else(
        || visible.center(),
        |caret| caret.left_bottom() + egui::vec2(0.0, 4.0),
    ))
}
fn valid_note_body(body: &str) -> bool {
    !body.trim().is_empty() && body.chars().count() <= 10_000
}
fn note_length(ui: &mut egui::Ui, body: &str, l: Language, t: Tokens) {
    let count = body.chars().count();
    ui.label(
        RichText::new(l.format(
            "{count} / 10000 characters",
            &[("count", count.to_string())],
        ))
        .size(12.0)
        .color(if count > 10_000 { t.warning } else { t.muted }),
    );
}
fn displayed_quote(anchor: &AnnotationAnchor, tree: Option<&RichTree>, l: Language) -> String {
    let displayed = tree.and_then(|tree| {
        let (a, b) = anchor.selection(tree).ok()?;
        let projection = BlockTextProjection::with_options(
            tree,
            &anchor.block_id,
            TextProjectionOptions {
                block_separator: "\n",
                hard_break: "\n",
                inline_math: InlineAtomText::WithAffixes {
                    prefix: "$",
                    suffix: "$",
                },
                block_atom: "",
            },
        )
        .ok()?;
        let start = projection.offset(&a).ok()?;
        let end = projection.offset(&b).ok()?;
        let start = scalar_to_byte(projection.text(), start).ok()?.0;
        let end = scalar_to_byte(projection.text(), end).ok()?.0;
        projection.text().get(start..end).map(str::to_owned)
    });
    displayed.unwrap_or_else(|| {
        anchor
            .exact_text
            .replace('\u{fffc}', l.text("[Embedded content]"))
    })
}
fn excerpt(text: &str, limit: usize) -> String {
    let mut result: String = text.chars().take(limit).collect();
    if text.chars().count() > limit {
        result.push('…');
    }
    result
}
fn captured_selection(
    session: &craft_editor_egui::EditorSession,
) -> AppResult<(String, ScalarOffset, ScalarOffset, String)> {
    if session.is_composing() {
        return Err("annotation_composition_active".into());
    }
    let selection = session
        .selection()
        .ok_or("annotation_requires_text_selection")?;
    let anchor = AnnotationAnchor::from_selection(
        session.document(),
        &TextPoint {
            block_id: selection.anchor.block_id.clone(),
            offset: selection.anchor.offset,
        },
        &TextPoint {
            block_id: selection.head.block_id.clone(),
            offset: selection.head.offset,
        },
    )?;
    Ok((anchor.block_id, anchor.start, anchor.end, anchor.exact_text))
}

impl StudioApp {
    fn ensure_note_writable(&self) -> AppResult<()> {
        if self.is_recovery_read_only() {
            return Err(recovery::ReadOnlyRecoveryView.into());
        }
        if !self.can_edit_manuscript() {
            return Err(studio_core::StudioError::ReadOnly {
                document_id: self
                    .document
                    .as_ref()
                    .map(|document| document.id.clone())
                    .unwrap_or_default(),
            }
            .into());
        }
        Ok(())
    }
    pub(super) fn begin_note(&mut self) -> AppResult<()> {
        self.ensure_note_writable()?;
        if self.notes.invalid {
            return Err("annotation_preview_unverified".into());
        }
        self.save()?;
        let document = self.document.as_ref().ok_or("missing_note_document")?;
        let session = &self.rich.as_ref().ok_or("missing_note_editor")?.session;
        let (id, start, end, quote) = captured_selection(session)?;
        let anchor = self.workspace.capture_annotation_anchor(
            &document.id,
            document.revision,
            &id,
            start,
            end,
            &quote,
        )?;
        self.notes.composer = Some(NoteComposer {
            id: uuid::Uuid::new_v4().to_string(),
            document_id: document.id.clone(),
            base_revision: document.revision,
            anchor,
            body: String::new(),
            focus: true,
        });
        Ok(())
    }
    pub(super) fn open_mark_note(&mut self, id: &str) -> AppResult<()> {
        if self.notes.selected.as_deref() == Some(id) {
            self.inspector = Inspector::Notes;
            self.inspector_visible = true;
            self.prefer_inspector = true;
            self.focus_mode = false;
            return Ok(());
        }
        self.activate_note(id)
    }
    pub(super) fn save_focused_note(&mut self, ctx: &egui::Context) -> Option<AppResult<()>> {
        let focused = ctx.memory(|memory| memory.focused());
        let id = self
            .notes
            .drafts
            .keys()
            .find(|id| focused == Some(egui::Id::new(("author-note-body", *id))))?
            .clone();
        Some(self.save_note_draft(&id))
    }
    pub(super) fn activate_note(&mut self, id: &str) -> AppResult<()> {
        self.notes.activate(id);
        self.inspector = Inspector::Notes;
        self.inspector_visible = true;
        self.prefer_inspector = true;
        self.focus_mode = false;
        if self.notes.invalid {
            return Ok(());
        }
        if let Some(note) = self
            .notes
            .preview
            .iter()
            .find(|note| note.id == id && note.anchor_status == AnchorStatus::Attached)
            && let Some(rich) = &mut self.rich
        {
            rich.session
                .set_selection(selection_for(note, rich.session.document())?)?;
            rich.session.request_scroll_to_caret();
            self.focus_manuscript = true;
        }
        Ok(())
    }
    /// Explicit Save only; partial conflicts keep all still-unsaved bodies reachable.
    pub(super) fn save_note_drafts(&mut self) -> AppResult<()> {
        let blocked_document = self
            .is_recovery_read_only()
            .then(|| self.document.as_ref().map(|document| document.id.clone()))
            .flatten();
        let mut blocked = false;
        if blocked_document.is_none() {
            self.save()?;
        }
        let drafts: Vec<_> = self
            .notes
            .drafts
            .iter()
            .filter(|(_, draft)| draft.body.trim() != draft.original.note.trim())
            .map(|(id, draft)| (id.clone(), draft.clone()))
            .collect();
        self.notes.invalidate();
        for (id, draft) in drafts {
            if blocked_document.as_ref() == Some(&draft.original.document_id) {
                blocked = true;
                continue;
            }
            let document = self.workspace.document(&draft.original.document_id)?;
            self.workspace.update_annotation(
                &id,
                document.revision,
                draft.original.revision,
                AnnotationPatch {
                    note: Some(draft.body),
                    status: None,
                },
            )?;
            self.notes.drafts.remove(&id);
        }
        let active = self
            .notes
            .composer
            .as_ref()
            .filter(|composer| !composer.body.trim().is_empty())
            .map(|composer| {
                (
                    composer.document_id.clone(),
                    composer.base_revision,
                    composer.anchor.clone(),
                    composer.body.clone(),
                )
            });
        if let Some((id, revision, anchor, body)) = active {
            if blocked_document.as_ref() == Some(&id) {
                blocked = true;
            } else {
                self.workspace
                    .create_annotation(&id, revision, anchor, &body)?;
                self.notes.composer = None;
            }
        }
        let paused: Vec<_> = self
            .notes
            .paused_composers
            .iter()
            .filter(|(_, composer)| !composer.body.trim().is_empty())
            .map(|(key, composer)| {
                (
                    key.clone(),
                    composer.document_id.clone(),
                    composer.base_revision,
                    composer.anchor.clone(),
                    composer.body.clone(),
                )
            })
            .collect();
        for (key, id, revision, anchor, body) in paused {
            if blocked_document.as_ref() == Some(&id) {
                blocked = true;
                continue;
            }
            self.workspace
                .create_annotation(&id, revision, anchor, &body)?;
            self.notes.paused_composers.remove(&key);
        }
        if self
            .document
            .as_ref()
            .is_some_and(|document| document.rich_json.is_some())
        {
            self.refresh_notes()?;
        }
        if blocked {
            return Err(recovery::ReadOnlyRecoveryView.into());
        }
        Ok(())
    }
    /// Called only after explicit Discard in the close confirmation.
    pub(super) fn discard_note_drafts(&mut self, ctx: &egui::Context) {
        for id in self.notes.drafts.keys() {
            egui::text_edit::TextEditState::default()
                .store(ctx, egui::Id::new(("author-note-body", id)));
        }
        if let Some(composer) = &self.notes.composer {
            egui::text_edit::TextEditState::default()
                .store(ctx, egui::Id::new(("note-composer-body", &composer.id)));
        }
        for composer in self.notes.paused_composers.values() {
            egui::text_edit::TextEditState::default()
                .store(ctx, egui::Id::new(("note-composer-body", &composer.id)));
        }
        self.notes.drafts.clear();
        self.notes.composer = None;
        self.notes.paused_composers.clear();
    }
    fn confirm_notes_close(&mut self, ctx: &egui::Context, t: Tokens) {
        let l = self.language;
        let mut save = false;
        let mut discard = false;
        let mut cancel = false;
        let response = components::modal(
            ctx,
            l,
            "unsaved-note-close",
            l.text("Save note drafts before closing?"),
            540.0,
            |ui| {
                ui.label(
                    l.text("Choose whether to save your note drafts before closing WriterCraft."),
                );
                components::footer(ui, |ui| {
                    save = components::action(ui, l.text("Save drafts"), true, Some(t)).clicked();
                    discard =
                        components::action(ui, l.text("Discard drafts"), true, None).clicked();
                    cancel = components::action(ui, l.text("Cancel"), true, None).clicked();
                });
            },
        );
        if save {
            let result = self.save_note_drafts();
            if result.is_ok() {
                self.notes.close_confirmation = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            self.record_error(result);
        } else if discard {
            self.discard_note_drafts(ctx);
            self.notes.close_confirmation = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if cancel || response.should_close() {
            self.notes.close_confirmation = false;
            if let Some(composer) = &mut self.notes.composer {
                composer.focus = true;
            }
        }
    }
    fn note_guard_after_flush(&mut self, before: &PassageAnnotation) -> AppResult<(i64, i64)> {
        self.ensure_note_writable()?;
        if self.notes.invalid {
            return Err("annotation_preview_unverified".into());
        }
        let prior = self.document.as_ref().ok_or("missing_note_document")?;
        if prior.id != before.document_id || prior.revision != before.base_revision {
            return Err("annotation_document_context_changed".into());
        }
        let prior_revision = prior.revision;
        self.save()?;
        let current = self.document.as_ref().ok_or("missing_note_document")?;
        let steps = current
            .revision
            .checked_sub(prior_revision)
            .ok_or("annotation_revision_moved_backwards")?;
        if steps > 1 {
            return Err("annotation_document_context_changed".into());
        }
        let expected = before
            .revision
            .checked_add(steps)
            .ok_or("annotation_revision_overflow")?;
        Ok((current.revision, expected))
    }
    fn refresh_notes(&mut self) -> AppResult<()> {
        let document = self.document.as_ref().ok_or("missing_note_document")?;
        self.notes.invalidate();
        self.notes.prepare(&self.workspace, document)
    }
    fn save_note_draft(&mut self, id: &str) -> AppResult<()> {
        let draft = self
            .notes
            .drafts
            .get(id)
            .cloned()
            .ok_or("missing_note_draft")?;
        let (doc_revision, note_revision) = self.note_guard_after_flush(&draft.original)?;
        let note = self.workspace.update_annotation(
            id,
            doc_revision,
            note_revision,
            AnnotationPatch {
                note: Some(draft.body),
                status: None,
            },
        )?;
        self.notes.drafts.remove(id);
        self.notes.acknowledge(&note);
        self.refresh_notes()
    }
    fn set_note_status(
        &mut self,
        note: &PassageAnnotation,
        status: AnnotationStatus,
    ) -> AppResult<()> {
        let (doc_revision, note_revision) = self.note_guard_after_flush(note)?;
        let updated = self.workspace.update_annotation(
            &note.id,
            doc_revision,
            note_revision,
            AnnotationPatch {
                note: None,
                status: Some(status),
            },
        )?;
        self.notes.acknowledge(&updated);
        self.refresh_notes()
    }
    fn reattach_note_selection(&mut self, note: &PassageAnnotation) -> AppResult<()> {
        let (doc_revision, note_revision) = self.note_guard_after_flush(note)?;
        let session = &self.rich.as_ref().ok_or("missing_note_editor")?.session;
        let (id, start, end, quote) = captured_selection(session)?;
        let anchor = self.workspace.capture_annotation_anchor(
            &note.document_id,
            doc_revision,
            &id,
            start,
            end,
            &quote,
        )?;
        let updated =
            self.workspace
                .reattach_annotation(&note.id, doc_revision, note_revision, anchor)?;
        self.notes.acknowledge(&updated);
        self.refresh_notes()
    }
    pub(super) fn rich_note_selection_menu(&mut self, response: &egui::Response) {
        let l = self.language;
        let allowed = self.can_edit_manuscript()
            && !self.notes.invalid
            && self
                .rich
                .as_ref()
                .is_some_and(|rich| captured_selection(&rich.session).is_ok());
        let keyboard = response.has_focus()
            && response
                .ctx
                .input_mut(|input| input.consume_key(egui::Modifiers::SHIFT, egui::Key::F10));
        let popup_id = response.id.with("rich-selection-menu");
        if keyboard {
            egui::Popup::open_id(&response.ctx, popup_id);
        }
        let mut popup = egui::Popup::context_menu(response).id(popup_id);
        if keyboard {
            let visible = response
                .interact_rect
                .intersect(response.ctx.content_rect());
            let visible = if visible.is_positive() {
                visible
            } else {
                response.ctx.content_rect().shrink(24.0)
            };
            let caret = match self
                .rich
                .as_ref()
                .map(|rich| rich.session.canvas_caret_rect(response))
                .transpose()
            {
                Ok(caret) => caret.flatten(),
                Err(error) => {
                    self.record_error(Err(error.into()));
                    None
                }
            };
            popup = popup.at_position(visible_menu_anchor(visible, caret));
        }
        popup.show(|ui| {
            if ui
                .add_enabled(allowed, egui::Button::new(l.text("Add note")))
                .clicked()
            {
                let result = self.begin_note();
                self.record_error(result);
                ui.close();
            }
            if ui.button(l.text("Notes")).clicked() {
                self.inspector = Inspector::Notes;
                self.inspector_visible = true;
                self.prefer_inspector = true;
                self.focus_mode = false;
                ui.close();
            }
        });
    }

    /// A bounded list/detail inspector; the parent must not wrap this in another ScrollArea.
    pub(super) fn notes_panel(&mut self, ui: &mut egui::Ui, t: Tokens) {
        let l = self.language;
        let writable = self.can_edit_manuscript();
        if self
            .document
            .as_ref()
            .is_none_or(|doc| doc.rich_json.is_none())
        {
            ui.label(l.text("Select a passage to add a note."));
            return;
        }
        if self.notes.invalid {
            ui.colored_label(
                t.warning,
                l.text("Notes could not be refreshed. Your text is preserved."),
            );
        }
        ui.horizontal(|ui| {
            let allowed = writable
                && !self.notes.invalid
                && self
                    .rich
                    .as_ref()
                    .is_some_and(|rich| captured_selection(&rich.session).is_ok());
            if components::action(ui, l.text("Add note"), allowed, Some(t)).clicked() {
                let result = self.begin_note();
                self.record_error(result);
            }
            if icons::icon_button(ui, Icon::History, l.text("Reload notes")).clicked() {
                let result = self.refresh_notes();
                self.record_error(result);
            }
        });
        ui.add_space(12.0);
        let mut activate = None;
        let list_height = (ui.available_height() * 0.38).clamp(100.0, 240.0);
        let mut list = egui::ScrollArea::vertical()
            .id_salt("document-notes-list")
            .max_height(list_height)
            .auto_shrink([false, false]);
        let rows: Vec<_> = self
            .notes
            .preview
            .iter()
            .filter(|note| note.status != AnnotationStatus::Resolved)
            .chain(
                self.notes
                    .preview
                    .iter()
                    .filter(|note| note.status == AnnotationStatus::Resolved),
            )
            .collect();
        if self.notes.reveal_row {
            if let Some(index) = rows
                .iter()
                .position(|note| Some(&note.id) == self.notes.selected.as_ref())
            {
                list = list.vertical_scroll_offset(
                    (index as f32 * (44.0 + ui.spacing().item_spacing.y) - list_height * 0.5
                        + 22.0)
                        .max(0.0),
                );
            }
            self.notes.reveal_row = false;
        }
        list.show_rows(ui, 44.0, rows.len(), |ui, range| {
            for note in &rows[range] {
                let selected = self.notes.selected.as_ref() == Some(&note.id);
                let tree = if self.notes.invalid {
                    None
                } else {
                    self.rich.as_ref().map(|rich| rich.session.document())
                };
                let quote = displayed_quote(&note.anchor, tree, l);
                if ui
                    .push_id(&note.id, |ui| note_row(ui, note, &quote, selected, l, t))
                    .inner
                    .clicked()
                {
                    activate = Some(note.id.clone());
                }
            }
        });
        if rows.is_empty() {
            ui.label(RichText::new(l.text("No notes in this document.")).color(t.muted));
        }
        drop(rows);
        if let Some(id) = activate {
            let result = self.activate_note(&id);
            self.record_error(result);
        }
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);
        let selected = self
            .notes
            .selected
            .as_ref()
            .and_then(|id| self.notes.preview.iter().find(|note| &note.id == id))
            .or_else(|| {
                self.notes
                    .preview
                    .iter()
                    .find(|note| note.status != AnnotationStatus::Resolved)
            })
            .or_else(|| self.notes.preview.first())
            .cloned();
        let Some(note) = selected else {
            ui.label(RichText::new(l.text("Select a passage to add a note.")).color(t.muted));
            return;
        };
        self.notes.selected = Some(note.id.clone());
        let mut save = false;
        let mut toggle = false;
        let mut reattach = false;
        let mut remove = false;
        let mut reveal = false;
        let mut reload_draft = false;
        let quote = displayed_quote(
            &note.anchor,
            self.rich.as_ref().map(|rich| rich.session.document()),
            l,
        );
        egui::ScrollArea::vertical()
            .id_salt(("document-note-details", &note.id))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Frame::new()
                    .fill(t.surface)
                    .stroke(Stroke::new(1.0, t.border))
                    .inner_margin(12)
                    .corner_radius(6)
                    .show(ui, |ui| {
                        ui.label(RichText::new(excerpt(&quote, 420)).size(14.0).strong());
                    });
                ui.add_space(12.0);
                if note.anchor_status == AnchorStatus::Detached || note.status == AnnotationStatus::NeedsAttention {
                    ui.colored_label(t.warning, l.text("The passage changed. The note is preserved; select text to attach it again."));
                    let allowed = writable && !self.notes.invalid && self.rich.as_ref().is_some_and(|rich| captured_selection(&rich.session).is_ok());
                    reattach = components::action(ui, l.text("Attach to selection"), allowed, None).clicked();
                    ui.add_space(12.0);
                }
                ui.label(RichText::new(l.text("Author note")).strong());
                let draft = self.notes.drafts.entry(note.id.clone()).or_insert_with(|| NoteDraft {
                    original: note.clone(), body: note.note.clone(),
                });
                let conflicted = draft.original.revision != note.revision;
                if conflicted {
                    ui.colored_label(t.warning, l.text("This note changed elsewhere. Your draft is preserved."));
                    ui.horizontal_wrapped(|ui| {
                        if components::action(ui, l.text("Copy draft"), true, None).clicked() {
                            ui.ctx().copy_text(draft.body.clone());
                        }
                        reload_draft = components::action(ui, l.text("Discard draft and reload"), true, None).clicked();
                    });
                    ui.add_space(12.0);
                }
                note_input(ui, &mut draft.body, egui::Id::new(("author-note-body", &note.id)), l, t, !writable);
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    if draft.body.trim() != note.note.trim() {
                        save = components::action(ui, l.text("Save note"), valid_note_body(&draft.body) && !conflicted && !self.notes.invalid && writable, Some(t)).clicked();
                    }
                    toggle = components::action(ui, l.text(if note.status == AnnotationStatus::Resolved {"Reopen note"} else {"Resolve note"}), !self.notes.invalid && writable, None).clicked();
                    reveal = components::action(ui, l.text("Show passage"), note.anchor_status == AnchorStatus::Attached && !self.notes.invalid, None).clicked();
                    remove = components::action(ui, l.text("Delete note"), !self.notes.invalid && writable, None).clicked();
                });
            });
        if reload_draft {
            let id = egui::Id::new(("author-note-body", &note.id));
            egui::text_edit::TextEditState::default().store(ui.ctx(), id);
            self.notes.drafts.insert(
                note.id.clone(),
                NoteDraft {
                    original: note.clone(),
                    body: note.note.clone(),
                },
            );
        }
        if save {
            let result = self.save_note_draft(&note.id);
            self.record_error(result);
        }
        if toggle {
            let result = self.set_note_status(
                &note,
                if note.status == AnnotationStatus::Resolved {
                    AnnotationStatus::Open
                } else {
                    AnnotationStatus::Resolved
                },
            );
            self.record_error(result);
        }
        if reattach {
            let result = self.reattach_note_selection(&note);
            self.record_error(result);
        }
        if reveal {
            let result = self.activate_note(&note.id);
            self.record_error(result);
        }
        if remove {
            self.notes.delete_confirmation = Some(note);
        }
    }

    pub(super) fn notes_dialogs(&mut self, ctx: &egui::Context, t: Tokens) {
        let l = self.language;
        let writable = self.can_edit_manuscript();
        if self.notes.close_confirmation {
            self.confirm_notes_close(ctx, t);
            return;
        }
        if let Some(mut composer) = self.notes.composer.take() {
            let mut save = false;
            let mut cancel = false;
            let response = components::modal(
                ctx,
                l,
                "new-passage-note",
                l.text("Add note"),
                540.0,
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt(("new-note-body-scroll", &composer.id))
                        .max_height((ctx.content_rect().height() - 300.0).clamp(0.0, 440.0))
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.label(RichText::new(l.text("Selected passage")).strong());
                            ui.label(excerpt(
                                &displayed_quote(
                                    &composer.anchor,
                                    self.rich.as_ref().map(|rich| rich.session.document()),
                                    l,
                                ),
                                480,
                            ));
                            ui.add_space(16.0);
                            ui.label(l.text("Author note"));
                            let field = note_input(
                                ui,
                                &mut composer.body,
                                egui::Id::new(("note-composer-body", &composer.id)),
                                l,
                                t,
                                !writable,
                            );
                            if composer.focus {
                                field.request_focus();
                                composer.focus = false;
                            }
                        });
                    components::footer(ui, |ui| {
                        save = components::action(
                            ui,
                            l.text("Add note"),
                            valid_note_body(&composer.body) && !self.notes.invalid && writable,
                            Some(t),
                        )
                        .clicked();
                        cancel = components::action(ui, l.text("Cancel"), true, None).clicked();
                    });
                },
            );
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::S))
                && valid_note_body(&composer.body)
                && !self.notes.invalid
                && writable
            {
                save = true;
            }
            if save {
                let result = self
                    .workspace
                    .create_annotation(
                        &composer.document_id,
                        composer.base_revision,
                        composer.anchor.clone(),
                        &composer.body,
                    )
                    .map_err(Into::into)
                    .and_then(|note| {
                        self.notes.activate(&note.id);
                        self.inspector = Inspector::Notes;
                        self.inspector_visible = true;
                        self.prefer_inspector = true;
                        self.focus_mode = false;
                        self.refresh_notes()
                    });
                if result.is_err() {
                    self.notes.composer = Some(composer);
                }
                self.record_error(result);
            } else if !cancel && !response.should_close() {
                self.notes.composer = Some(composer);
            } else {
                self.focus_manuscript = true;
            }
        }
        if let Some(note) = self.notes.delete_confirmation.take() {
            let mut confirm = false;
            let mut cancel = false;
            let response = components::modal(
                ctx,
                l,
                "delete-passage-note",
                l.text("Delete note?"),
                440.0,
                |ui| {
                    ui.label(
                        RichText::new(excerpt(
                            &displayed_quote(
                                &note.anchor,
                                self.rich.as_ref().map(|rich| rich.session.document()),
                                l,
                            ),
                            180,
                        ))
                        .strong(),
                    );
                    ui.label(excerpt(&note.note, 180));
                    ui.add_space(12.0);
                    ui.label(l.text("The manuscript text will stay unchanged."));
                    components::footer(ui, |ui| {
                        confirm = components::action(
                            ui,
                            l.text("Delete note"),
                            !self.notes.invalid && writable,
                            Some(t),
                        )
                        .clicked();
                        cancel = components::action(ui, l.text("Cancel"), true, None).clicked();
                    });
                },
            );
            if confirm {
                let result =
                    self.note_guard_after_flush(&note)
                        .and_then(|(doc_revision, note_revision)| {
                            self.workspace.delete_annotation(
                                &note.id,
                                doc_revision,
                                note_revision,
                            )?;
                            self.notes.drafts.remove(&note.id);
                            self.notes.selected = None;
                            self.refresh_notes()
                        });
                if result.is_err() {
                    self.notes.delete_confirmation = Some(note);
                }
                self.record_error(result);
            } else if !cancel && !response.should_close() {
                self.notes.delete_confirmation = Some(note);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_editor_model::{Command, Editor, TextRange, Transaction};
    type TestResult = AppResult<()>;
    fn fixture() -> AppResult<(Workspace, Document, PassageAnnotation, Editor)> {
        let workspace = Workspace::memory()?;
        let project = workspace.create_project("Synthetic note controller")?;
        let tree = RichTree::plain("p", "Я😀 echo echo")?;
        let document = workspace.create_rich_document(&project.id, "Notes", tree.to_json()?)?;
        let anchor = workspace.capture_annotation_anchor(
            &document.id,
            document.revision,
            "p",
            ScalarOffset(8),
            ScalarOffset(12),
            "echo",
        )?;
        let note = workspace.create_annotation(
            &document.id,
            document.revision,
            anchor,
            "Revise this passage",
        )?;
        Ok((workspace, document, note, Editor::new(tree, Revision(1))))
    }
    #[test]
    fn frame_maps_fresh_native_input_purely_without_metadata_writes() -> TestResult {
        let (workspace, document, note, mut editor) = fixture()?;
        let mut state = NotesState::default();
        state.prepare(&workspace, &document)?;
        let change = editor.apply(Transaction {
            base_revision: editor.revision(),
            commands: vec![Command::ReplaceText {
                range: TextRange {
                    block_id: "p".into(),
                    start: ScalarOffset(10),
                    end: ScalarOffset(10),
                },
                expected: "".into(),
                replacement: "中".into(),
            }],
        })?;
        let tokens = crate::theme::tokens(&egui::Context::default());
        let marks = state.frame(&[], Language::English, tokens).decorations(
            editor.document(),
            editor.revision(),
            &[change.replay_proof()],
        )?;
        assert_eq!(marks.len(), 1);
        assert_eq!(marks[0].range.head.offset, ScalarOffset(13));
        assert_eq!(workspace.annotation(&note.id)?, note);
        Ok(())
    }
    #[test]
    fn refreshing_metadata_preserves_dirty_draft_and_detects_the_conflict() -> TestResult {
        let (workspace, document, note, _) = fixture()?;
        let mut state = NotesState::default();
        state.prepare(&workspace, &document)?;
        state.drafts.insert(
            note.id.clone(),
            NoteDraft {
                original: note.clone(),
                body: "My unsaved draft".into(),
            },
        );
        workspace.update_annotation(
            &note.id,
            document.revision,
            note.revision,
            AnnotationPatch {
                note: Some("Changed elsewhere".into()),
                status: None,
            },
        )?;
        state.invalidate();
        state.prepare(&workspace, &document)?;
        let draft = state
            .drafts
            .get(&note.id)
            .ok_or("missing preserved draft")?;
        let current = state
            .preview
            .iter()
            .find(|current| current.id == note.id)
            .ok_or("missing current note")?;
        assert_eq!(draft.body, "My unsaved draft");
        assert_ne!(draft.original.revision, current.revision);
        Ok(())
    }
    #[test]
    fn changed_document_clears_selection_but_preserves_authored_drafts_for_return() -> TestResult {
        let (workspace, document, note, _) = fixture()?;
        let mut state = NotesState::default();
        state.prepare(&workspace, &document)?;
        state.activate(&note.id);
        state.drafts.insert(
            note.id.clone(),
            NoteDraft {
                original: note,
                body: "Private draft".into(),
            },
        );
        let other = workspace.create_rich_document(
            &document.project_id,
            "Other",
            RichTree::plain("other", "Text")?.to_json()?,
        )?;
        state.prepare(&workspace, &other)?;
        assert!(state.selected.is_none());
        assert_eq!(state.drafts.len(), 1);
        assert!(state.has_unsaved_drafts());
        assert!(state.preview.is_empty());
        state.prepare(&workspace, &document)?;
        assert_eq!(
            state
                .drafts
                .values()
                .next()
                .ok_or("missing preserved draft")?
                .body,
            "Private draft"
        );
        Ok(())
    }
    #[test]
    fn displayed_note_quote_uses_authored_math_and_never_exposes_native_atom_scalars() -> TestResult
    {
        let tree = RichTree::from_json(
            serde_json::json!({"type":"doc","attrs":{"schemaVersion":4},"content":[{"type":"paragraph","attrs":{"blockId":"p"},"content":[{"type":"text","text":"中"},{"type":"inlineMath","attrs":{"latex":"x^2"}},{"type":"text","text":"文"}]}]}),
        )?;
        let anchor = AnnotationAnchor::from_selection(
            &tree,
            &TextPoint {
                block_id: "p".into(),
                offset: ScalarOffset(0),
            },
            &TextPoint {
                block_id: "p".into(),
                offset: ScalarOffset(3),
            },
        )?;
        assert_eq!(anchor.exact_text, "中\u{fffc}文");
        assert_eq!(
            displayed_quote(&anchor, Some(&tree), Language::English),
            "中$x^2$文"
        );
        for language in Language::ALL {
            assert!(!displayed_quote(&anchor, None, language).contains('\u{fffc}'));
        }
        Ok(())
    }
    fn field_frame(
        ctx: &egui::Context,
        body: &mut String,
        id: egui::Id,
        events: Vec<egui::Event>,
        time: f64,
    ) {
        ctx.memory_mut(|memory| memory.request_focus(id));
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(640.0, 480.0),
                )),
                time: Some(time),
                focused: true,
                events,
                ..Default::default()
            },
            |root| {
                egui::CentralPanel::default().show(root, |ui| {
                    note_input(ui, body, id, Language::English, theme::tokens(ctx), false);
                });
            },
        );
        output.textures_delta.clear();
    }
    #[test]
    fn full_unicode_paste_is_preserved_above_validation_limit() {
        let ctx = egui::Context::default();
        let id = egui::Id::new(("author-note-body", "large"));
        let mut body = String::new();
        field_frame(&ctx, &mut body, id, vec![], 0.0);
        let original = "Я😀中".repeat(3_334);
        field_frame(
            &ctx,
            &mut body,
            id,
            vec![egui::Event::Paste(original.clone())],
            1.0,
        );
        assert_eq!(body, original);
        assert!(!valid_note_body(&body));
    }
    #[test]
    fn note_input_undo_history_cannot_restore_another_notes_body() {
        let ctx = egui::Context::default();
        let a = egui::Id::new(("author-note-body", "a"));
        let b = egui::Id::new(("author-note-body", "b"));
        let mut first = String::from("Alpha");
        field_frame(&ctx, &mut first, a, vec![], 0.0);
        field_frame(
            &ctx,
            &mut first,
            a,
            vec![egui::Event::Text("X".into())],
            1.0,
        );
        let mut second = String::from("Beta");
        field_frame(
            &ctx,
            &mut second,
            b,
            vec![egui::Event::Key {
                key: egui::Key::Z,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            }],
            2.0,
        );
        assert_eq!(second, "Beta");
        assert!(first.contains('X'));
    }
    #[test]
    fn failed_new_document_snapshot_never_exposes_previous_documents_notes_on_retry() -> TestResult
    {
        let (workspace, document, note, _) = fixture()?;
        let mut state = NotesState::default();
        state.prepare(&workspace, &document)?;
        state.activate(&note.id);
        let mut other = workspace.create_rich_document(
            &document.project_id,
            "Identical template",
            document.rich_json.clone().ok_or("missing tree")?,
        )?;
        other.revision += 1; // Simulate a stale captured document revision during an external update.
        assert!(state.prepare(&workspace, &other).is_err());
        assert!(state.preview.is_empty());
        assert!(state.selected.is_none());
        assert!(state.invalid);
        state.prepare(&workspace, &other)?;
        assert!(state.preview.is_empty());
        assert!(state.selected.is_none());
        assert!(state.invalid);
        Ok(())
    }
    #[test]
    fn failed_note_context_does_not_raise_new_errors_for_each_prose_revision() -> TestResult {
        let (workspace, document, _, mut editor) = fixture()?;
        let mut state = NotesState::default();
        let mut stale = document.clone();
        stale.revision += 1;
        assert!(state.prepare(&workspace, &stale).is_err());
        let t = theme::tokens(&egui::Context::default());
        for revision in 1..=3 {
            let mut frame = state.frame(&[], Language::English, t);
            assert!(
                frame
                    .decorations(editor.document(), Revision(revision), &[])
                    .is_err()
            );
            assert!(frame.take_error().is_none());
        }
        state.invalidate();
        state.prepare(&workspace, &document)?;
        editor.apply(Transaction {
            base_revision: editor.revision(),
            commands: vec![Command::ReplaceText {
                range: TextRange {
                    block_id: "p".into(),
                    start: ScalarOffset(0),
                    end: ScalarOffset(0),
                },
                expected: "".into(),
                replacement: "New ".into(),
            }],
        })?;
        for (index, revision) in (5..=7).enumerate() {
            let mut frame = state.frame(&[], Language::English, t);
            // Deliberately omit the required proofs: derived projection fails,
            // but only the first failure in this metadata epoch is reported.
            assert!(
                frame
                    .decorations(editor.document(), Revision(revision), &[])
                    .is_err()
            );
            assert_eq!(frame.take_error().is_some(), index == 0);
        }
        Ok(())
    }
    #[test]
    fn keyboard_note_menu_tracks_measured_caret_and_stays_in_visible_scrolled_editor() {
        let visible = egui::Rect::from_min_max(egui::pos2(300.0, 160.0), egui::pos2(900.0, 650.0));
        let caret = egui::Rect::from_min_size(egui::pos2(420.0, 360.0), egui::vec2(1.0, 24.0));
        assert_eq!(
            visible_menu_anchor(visible, Some(caret)),
            egui::pos2(420.0, 388.0)
        );
        let offscreen = caret.translate(egui::vec2(0.0, -2400.0));
        assert!(visible.contains(visible_menu_anchor(visible, Some(offscreen))));
        assert!(visible.contains(visible_menu_anchor(visible, None)));
    }
    #[test]
    fn saved_recovery_view_preserves_blocked_note_drafts_and_can_save_other_documents() -> TestResult
    {
        let directory = tempfile::tempdir()?;
        let context = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(context);
        let mut app = StudioApp::new(
            &creation,
            directory.path().join("readonly-recovery.sqlite"),
            false,
            None,
        )?;
        let project = app.workspace.create_project("Recovery guard")?;
        let a = app.workspace.create_rich_document(
            &project.id,
            "Saved",
            RichTree::plain("a", "echo")?.to_json()?,
        )?;
        let b = app.workspace.create_rich_document(
            &project.id,
            "Other",
            RichTree::plain("b", "other")?.to_json()?,
        )?;
        let aa = app.workspace.capture_annotation_anchor(
            &a.id,
            a.revision,
            "a",
            ScalarOffset(0),
            ScalarOffset(4),
            "echo",
        )?;
        let ba = app.workspace.capture_annotation_anchor(
            &b.id,
            b.revision,
            "b",
            ScalarOffset(0),
            ScalarOffset(5),
            "other",
        )?;
        let an = app
            .workspace
            .create_annotation(&a.id, a.revision, aa, "Saved note")?;
        let bn = app
            .workspace
            .create_annotation(&b.id, b.revision, ba, "Other note")?;
        let draft = app.workspace.save_rich_draft(
            &a.id,
            a.revision,
            None,
            RichTree::plain("a", "Unsaved preserved prose")?.to_json()?,
        )?;
        let snapshot = app.workspace.rich_recovery_snapshot(&a.id)?;
        app.open_saved_recovery(&snapshot)?;
        assert!(app.is_recovery_read_only());
        assert!(!app.can_edit_manuscript());
        app.notes.drafts.insert(
            an.id.clone(),
            NoteDraft {
                original: an.clone(),
                body: "Preserved local note".into(),
            },
        );
        app.notes.drafts.insert(
            bn.id.clone(),
            NoteDraft {
                original: bn.clone(),
                body: "Saved parked note".into(),
            },
        );
        assert!(app.begin_note().is_err());
        assert!(app.save_note_drafts().is_err());
        assert_eq!(app.workspace.annotation(&an.id)?, an);
        assert_eq!(app.workspace.annotation(&bn.id)?.note, "Saved parked note");
        assert_eq!(
            app.notes
                .drafts
                .get(&an.id)
                .ok_or("missing blocked draft")?
                .body,
            "Preserved local note"
        );
        assert_eq!(app.workspace.rich_draft(&a.id)?, Some(draft));
        assert_eq!(app.workspace.document(&a.id)?, a);
        Ok(())
    }
}
