//! Explicit preservation choices after automatic rich-draft recovery was refused.
use super::*;
use serde_json::json;
use std::path::{Path, PathBuf};
use studio_core::{RichRecoverySnapshot, StudioError};

#[derive(Debug)]
pub(super) struct ReadOnlyRecoveryView;
impl std::fmt::Display for ReadOnlyRecoveryView {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("recovery_view_is_read_only")
    }
}
impl std::error::Error for ReadOnlyRecoveryView {}

#[derive(Clone, Copy)]
enum Reason {
    Attachments,
    StaleBase,
    Changed,
    InvalidData,
    Other,
}
impl Reason {
    fn from_error(error: &(dyn std::error::Error + Send + Sync + 'static)) -> Self {
        match error.downcast_ref::<StudioError>() {
            Some(StudioError::DraftAnnotationConflict) => Self::Attachments,
            Some(StudioError::DraftConflict { .. } | StudioError::RevisionConflict { .. }) => {
                Self::StaleBase
            }
            Some(StudioError::DraftChanged) => Self::Changed,
            Some(StudioError::Json(_) | StudioError::CorruptData(_)) => Self::InvalidData,
            _ => Self::Other,
        }
    }
    fn message(self) -> &'static str {
        match self {
            Self::Attachments => "Recovery paused because note attachments changed.",
            Self::StaleBase => {
                "Recovery paused because the draft belongs to another saved revision."
            }
            Self::Changed => "Recovery paused because the draft changed in another session.",
            Self::InvalidData => "Recovery paused because the stored data needs inspection.",
            Self::Other => "Automatic recovery paused to preserve both copies.",
        }
    }
}
#[derive(Clone, Copy)]
enum Status {
    Changed,
    Reloaded,
    ReloadFailed,
    Exported,
    ExportFailed,
    ProtectedPath,
    OpenFailed,
}
impl Status {
    fn message(self) -> &'static str {
        match self {
            Self::Changed => {
                "Recovery data changed. Your captured copy is preserved; reload before continuing."
            }
            Self::Reloaded => "Recovery data refreshed.",
            Self::ReloadFailed => {
                "Recovery data could not be refreshed. Your captured copy is preserved."
            }
            Self::Exported => {
                "Recovery JSON exported. The original draft remains in this workspace."
            }
            Self::ExportFailed => {
                "Recovery JSON could not be exported. Choose another file or check write permissions."
            }
            Self::ProtectedPath => {
                "Choose a recovery file outside the active workspace database and its sidecar files."
            }
            Self::OpenFailed => {
                "The saved manuscript could not be opened. Both preserved copies remain available."
            }
        }
    }
}
pub(super) struct RecoveryDialog {
    snapshot: RichRecoverySnapshot,
    reason: Reason,
    status: Option<Status>,
    saved_preview: String,
    draft_preview: Option<String>,
}
impl RecoveryDialog {
    fn new(snapshot: RichRecoverySnapshot, reason: Reason, language: Language) -> Self {
        let saved_preview = preview_text(&snapshot.document().body, language);
        let draft_preview =
            craft_editor_model::Document::from_json(snapshot.draft().content.clone())
                .ok()
                .map(|tree| preview_text(&tree.plain_text(), language));
        Self {
            snapshot,
            reason,
            status: None,
            saved_preview,
            draft_preview,
        }
    }
}
fn preview_text(text: &str, l: Language) -> String {
    text.replace('\u{fffc}', l.text("[Embedded content]"))
}
fn recovery_json(snapshot: &RichRecoverySnapshot) -> serde_json::Result<Vec<u8>> {
    serde_json::to_vec_pretty(&json!({
        "format": "writercraft.document-recovery", "format_version": 1,
        "preservation_notice": "Data-only recovery evidence. This is not a portable workspace backup. It cannot authorize attachment remapping or native replay when imported.",
        "media": { "workspace_media_files_included": false, "embedded_json_preserved": true }, "snapshot": snapshot,
    }))
}
fn sidecar(database: &Path, suffix: &str) -> PathBuf {
    let mut path = database.as_os_str().to_owned();
    path.push(suffix);
    path.into()
}
fn normalized_file(path: &Path) -> std::io::Result<PathBuf> {
    match std::fs::canonicalize(path) {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            Ok(
                std::fs::canonicalize(parent)?.join(path.file_name().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "missing_export_file_name",
                    )
                })?),
            )
        }
        Err(error) => Err(error),
    }
}
/// Canonical aliases and portable file identities both protect the live database.
fn protected_export_path(target: &Path, database: &Path) -> std::io::Result<bool> {
    let target_normalized = normalized_file(target)?;
    for protected in [
        database.to_path_buf(),
        sidecar(database, "-wal"),
        sidecar(database, "-shm"),
    ] {
        let protected_normalized = normalized_file(&protected)?;
        let same_parent_name = target_normalized.parent() == protected_normalized.parent()
            && target_normalized
                .file_name()
                .zip(protected_normalized.file_name())
                .is_some_and(|(a, b)| {
                    a.to_string_lossy()
                        .eq_ignore_ascii_case(&b.to_string_lossy())
                });
        if target_normalized == protected_normalized || same_parent_name {
            return Ok(true);
        }
        if target.try_exists()?
            && protected.try_exists()?
            && same_file::is_same_file(target, &protected)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}
fn read_only_preview(
    ui: &mut egui::Ui,
    id: egui::Id,
    title: String,
    text: Option<&str>,
    height: f32,
    l: Language,
    t: Tokens,
) {
    ui.label(RichText::new(title).strong().size(14.0));
    egui::Frame::new()
        .fill(t.editor)
        .stroke(Stroke::new(1.0, t.border))
        .inner_margin(10)
        .corner_radius(6)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt(id)
                .max_height(height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(mut text) = text {
                        // An immutable TextBuffer keeps selection/Copy while refusing edits.
                        ui.add(
                            egui::TextEdit::multiline(&mut text)
                                .id(id.with("text"))
                                .font(egui::FontId::proportional(14.0))
                                .desired_width(f32::INFINITY)
                                .frame(egui::Frame::NONE),
                        );
                    } else {
                        ui.label(l.text(
                            "Preview unavailable. The exact draft remains in recovery JSON.",
                        ));
                    }
                });
        });
}
impl StudioApp {
    pub(super) fn show_recovery_conflict(
        &mut self,
        document_id: &str,
        error: &(dyn std::error::Error + Send + Sync + 'static),
    ) -> AppResult<()> {
        let snapshot = self.workspace.rich_recovery_snapshot(document_id)?;
        self.recovery = Some(RecoveryDialog::new(
            snapshot,
            Reason::from_error(error),
            self.language,
        ));
        Ok(())
    }
    pub(super) fn recovery_dialogs(&mut self, ctx: &egui::Context, t: Tokens) {
        if self.notes.close_confirmation_open() || self.error.is_some() {
            return;
        }
        let Some(mut dialog) = self.recovery.take() else {
            return;
        };
        let l = self.language;
        let mut export = false;
        let mut open_saved = false;
        let mut reload = false;
        let mut cancel = false;
        let response = components::modal(
            ctx,
            l,
            "blocked-document-recovery",
            l.text("Choose a preserved manuscript"),
            860.0,
            |ui| {
                egui::ScrollArea::vertical().id_salt("recovery-dialog-body").max_height((ctx.content_rect().height()-300.0).clamp(0.0,440.0)).auto_shrink([false,true]).show(ui, |ui| {
                ui.label(RichText::new(&dialog.snapshot.document().title).size(18.0).strong());
                ui.label(l.text(dialog.reason.message()));
                ui.add_space(8.0);
                ui.label(RichText::new(l.text("Read-only plain-text previews. Formatting and embedded content remain in the exact recovery JSON.")).size(13.0).color(t.muted));
                ui.add_space(12.0);
                let preview_height = (ctx.content_rect().height()*0.28).clamp(80.0,220.0);
                ui.columns(2, |columns| {
                    read_only_preview(&mut columns[0], egui::Id::new(("recovery-saved",&dialog.snapshot.document().id,&dialog.snapshot.draft().version)), l.format("Saved · revision {revision}",&[("revision",dialog.snapshot.document().revision.to_string())]), Some(&dialog.saved_preview), preview_height, l, t);
                    read_only_preview(&mut columns[1], egui::Id::new(("recovery-draft",&dialog.snapshot.document().id,&dialog.snapshot.draft().version)), l.format("Draft · base revision {revision}",&[("revision",dialog.snapshot.draft().base_revision.to_string())]), dialog.draft_preview.as_deref(), preview_height, l, t);
                });
                ui.add_space(12.0);
                ui.label(l.format("Preserved note records: {count}",&[("count",dialog.snapshot.annotations().len().to_string())]));
                ui.label(RichText::new(l.text("Includes deleted notes and their history.")).size(13.0).color(t.muted));
                ui.label(RichText::new(l.text("Separately stored media files remain in this workspace. Embedded JSON is preserved; recovery JSON is not a portable backup.")).size(13.0).color(t.muted));
                if let Some(status) = dialog.status { ui.add_space(8.0); ui.colored_label(t.warning,l.text(status.message())); }
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    export = components::action(ui,l.text("Export recovery JSON…"),true,None).clicked();
                    reload = components::action(ui,l.text("Reload recovery data"),true,None).clicked();
                });
            });
                components::footer(ui, |ui| {
                    open_saved =
                        components::action(ui, l.text("Open saved · read only"), true, Some(t))
                            .clicked();
                    cancel = components::action(ui, l.text("Cancel"), true, None).clicked();
                });
            },
        );
        if export {
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name("writercraft-document-recovery.json")
                .add_filter(l.text("WriterCraft recovery JSON"), &["json"])
                .save_file()
            {
                match protected_export_path(&path, &self.database) {
                    Ok(true) => dialog.status = Some(Status::ProtectedPath),
                    Err(_) => dialog.status = Some(Status::ExportFailed),
                    Ok(false) => {
                        if self
                            .workspace
                            .verify_rich_recovery_snapshot(&dialog.snapshot)
                            .is_err()
                        {
                            dialog.status = Some(Status::Changed);
                        } else {
                            let result = recovery_json(&dialog.snapshot)
                                .map_err(|error| -> Box<dyn std::error::Error + Send + Sync> {
                                    Box::new(error)
                                })
                                .and_then(|bytes| atomic_write(&path, &bytes));
                            dialog.status = Some(if result.is_ok() {
                                Status::Exported
                            } else {
                                Status::ExportFailed
                            });
                        }
                    }
                }
            }
        }
        if reload {
            match self
                .workspace
                .rich_recovery_snapshot(&dialog.snapshot.document().id)
            {
                Ok(snapshot)
                    if snapshot.document().project_id == dialog.snapshot.document().project_id =>
                {
                    dialog = RecoveryDialog::new(snapshot, dialog.reason, l);
                    dialog.status = Some(Status::Reloaded);
                }
                _ => dialog.status = Some(Status::ReloadFailed),
            }
        }
        if open_saved {
            if self
                .workspace
                .verify_rich_recovery_snapshot(&dialog.snapshot)
                .is_err()
            {
                dialog.status = Some(Status::Changed);
            } else if self.open_saved_recovery(&dialog.snapshot).is_ok() {
                return;
            } else {
                dialog.status = Some(Status::OpenFailed);
            }
        }
        if !cancel && !response.should_close() {
            self.recovery = Some(dialog);
        } else {
            self.focus_manuscript = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult = AppResult<()>;
    #[test]
    fn recovery_export_refuses_database_sidecars_and_existing_aliases() -> TestResult {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("workspace.sqlite");
        std::fs::write(&database, b"synthetic workspace")?;
        assert!(protected_export_path(&database, &database)?);
        assert!(protected_export_path(
            &sidecar(&database, "-wal"),
            &database
        )?);
        assert!(protected_export_path(
            &sidecar(&database, "-shm"),
            &database
        )?);
        let hardlink = directory.path().join("alias.json");
        std::fs::hard_link(&database, &hardlink)?;
        assert!(protected_export_path(&hardlink, &database)?);
        #[cfg(unix)]
        {
            let symlink = directory.path().join("symlink.json");
            std::os::unix::fs::symlink(&database, &symlink)?;
            assert!(protected_export_path(&symlink, &database)?);
        }
        assert!(!protected_export_path(
            &directory.path().join("recovery.json"),
            &database
        )?);
        assert_eq!(std::fs::read(&database)?, b"synthetic workspace");
        Ok(())
    }
    #[test]
    fn derived_readonly_preview_preserves_complete_unicode_without_atom_boxes() {
        let original = format!("{}\u{fffc}", "Я😀中\n".repeat(3_000));
        let preview = preview_text(&original, Language::English);
        assert!(!preview.contains('\u{fffc}'));
        assert!(preview.starts_with(&"Я😀中\n".repeat(3_000)));
        assert_eq!(original.chars().last(), Some('\u{fffc}'));
    }
    fn paste_into(buffer: &mut dyn egui::TextBuffer) {
        let context = egui::Context::default();
        let id = egui::Id::new("readonly-recovery-preview");
        context.memory_mut(|memory| memory.request_focus(id));
        for step in 0..2 {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(600.0, 400.0),
                    )),
                    focused: true,
                    events: if step == 1 {
                        vec![egui::Event::Paste("replacement".into())]
                    } else {
                        vec![]
                    },
                    ..Default::default()
                },
                |root| {
                    egui::CentralPanel::default().show(root, |ui| {
                        ui.add(egui::TextEdit::multiline(buffer).id(id));
                    });
                },
            );
            output.textures_delta.clear();
        }
    }
    #[test]
    fn readonly_preview_buffer_refuses_native_paste() {
        let original = "Preserved Я😀中";
        let mut value = original;
        paste_into(&mut value);
        assert_eq!(value, original);
        let mut control = original.to_owned();
        paste_into(&mut control);
        assert!(
            control.contains("replacement"),
            "the same native event must reach an editable control"
        );
    }
}
