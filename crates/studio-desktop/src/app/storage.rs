//! Actual storage location and explicit whole-workspace backup jobs.
use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Default)]
pub(super) struct StorageState {
    job: Option<StorageJob>,
    last_backup: Option<studio_core::WorkspaceBackup>,
}
struct StorageJob {
    receiver: mpsc::Receiver<AppResult<StorageOutcome>>,
    cancel: Option<Arc<AtomicBool>>,
}
enum StorageOutcome {
    Revealed,
    Backup(studio_core::WorkspaceBackup),
}

impl StorageState {
    pub(super) fn modal_open(&self) -> bool {
        self.job.as_ref().is_some_and(|job| job.cancel.is_some())
    }
    pub(super) fn busy(&self) -> bool {
        self.job.is_some()
    }
}
impl StudioApp {
    pub(super) fn storage_settings(&mut self, ui: &mut egui::Ui, t: Tokens) {
        let l = self.language;
        ui.label(RichText::new(l.text("Workspace database")).strong());
        let path = self
            .database
            .canonicalize()
            .unwrap_or_else(|_| self.database.clone());
        ui.add(egui::Label::new(path.to_string_lossy()).wrap())
            .on_hover_text(path.to_string_lossy());
        ui.label(RichText::new(l.text("Writing, history, recovery and owned artifacts are stored together in this database.")).size(13.0).color(t.muted));
        ui.add_space(8.0);
        let busy = self.storage.busy();
        ui.add_enabled_ui(!busy, |ui| {
            ui.horizontal_wrapped(|ui| {
                if components::action(ui, l.text("Open data folder"), true, None).clicked() {
                    self.open_data_folder(ui.ctx());
                }
                if components::action(ui, l.text("Back up workspace…"), true, Some(t)).clicked() {
                    let result = self.start_workspace_backup(ui.ctx());
                    self.record_error(result);
                }
            });
        });
        ui.label(RichText::new(l.text("A workspace backup includes saved writing, history, recovery and artifacts. Interface preferences are separate.")).size(13.0).color(t.muted));
        ui.label(
            RichText::new(l.text("Choose a new filename. Existing files are never replaced."))
                .size(13.0)
                .color(t.muted),
        );
        if let Some(backup) = &self.storage.last_backup {
            ui.add_space(12.0);
            ui.label(
                RichText::new(l.text("Verified workspace backup"))
                    .strong()
                    .color(t.good),
            );
            ui.add(egui::Label::new(backup.path.to_string_lossy()).wrap());
            ui.label(l.format(
                "Size: {bytes} bytes",
                &[("bytes", backup.byte_size.to_string())],
            ));
            ui.add(egui::Label::new(format!("SHA-256: {}", backup.sha256)).wrap());
        }
    }

    fn open_data_folder(&mut self, ctx: &egui::Context) {
        let database = self.database.clone();
        let (sender, receiver) = mpsc::channel();
        let ctx = ctx.clone();
        self.storage.job = Some(StorageJob {
            receiver,
            cancel: None,
        });
        std::thread::spawn(move || {
            let result = reveal_database(&database).map(|()| StorageOutcome::Revealed);
            let _ = sender.send(result);
            ctx.request_repaint();
        });
    }

    fn start_workspace_backup(&mut self, ctx: &egui::Context) -> AppResult<()> {
        self.save_explicit()?;
        #[cfg(feature = "editor-preview")]
        if self.notes.has_unsaved_drafts() {
            self.error = Some(UiError {
                msgid: "Save or discard pending note edits before backing up.",
                details: String::new(),
            });
            return Ok(());
        }
        let Some(destination) = rfd::FileDialog::new()
            .set_title(self.language.text("Back up workspace…"))
            .set_file_name("WriterCraft-workspace-backup.sqlite")
            .add_filter("SQLite", &["sqlite"])
            .save_file()
        else {
            return Ok(());
        };
        let source = self.database.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let (sender, receiver) = mpsc::channel();
        let ctx = ctx.clone();
        self.storage.job = Some(StorageJob {
            receiver,
            cancel: Some(cancel),
        });
        std::thread::spawn(move || {
            let result = studio_core::backup_workspace(source, destination, &worker_cancel)
                .map(StorageOutcome::Backup)
                .map_err(Into::into);
            let _ = sender.send(result);
            ctx.request_repaint();
        });
        Ok(())
    }

    pub(super) fn poll_storage(&mut self, ctx: &egui::Context) {
        let Some(job) = self.storage.job.take() else {
            return;
        };
        match job.receiver.try_recv() {
            Err(mpsc::TryRecvError::Empty) => {
                self.storage.job = Some(job);
                ctx.request_repaint_after(Duration::from_millis(100));
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.error = Some(UiError {
                    msgid: "The storage operation was interrupted. Try again.",
                    details: String::new(),
                });
            }
            Ok(Ok(StorageOutcome::Backup(receipt))) => self.storage.last_backup = Some(receipt),
            Ok(Ok(StorageOutcome::Revealed)) => {}
            Ok(Err(error)) => {
                if job
                    .cancel
                    .as_ref()
                    .is_some_and(|flag| flag.load(Ordering::Acquire))
                {
                    // A successfully published receipt above always wins over a late Cancel.
                    return;
                }
                self.error = Some(UiError {
                    msgid: if job.cancel.is_some() {
                        "The workspace backup could not be created. Choose a new filename or check the destination."
                    } else {
                        "The data folder could not be opened."
                    },
                    details: error.to_string(),
                });
            }
        }
    }

    pub(super) fn storage_dialog(&mut self, ctx: &egui::Context, t: Tokens) {
        let Some(cancel) = self.storage.job.as_ref().and_then(|job| job.cancel.clone()) else {
            return;
        };
        let l = self.language;
        let response = modal(
            ctx,
            l,
            "workspace-backup",
            l.text("Backing up workspace…"),
            480.0,
            |ui| {
                ui.label(l.text(
                    "Creating and verifying a consistent copy. Your workspace remains unchanged.",
                ));
                ui.add_space(12.0);
                ui.spinner();
                components::footer(ui, |ui| {
                    if components::action(
                        ui,
                        l.text("Cancel"),
                        !cancel.load(Ordering::Acquire),
                        Some(t),
                    )
                    .clicked()
                    {
                        cancel.store(true, Ordering::Release);
                    }
                });
            },
        );
        if response.should_close() {
            cancel.store(true, Ordering::Release);
        }
    }
}

fn reveal_database(database: &std::path::Path) -> AppResult<()> {
    let database = database.canonicalize()?;
    #[cfg(target_os = "macos")]
    let output = std::process::Command::new("open")
        .arg("-R")
        .arg(&database)
        .output()?;
    #[cfg(target_os = "windows")]
    let output = std::process::Command::new("explorer.exe")
        .arg(format!("/select,{}", database.display()))
        .output()?;
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let output = std::process::Command::new("xdg-open")
        .arg(database.parent().ok_or("missing_workspace_parent")?)
        .output()?;
    if !output.status.success() {
        return Err(
            std::io::Error::other(String::from_utf8_lossy(&output.stderr).into_owned()).into(),
        );
    }
    Ok(())
}
