//! The home body, using native controls inside the unchanged application shell.
use super::*;

mod artwork;
#[cfg(test)]
mod tests;
mod view;

impl StudioApp {
    pub(super) fn library(&mut self, ui: &mut egui::Ui, t: Tokens) {
        let resume = continuation(&self.projects, self.project.as_ref());
        let action = egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(t.bg)
                    .inner_margin(egui::Margin::symmetric(20, 20)),
            )
            .show(ui, |ui| {
                view::show(ui, &self.projects, resume, self.language, t)
            })
            .inner;
        match action {
            Some(view::Action::New) => self.project_dialog = true,
            Some(view::Action::Import) => self.choose_archive(),
            Some(view::Action::Sample) => {
                let result = self.create_example();
                self.record_error(result);
            }
            Some(view::Action::Open(id)) => {
                // Never resolve a duplicate title or use an index after a refresh.
                if let Some(project) = self
                    .projects
                    .iter()
                    .find(|project| project.id == id)
                    .cloned()
                {
                    let result = self.open_project(project);
                    self.record_error(result);
                }
            }
            None => {}
        }
    }
}

fn continuation<'a>(projects: &'a [Project], current: Option<&Project>) -> Option<&'a Project> {
    current.and_then(|current| projects.iter().find(|project| project.id == current.id))
}
