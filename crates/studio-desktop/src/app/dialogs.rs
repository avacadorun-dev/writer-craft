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
            let response = modal(ctx, l, "new-project", l.text("New project"), 400.0, |ui| {
                ui.label(l.text("Project title"));
                ui.add_space(6.0);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.project_name)
                        .desired_width(f32::INFINITY)
                        .hint_text(l.text("My new story")),
                );
                let focus_id = egui::Id::new("project-title-initial-focus");
                if !ctx.data(|data| data.get_temp::<bool>(focus_id).unwrap_or(false)) {
                    response.request_focus();
                    ctx.data_mut(|data| data.insert_temp(focus_id, true));
                }
                let submitted =
                    response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                ui.add_space(16.0);
                if ui
                    .add_enabled(
                        !self.project_name.trim().is_empty(),
                        egui::Button::new(l.text("Create project")),
                    )
                    .clicked()
                    || (submitted && !self.project_name.trim().is_empty())
                {
                    let result = (|| -> AppResult<()> {
                        let project = self.workspace.create_project(self.project_name.trim())?;
                        self.workspace
                            .create_document(&project.id, l.text("First chapter"), "")?;
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
                400.0,
                |ui| {
                    ui.label(l.text("Document title"));
                    ui.add_space(6.0);
                    let field = ui.add(
                        egui::TextEdit::singleline(&mut self.document_name)
                            .desired_width(f32::INFINITY)
                            .hint_text(l.text("New chapter")),
                    );
                    let focus_id = egui::Id::new("document-title-initial-focus");
                    if !ctx.data(|data| data.get_temp::<bool>(focus_id).unwrap_or(false)) {
                        field.request_focus();
                        ctx.data_mut(|data| data.insert_temp(focus_id, true));
                    }
                    let submitted =
                        field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                    ui.add_space(16.0);
                    if ui
                        .add_enabled(
                            !self.document_name.trim().is_empty(),
                            egui::Button::new(l.text("Add document")),
                        )
                        .clicked()
                        || (submitted && !self.document_name.trim().is_empty())
                    {
                        let result = (|| -> AppResult<()> {
                            self.save()?;
                            if let Some(project) = &self.project {
                                let document = self.workspace.create_document(
                                    &project.id,
                                    self.document_name.trim(),
                                    "",
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
        if let Some(preview) = &self.import_preview {
            let response = modal(
                ctx,
                l,
                "archive-preview",
                l.text(".wstudio project preview"),
                540.0,
                |ui| {
                    ui.heading(&preview.title);
                    ui.add_space(10.0);
                    ui.label(l.format(
                        "Format version: {version} · Documents: {count}",
                        &[
                            ("version", preview.format_version.to_string()),
                            ("count", preview.documents.len().to_string()),
                        ],
                    ));
                    ui.add_space(12.0);
                    ui.label(l.text("Archive checked. Text preview is available. Full project import and rich-document editing are not implemented yet."));
                    ui.add_space(12.0);
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for document in &preview.documents {
                                egui::CollapsingHeader::new(&document.title)
                                    .id_salt(&document.id)
                                    .show(ui, |ui| {
                                        ui.label(&document.plain_text);
                                    });
                            }
                        });
                },
            );
            if response.should_close() {
                self.import_preview = None;
            }
        }
        if let Some(message) = self.error.clone() {
            let response = modal(ctx, l, "error", l.text("Attention needed"), 440.0, |ui| {
                ui.label(l.text(message.msgid));
                ui.add_space(8.0);
                ui.collapsing(l.text("Technical details"), |ui| {
                    ui.label(&message.details);
                });
                ui.add_space(12.0);
                if ui.button(l.text("Understood")).clicked() {
                    self.error = None;
                }
            });
            if response.should_close() {
                self.error = None;
            }
        }
    }
}
