//! Persistent passage-note presentation over guarded core commands and ephemeral native ranges.
use super::*;
use craft_editor_egui::{
    DocPoint, Selection,
    decorations::{DecorationSource, TextDecoration},
};
use craft_editor_model::{Document as RichTree, ReplayProof, Revision, ScalarOffset, TextPoint};
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
    delete_confirmation: Option<PassageAnnotation>,
    reveal_row: bool,
    last_error: Option<Box<dyn std::error::Error + Send + Sync>>,
}
impl NotesState {
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }
    pub(super) fn modal_open(&self) -> bool {
        self.composer.is_some() || self.delete_confirmation.is_some()
    }
    pub(super) fn invalidate(&mut self) {
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
        let prior_revision = self
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.base_revision());
        let prior_id = self
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.document_id().to_owned());
        let snapshot = workspace.annotation_preview_snapshot(&document.id, document.revision)?;
        if prior_id.as_deref().is_some_and(|id| id != document.id) {
            self.selected = None;
            self.drafts.clear();
            self.composer = None;
            self.delete_confirmation = None;
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
                }
                Err(error) => {
                    self.state.invalid = true;
                    self.state.last_error = Some(error);
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
    pub(super) fn begin_note(&mut self) -> AppResult<()> {
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
            document_id: document.id.clone(),
            base_revision: document.revision,
            anchor,
            body: String::new(),
            focus: true,
        });
        Ok(())
    }
    pub(super) fn activate_note(&mut self, id: &str) -> AppResult<()> {
        self.notes.activate(id);
        self.inspector = Inspector::Notes;
        self.inspector_visible = true;
        self.prefer_inspector = true;
        self.focus_mode = false;
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
    fn note_guard_after_flush(&mut self, before: &PassageAnnotation) -> AppResult<(i64, i64)> {
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
        let allowed = self
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
            popup = popup.at_position(response.rect.left_top() + egui::vec2(28.0, 28.0));
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
        if self
            .document
            .as_ref()
            .is_none_or(|doc| doc.rich_json.is_none())
        {
            ui.label(l.text("Select a passage to add a note."));
            return;
        }
        if let Some(document) = &self.document {
            let result = self.notes.prepare(&self.workspace, document);
            self.record_error(result);
        }
        if self.notes.invalid {
            ui.colored_label(
                t.warning,
                l.text("Notes could not be refreshed. Your text is preserved."),
            );
        }
        ui.horizontal(|ui| {
            let allowed = self
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
                    (index as f32 * 44.0 - list_height * 0.5 + 22.0).max(0.0),
                );
            }
            self.notes.reveal_row = false;
        }
        list.show_rows(ui, 44.0, rows.len(), |ui, range| {
            for note in &rows[range] {
                let selected = self.notes.selected.as_ref() == Some(&note.id);
                let mut label = excerpt(&note.note, 72);
                if note.status == AnnotationStatus::Resolved {
                    label = format!("{} · {}", l.text("Resolved"), label);
                }
                if nav_row(ui, Icon::Assistant, &label, selected, t).clicked() {
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
        egui::ScrollArea::vertical().id_salt("document-note-details").auto_shrink([false,false]).show(ui,|ui|{
            egui::Frame::new().fill(t.surface).stroke(Stroke::new(1.0,t.border)).inner_margin(12).corner_radius(6).show(ui,|ui|{
                ui.label(RichText::new(excerpt(&note.anchor.exact_text,420)).size(14.0).strong());
            });
            ui.add_space(12.0);
            if note.anchor_status==AnchorStatus::Detached || note.status==AnnotationStatus::NeedsAttention {
                ui.colored_label(t.warning,l.text("The passage changed. The note is preserved; select text to attach it again."));
                let allowed=self.rich.as_ref().is_some_and(|rich|captured_selection(&rich.session).is_ok());
                reattach=ui.add_enabled(allowed,egui::Button::new(l.text("Attach to selection"))).clicked();ui.add_space(12.0);
            }
            ui.label(RichText::new(l.text("Author note")).strong());
            let draft=self.notes.drafts.entry(note.id.clone()).or_insert_with(||NoteDraft{original:note.clone(),body:note.note.clone()});
            let conflicted = draft.original.revision != note.revision;
            if conflicted {
                ui.colored_label(t.warning, l.text("This note changed elsewhere. Your draft is preserved."));
                ui.horizontal_wrapped(|ui| {
                    if ui.button(l.text("Copy draft")).clicked() {
                        ui.ctx().copy_text(draft.body.clone());
                    }
                    reload_draft = ui.button(l.text("Reload current note")).clicked();
                });
                ui.add_space(12.0);
            }
            ui.add(egui::TextEdit::multiline(&mut draft.body).desired_width(f32::INFINITY).desired_rows(6).margin(12).char_limit(10_000));
            ui.add_space(12.0);
            ui.horizontal_wrapped(|ui|{
                if draft.body.trim()!=note.note.trim(){save=components::action(ui,l.text("Save note"),!draft.body.trim().is_empty() && !conflicted,Some(t)).clicked();}
                toggle=components::action(ui,l.text(if note.status==AnnotationStatus::Resolved{"Reopen note"}else{"Resolve note"}),true,None).clicked();
                reveal=components::action(ui,l.text("Show passage"),note.anchor_status==AnchorStatus::Attached,None).clicked();
                remove=components::action(ui,l.text("Delete note"),true,None).clicked();
            });
        });
        if reload_draft {
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
                    ui.label(RichText::new(l.text("Selected passage")).strong());
                    ui.label(excerpt(&composer.anchor.exact_text, 480));
                    ui.add_space(16.0);
                    ui.label(l.text("Author note"));
                    let field = ui.add(
                        egui::TextEdit::multiline(&mut composer.body)
                            .desired_rows(6)
                            .desired_width(f32::INFINITY)
                            .margin(12)
                            .char_limit(10_000),
                    );
                    if composer.focus {
                        field.request_focus();
                        composer.focus = false;
                    }
                    components::footer(ui, |ui| {
                        save = components::action(
                            ui,
                            l.text("Add note"),
                            !composer.body.trim().is_empty(),
                            Some(t),
                        )
                        .clicked();
                        cancel = components::action(ui, l.text("Cancel"), true, None).clicked();
                    });
                },
            );
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
                    ui.label(l.text("The manuscript text will stay unchanged."));
                    components::footer(ui, |ui| {
                        confirm =
                            components::action(ui, l.text("Delete note"), true, Some(t)).clicked();
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
    fn changed_document_clears_previous_note_selection_and_drafts() -> TestResult {
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
        assert!(state.drafts.is_empty());
        assert!(state.preview.is_empty());
        Ok(())
    }
}
