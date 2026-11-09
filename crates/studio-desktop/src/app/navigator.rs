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
                                l.text("New item")
                            },
                        )
                        .clicked()
                        {
                            if self.library {
                                self.project_dialog = true;
                            } else {
                                let result = self.tree_action(project_tree::TreeAction::Create(
                                    self.tree_ui.selected.clone(),
                                ));
                                self.record_error(result);
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
                    self.reference_return_control(ui);
                    #[cfg(feature = "editor-preview")]
                    self.canvas_navigation(ui, t);
                    let search =
                        components::singleline(ui, &mut self.search, l.text("Find in the tree…"));
                    #[cfg(feature = "editor-preview")]
                    super::capture_connected_writing::control(
                        ui.ctx(),
                        "navigator.search",
                        search.rect,
                    );
                    #[cfg(not(feature = "editor-preview"))]
                    let _ = search;
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        icons::icon(ui, Icon::Folder, 20.0, t.muted);
                        ui.label(
                            RichText::new(l.text("Project contents"))
                                .size(13.0)
                                .strong(),
                        );
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
                    if let Some(action) = self.project_outline.show(
                        ui,
                        &self.documents,
                        self.document.as_ref().map(|document| document.id.as_str()),
                        self.tree_ui.selected.as_deref(),
                        &self.search,
                        l,
                        t,
                    ) {
                        let result = self.tree_action(action);
                        self.record_error(result);
                    }
                    ui.add_space(14.0);
                    ui.separator();
                    ui.add_space(10.0);
                    ui.menu_button(l.text("Organize project"), |ui| {
                        if ui.button(l.text("Add at root…")).clicked() {
                            let result = self.tree_action(project_tree::TreeAction::Create(None));
                            self.record_error(result);
                            ui.close();
                        }
                        if ui.button(l.text("Trash")).clicked() {
                            let result = self.refresh_project_tree();
                            if result.is_ok() {
                                self.tree_ui.trash_open = true;
                            }
                            self.record_error(result);
                            ui.close();
                        }
                        if ui.button(l.text("Refresh project")).clicked() {
                            let result = self.refresh_project_tree();
                            self.record_error(result);
                            ui.close();
                        }
                    });
                    #[cfg(feature = "editor-preview")]
                    if nav_row(ui, Icon::Search, l.text("Project markup"), false, t).clicked() {
                        let result = self.open_project_marks();
                        self.record_error(result);
                    }
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
