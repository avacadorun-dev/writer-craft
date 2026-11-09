//! Native library component.
use super::*;

impl StudioApp {
    pub(super) fn library(&mut self, ui: &mut egui::Ui, t: Tokens) {
        let l = self.language;
        egui::CentralPanel::default().frame(egui::Frame::new().fill(t.bg).inner_margin(egui::Margin::symmetric(32, 24)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(l.text(if self.projects.is_empty() { "Your first chapter starts here" } else { "All projects" })).size(24.0).strong());
                        ui.add_space(6.0);
                        ui.label(RichText::new(l.text(if self.projects.is_empty() { "Create a project. Writing needs no account, server or internet connection." } else { "Open a manuscript and continue where you left off." })).size(14.0).color(t.muted));
                    });
                });
                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    if primary(ui, l.text("New project"), t).clicked() { self.project_dialog = true; }
                    if ui.button(l.text("Open .wstudio…")).clicked() { self.choose_archive(); }
                    if self.projects.is_empty() && ui.button(l.text("Try a sample")).clicked() {
                        let result = self.create_example(); self.record_error(result);
                    }
                });
                ui.add_space(20.0);
                if !self.projects.is_empty() { ui.separator(); }
                let mut selected = None;
                egui::ScrollArea::vertical().id_salt("library-projects").show(ui, |ui| {
                    for project in &self.projects {
                        let row = egui::Frame::new().inner_margin(egui::Margin::symmetric(8, 12)).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                icons::icon(ui, Icon::Book, 24.0, t.accent); ui.add_space(10.0);
                                if ui.add_sized([(ui.available_width()-148.0).max(80.0),34.0],egui::Button::new("").left_text(RichText::new(&project.title).size(17.0).strong()).truncate().frame(false)).on_hover_text(&project.title).clicked() { selected = Some(project.clone()); }
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if icons::icon_button(ui, Icon::ArrowRight, l.text("Open project")).clicked() { selected = Some(project.clone()); }
                                    ui.label(RichText::new(l.text("Local project")).size(12.0).color(t.muted));
                                });
                            });
                        });
                        ui.painter().hline(row.response.rect.x_range(), row.response.rect.bottom(), Stroke::new(1.0,t.border));
                    }
                });
                if let Some(project) = selected { let result = self.open_project(project); self.record_error(result); }
            });
    }
}
