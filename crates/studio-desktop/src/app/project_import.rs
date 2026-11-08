//! A metadata-only preview retains its opened archive through the actual import transaction.
use super::*;
use studio_core::{ImportedProject, ProjectImportWorker, ProjectTransferCancellation};
use studio_transfer::ProjectArchive;

struct ImportCompletion {
    archive: ProjectArchive,
    result: AppResult<ImportedProject>,
}
/// Archive readers and import workers stay bounded to one selected transfer per app window.
#[derive(Default)]
pub(super) struct ProjectImportState {
    checking: Option<mpsc::Receiver<AppResult<ProjectArchive>>>,
    importing: Option<mpsc::Receiver<ImportCompletion>>,
    archive: Option<ProjectArchive>,
    cancel: ProjectTransferCancellation,
    cancel_requested: bool,
    error: Option<String>,
}
impl ProjectImportState {
    pub(super) fn modal_open(&self) -> bool {
        self.checking.is_some()
            || self.importing.is_some()
            || self.archive.is_some()
            || self.error.is_some()
    }
    pub(super) fn busy(&self) -> bool {
        self.checking.is_some() || self.importing.is_some()
    }
    fn check(&mut self, path: PathBuf) -> AppResult<()> {
        if self.modal_open() {
            return Err(studio_core::StudioError::Validation(
                "Finish the selected project import before opening another archive".into(),
            )
            .into());
        }
        self.cancel = ProjectTransferCancellation::default();
        self.cancel_requested = false;
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let result = ProjectArchive::open(path).map_err(Into::into);
            let _ = sender.send(result);
        });
        self.checking = Some(receiver);
        Ok(())
    }
    fn begin_import(&mut self, database: &std::path::Path) -> AppResult<()> {
        if self.busy() {
            return Ok(());
        }
        let archive = self.archive.take().ok_or_else(|| {
            studio_core::StudioError::Validation(
                "Choose and check a project archive before importing".into(),
            )
        })?;
        let database = database.to_owned();
        let cancel = ProjectTransferCancellation::default();
        self.cancel = cancel.clone();
        self.cancel_requested = false;
        self.error = None;
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let completion = import_open_archive(archive, &database, &cancel);
            let _ = sender.send(completion);
        });
        self.importing = Some(receiver);
        Ok(())
    }
    fn cancel(&mut self) {
        if self.busy() {
            self.cancel_requested = true;
            self.cancel.cancel();
        } else {
            self.archive = None;
            self.error = None;
        }
    }
    /// A completed import becomes visible only after the core committed all project/asset rows.
    fn poll(&mut self, ctx: &egui::Context) -> AppResult<Option<ImportedProject>> {
        if let Some(receiver) = &self.checking {
            match receiver.try_recv() {
                Ok(result) => {
                    self.checking = None;
                    if self.cancel_requested {
                        self.cancel_requested = false;
                    } else {
                        match result {
                            Ok(archive) => self.archive = Some(archive),
                            Err(error) => self.error = Some(error.to_string()),
                        }
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(30))
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.checking = None;
                    self.error = Some("Project checking worker stopped before completion".into());
                }
            }
        }
        if let Some(receiver) = &self.importing {
            match receiver.try_recv() {
                Ok(completion) => {
                    self.importing = None;
                    self.cancel_requested = false;
                    match completion.result {
                        Ok(imported) => {
                            self.error = None;
                            return Ok(Some(imported));
                        }
                        Err(error) => {
                            self.archive = Some(completion.archive);
                            self.error = Some(error.to_string());
                        }
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(30))
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.importing = None;
                    self.error=Some("Project import worker stopped before acknowledgement; inspect the project list before retrying".into());
                }
            }
        }
        Ok(None)
    }
    pub(super) fn dialog(
        &mut self,
        ctx: &egui::Context,
        database: &std::path::Path,
        l: Language,
        t: Tokens,
    ) {
        if !self.modal_open() {
            return;
        }
        let busy = self.busy();
        let mut import = false;
        let mut cancel = false;
        let response = components::modal(
            ctx,
            l,
            "project-archive-import",
            l.text("Import Writers Studio project"),
            560.0,
            |ui| {
                if self.checking.is_some() {
                    ui.spinner();
                    ui.label(l.text("Checking project archive…"));
                } else if self.importing.is_some() {
                    ui.spinner();
                    ui.label(l.text(if self.cancel_requested {
                        "Cancelling project import…"
                    } else {
                        "Importing project and verifying artifacts…"
                    }));
                }
                if let Some(archive) = &self.archive {
                    let project = archive.project();
                    ui.label(RichText::new(project.title()).size(19.0).strong());
                    ui.add_space(12.0);
                    let manifest = project.manifest();
                    let count = |key: &str| manifest[key].as_array().map_or(0, Vec::len);
                    egui::Grid::new("project-import-summary")
                        .spacing([24.0, 10.0])
                        .show(ui, |ui| {
                            for (key, value) in [
                                ("Project format", project.format_version().to_string()),
                                ("Project items", count("nodes").to_string()),
                                (
                                    "Saved document revisions",
                                    count("documentRevisions").to_string(),
                                ),
                                ("Boards", count("canvases").to_string()),
                                ("Artifacts", project.attachments().len().to_string()),
                            ] {
                                ui.label(l.text(key));
                                ui.label(value);
                                ui.end_row();
                            }
                        });
                    ui.add_space(16.0);
                    ui.label(RichText::new(l.text("Import creates an independent local project. Original structure, histories and artifact bytes are retained; every artifact is verified before completion.")).color(t.muted));
                }
                if let Some(error) = &self.error {
                    ui.add_space(12.0);
                    ui.colored_label(t.warning,l.text("The project could not be imported. Existing projects and the original archive are unchanged."));
                    ui.collapsing(l.text("Technical details"), |ui| {
                        ui.label(error);
                    });
                }
                components::footer(ui, |ui| {
                    if self.archive.is_some() {
                        import = components::action(ui, l.text("Import project"), !busy, Some(t))
                            .clicked();
                    }
                    cancel = components::action(ui, l.text("Cancel"), !self.cancel_requested, None)
                        .clicked();
                });
            },
        );
        if cancel || response.should_close() {
            self.cancel();
        } else if import {
            if let Err(error) = self.begin_import(database) {
                self.error = Some(error.to_string());
            }
        }
    }
}
fn import_open_archive(
    mut archive: ProjectArchive,
    database: &std::path::Path,
    cancel: &ProjectTransferCancellation,
) -> ImportCompletion {
    let result = (|| -> AppResult<ImportedProject> {
        let mut worker = ProjectImportWorker::open(database)?;
        Ok(archive.import(&mut worker, cancel)?)
    })();
    ImportCompletion { archive, result }
}
impl StudioApp {
    pub(super) fn choose_project_archive(&mut self) {
        if self.project_import.modal_open() {
            return;
        }
        #[cfg(feature = "editor-preview")]
        if let Err(error) = self.ensure_canvas_navigation() {
            self.record_error(Err(error));
            return;
        }
        let saved = self.commit_title_edit().and_then(|()| self.save());
        if saved.is_err() {
            self.record_error(saved);
            return;
        }
        let l = self.language;
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(l.text("Writers Studio project"), &["wstudio"])
            .pick_file()
        {
            let result = self.project_import.check(path);
            self.record_error(result);
        }
    }
    pub(super) fn poll_project_import(&mut self, ctx: &egui::Context) {
        match self.project_import.poll(ctx) {
            Ok(Some(imported)) => {
                let result = (|| -> AppResult<()> {
                    self.projects = self.workspace.list_projects()?;
                    let project = self
                        .projects
                        .iter()
                        .find(|project| project.id == imported.project_id)
                        .cloned()
                        .ok_or_else(|| {
                            studio_core::StudioError::CorruptData(
                                "Acknowledged imported project is absent".into(),
                            )
                        })?;
                    self.open_project(project)?;
                    self.status = "Project imported".into();
                    Ok(())
                })();
                self.record_error(result);
            }
            Ok(None) => {}
            Err(error) => self.record_error(Err(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn archive_fixture(path: &std::path::Path) -> TestResult {
        let workspace = Workspace::memory()?;
        let project = workspace.create_project("Synthetic native import")?;
        workspace.create_rich_document(&project.id,"Chapter",serde_json::json!({"type":"doc","attrs":{"schemaVersion":4},"content":[{"type":"paragraph","attrs":{"blockId":"p"},"content":[{"type":"text","text":"Private fixture substitute 😀"}]}]}))?;
        let snapshot = workspace.project_export_snapshot(&project.id)?;
        studio_transfer::write_project_archive(
            &snapshot,
            std::fs::File::create(path)?,
            &ProjectTransferCancellation::default(),
        )?;
        Ok(())
    }
    #[test]
    fn a_pinned_import_preview_uses_the_opened_archive_after_its_path_is_replaced() -> TestResult {
        let directory = tempfile::tempdir()?;
        let source = directory.path().join("source.wstudio");
        archive_fixture(&source)?;
        let archive = ProjectArchive::open(&source)?;
        std::fs::rename(&source, directory.path().join("original.wstudio"))?;
        std::fs::write(&source, b"unrelated replacement")?;
        let database = directory.path().join("workspace.sqlite");
        let completion =
            import_open_archive(archive, &database, &ProjectTransferCancellation::default());
        let imported = completion.result?;
        let reopened = Workspace::open(&database)?;
        let docs = reopened.list_documents(&imported.project_id)?;
        assert_eq!(docs.len(), 1);
        assert!(docs[0].rich_json.is_some());
        assert!(!docs[0].read_only);
        assert!(docs[0].body.contains("😀"));
        assert_eq!(std::fs::read(&source)?, b"unrelated replacement");
        Ok(())
    }
    #[test]
    fn cancelling_an_import_preserves_the_open_archive_for_retry_without_creating_a_project()
    -> TestResult {
        let directory = tempfile::tempdir()?;
        let source = directory.path().join("source.wstudio");
        archive_fixture(&source)?;
        let archive = ProjectArchive::open(&source)?;
        let database = directory.path().join("workspace.sqlite");
        let cancel = ProjectTransferCancellation::default();
        cancel.cancel();
        let completion = import_open_archive(archive, &database, &cancel);
        assert!(completion.result.is_err());
        assert!(Workspace::open(&database)?.list_projects()?.is_empty());
        let retry = import_open_archive(
            completion.archive,
            &database,
            &ProjectTransferCancellation::default(),
        );
        let imported = retry.result?;
        assert_eq!(
            Workspace::open(&database)?
                .list_documents(&imported.project_id)?
                .len(),
            1
        );
        Ok(())
    }
}
