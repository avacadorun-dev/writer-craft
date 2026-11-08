//! Native navigator component.
use super::*;

impl StudioApp {
    pub(super) fn navigation(&mut self, ui: &mut egui::Ui, t: Tokens) {
        let l = self.language;
        egui::Panel::left("navigator")
            .default_size(264.0)
            .min_size(210.0)
            .max_size(340.0)
            .frame(
                egui::Frame::new()
                    .fill(t.panel)
                    .inner_margin(egui::Margin::symmetric(16, 14)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(if self.library {
                            l.text("LIBRARY")
                        } else {
                            l.text("STRUCTURE")
                        })
                        .size(11.0)
                        .strong()
                        .color(t.muted),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::icon_button(
                            ui,
                            Icon::Plus,
                            if self.library {
                                l.text("Create project")
                            } else {
                                l.text("Add document")
                            },
                        )
                        .clicked()
                        {
                            if self.library {
                                self.project_dialog = true;
                            } else {
                                self.document_dialog = true;
                            }
                        }
                    });
                });
                ui.add_space(12.0);
                if self.library {
                    nav_row(ui, Icon::Book, l.text("All projects"), true, t);
                    ui.add_space(6.0);
                    if nav_row(ui, Icon::Import, l.text("Open .wstudio"), false, t).clicked() {
                        self.choose_archive();
                    }
                    ui.add_space(24.0);
                    ui.label(
                        RichText::new(l.text("WORKSPACE"))
                            .size(10.5)
                            .strong()
                            .color(t.muted),
                    );
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(l.text(
                            "Your manuscript, materials and history, together in your project.",
                        ))
                        .size(13.0)
                        .color(t.muted),
                    );
                } else {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text(l.text("Find a document…"))
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        icons::icon(ui, Icon::Folder, 20.0, t.muted);
                        ui.label(RichText::new(l.text("Manuscript")).size(13.0).strong());
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(self.documents.len().to_string())
                                    .size(12.0)
                                    .color(t.muted),
                            )
                            .on_hover_text(l.format(
                                "Documents: {count}",
                                &[("count", self.documents.len().to_string())],
                            ));
                        });
                    });
                    ui.add_space(6.0);
                    let mut selected = None;
                    egui::ScrollArea::vertical()
                        .id_salt("document-tree")
                        .auto_shrink([false, false])
                        .max_height((ui.available_height() - 90.0).max(80.0))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            for document in &self.documents {
                                if !self.search.is_empty()
                                    && !document
                                        .title
                                        .to_lowercase()
                                        .contains(&self.search.to_lowercase())
                                {
                                    continue;
                                }
                                let active = self
                                    .document
                                    .as_ref()
                                    .is_some_and(|current| current.id == document.id);
                                if nav_row(ui, Icon::Document, &document.title, active, t).clicked()
                                {
                                    selected = Some(document.clone());
                                }
                            }
                        });
                    if let Some(document) = selected {
                        let result = self.open_document(document);
                        self.record_error(result);
                    }
                    ui.add_space(14.0);
                    ui.separator();
                    ui.add_space(10.0);
                    if nav_row(
                        ui,
                        Icon::History,
                        l.text("Document history"),
                        self.inspector_visible && self.inspector == Inspector::Versions,
                        t,
                    )
                    .clicked()
                    {
                        self.inspector = Inspector::Versions;
                        self.inspector_visible = true;
                        self.prefer_inspector = true;
                        self.focus_mode = false;
                    }
                }
            });
    }
}
