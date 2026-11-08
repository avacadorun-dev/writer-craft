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
    attempt: Option<BackupAttempt>,
    #[cfg(feature = "editor-preview")]
    preflight: Option<BackupPreflight>,
    #[cfg(feature = "editor-preview")]
    board_preflight: Option<BoardBackupPreflight>,
}
struct StorageJob {
    receiver: mpsc::Receiver<AppResult<StorageOutcome>>,
    cancel: Option<Arc<AtomicBool>>,
}
enum StorageOutcome {
    Revealed,
    Backup(studio_core::WorkspaceBackup),
}
#[derive(Clone, Copy)]
enum BackupAttempt {
    #[cfg(feature = "editor-preview")]
    WaitingForNotes,
    #[cfg(feature = "editor-preview")]
    WaitingForBoard,
    Selecting,
    Running,
    Verified,
    Cancelled,
    Failed,
}
impl BackupAttempt {
    fn message(self) -> &'static str {
        match self {
            #[cfg(feature = "editor-preview")]
            Self::WaitingForNotes => "Review pending notes before backing up.",
            #[cfg(feature = "editor-preview")]
            Self::WaitingForBoard => "Waiting for the board to finish saving.",
            Self::Selecting => "Choose a backup file.",
            Self::Running => "Backing up workspace…",
            Self::Verified => "Backup created and verified.",
            Self::Cancelled => "The latest backup attempt was cancelled.",
            Self::Failed => "The latest backup attempt failed.",
        }
    }
}
#[cfg(feature = "editor-preview")]
struct BackupPreflight {
    documents: Vec<PendingNoteDocument>,
}
#[cfg(feature = "editor-preview")]
struct BoardBackupPreflight {
    project_id: String,
    board_id: String,
    title: String,
}
#[cfg(feature = "editor-preview")]
struct PendingNoteDocument {
    id: String,
    title: String,
    project_title: String,
    available: bool,
}
impl StorageState {
    pub(super) fn modal_open(&self) -> bool {
        #[cfg(feature = "editor-preview")]
        if self.preflight.is_some() || self.board_preflight.is_some() {
            return true;
        }
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
        if let Some(attempt) = self.storage.attempt {
            let color = match attempt {
                BackupAttempt::Verified => t.good,
                BackupAttempt::Failed => t.warning,
                _ => t.muted,
            };
            ui.add(egui::Label::new(RichText::new(l.text(attempt.message())).color(color)).wrap());
        }
        if let Some(backup) = &self.storage.last_backup {
            ui.add_space(12.0);
            ui.label(RichText::new(l.text("Last verified workspace backup")).strong());
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

    /// Prepare explicit backup prerequisites without committing a note or opening a file dialog.
    pub(super) fn prepare_workspace_backup(&mut self) -> AppResult<bool> {
        self.storage.attempt = Some(BackupAttempt::Selecting);
        #[cfg(feature = "editor-preview")]
        match self.flush_canvas_for_transfer(None) {
            Ok(false) => {
                if let Some(state) = &self.canvas {
                    self.storage.board_preflight = Some(BoardBackupPreflight {
                        project_id: state.project_id.clone(),
                        board_id: state.snapshot.canvas.id.clone(),
                        title: state.snapshot.canvas.title.clone(),
                    });
                    self.storage.attempt = Some(BackupAttempt::WaitingForBoard);
                    return Ok(false);
                }
            }
            Err(error) => {
                self.storage.attempt = Some(BackupAttempt::Failed);
                return Err(error);
            }
            Ok(true) => {}
        }
        if let Err(error) = self.save_explicit() {
            self.storage.attempt = Some(BackupAttempt::Failed);
            return Err(error);
        }
        #[cfg(feature = "editor-preview")]
        if self.notes.has_unsaved_drafts() {
            if let Err(error) = self.refresh_backup_note_preflight() {
                self.storage.attempt = Some(BackupAttempt::Failed);
                return Err(error);
            }
            self.storage.attempt = Some(BackupAttempt::WaitingForNotes);
            return Ok(false);
        }
        #[cfg(feature = "editor-preview")]
        {
            self.storage.preflight = None;
            self.storage.board_preflight = None;
        }
        Ok(true)
    }
    fn start_workspace_backup(&mut self, ctx: &egui::Context) -> AppResult<()> {
        if self.prepare_workspace_backup()? {
            self.choose_workspace_backup_destination(ctx);
        }
        Ok(())
    }
    fn choose_workspace_backup_destination(&mut self, ctx: &egui::Context) {
        // Every entry route rechecks asynchronous writing and note prerequisites immediately.
        match self.prepare_workspace_backup() {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => {
                self.record_error(Err(error));
                return;
            }
        }
        self.storage.attempt = Some(BackupAttempt::Selecting);
        let mut dialog = rfd::FileDialog::new()
            .set_title(self.language.text("Back up workspace…"))
            .set_file_name(backup_file_name(time::OffsetDateTime::now_utc()))
            .add_filter("SQLite", &["sqlite"]);
        if let Some(parent) = self
            .storage
            .last_backup
            .as_ref()
            .and_then(|backup| backup.path.parent())
        {
            dialog = dialog.set_directory(parent);
        }
        let Some(destination) = dialog.save_file() else {
            self.storage.attempt = Some(BackupAttempt::Cancelled);
            return;
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
        self.storage.attempt = Some(BackupAttempt::Running);
        std::thread::spawn(move || {
            let result = studio_core::backup_workspace(source, destination, &worker_cancel)
                .map(StorageOutcome::Backup)
                .map_err(Into::into);
            let _ = sender.send(result);
            ctx.request_repaint();
        });
    }

    #[cfg(feature = "editor-preview")]
    fn refresh_backup_note_preflight(&mut self) -> AppResult<()> {
        let projects = self.workspace.list_projects()?;
        let mut documents: Vec<_> = self
            .notes
            .pending_documents()
            .into_iter()
            .map(|id| match self.workspace.document(&id) {
                Ok(document) => PendingNoteDocument {
                    id,
                    title: document.title,
                    project_title: projects
                        .iter()
                        .find(|project| project.id == document.project_id)
                        .map_or_else(String::new, |project| project.title.clone()),
                    available: true,
                },
                Err(_) => PendingNoteDocument {
                    id,
                    title: self.language.text("Unavailable document").into(),
                    project_title: String::new(),
                    available: false,
                },
            })
            .collect();
        documents.sort_by(|a, b| {
            (&a.project_title, &a.title, &a.id).cmp(&(&b.project_title, &b.title, &b.id))
        });
        self.storage.preflight = Some(BackupPreflight { documents });
        Ok(())
    }
    /// Save only after the explicit preflight action. Failures keep every unacknowledged body.
    #[cfg(feature = "editor-preview")]
    pub(super) fn save_backup_note_prerequisites(&mut self) -> AppResult<()> {
        let result = self.save_note_drafts();
        if result.is_err() || self.notes.has_unsaved_drafts() {
            let pending = self.notes.pending_documents();
            if let Some(preflight) = &mut self.storage.preflight {
                preflight
                    .documents
                    .retain(|document| pending.contains(&document.id));
            }
            result?;
            return Err("pending_note_drafts_remain".into());
        }
        self.storage.preflight = None;
        Ok(())
    }
    #[cfg(feature = "editor-preview")]
    pub(super) fn review_backup_notes(&mut self, id: &str) -> AppResult<()> {
        let document = self.workspace.document(id)?;
        let project = self
            .workspace
            .list_projects()?
            .into_iter()
            .find(|project| project.id == document.project_id)
            .ok_or("missing_pending_note_project")?;
        self.open_project(project)?;
        self.open_document(document)?;
        self.notes.select_pending_draft(id);
        self.storage.preflight = None;
        self.storage.board_preflight = None;
        self.storage.attempt = Some(BackupAttempt::Cancelled);
        self.settings_open = false;
        self.inspector = Inspector::Notes;
        self.inspector_visible = true;
        self.prefer_inspector = true;
        self.focus_mode = false;
        Ok(())
    }

    #[cfg(feature = "editor-preview")]
    pub(super) fn cancel_workspace_backup_preflight(&mut self) {
        self.storage.preflight = None;
        self.storage.attempt = Some(BackupAttempt::Cancelled);
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
                if job.cancel.is_some() {
                    self.storage.attempt = Some(BackupAttempt::Failed);
                }
                self.error = Some(UiError {
                    msgid: "The storage operation was interrupted. Try again.",
                    details: String::new(),
                });
            }
            Ok(Ok(StorageOutcome::Backup(receipt))) => {
                self.storage.last_backup = Some(receipt);
                self.storage.attempt = Some(BackupAttempt::Verified);
            }
            Ok(Ok(StorageOutcome::Revealed)) => {}
            Ok(Err(error)) => {
                if job
                    .cancel
                    .as_ref()
                    .is_some_and(|flag| flag.load(Ordering::Acquire))
                {
                    // A successfully published receipt above always wins over a late Cancel.
                    self.storage.attempt = Some(BackupAttempt::Cancelled);
                    return;
                }
                if job.cancel.is_some() {
                    self.storage.attempt = Some(BackupAttempt::Failed);
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

    #[cfg(feature = "editor-preview")]
    fn board_backup_dialog(&mut self, ctx: &egui::Context, t: Tokens) -> bool {
        let Some(preflight) = &self.storage.board_preflight else {
            return false;
        };
        let l = self.language;
        let same = self.canvas.as_ref().is_some_and(|state| {
            state.project_id == preflight.project_id
                && state.snapshot.canvas.id == preflight.board_id
        });
        let ready = same
            && self
                .canvas
                .as_ref()
                .is_some_and(|state| !state.blocks_navigation());
        let status = self
            .canvas
            .as_ref()
            .map(|state| state.durability_status())
            .unwrap_or("Attention needed");
        let project_title = self
            .projects
            .iter()
            .find(|project| project.id == preflight.project_id)
            .map(|project| project.title.as_str())
            .unwrap_or_default();
        let mut review = false;
        let mut cancel = false;
        let mut proceed = false;
        let response = modal(
            ctx,
            l,
            "workspace-board-prerequisite",
            l.text("Back up workspace…"),
            560.0,
            |ui| {
                ui.add(egui::Label::new(RichText::new(&preflight.title).strong()).truncate())
                    .on_hover_text(&preflight.title);
                if !project_title.is_empty() {
                    ui.add(
                        egui::Label::new(RichText::new(project_title).color(t.muted)).truncate(),
                    );
                }
                ui.add_space(12.0);
                ui.label(l.text(if same {
                    "Waiting for the board to finish saving."
                } else {
                    "The board changed. Review it before backing up."
                }));
                ui.label(RichText::new(l.text(status)).color(if ready { t.good } else { t.muted }));
                components::footer(ui, |ui| {
                    proceed = components::action(ui, l.text("Back up workspace…"), ready, Some(t))
                        .clicked();
                    review = components::action(ui, l.text("Review board"), same, None).clicked();
                    cancel = components::action(ui, l.text("Cancel"), true, None).clicked();
                });
            },
        );
        if cancel || response.should_close() {
            self.cancel_workspace_backup_preflight();
        } else if review {
            // End only the backup attempt, preserving the same native card/editor/draft.
            self.cancel_workspace_backup_preflight();
            self.settings_open = false;
        } else if proceed {
            self.choose_workspace_backup_destination(ctx);
        }
        true
    }

    pub(super) fn storage_dialog(&mut self, ctx: &egui::Context, t: Tokens) {
        // Let the existing actionable error modal win; dismissing it returns to this preflight.
        if self.error.is_some() {
            return;
        }
        #[cfg(feature = "editor-preview")]
        if self.board_backup_dialog(ctx, t) {
            return;
        }
        #[cfg(feature = "editor-preview")]
        if let Some(preflight) = self.storage.preflight.take() {
            let l = self.language;
            let mut save = false;
            let mut review = None;
            let mut cancel = false;
            let response = modal(
                ctx,
                l,
                "backup-note-prerequisites",
                l.text("Save pending notes before backup?"),
                620.0,
                |ui| {
                    ui.label(l.text("These documents have unsaved notes. Save them explicitly or open a document to review its drafts."));
                    ui.add_space(12.0);
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for document in &preflight.documents {
                                egui::Frame::new()
                                    .inner_margin(egui::Margin::symmetric(0, 8))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let action_width = ui
                                                .painter()
                                                .layout_no_wrap(
                                                    l.text("Review drafts").into(),
                                                    egui::TextStyle::Button.resolve(ui.style()),
                                                    t.text,
                                                )
                                                .size()
                                                .x
                                                + ui.spacing().button_padding.x * 2.0;
                                            let title_width = (ui.available_width()
                                                - action_width.max(112.0)
                                                - 8.0)
                                                .max(80.0);
                                            ui.allocate_ui_with_layout(
                                                egui::vec2(title_width, 40.0),
                                                Layout::top_down(Align::Min),
                                                |ui| {
                                                    ui.add(
                                                        egui::Label::new(
                                                            RichText::new(&document.title).strong(),
                                                        )
                                                        .truncate(),
                                                    )
                                                    .on_hover_text(&document.title);
                                                    if !document.project_title.is_empty() {
                                                        ui.add(
                                                            egui::Label::new(
                                                                RichText::new(
                                                                    &document.project_title,
                                                                )
                                                                .size(12.0)
                                                                .color(t.muted),
                                                            )
                                                            .truncate(),
                                                        )
                                                        .on_hover_text(&document.project_title);
                                                    }
                                                },
                                            );
                                            if components::action(
                                                ui,
                                                l.text("Review drafts"),
                                                document.available,
                                                None,
                                            )
                                            .clicked()
                                            {
                                                review = Some(document.id.clone());
                                            }
                                        });
                                    });
                            }
                        });
                    components::footer(ui, |ui| {
                        save = components::action(
                            ui,
                            l.text("Save all notes and continue"),
                            true,
                            Some(t),
                        )
                        .clicked();
                        cancel = components::action(ui, l.text("Cancel"), true, None).clicked();
                    });
                },
            );
            self.storage.preflight = Some(preflight);
            if cancel || response.should_close() {
                self.cancel_workspace_backup_preflight();
            } else if let Some(id) = review {
                let result = self.review_backup_notes(&id);
                self.record_error(result);
            } else if save {
                let result = self.save_backup_note_prerequisites();
                if result.is_ok() {
                    self.choose_workspace_backup_destination(ctx);
                }
                self.record_error(result);
            }
            return;
        }
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

fn backup_file_name(now: time::OffsetDateTime) -> String {
    format!(
        "WriterCraft-backup-{:04}-{:02}-{:02}-{:02}{:02}{:02}-{:03}Z.sqlite",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
        now.millisecond()
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult = AppResult<()>;

    fn fixture() -> AppResult<(tempfile::TempDir, StudioApp)> {
        let directory = tempfile::tempdir()?;
        let creation = eframe::CreationContext::_new_kittest(egui::Context::default());
        let app = StudioApp::new(
            &creation,
            directory.path().join("backup-status.sqlite"),
            false,
            None,
        )?;
        Ok((directory, app))
    }

    #[test]
    fn backup_filename_is_readable_utc_and_changes_between_milliseconds() -> TestResult {
        let first = time::OffsetDateTime::from_unix_timestamp_nanos(123_000_000)?;
        let second = time::OffsetDateTime::from_unix_timestamp_nanos(124_000_000)?;
        assert_eq!(
            backup_file_name(first),
            "WriterCraft-backup-1970-01-01-000000-123Z.sqlite"
        );
        assert_ne!(backup_file_name(first), backup_file_name(second));
        Ok(())
    }

    #[test]
    fn a_cancelled_attempt_does_not_masquerade_as_the_previous_verified_backup() -> TestResult {
        let (directory, mut app) = fixture()?;
        let prior = studio_core::backup_workspace(
            &app.database,
            directory.path().join("prior.sqlite"),
            &AtomicBool::new(false),
        )?;
        app.storage.last_backup = Some(prior.clone());
        let (sender, receiver) = mpsc::channel();
        sender
            .send(Err(std::io::Error::other("cancelled worker").into()))
            .map_err(|_| "test backup channel disconnected")?;
        app.storage.job = Some(StorageJob {
            receiver,
            cancel: Some(Arc::new(AtomicBool::new(true))),
        });
        app.poll_storage(&egui::Context::default());
        assert!(matches!(
            app.storage.attempt,
            Some(BackupAttempt::Cancelled)
        ));
        assert_eq!(app.storage.last_backup, Some(prior));
        assert!(app.error.is_none());
        Ok(())
    }

    #[test]
    fn a_failed_attempt_keeps_the_last_verified_receipt_and_explicit_failure() -> TestResult {
        let (directory, mut app) = fixture()?;
        let prior = studio_core::backup_workspace(
            &app.database,
            directory.path().join("prior.sqlite"),
            &AtomicBool::new(false),
        )?;
        app.storage.last_backup = Some(prior.clone());
        let (sender, receiver) = mpsc::channel();
        sender
            .send(Err(std::io::Error::other("failed worker").into()))
            .map_err(|_| "test backup channel disconnected")?;
        app.storage.job = Some(StorageJob {
            receiver,
            cancel: Some(Arc::new(AtomicBool::new(false))),
        });
        app.poll_storage(&egui::Context::default());
        assert!(matches!(app.storage.attempt, Some(BackupAttempt::Failed)));
        assert_eq!(app.storage.last_backup, Some(prior));
        assert!(app.error.is_some());
        Ok(())
    }

    #[test]
    fn an_acknowledged_publication_wins_over_a_late_cancel() -> TestResult {
        let (directory, mut app) = fixture()?;
        let receipt = studio_core::backup_workspace(
            &app.database,
            directory.path().join("complete.sqlite"),
            &AtomicBool::new(false),
        )?;
        let (sender, receiver) = mpsc::channel();
        sender
            .send(Ok(StorageOutcome::Backup(receipt.clone())))
            .map_err(|_| "test backup channel disconnected")?;
        app.storage.job = Some(StorageJob {
            receiver,
            cancel: Some(Arc::new(AtomicBool::new(true))),
        });
        app.poll_storage(&egui::Context::default());
        assert!(matches!(app.storage.attempt, Some(BackupAttempt::Verified)));
        assert_eq!(app.storage.last_backup, Some(receipt));
        Ok(())
    }
}
