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
                    let result = (|| -> AppResult<()> {
                        let project = self.workspace.create_project(self.project_name.trim())?;
                        self.create_empty_manuscript(&project.id, l.text("First chapter"))?;
                        self.projects = self.workspace.list_projects()?;
                        self.open_project(project)?;
                        Ok(())
                    })();
                    if result.is_ok() {
                        self.project_dialog = false;
                        self.project_name.clear();
                    }
                    self.record_error(result);
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
                        let result = (|| -> AppResult<()> {
                            self.save()?;
                            if let Some(project) = &self.project {
                                let document = self.create_empty_manuscript(
                                    &project.id,
                                    self.document_name.trim(),
                                )?;
                                self.documents = self.workspace.list_documents(&project.id)?;
                                self.open_document(document)?;
                            }
                            Ok(())
                        })();
                        if result.is_ok() {
                            self.document_dialog = false;
                            self.document_name.clear();
                        }
                        self.record_error(result);
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
