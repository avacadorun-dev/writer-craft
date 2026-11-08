//! Native dialogs component.
use super::*;

impl StudioApp {
    pub(super) fn dialogs(&mut self, ctx: &egui::Context, t: Tokens) {
        let l = self.language;
        if !self.project_dialog {
            ctx.data_mut(|data| data.remove::<bool>(egui::Id::new("project-title-initial-focus")));
        }
        if !self.document_dialog {
            ctx.data_mut(|data| data.remove::<bool>(egui::Id::new("document-title-initial-focus")));
        }
        if self.project_dialog {
            let response = modal(ctx, l, "new-project", l.text("New project"), 440.0, |ui| {
                ui.label(l.text("Project title"));
                ui.add_space(6.0);
                let response =
                    components::singleline(ui, &mut self.project_name, l.text("My new story"));
                let focus_id = egui::Id::new("project-title-initial-focus");
                if !ctx.data(|data| data.get_temp::<bool>(focus_id).unwrap_or(false)) {
                    response.request_focus();
                    ctx.data_mut(|data| data.insert_temp(focus_id, true));
                }
                let submitted =
                    response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                let valid = !self.project_name.trim().is_empty();
                let mut create = submitted && valid;
                components::footer(ui, |ui| {
                    create |=
                        components::action(ui, l.text("Create project"), valid, Some(t)).clicked();
                    if components::action(ui, l.text("Cancel"), true, None).clicked() {
                        create = false;
                        self.project_dialog = false;
                    }
                });
                if create {
                    match self.create_project_from_form() {
                        Ok(project) => {
                            // A creation receipt closes the form even if subsequent navigation fails.
                            self.project_dialog = false;
                            self.project_name.clear();
                            let result = (|| -> AppResult<()> {
                                self.projects = self.workspace.list_projects()?;
                                self.open_project(project)
                            })();
                            self.record_error(result);
                        }
                        Err(error) => self.record_error(Err(error)),
                    }
                }
            });
            if response.should_close() {
                self.project_dialog = false;
            }
        }
        if self.document_dialog {
            let response = modal(
                ctx,
                l,
                "new-document",
                l.text("New document"),
                440.0,
                |ui| {
                    ui.label(l.text("Document title"));
                    ui.add_space(6.0);
                    let field =
                        components::singleline(ui, &mut self.document_name, l.text("New chapter"));
                    let focus_id = egui::Id::new("document-title-initial-focus");
                    if !ctx.data(|data| data.get_temp::<bool>(focus_id).unwrap_or(false)) {
                        field.request_focus();
                        ctx.data_mut(|data| data.insert_temp(focus_id, true));
                    }
                    let submitted =
                        field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                    let valid = !self.document_name.trim().is_empty();
                    let mut create = submitted && valid;
                    components::footer(ui, |ui| {
                        create |= components::action(ui, l.text("Add document"), valid, Some(t))
                            .clicked();
                        if components::action(ui, l.text("Cancel"), true, None).clicked() {
                            create = false;
                            self.document_dialog = false;
                        }
                    });
                    if create {
                        match self.create_document_from_form() {
                            Ok(document) => {
                                self.document_dialog = false;
                                self.document_name.clear();
                                let result = (|| -> AppResult<()> {
                                    self.documents =
                                        self.workspace.list_documents(&document.project_id)?;
                                    self.open_document(document)
                                })();
                                self.record_error(result);
                            }
                            Err(error) => self.record_error(Err(error)),
                        }
                    }
                },
            );
            if response.should_close() {
                self.document_dialog = false;
            }
        }
        if self.settings_open {
            self.settings_dialog(ctx, t);
        }
        self.project_import
            .dialog(ctx, &self.database, self.language, t);
        if let Some(message) = self.error.clone() {
            let response = modal(ctx, l, "error", l.text("Attention needed"), 440.0, |ui| {
                ui.label(l.text(message.msgid));
                ui.add_space(8.0);
                ui.collapsing(l.text("Technical details"), |ui| {
                    ui.label(&message.details);
                });
                ui.add_space(12.0);
                if components::action(ui, l.text("Understood"), true, Some(t)).clicked() {
                    self.error = None;
                }
            });
            if response.should_close() {
                self.error = None;
            }
        }
    }

    fn prepare_creation(&mut self) -> AppResult<()> {
        #[cfg(feature = "editor-preview")]
        self.ensure_canvas_navigation()?;
        self.commit_title_edit()?;
        self.save()
    }

    pub(super) fn create_project_from_form(&mut self) -> AppResult<Project> {
        self.prepare_creation()?;
        #[cfg(feature = "editor-preview")]
        let format = studio_core::InitialManuscript::Structured;
        #[cfg(not(feature = "editor-preview"))]
        let format = studio_core::InitialManuscript::Plain;
        Ok(self
            .workspace
            .create_project_with_manuscript(
                self.project_name.trim(),
                self.language.text("First chapter"),
                format,
            )?
            .project)
    }

    pub(super) fn create_document_from_form(&mut self) -> AppResult<Document> {
        self.prepare_creation()?;
        let project = self.project.as_ref().ok_or_else(|| {
            studio_core::StudioError::Validation(
                "Choose a project before creating a manuscript".into(),
            )
        })?;
        self.create_empty_manuscript(&project.id, self.document_name.trim())
    }

    fn create_empty_manuscript(&self, project_id: &str, title: &str) -> AppResult<Document> {
        #[cfg(feature = "editor-preview")]
        {
            let content =
                craft_editor_model::Document::plain(uuid::Uuid::new_v4().to_string(), "")?
                    .to_json()?;
            Ok(self
                .workspace
                .create_rich_document(project_id, title, content)?)
        }
        #[cfg(not(feature = "editor-preview"))]
        {
            Ok(self.workspace.create_document(project_id, title, "")?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(app: &mut StudioApp, ctx: &egui::Context, events: Vec<egui::Event>, time: f64) {
        let tokens = theme::resolve(&ThemeSettings::default(), false);
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 680.0),
                )),
                events,
                time: Some(time),
                ..Default::default()
            },
            |ui| app.dialogs(ui.ctx(), tokens),
        );
        output.textures_delta.clear();
    }
    fn enter() -> egui::Event {
        egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }
    #[test]
    fn native_project_form_should_create_and_open_exactly_one_initial_manuscript() -> AppResult<()>
    {
        let directory = tempfile::tempdir()?;
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = StudioApp::new(
            &creation,
            directory.path().join("project-form.sqlite"),
            false,
            None,
        )?;
        app.project_dialog = true;
        frame(&mut app, &ctx, vec![], 0.0);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Text("Created from native form 日本語".into())],
            0.1,
        );
        frame(&mut app, &ctx, vec![enter()], 0.2);
        assert!(!app.project_dialog);
        assert!(!app.document_dialog);
        assert!(app.project_name.is_empty());
        let project = app.project.as_ref().ok_or("new project did not open")?;
        assert_eq!(project.title, "Created from native form 日本語");
        assert_eq!(app.workspace.list_projects()?.len(), 1);
        let documents = app.workspace.list_documents(&project.id)?;
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].title, app.language.text("First chapter"));
        assert_eq!(
            app.document.as_ref().map(|document| document.id.as_str()),
            Some(documents[0].id.as_str())
        );
        Ok(())
    }
    #[test]
    fn native_document_form_should_create_and_open_the_typed_document_once() -> AppResult<()> {
        let directory = tempfile::tempdir()?;
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = StudioApp::new(
            &creation,
            directory.path().join("document-form.sqlite"),
            false,
            None,
        )?;
        let project = app.workspace.create_project("Existing project")?;
        app.open_project(project.clone())?;
        app.document_dialog = true;
        frame(&mut app, &ctx, vec![], 0.0);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Text("Created chapter 😀".into())],
            0.1,
        );
        frame(&mut app, &ctx, vec![enter()], 0.2);
        assert!(!app.document_dialog);
        assert!(app.document_name.is_empty());
        assert_eq!(app.workspace.list_projects()?.len(), 1);
        let documents = app.workspace.list_documents(&project.id)?;
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].title, "Created chapter 😀");
        assert_eq!(
            app.document.as_ref().map(|document| document.id.as_str()),
            Some(documents[0].id.as_str())
        );
        Ok(())
    }
}
