//! Source-backed Find and inline document-title editing.
use super::*;

#[derive(Default)]
pub(super) struct DailyControls {
    find: Option<FindDialog>,
    title: Option<TitleDraft>,
}

impl DailyControls {
    pub(super) fn modal_open(&self) -> bool {
        self.find.is_some()
    }

    pub(super) fn editing_title(&self) -> bool {
        self.title.is_some()
    }

    pub(super) fn title_has_focus(&self, ctx: &egui::Context) -> bool {
        self.title.as_ref().is_some_and(|draft| {
            draft.composing
                || draft
                    .field_id
                    .is_some_and(|id| ctx.memory(|memory| memory.has_focus(id)))
        })
    }

    /// Navigation closes Find after the host has successfully committed title metadata.
    /// A title draft is deliberately retained if a host forgets the commit guard.
    pub(super) fn document_changed(&mut self) {
        self.find = None;
    }
}

struct FindDialog {
    document_id: String,
    project_id: String,
    query: String,
    focus: bool,
    error: Option<&'static str>,
    // Plain fields retain an exact snapshot; rich sessions use their semantic revision.
    plain_body: Option<String>,
    #[cfg(feature = "editor-preview")]
    session_revision: Option<craft_editor_model::Revision>,
}

struct TitleDraft {
    document_id: String,
    project_id: String,
    expected_title: String,
    value: String,
    focus: bool,
    error: Option<String>,
    field_id: Option<egui::Id>,
    field_context: Option<egui::Context>,
    field_focused: bool,
    composing: bool,
}

fn clear_title_input_state(draft: &TitleDraft) {
    if let (Some(ctx), Some(id)) = (&draft.field_context, draft.field_id) {
        ctx.data_mut(|data| data.remove::<egui::text_edit::TextEditState>(id));
        ctx.memory_mut(|memory| memory.surrender_focus(id));
    }
}

impl StudioApp {
    /// Shared pointer/keyboard entry point. It never replaces or normalizes manuscript text.
    pub(super) fn open_document_find(&mut self) {
        if self.modal_open() {
            return;
        }
        let Some(document) = &self.document else {
            return;
        };
        #[cfg(feature = "editor-preview")]
        if self
            .rich
            .as_ref()
            .is_some_and(|rich| rich.session.is_composing())
        {
            self.status = "Finish text composition before finding.".into();
            return;
        }
        #[cfg(feature = "editor-preview")]
        let session_revision = self.rich.as_ref().map(|rich| rich.session.revision());
        #[cfg(feature = "editor-preview")]
        let plain_body = session_revision.is_none().then(|| self.body.clone());
        #[cfg(not(feature = "editor-preview"))]
        let plain_body = Some(self.body.clone());
        self.daily_controls.find = Some(FindDialog {
            document_id: document.id.clone(),
            project_id: document.project_id.clone(),
            query: String::new(),
            focus: true,
            error: None,
            plain_body,
            #[cfg(feature = "editor-preview")]
            session_revision,
        });
    }

    /// Commit title metadata before Save, navigation or close. On failure the exact
    /// title draft and current manuscript context remain available for correction.
    pub(super) fn commit_title_edit(&mut self) -> AppResult<()> {
        if self.is_recovery_read_only() && self.daily_controls.editing_title() {
            return Err(studio_core::StudioError::ReadOnly {
                document_id: self
                    .document
                    .as_ref()
                    .map_or_else(String::new, |document| document.id.clone()),
            }
            .into());
        }
        let Some(mut draft) = self.daily_controls.title.take() else {
            return Ok(());
        };
        let result = (|| -> AppResult<()> {
            if draft.composing {
                self.status = "Finish text composition before saving the title.".into();
                return Err(studio_core::StudioError::Validation(self.status.clone()).into());
            }
            if !self.document.as_ref().is_some_and(|document| {
                document.id == draft.document_id && document.project_id == draft.project_id
            }) {
                return Err(studio_core::StudioError::Validation(
                    "The document changed; return to the document before saving its title".into(),
                )
                .into());
            }
            let title = draft.value.trim();
            if title.is_empty() || title == draft.expected_title.trim() {
                return Ok(());
            }
            if title.chars().count() > 200 {
                self.status = "Document titles can contain at most 200 characters.".into();
                return Err(studio_core::StudioError::Validation(self.status.clone()).into());
            }
            let renamed = self.workspace.rename_document(
                &draft.project_id,
                &draft.document_id,
                &draft.expected_title,
                title,
            )?;
            for listed in &mut self.documents {
                if listed.id == renamed.id {
                    listed.title.clone_from(&renamed.title);
                }
            }
            // Updating metadata in place keeps the live text session, drafts and Undo intact.
            if let Some(active) = &mut self.document {
                active.title.clone_from(&renamed.title);
            }
            self.status = if self.dirty.is_some() {
                "Saving…"
            } else {
                "All changes saved"
            }
            .into();
            Ok(())
        })();
        if let Err(error) = result {
            if !draft.composing && draft.value.trim().chars().count() <= 200 {
                self.status = "The title could not be saved. Your draft is preserved.".into();
            }
            draft.error = Some(error.to_string());
            self.daily_controls.title = Some(draft);
            return Err(error);
        }
        clear_title_input_state(&draft);
        Ok(())
    }

    /// Render within the existing allocated title region; the field uses shared geometry.
    pub(super) fn document_title(
        &mut self,
        ui: &mut egui::Ui,
        document: &Document,
        t: Tokens,
    ) -> egui::Response {
        let l = self.language;
        if self.daily_controls.title.as_ref().is_some_and(|draft| {
            draft.document_id != document.id || draft.project_id != document.project_id
        }) {
            // A host navigation guard must not be able to destroy an older title draft.
            return ui.add(
                egui::Label::new(
                    RichText::new(&document.title)
                        .size(18.0)
                        .strong()
                        .color(t.text),
                )
                .truncate(),
            );
        }
        let active = self.daily_controls.title.take();
        if let Some(mut draft) = active {
            let had_focus = draft
                .field_id
                .is_some_and(|id| ui.memory(|memory| memory.has_focus(id)));
            let was_composing = draft.composing;
            let escape = (had_focus || draft.field_focused)
                && ui
                    .input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            if had_focus {
                ui.input(|input| {
                    for event in &input.events {
                        match event {
                            egui::Event::Ime(egui::ImeEvent::Preedit { text, .. }) => {
                                draft.composing = !text.is_empty()
                            }
                            egui::Event::Ime(egui::ImeEvent::Commit(_)) => draft.composing = false,
                            _ => {}
                        }
                    }
                });
            }
            let field = ui
                .push_id(("inline-document-title", &draft.document_id), |ui| {
                    components::singleline(ui, &mut draft.value, l.text("Document title"))
                })
                .inner;
            draft.field_id = Some(field.id);
            draft.field_context = Some(ui.ctx().clone());
            draft.field_focused = field.has_focus();
            if draft.focus {
                field.request_focus();
                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), field.id) {
                    state.clear_undoer();
                    state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::two(
                            egui::text::CCursor::new(0),
                            egui::text::CCursor::new(draft.value.chars().count()),
                        )));
                    state.store(ui.ctx(), field.id);
                }
                draft.focus = false;
            }
            draft.field_focused = field.has_focus();
            let enter = ui.input(|input| input.key_pressed(egui::Key::Enter));
            if escape {
                clear_title_input_state(&draft);
                self.focus_manuscript = true;
            } else if !was_composing
                && !draft.composing
                && (field.lost_focus() || (field.has_focus() && enter))
            {
                self.daily_controls.title = Some(draft);
                let committed = self.commit_title_edit().is_ok();
                if enter && committed {
                    self.focus_manuscript = true;
                }
            } else {
                self.daily_controls.title = Some(draft);
            }
            if let Some(error) = self
                .daily_controls
                .title
                .as_ref()
                .and_then(|draft| draft.error.as_ref())
            {
                return field.on_hover_text(format!(
                    "{}\n{error}",
                    l.text("The title could not be saved. Your draft is preserved.")
                ));
            }
            field
        } else {
            let response = ui
                .add(
                    egui::Label::new(
                        RichText::new(&document.title)
                            .size(18.0)
                            .strong()
                            .color(t.text),
                    )
                    .truncate()
                    .sense(egui::Sense::click()),
                )
                .on_hover_text(l.text("Double-click to rename. Enter saves; Escape cancels."));
            if response.double_clicked() && !self.modal_open() {
                self.daily_controls.title = Some(TitleDraft {
                    document_id: document.id.clone(),
                    project_id: document.project_id.clone(),
                    expected_title: document.title.clone(),
                    value: document.title.clone(),
                    focus: true,
                    error: None,
                    field_id: None,
                    field_context: None,
                    field_focused: false,
                    composing: false,
                });
                ui.ctx().request_repaint();
            }
            response
        }
    }

    /// Called with other app dialogs after the manuscript node has been emitted.
    pub(super) fn daily_control_dialogs(&mut self, ctx: &egui::Context, t: Tokens) {
        let Some(mut draft) = self.daily_controls.find.take() else {
            return;
        };
        if !self.document.as_ref().is_some_and(|document| {
            document.id == draft.document_id && document.project_id == draft.project_id
        }) {
            return;
        }
        let l = self.language;
        let mut submit = false;
        let mut cancel = false;
        let response = components::modal(
            ctx,
            l,
            "find-in-document",
            l.text("Find in document"),
            440.0,
            |ui| {
                ui.label(l.text("The first match will be selected in the open document."));
                ui.add_space(8.0);
                let label = ui.label(l.text("Text to find"));
                let field = ui
                    .push_id(("document-find-query", &draft.document_id), |ui| {
                        components::singleline(ui, &mut draft.query, l.text("Text to find"))
                    })
                    .inner
                    .labelled_by(label.id);
                if draft.focus {
                    field.request_focus();
                    draft.focus = false;
                }
                if field.changed() {
                    draft.error = None;
                }
                submit =
                    field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                if let Some(message) = draft.error {
                    ui.colored_label(t.warning, l.text(message));
                }
                components::footer(ui, |ui| {
                    submit |=
                        components::action(ui, l.text("Find"), !draft.query.is_empty(), Some(t))
                            .clicked();
                    cancel = components::action(ui, l.text("Cancel"), true, None).clicked();
                });
            },
        );
        if cancel || response.should_close() {
            self.focus_manuscript = true;
            return;
        }
        if submit && !draft.query.is_empty() {
            match self.select_document_find_match(ctx, &draft) {
                Ok(true) => {
                    self.focus_manuscript = true;
                    return;
                }
                Ok(false) => draft.error = Some("No matches found."),
                Err(message) => draft.error = Some(message),
            }
        }
        self.daily_controls.find = Some(draft);
    }

    fn select_document_find_match(
        &mut self,
        ctx: &egui::Context,
        draft: &FindDialog,
    ) -> Result<bool, &'static str> {
        if !self.document.as_ref().is_some_and(|document| {
            document.id == draft.document_id && document.project_id == draft.project_id
        }) {
            return Err("The document changed. Close Find and try again.");
        }
        #[cfg(feature = "editor-preview")]
        if let Some(rich) = &mut self.rich {
            if draft.session_revision != Some(rich.session.revision()) {
                return Err("The document changed. Close Find and try again.");
            }
            let found =
                studio_core::find_first_document_match(rich.session.document(), &draft.query)
                    .map_err(
                        |_| "The selection could not be restored. Close Find and try again.",
                    )?;
            let Some(found) = found else {
                return Ok(false);
            };
            rich.session
                .set_selection(craft_editor_egui::Selection {
                    anchor: craft_editor_egui::DocPoint {
                        block_id: found.anchor.block_id,
                        offset: found.anchor.offset,
                    },
                    head: craft_editor_egui::DocPoint {
                        block_id: found.head.block_id,
                        offset: found.head.offset,
                    },
                })
                .map_err(|_| "The selection could not be restored. Close Find and try again.")?;
            rich.session.request_scroll_to_caret();
            return Ok(true);
        }
        if draft.plain_body.as_deref() != Some(self.body.as_str()) {
            return Err("The document changed. Close Find and try again.");
        }
        let Some(range) = studio_core::find_first_text_match(&self.body, &draft.query) else {
            return Ok(false);
        };
        let id = egui::Id::new(("manuscript-body", &draft.document_id));
        let Some(mut state) = egui::TextEdit::load_state(ctx, id) else {
            return Err("The selection could not be restored. Close Find and try again.");
        };
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(range.start),
                egui::text::CCursor::new(range.end),
            )));
        state.store(ctx, id);
        Ok(true)
    }
}

#[cfg(all(test, feature = "editor-preview"))]
mod tests {
    use super::*;
    use craft_editor_egui::{DocPoint, Selection};
    use craft_editor_model::{Document as RichDocument, ScalarOffset};

    fn run_test_frame(
        context: &egui::Context,
        input: egui::RawInput,
        draw: impl FnMut(&mut egui::Ui),
    ) {
        let mut output = context.run_ui(input, draw);
        // The test has no GPU; acknowledge the generated texture updates explicitly.
        output.textures_delta.clear();
    }

    fn fixture() -> AppResult<(tempfile::TempDir, StudioApp)> {
        let directory = tempfile::tempdir()?;
        let context = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(context);
        let mut app = StudioApp::new(
            &creation,
            directory.path().join("daily.sqlite3"),
            false,
            None,
        )?;
        let project = app.workspace.create_project("Synthetic daily controls")?;
        app.workspace.create_rich_document(
            &project.id,
            "Original title",
            RichDocument::plain("paragraph", "Я 😀 İSTANBUL echo echo")?.to_json()?,
        )?;
        app.open_project(project)?;
        Ok((directory, app))
    }

    fn title_draft(app: &mut StudioApp, value: &str) -> AppResult<()> {
        let document = app.document.as_ref().ok_or("missing document")?;
        app.daily_controls.title = Some(TitleDraft {
            document_id: document.id.clone(),
            project_id: document.project_id.clone(),
            expected_title: document.title.clone(),
            value: value.into(),
            focus: true,
            error: None,
            field_id: None,
            field_context: None,
            field_focused: false,
            composing: false,
        });
        Ok(())
    }

    #[test]
    fn find_changes_only_semantic_selection_and_preserves_canonical_history() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let document = app.document.clone().ok_or("missing document")?;
        let rich = app.rich.as_ref().ok_or("missing session")?;
        let before = rich.session.document().to_json()?;
        let revision = rich.session.revision();
        let history = app.workspace.rich_revisions(&document.id)?;
        app.open_document_find();
        let mut draft = app
            .daily_controls
            .find
            .take()
            .ok_or("missing find dialog")?;
        draft.query = "i̇st".into();
        assert!(app.select_document_find_match(&egui::Context::default(), &draft)?);
        let rich = app.rich.as_ref().ok_or("missing session")?;
        assert_eq!(
            rich.session.selection(),
            Some(&Selection {
                anchor: DocPoint {
                    block_id: "paragraph".into(),
                    offset: ScalarOffset(4)
                },
                head: DocPoint {
                    block_id: "paragraph".into(),
                    offset: ScalarOffset(7)
                },
            })
        );
        assert_eq!(rich.session.document().to_json()?, before);
        assert_eq!(rich.session.revision(), revision);
        assert!(!rich.session.can_undo());
        assert_eq!(app.workspace.document(&document.id)?, document);
        assert_eq!(app.workspace.rich_revisions(&document.id)?, history);
        assert!(app.workspace.rich_draft(&document.id)?.is_none());
        Ok(())
    }

    #[test]
    fn stale_find_preserves_the_later_revision_and_exact_selection() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        app.open_document_find();
        let mut draft = app
            .daily_controls
            .find
            .take()
            .ok_or("missing find dialog")?;
        draft.query = "echo".into();
        let rich = app.rich.as_mut().ok_or("missing session")?;
        rich.session.set_selection(Selection {
            anchor: DocPoint {
                block_id: "paragraph".into(),
                offset: ScalarOffset(0),
            },
            head: DocPoint {
                block_id: "paragraph".into(),
                offset: ScalarOffset(0),
            },
        })?;
        rich.session.insert_plain_text("new ")?;
        let after = rich.session.document().to_json()?;
        let selection = rich.session.selection().cloned();
        assert!(
            app.select_document_find_match(&egui::Context::default(), &draft)
                .is_err()
        );
        let rich = app.rich.as_ref().ok_or("missing session")?;
        assert_eq!(rich.session.document().to_json()?, after);
        assert_eq!(rich.session.selection(), selection.as_ref());
        Ok(())
    }

    #[test]
    fn find_never_selects_into_a_later_document_with_the_same_session_revision() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        app.open_document_find();
        let mut draft = app
            .daily_controls
            .find
            .take()
            .ok_or("missing find dialog")?;
        draft.query = "echo".into();
        let project = app.project.as_ref().ok_or("missing project")?;
        let other = app.workspace.create_rich_document(
            &project.id,
            "Other",
            RichDocument::plain("other", "echo")?.to_json()?,
        )?;
        app.open_document(other.clone())?;
        let before = app
            .rich
            .as_ref()
            .ok_or("missing session")?
            .session
            .selection()
            .cloned();
        assert!(
            app.select_document_find_match(&egui::Context::default(), &draft)
                .is_err()
        );
        assert_eq!(
            app.rich
                .as_ref()
                .ok_or("missing session")?
                .session
                .selection(),
            before.as_ref()
        );
        assert_eq!(app.document.as_ref(), Some(&other));
        Ok(())
    }

    #[test]
    fn title_commit_does_not_flush_or_replace_unsaved_rich_work_or_undo() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let rich = app.rich.as_mut().ok_or("missing session")?;
        rich.session.set_selection(Selection {
            anchor: DocPoint {
                block_id: "paragraph".into(),
                offset: ScalarOffset(0),
            },
            head: DocPoint {
                block_id: "paragraph".into(),
                offset: ScalarOffset(0),
            },
        })?;
        rich.session.insert_plain_text("unsaved ")?;
        let before = rich.session.document().to_json()?;
        let revision = rich.session.revision();
        let selection = rich.session.selection().cloned();
        title_draft(&mut app, "  Changed 😀  ")?;
        app.commit_title_edit()?;
        let rich = app.rich.as_ref().ok_or("missing session")?;
        assert_eq!(rich.session.document().to_json()?, before);
        assert_eq!(rich.session.revision(), revision);
        assert_eq!(rich.session.selection(), selection.as_ref());
        assert!(rich.session.can_undo());
        assert!(!app.daily_controls.editing_title());
        let document = app.document.as_ref().ok_or("missing document")?;
        assert_eq!(document.title, "Changed 😀");
        assert_eq!(document.revision, 1);
        assert_eq!(app.workspace.document(&document.id)?.title, document.title);
        Ok(())
    }

    #[test]
    fn failed_title_commit_keeps_the_draft_and_original_context_for_navigation_guard()
    -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let document = app.document.clone().ok_or("missing document")?;
        title_draft(&mut app, "Unsaved title draft")?;
        app.workspace.rename_document(
            &document.project_id,
            &document.id,
            &document.title,
            "External rename",
        )?;
        assert!(app.commit_title_edit().is_err());
        app.daily_controls.document_changed();
        let draft = app
            .daily_controls
            .title
            .as_ref()
            .ok_or("lost title draft")?;
        assert_eq!(draft.value, "Unsaved title draft");
        assert!(draft.error.is_some());
        assert_eq!(app.document.as_ref(), Some(&document));
        Ok(())
    }

    #[test]
    fn escape_closes_find_without_mutating_selection_or_text() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let before = app
            .rich
            .as_ref()
            .ok_or("missing session")?
            .session
            .document()
            .to_json()?;
        let selection = app
            .rich
            .as_ref()
            .ok_or("missing session")?
            .session
            .selection()
            .cloned();
        let context = egui::Context::default();
        let tokens = theme::resolve(&app.settings, false);
        app.open_document_find();
        let _ = run_test_frame(&context, egui::RawInput::default(), |ctx| {
            app.daily_control_dialogs(ctx.ctx(), tokens)
        });
        let mut input = egui::RawInput::default();
        input.events.push(egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        let _ = run_test_frame(&context, input, |ctx| {
            app.daily_control_dialogs(ctx.ctx(), tokens)
        });
        assert!(!app.daily_controls.modal_open());
        let rich = app.rich.as_ref().ok_or("missing session")?;
        assert_eq!(rich.session.document().to_json()?, before);
        assert_eq!(rich.session.selection(), selection.as_ref());
        Ok(())
    }

    #[test]
    fn title_escape_discards_composition_draft_and_clears_native_edit_state() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let document = app.document.clone().ok_or("missing document")?;
        let context = egui::Context::default();
        let tokens = theme::resolve(&app.settings, false);
        title_draft(&mut app, "Original title")?;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 400.0),
            )),
            ..Default::default()
        };
        let _ = run_test_frame(&context, input.clone(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                app.document_title(ui, &document, tokens);
            });
        });
        let id = app
            .daily_controls
            .title
            .as_ref()
            .and_then(|draft| draft.field_id)
            .ok_or("missing title field")?;
        let mut typed = input.clone();
        typed.events.push(egui::Event::Text("Changed title".into()));
        let _ = run_test_frame(&context, typed, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                app.document_title(ui, &document, tokens);
            });
        });
        app.daily_controls
            .title
            .as_mut()
            .ok_or("missing title draft")?
            .composing = true;
        let mut escape = input;
        escape.events.push(egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        let _ = run_test_frame(&context, escape, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                app.document_title(ui, &document, tokens);
            });
        });
        assert!(!app.daily_controls.editing_title());
        assert!(egui::TextEdit::load_state(&context, id).is_none());
        assert_eq!(app.workspace.document(&document.id)?, document);
        Ok(())
    }

    #[test]
    fn explicit_title_save_never_persists_uncommitted_composition() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let document = app.document.clone().ok_or("missing document")?;
        title_draft(&mut app, "未確定")?;
        app.daily_controls
            .title
            .as_mut()
            .ok_or("missing title draft")?
            .composing = true;
        assert!(app.commit_title_edit().is_err());
        assert_eq!(app.workspace.document(&document.id)?, document);
        assert_eq!(
            app.daily_controls
                .title
                .as_ref()
                .map(|draft| draft.value.as_str()),
            Some("未確定")
        );
        Ok(())
    }
}
