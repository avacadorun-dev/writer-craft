//! Bounded native writer workspace: state and command orchestration.
mod commands;
mod components;
mod dialogs;
mod library;
mod manuscript;
mod navigator;
#[cfg(feature = "editor-preview")]
mod rendering;
mod review;
#[cfg(feature = "editor-preview")]
mod rich_editor;
mod settings;
use crate::{
    i18n::{InterfacePreferences, Language},
    icons::{self, Icon},
    theme::{self, Appearance, Palette, ThemeSettings, Tokens},
};
use egui::{Align, Color32, FontFamily, FontId, Layout, RichText, Stroke, Vec2};
use std::{
    io::Write,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};
use studio_core::{Document, Project, Proposal, ProposalStatus, TextAnchor, Workspace};

#[derive(Clone)]
struct UiError {
    msgid: &'static str,
    details: String,
}

type AppResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Copy, PartialEq)]
enum Inspector {
    Review,
    Versions,
}

#[derive(Clone)]
struct FrozenSelection {
    document_id: String,
    base_revision: i64,
    body: String,
    anchor: TextAnchor,
}

pub struct StudioApp {
    workspace: Workspace,
    database: PathBuf,
    projects: Vec<Project>,
    project: Option<Project>,
    documents: Vec<Document>,
    document: Option<Document>,
    body: String,
    #[cfg(feature = "editor-preview")]
    rich: Option<rich_editor::RichState>,
    dirty: Option<Instant>,
    autosave_paused: bool,
    library: bool,
    navigator: bool,
    focus_mode: bool,
    prefer_inspector: bool,
    frozen_selection: Option<FrozenSelection>,
    full_review: Option<Proposal>,
    inspector_visible: bool,
    inspector: Inspector,
    search: String,
    status: String,
    error: Option<UiError>,
    language: Language,
    settings: ThemeSettings,
    settings_open: bool,
    settings_section: settings::Section,
    focus_manuscript: bool,
    font_size: f32,
    project_dialog: bool,
    project_name: String,
    document_dialog: bool,
    document_name: String,
    expected: String,
    replacement: String,
    focus_replacement: bool,
    reason: String,
    proposals: Vec<Proposal>,
    import_job: Option<
        mpsc::Receiver<Result<studio_transfer::ImportPreview, studio_transfer::TransferError>>,
    >,
    import_preview: Option<studio_transfer::ImportPreview>,
    capture: Option<PathBuf>,
    frames: u32,
}

impl StudioApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        database: PathBuf,
        demo: bool,
        capture: Option<PathBuf>,
    ) -> AppResult<Self> {
        let (preferences, preferences_error) =
            match InterfacePreferences::load(database.with_file_name("interface.json")) {
                Ok(preferences) => (preferences, None),
                Err(error) => (
                    InterfacePreferences::default(),
                    Some(UiError {
                        msgid: "The saved data could not be decoded.",
                        details: error.to_string(),
                    }),
                ),
            };
        let workspace = Workspace::open(&database)?;
        let settings = std::fs::read(database.with_file_name("appearance.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        install_fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx, &settings);
        let mut app = Self {
            workspace,
            database,
            projects: vec![],
            project: None,
            documents: vec![],
            document: None,
            body: String::new(),
            #[cfg(feature = "editor-preview")]
            rich: None,
            dirty: None,
            autosave_paused: false,
            library: true,
            navigator: true,
            focus_mode: false,
            prefer_inspector: false,
            frozen_selection: None,
            full_review: None,
            inspector_visible: false,
            inspector: Inspector::Review,
            search: String::new(),
            status: if preferences_error.is_some() {
                "Attention needed"
            } else {
                "All changes saved"
            }
            .into(),
            error: preferences_error,
            language: preferences.language,
            settings,
            settings_open: false,
            settings_section: settings::Section::Appearance,
            focus_manuscript: false,
            font_size: preferences.font_size,
            project_dialog: false,
            project_name: String::new(),
            document_dialog: false,
            document_name: String::new(),
            expected: String::new(),
            replacement: String::new(),
            focus_replacement: false,
            reason: String::new(),
            proposals: vec![],
            import_job: None,
            import_preview: None,
            capture,
            frames: 0,
        };
        app.projects = app.workspace.list_projects()?;
        if std::env::args().any(|argument| argument == "--settings") {
            app.settings_open = true;
        }
        if std::env::args().any(|argument| argument == "--light") {
            app.settings.appearance = Appearance::Light;
            theme::apply(&cc.egui_ctx, &app.settings);
        }
        if let Ok(code) = std::env::var("WRITERCRAFT_CAPTURE_LANGUAGE")
            && app.capture.is_some()
            && let Some(language) = Language::ALL
                .into_iter()
                .find(|language| language.code() == code)
        {
            app.language = language;
        }
        if app.capture.is_some() {
            app.settings_section = match std::env::var("WRITERCRAFT_CAPTURE_SECTION").as_deref() {
                Ok("general") => settings::Section::General,
                Ok("editor") => settings::Section::Editor,
                Ok("mcp") => settings::Section::Mcp,
                _ => app.settings_section,
            };
        }
        if demo {
            if app.projects.is_empty() {
                #[cfg(feature = "editor-preview")]
                if crate::editor_preview_requested() {
                    app.create_editor_example()?;
                } else {
                    app.create_example()?;
                }
                #[cfg(not(feature = "editor-preview"))]
                app.create_example()?;
            }
            if let Some(project) = app.projects.first().cloned() {
                #[cfg(feature = "editor-preview")]
                let showcase = if crate::editor_preview_requested() {
                    Some(app.ensure_editor_showcase(&project)?)
                } else {
                    None
                };
                app.open_project(project)?;
                #[cfg(feature = "editor-preview")]
                if let Some(showcase) = showcase {
                    app.open_document(showcase)?;
                }
            }
        }
        if app.capture.is_some() && std::env::args().any(|argument| argument == "--capture-review")
        {
            app.expected = "the sea was almost white".into();
            app.replacement = "the sea was pale beneath the morning sky".into();
            app.reason = "Keep the image precise without changing the author's voice.".into();
            app.inspector_visible = true;
            app.prefer_inspector = true;
            app.stage()?;
        }
        Ok(app)
    }

    fn create_example(&mut self) -> AppResult<()> {
        let project = self.workspace.create_project("Where the sea begins")?;
        self.workspace
            .create_document(&project.id, "01. The lighthouse house", SAMPLE)?;
        self.workspace.create_document(&project.id, "02. A letter without an address", "There was only one word on the back of the envelope: Return.\n\nAsya read it twice and left the letter on the windowsill. Outside, the rain was beginning.")?;
        self.workspace.create_document(&project.id, "Story notes", "Asya is an art restorer returning to the seaside town where she grew up.\n\nThe lighthouse stopped working nineteen years ago.\n\nA central theme: sometimes coming home means choosing your own direction for the first time.")?;
        self.projects = self.workspace.list_projects()?;
        Ok(())
    }

    fn open_project(&mut self, project: Project) -> AppResult<()> {
        self.save()?;
        self.documents = self.workspace.list_documents(&project.id)?;
        self.project = Some(project);
        self.document = None;
        self.body.clear();
        #[cfg(feature = "editor-preview")]
        {
            self.rich = None;
        }
        self.library = false;
        if let Some(document) = self.documents.first().cloned() {
            self.open_document(document)?;
        }
        Ok(())
    }

    fn open_document(&mut self, document: Document) -> AppResult<()> {
        self.save()?;
        let document = self.workspace.document(&document.id)?;
        #[cfg(feature = "editor-preview")]
        let rich = rich_editor::RichState::open(&self.workspace, &document)?;
        self.body.clone_from(&document.body);
        self.dirty = None;
        self.autosave_paused = false;
        if self.error.is_none() {
            self.status = "All changes saved".into();
        }
        if document.rich_json.is_none()
            && let Some(draft) = self.workspace.draft(&document.id)?
        {
            if draft.base_revision == document.revision && draft.body != document.body {
                self.body = draft.body;
                self.dirty = Some(Instant::now());
                self.status = "Unsaved text recovered".into();
            } else if draft.base_revision != document.revision {
                self.workspace.recover_draft_as_document(&document.id)?;
                self.documents = self.workspace.list_documents(&document.project_id)?;
                self.status = "Unsaved text recovered as a separate document".into();
            }
        }
        #[cfg(feature = "editor-preview")]
        {
            if rich.as_ref().is_some_and(|state| state.recovered) {
                self.dirty = Some(Instant::now());
                self.status = "Unsaved text recovered".into();
            }
            if let Some(state) = &rich {
                self.body = state.session.document().plain_text();
            }
            self.rich = rich;
        }
        self.proposals = self.workspace.proposals(&document.id)?;
        self.document = Some(document);
        self.expected.clear();
        self.frozen_selection = None;
        self.replacement.clear();
        Ok(())
    }

    fn save(&mut self) -> AppResult<()> {
        let result = self.save_dirty_document();
        self.autosave_paused = result.is_err();
        result
    }

    fn save_dirty_document(&mut self) -> AppResult<()> {
        if self.dirty.is_none() {
            return Ok(());
        }
        #[cfg(feature = "editor-preview")]
        if self.rich.is_some() {
            return self.save_rich_editor();
        }
        if let Some(document) = self.document.as_ref() {
            let saved =
                self.workspace
                    .save_document(&document.id, document.revision, &self.body)?;
            self.document = Some(saved);
            if let Some(project) = &self.project {
                self.documents = self.workspace.list_documents(&project.id)?;
            }
            self.dirty = None;
            self.status = "All changes saved".into();
        }
        Ok(())
    }

    fn record_error(&mut self, result: AppResult<()>) {
        if let Err(error) = result {
            self.error = Some(UiError {
                msgid: error_message(error.as_ref()),
                details: error.to_string(),
            });
            self.status = "Attention needed".into();
        }
    }

    fn stage(&mut self) -> AppResult<()> {
        let l = self.language;
        self.save()?;
        if let Some(document) = &self.document {
            let reason = if self.reason.trim().is_empty() {
                l.text("Local author edit")
            } else {
                &self.reason
            };
            if let Some(selection) = &self.frozen_selection {
                if selection.document_id != document.id
                    || selection.body != self.body
                    || selection.base_revision != document.revision
                {
                    return Err(studio_core::StudioError::StaleProposal {
                        expected: selection.base_revision,
                        actual: document.revision,
                    }
                    .into());
                }
                self.workspace.stage_replace_range(
                    &document.id,
                    document.revision,
                    &selection.anchor,
                    &self.replacement,
                    reason,
                )?;
            } else {
                self.workspace.stage_replace(
                    &document.id,
                    document.revision,
                    &self.expected,
                    &self.replacement,
                    reason,
                )?;
            }
            self.refresh_proposals()?;
            self.status = "Proposal prepared; manuscript unchanged".into();
        }
        Ok(())
    }
    fn refresh_proposals(&mut self) -> AppResult<()> {
        if let Some(document) = &self.document {
            self.proposals = self.workspace.proposals(&document.id)?;
        }
        Ok(())
    }
    fn apply(&mut self, id: &str) -> AppResult<()> {
        self.save()?;
        let document = self.workspace.apply_proposal(id)?;
        self.open_document(document)?;
        self.status = "Changes applied; undo is available".into();
        Ok(())
    }
    fn undo(&mut self, id: &str) -> AppResult<()> {
        self.save()?;
        let document = self.workspace.undo_proposal(id)?;
        self.open_document(document)?;
        self.status = "Apply undone".into();
        Ok(())
    }

    fn choose_archive(&mut self) {
        let l = self.language;
        if self.import_job.is_some() {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(l.text("Writers Studio project"), &["wstudio"])
            .pick_file()
        {
            let (sender, receiver) = mpsc::channel();
            std::thread::spawn(move || {
                let result = studio_transfer::inspect_archive(path);
                let _ = sender.send(result);
            });
            self.import_job = Some(receiver);
            self.status = "Checking project archive…".into();
        }
    }
    fn export_text(&mut self) {
        let l = self.language;
        let Some(document) = &self.document else {
            return;
        };
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name(format!("{}.txt", document.title))
            .add_filter(l.text("Plain text"), &["txt"])
            .save_file()
        {
            let result = atomic_write(&path, self.body.as_bytes());
            if result.is_ok() {
                self.status = "Text exported".into();
            }
            self.record_error(result);
        }
    }

    fn navigator_width(&self, ctx: &egui::Context) -> f32 {
        egui::containers::panel::PanelState::load(ctx, egui::Id::new("navigator"))
            .map_or(264.0, |state| state.outer_rect.width().clamp(210.0, 340.0))
    }
    fn effective_panes(&self, ctx: &egui::Context) -> (bool, bool) {
        let nav = self.navigator_width(ctx);
        let inspector = egui::containers::panel::PanelState::load(ctx, egui::Id::new("inspector"))
            .map_or(320.0, |state| state.outer_rect.width().clamp(280.0, 460.0));
        resolve_panes(
            ctx.content_rect().width(),
            nav,
            inspector,
            self.navigator,
            self.inspector_visible,
            self.prefer_inspector,
            self.focus_mode,
        )
    }
    fn rich_modal_open(&self) -> bool {
        #[cfg(feature = "editor-preview")]
        {
            self.rich
                .as_ref()
                .is_some_and(rich_editor::RichState::dialog_open)
        }
        #[cfg(not(feature = "editor-preview"))]
        {
            false
        }
    }
    fn modal_open(&self) -> bool {
        self.settings_open
            || self.project_dialog
            || self.document_dialog
            || self.import_preview.is_some()
            || self.error.is_some()
            || self.full_review.is_some()
            || self.rich_modal_open()
    }
}

impl eframe::App for StudioApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let t = theme::tokens(&ctx);
        if !self.modal_open() {
            self.shortcuts(&ctx);
        }
        if !self.autosave_paused
            && self
                .dirty
                .is_some_and(|instant| instant.elapsed() > Duration::from_millis(650))
        {
            let result = self.save();
            self.record_error(result);
        }
        if (self.dirty.is_some() && !self.autosave_paused) || self.import_job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            let result = self.save();
            if result.is_err() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
            self.record_error(result);
        }
        if let Some(receiver) = &self.import_job {
            let received = receiver.try_recv();
            if matches!(received, Err(mpsc::TryRecvError::Disconnected)) {
                self.import_job = None;
                self.error = Some(UiError {
                    msgid: "Archive checking was interrupted. Try opening it again.",
                    details: "Archive worker disconnected".into(),
                });
                self.status = "Attention needed".into();
            } else if let Ok(result) = received {
                match result {
                    Ok(preview) => {
                        self.import_preview = Some(preview);
                        self.status = "Archive checked".into();
                    }
                    Err(error) => {
                        self.record_error(Err(Box::new(error)));
                    }
                }
                self.import_job = None;
            }
        }
        self.toolbar(ui, t);
        self.status_bar(ui, t);
        let (show_navigator, show_inspector) = self.effective_panes(&ctx);
        if self.library || show_navigator {
            self.navigation(ui, t);
        }
        if self.library {
            self.library(ui, t);
        } else {
            if show_inspector {
                self.inspector(ui, t);
            }
            self.editor(ui, t);
        }
        let had_modal = self.modal_open();
        self.full_review_dialog(&ctx, t);
        self.dialogs(&ctx, t);
        if had_modal && !self.modal_open() {
            // The next manuscript may not have been emitted in this frame yet.
            // Focus only after its TextEdit has registered an accessibility node.
            self.focus_manuscript = true;
            ctx.request_repaint();
        }

        self.frames += 1;
        if self.capture.is_some() && self.frames >= 8 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let screenshots = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|event| {
                    if let egui::Event::Screenshot { image, .. } = event {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        });
        if !screenshots.is_empty()
            && let Some(path) = self.capture.take()
            && let Some(image) = screenshots.first()
        {
            let result = image::save_buffer(
                &path,
                image.as_raw(),
                image.width() as u32,
                image.height() as u32,
                image::ColorType::Rgba8,
            )
            .map_err(Into::into);
            let saved = result.is_ok();
            self.record_error(result);
            if saved {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if self.capture.is_some() {
            ctx.request_repaint();
        }
    }
}

fn error_message(error: &(dyn std::error::Error + Send + Sync + 'static)) -> &'static str {
    if let Some(error) = error.downcast_ref::<studio_core::StudioError>() {
        use studio_core::StudioError;
        return match error {
            StudioError::Validation(_) | StudioError::InvalidTextRange { .. } => {
                "Check the entered values and try again."
            }
            StudioError::RevisionConflict { .. } => {
                "The document changed. Reload it before saving."
            }
            StudioError::AnnotationConflict { .. } => "The note changed. Reload it before editing.",
            StudioError::DraftChanged => {
                "The working draft changed. Preserve your changes before retrying."
            }
            StudioError::DraftConflict { .. } => {
                "Recover the unsaved draft from another revision first."
            }
            StudioError::ReadOnly { .. } => {
                "This document is read only to preserve its original structure."
            }
            StudioError::NoMatch | StudioError::ExpectedTextMismatch => {
                "The exact excerpt was not found. Select existing text."
            }
            StudioError::AmbiguousMatch => {
                "The excerpt occurs more than once. Select a longer unique excerpt."
            }
            StudioError::InvalidProposalState { .. } => {
                "The proposal state changed. Refresh the review panel."
            }
            StudioError::StaleProposal { .. } => {
                "The proposal is stale. Prepare it again using the current revision."
            }
            StudioError::UndoConflict { .. } => {
                "Undo would overwrite later edits. The text was left unchanged."
            }
            StudioError::NotFound { .. } => "The item was not found. Refresh the project list.",
            StudioError::UnsupportedSchema { .. } => {
                "This workspace requires a newer version of WriterCraft."
            }
            StudioError::NotWorkspaceDatabase => "Choose a WriterCraft workspace file.",
            StudioError::CorruptData(_) => "Workspace data is inconsistent. Restore a backup.",
            StudioError::LockUnavailable => "The workspace is unavailable. Close and reopen it.",
            StudioError::Database(_) => "The local database operation could not be completed.",
            StudioError::Io(_) => {
                "The file could not be read or saved. Check the folder and access permissions."
            }
            StudioError::Json(_) => "The saved data could not be decoded.",
        };
    }
    if error.is::<studio_transfer::TransferError>() {
        return "The archive could not be checked. The original project was not changed.";
    }
    if error.is::<std::io::Error>() {
        return "The file could not be read or saved. Check the folder and access permissions.";
    }
    if error.is::<serde_json::Error>() {
        return "The saved data could not be decoded.";
    }
    "The action could not be completed. Details are available below."
}

fn resolve_panes(
    width: f32,
    nav_width: f32,
    inspector_width: f32,
    navigator: bool,
    inspector: bool,
    prefer_inspector: bool,
    focus: bool,
) -> (bool, bool) {
    if focus {
        return (false, false);
    }
    if navigator && inspector && width < nav_width + inspector_width + 480.0 {
        return if prefer_inspector {
            (false, true)
        } else {
            (true, false)
        };
    }
    (navigator, inspector)
}

fn proposal_excerpts(proposal: &Proposal, max_chars: usize) -> (String, String) {
    let before_count = proposal.before.chars().count();
    let after_count = proposal.after.chars().count();
    let suffix_count = before_count.saturating_sub(proposal.anchor.end);
    let replacement_end = after_count.saturating_sub(suffix_count);
    let start = proposal.anchor.start.saturating_sub(45);
    let before_end = (proposal.anchor.end + 45).min(before_count);
    let after_end = (replacement_end + 45).min(after_count);
    let excerpt = |text: &str, end: usize, total: usize| {
        let mut result = String::new();
        if start > 0 {
            result.push('…');
        }
        result.extend(text.chars().skip(start).take((end - start).min(max_chars)));
        if end - start > max_chars || end < total {
            result.push('…');
        }
        result
    };
    (
        excerpt(&proposal.before, before_end, before_count),
        excerpt(&proposal.after, after_end, after_count),
    )
}

fn modal(
    ctx: &egui::Context,
    language: Language,
    id: &str,
    title: &str,
    width: f32,
    body: impl FnOnce(&mut egui::Ui),
) -> egui::ModalResponse<()> {
    components::modal(ctx, language, id, title, width, body)
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Source Sans".into(),
        egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/SourceSans3-Regular.ttf"
        ))
        .into(),
    );
    fonts.font_data.insert(
        "Source Serif".into(),
        egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/SourceSerif4-Regular.ttf"
        ))
        .into(),
    );
    fonts.font_data.insert(
        "Noto CJK".into(),
        egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/NotoSansCJKjp-Regular.otf"
        ))
        .into(),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "Source Sans".into());
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .push("Noto CJK".into());
    let mut manuscript_faces = vec!["Source Serif".into()];
    manuscript_faces.extend(
        fonts
            .families
            .get(&FontFamily::Proportional)
            .cloned()
            .unwrap_or_default(),
    );
    fonts
        .families
        .insert(FontFamily::Name("Manuscript".into()), manuscript_faces);
    ctx.set_fonts(fonts);
}
fn primary(ui: &mut egui::Ui, label: &str, t: Tokens) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(t.on_accent).strong())
            .fill(t.accent)
            .min_size(Vec2::new(128.0, 34.0)),
    )
}
fn nav_row(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    selected: bool,
    t: Tokens,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 44.0), egui::Sense::click());
    if selected || response.hovered() {
        ui.painter()
            .rect_filled(rect, 5.0, if selected { t.accent_soft } else { t.surface });
    }
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(8.0, 3.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );
    icons::icon(
        &mut child,
        icon,
        20.0,
        if selected { t.accent } else { t.muted },
    );
    child.add(
        egui::Label::new(RichText::new(label).size(14.0).color(if selected {
            t.text
        } else {
            t.muted
        }))
        .truncate(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            label,
        )
    });
    response.on_hover_text(label)
}
fn diff_block(ui: &mut egui::Ui, label: &str, body: &str, background: Color32, t: Tokens) {
    ui.label(RichText::new(label).size(11.0).color(t.muted));
    ui.add_space(4.0);
    egui::Frame::new()
        .fill(background)
        .corner_radius(5)
        .inner_margin(10)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(body).size(13.0));
        });
}
fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> AppResult<()> {
    let parent = path.parent().ok_or("Cannot determine the file folder")?;
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}

const SAMPLE: &str = "That morning, the sea was almost white. Asya stopped at the gate, rested her hand on the weathered wood, and listened. The house was silent, as though it had been waiting for her all these years.\n\nShe had not meant to come back. Her life was settled: a small workshop, a desk beside the window, familiar faces on the way to work. Then, on Monday, a letter arrived. There was no return address, only a photograph of the lighthouse inside.\n\nNow the photograph was in her coat pocket, and the lighthouse itself stood behind the house, tall and dark against the clouds. As a child, she had believed its light would never go out.\n\n“So you came after all.”\n\nShe turned. A man stood by the neighbouring fence, holding a tin mug. It took her a moment to recognise him: the grey hair, the heavy sweater, the same habit of looking slightly to one side.\n\n“Hello, Misha.”\n\nHe nodded as if they had parted yesterday and pushed the gate open. Its hinges creaked. Neither the passing years nor the salt wind had changed that sound.\n\nThe kitchen smelled of wood and tea. Two cups stood on the table. Asya took off her coat, laid it over a chair, and only then noticed that the clock above the door was running.\n\nShe remembered stopping it before she left.\n\n“Who has been here?” she asked.\n\nMisha put his mug on the windowsill. Outside, the wind slowly turned a dry branch of the apple tree.\n\n“Tea first,” he said. “There is time.”";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interface_and_manuscript_fonts_cover_all_eight_languages() {
        let context = egui::Context::default();
        install_fonts(&context);
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            ui.ctx().fonts_mut(|fonts| {
                for (family, primary_face) in [
                    (FontFamily::Proportional, "Source Sans"),
                    (FontFamily::Name("Manuscript".into()), "Source Serif"),
                ] {
                    let mut font = fonts.fonts.font(&family);
                    // epaint 0.36.2 has_glyph compares face IDs with the replacement face,
                    // reporting false for valid characters supplied by that same face.
                    let characters = font.characters();
                    for character in
                        "Русский English Deutsch Français Español Português 简体中文 日本語".chars()
                    {
                        assert!(
                            characters.contains_key(&character),
                            "Missing {character:?} in {family:?}",
                        );
                    }
                    for character in "АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯабвгдеёжзийклмнопрстуфхцчшщъыьэюяABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyzÄÖÜäöüßŒœŸÀàÂâÆæÇçÈèÉéÊêËëÎîÏïÔôÙùÛûÁáÍíÑñÓóÚúÃãÕõ".chars() {
                        let owner = characters
                            .get(&character)
                            .and_then(|owners| owners.first())
                            .map(String::as_str);
                        assert_eq!(
                            owner,
                            Some(primary_face),
                            "Expected the designed primary face for {character:?} in {family:?}",
                        );
                    }
                }
            });
        });
        output.textures_delta.clear();
    }

    #[test]
    fn every_static_ui_status_error_and_theme_message_is_in_all_catalogs()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = include_str!("app.rs");
        let demo_start = source
            .find("    fn create_example")
            .ok_or("demo boundary")?;
        let demo_end = source
            .find("    fn open_project")
            .ok_or("demo end boundary")?;
        let sample_start = source.find("const SAMPLE:").ok_or("sample boundary")?;
        let source = format!(
            "{}{}{}{}{}{}{}{}",
            &source[..demo_start],
            &source[demo_end..sample_start],
            include_str!("app/commands.rs"),
            include_str!("app/navigator.rs"),
            include_str!("app/library.rs"),
            include_str!("app/manuscript.rs"),
            include_str!("app/review.rs"),
            include_str!("app/dialogs.rs")
        );
        let mut offset = 0;
        while let Some(start) = source[offset..].find('"') {
            let start = offset + start;
            let mut escaped = false;
            let mut end = None;
            for (relative, character) in source[start + 1..].char_indices() {
                if escaped {
                    escaped = false;
                    continue;
                }
                if character == '\\' {
                    escaped = true;
                } else if character == '"' {
                    end = Some(start + 1 + relative + 1);
                    break;
                }
            }
            let end = end.ok_or("unterminated Rust string in UI source")?;
            let literal = &source[start..end];
            if literal
                .chars()
                .any(|character| ('\u{0400}'..='\u{04ff}').contains(&character))
            {
                let key: String = serde_json::from_str(literal)?;
                for language in Language::ALL {
                    assert_ne!(
                        language.text(&key),
                        "⟦missing translation⟧",
                        "{}: {key}",
                        language.code()
                    );
                }
            }
            offset = end;
        }
        for key in Appearance::ALL
            .into_iter()
            .map(Appearance::label)
            .chain(Palette::ALL.into_iter().map(Palette::label))
        {
            for language in Language::ALL {
                assert_ne!(
                    language.text(key),
                    "⟦missing translation⟧",
                    "{}: {key}",
                    language.code()
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod ux_regressions {
    use super::*;

    fn fixture() -> AppResult<(tempfile::TempDir, StudioApp)> {
        let directory = tempfile::tempdir()?;
        let context = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(context);
        let mut app = StudioApp::new(&creation, directory.path().join("test.sqlite"), false, None)?;
        let project = app.workspace.create_project("A test project")?;
        app.workspace
            .create_document(&project.id, "A chapter", "🪶 echo, echo.")?;
        app.open_project(project)?;
        Ok((directory, app))
    }

    #[test]
    fn selected_second_occurrence_reaches_review_and_changes_only_that_occurrence() -> AppResult<()>
    {
        let (_directory, mut app) = fixture()?;
        let document = app.document.clone().ok_or("missing document")?;
        app.expected = "echo".into();
        app.replacement = "answer".into();
        app.frozen_selection = Some(FrozenSelection {
            document_id: document.id.clone(),
            base_revision: document.revision,
            body: document.body.clone(),
            anchor: TextAnchor {
                start: 8,
                end: 12,
                expected_text: "echo".into(),
            },
        });
        app.stage()?;
        let proposal = app.proposals.first().ok_or("missing proposal")?.clone();
        app.apply(&proposal.id)?;
        assert_eq!(app.body, "🪶 echo, answer.");
        Ok(())
    }

    #[test]
    fn changed_draft_cannot_retarget_a_frozen_selection() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let document = app.document.clone().ok_or("missing document")?;
        app.expected = "echo".into();
        app.replacement = "answer".into();
        app.frozen_selection = Some(FrozenSelection {
            document_id: document.id.clone(),
            base_revision: document.revision,
            body: document.body.clone(),
            anchor: TextAnchor {
                start: 8,
                end: 12,
                expected_text: "echo".into(),
            },
        });
        app.body.insert_str(0, "New ");
        app.dirty = Some(Instant::now());
        assert!(app.stage().is_err());
        assert!(app.workspace.proposals(&document.id)?.is_empty());
        assert_eq!(
            app.workspace.document(&document.id)?.body,
            "New 🪶 echo, echo."
        );
        Ok(())
    }

    #[test]
    fn opening_the_current_navigation_row_keeps_the_just_saved_edit() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let old_navigation_snapshot = app.documents.first().cloned().ok_or("missing document")?;
        app.body = "A freshly edited chapter".into();
        app.dirty = Some(Instant::now());
        app.open_document(old_navigation_snapshot)?;
        assert_eq!(app.body, "A freshly edited chapter");
        Ok(())
    }

    #[test]
    fn compact_workspace_keeps_one_inspector_and_a_useful_manuscript() {
        assert_eq!(
            resolve_panes(1000.0, 340.0, 460.0, true, true, false, false),
            (true, false)
        );
        assert_eq!(
            resolve_panes(1000.0, 340.0, 460.0, true, true, true, false),
            (false, true)
        );
        assert_eq!(
            resolve_panes(1440.0, 248.0, 320.0, true, true, false, false),
            (true, true)
        );
    }

    #[test]
    fn leaving_focus_restores_the_requested_panes() {
        assert_eq!(
            resolve_panes(1440.0, 248.0, 320.0, true, true, false, true),
            (false, false)
        );
        assert_eq!(
            resolve_panes(1440.0, 248.0, 320.0, true, true, false, false),
            (true, true)
        );
    }

    #[test]
    fn long_chapter_review_uses_bounded_context_and_keeps_the_changed_phrase() -> AppResult<()> {
        let workspace = Workspace::memory()?;
        let project = workspace.create_project("Test")?;
        let body = format!("{}TARGET{}", "Before ".repeat(200), " after".repeat(200));
        let document = workspace.create_document(&project.id, "Chapter", &body)?;
        let proposal = workspace.stage_replace(
            &document.id,
            document.revision,
            "TARGET",
            "REPLACED",
            "An exact edit",
        )?;
        let (before, after) = proposal_excerpts(&proposal, 340);
        assert!(before.chars().count() <= 342 && after.chars().count() <= 342);
        assert!(after.contains("REPLACED"));
        Ok(())
    }

    #[test]
    fn persistent_autosave_conflict_pauses_retries_after_error_dismissal() -> AppResult<()> {
        let (directory, mut app) = fixture()?;
        let original = app.document.clone().ok_or("missing document")?;
        app.body = "My unfinished local paragraph".into();
        app.workspace
            .save_draft(&original.id, original.revision, &app.body)?;
        app.dirty = Some(
            Instant::now()
                .checked_sub(Duration::from_secs(1))
                .ok_or("test clock is too young")?,
        );
        let external = Workspace::open(&app.database)?;
        let newer =
            external.save_document(&original.id, original.revision, "Newer canonical writing")?;
        let context = egui::Context::default();
        install_fonts(&context);
        theme::apply(&context, &app.settings);
        let mut frame = eframe::Frame::_new_kittest();
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            eframe::App::ui(&mut app, ui, &mut frame)
        });
        output.textures_delta.clear();
        assert!(app.error.is_some());
        assert!(app.autosave_paused);
        assert!(app.dirty.is_some());
        assert_eq!(app.status, "Attention needed");

        let mut dismiss = egui::RawInput::default();
        dismiss.events.push(egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: Some(egui::Key::Escape),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        let mut output = context.run_ui(dismiss, |ui| eframe::App::ui(&mut app, ui, &mut frame));
        output.textures_delta.clear();
        assert!(app.error.is_none());
        for _ in 0..6 {
            let mut output = context.run_ui(egui::RawInput::default(), |ui| {
                eframe::App::ui(&mut app, ui, &mut frame)
            });
            output.textures_delta.clear();
            assert!(
                app.error.is_none(),
                "autosave reopened a dismissed persistent error"
            );
        }
        assert_eq!(app.body, "My unfinished local paragraph");
        assert_eq!(app.workspace.document(&original.id)?, newer);
        assert_eq!(
            app.workspace
                .draft(&original.id)?
                .ok_or("recovery draft was lost")?
                .body,
            app.body
        );
        let export = directory.path().join("recovered.txt");
        atomic_write(&export, app.body.as_bytes())?;
        assert_eq!(std::fs::read_to_string(export)?, app.body);

        let result = app.save();
        assert!(
            result.is_err(),
            "explicit save must attempt the unresolved conflict"
        );
        app.record_error(result);
        assert!(app.error.is_some());
        assert!(app.autosave_paused);
        assert!(app.dirty.is_some());
        assert_eq!(app.body, "My unfinished local paragraph");
        assert_eq!(app.workspace.document(&original.id)?, newer);
        Ok(())
    }

    #[test]
    fn explicit_save_can_retry_a_paused_autosave_and_clear_pause_after_success() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let original = app.document.clone().ok_or("missing document")?;
        app.body = "A recoverable local edit".into();
        app.workspace
            .save_draft(&original.id, original.revision, &app.body)?;
        app.dirty = Some(Instant::now());
        app.autosave_paused = true;
        app.save()?;
        assert!(!app.autosave_paused);
        assert!(app.dirty.is_none());
        assert_eq!(app.workspace.document(&original.id)?.body, app.body);
        assert!(app.workspace.draft(&original.id)?.is_none());
        assert_eq!(app.status, "All changes saved");
        Ok(())
    }

    #[test]
    fn project_creation_enter_keeps_accessibility_focus_on_an_emitted_node() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let context = egui::Context::default();
        context.enable_accesskit();
        install_fonts(&context);
        theme::apply(&context, &app.settings);
        let mut frame = eframe::Frame::_new_kittest();
        app.project_dialog = true;
        for events in [
            vec![],
            vec![egui::Event::Text("Keyboard project".into())],
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            vec![],
        ] {
            let mut output = context.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| eframe::App::ui(&mut app, ui, &mut frame),
            );
            let update = output
                .platform_output
                .accesskit_update
                .take()
                .ok_or("missing accessibility tree")?;
            assert!(
                update.nodes.iter().any(|(id, _)| *id == update.focus),
                "focus references a node absent from this frame: {:?}",
                update.focus
            );
            output.textures_delta.clear();
        }
        assert!(!app.project_dialog);
        assert_eq!(
            app.project.as_ref().map(|p| p.title.as_str()),
            Some("Keyboard project")
        );
        Ok(())
    }

    #[test]
    fn shortcuts_in_one_frame_never_stack_two_modals() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        let context = egui::Context::default();
        install_fonts(&context);
        let mut frame = eframe::Frame::_new_kittest();
        let events = [egui::Key::N, egui::Key::Comma]
            .map(|key| egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            })
            .to_vec();
        let mut output = context.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| eframe::App::ui(&mut app, ui, &mut frame),
        );
        output.textures_delta.clear();
        assert!(app.project_dialog ^ app.settings_open);
        Ok(())
    }

    #[test]
    fn preferences_modal_blocks_global_project_creation_shortcut() -> AppResult<()> {
        let (_directory, mut app) = fixture()?;
        app.settings_open = true;
        let context = egui::Context::default();
        install_fonts(&context);
        theme::apply(&context, &app.settings);
        let mut frame = eframe::Frame::_new_kittest();
        let mut input = egui::RawInput::default();
        input.events.push(egui::Event::Key {
            key: egui::Key::N,
            physical_key: Some(egui::Key::N),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        });
        let mut output = context.run_ui(input, |ui| eframe::App::ui(&mut app, ui, &mut frame));
        output.textures_delta.clear();
        assert!(!app.project_dialog);
        Ok(())
    }
}

#[cfg(test)]
mod catalog_source_checks {
    use super::*;
    #[test]
    fn every_literal_component_catalog_lookup_resolves_in_all_languages()
    -> Result<(), Box<dyn std::error::Error>> {
        let sources = [
            include_str!("app/commands.rs"),
            include_str!("app/navigator.rs"),
            include_str!("app/library.rs"),
            include_str!("app/manuscript.rs"),
            include_str!("app/review.rs"),
            include_str!("app/dialogs.rs"),
            include_str!("app/settings.rs"),
        ];
        for source in sources {
            for marker in [".text(", ".format("] {
                let mut remaining = source;
                while let Some(found) = remaining.find(marker) {
                    remaining = &remaining[found + marker.len()..];
                    let expression = remaining.trim_start();
                    if !expression.starts_with('"') {
                        continue;
                    }
                    let mut escaped = false;
                    let mut end = None;
                    for (index, character) in expression.char_indices().skip(1) {
                        if escaped {
                            escaped = false;
                        } else if character == '\\' {
                            escaped = true;
                        } else if character == '"' {
                            end = Some(index);
                            break;
                        }
                    }
                    let end = end.ok_or("unclosed catalog key")?;
                    let key: String = serde_json::from_str(&expression[..=end])?;
                    for language in Language::ALL {
                        assert_ne!(
                            language.text(&key),
                            "⟦missing translation⟧",
                            "{}: {key}",
                            language.code()
                        );
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod native_input_regressions {
    use super::*;

    fn fixture() -> AppResult<(tempfile::TempDir, StudioApp, egui::Context, eframe::Frame)> {
        let directory = tempfile::tempdir()?;
        let context = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(context.clone());
        let mut app = StudioApp::new(
            &creation,
            directory.path().join("input.sqlite"),
            false,
            None,
        )?;
        let project = app.workspace.create_project("Input test")?;
        app.workspace
            .create_document(&project.id, "Repeated words", "🪶 echo, echo.")?;
        app.open_project(project)?;
        Ok((directory, app, context, eframe::Frame::_new_kittest()))
    }

    fn render(
        app: &mut StudioApp,
        context: &egui::Context,
        frame: &mut eframe::Frame,
        events: Vec<egui::Event>,
        _modifiers: egui::Modifiers,
    ) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(1440.0, 940.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = context.run_ui(input, |ui| eframe::App::ui(app, ui, frame));
        output.textures_delta.clear();
    }

    fn select_second(app: &StudioApp, context: &egui::Context) -> AppResult<egui::Id> {
        let id = egui::Id::new((
            "manuscript-body",
            &app.document.as_ref().ok_or("missing document")?.id,
        ));
        let mut state = egui::TextEdit::load_state(context, id).unwrap_or_default();
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(8usize),
                egui::text::CCursor::new(12usize),
            )));
        state.store(context, id);
        context.memory_mut(|memory| memory.request_focus(id));
        Ok(id)
    }

    #[test]
    fn secondary_pointer_press_keeps_selection_and_opens_the_context_menu() -> AppResult<()> {
        let (_directory, mut app, context, mut frame) = fixture()?;
        render(
            &mut app,
            &context,
            &mut frame,
            vec![],
            egui::Modifiers::NONE,
        );
        let id = select_second(&app, &context)?;
        let position = context
            .read_response(id)
            .ok_or("missing editor response")?
            .rect
            .center();
        render(
            &mut app,
            &context,
            &mut frame,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Secondary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            egui::Modifiers::NONE,
        );
        render(
            &mut app,
            &context,
            &mut frame,
            vec![egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Secondary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            egui::Modifiers::NONE,
        );
        let range = egui::TextEdit::load_state(&context, id)
            .and_then(|state| state.cursor.char_range())
            .ok_or("missing selection")?
            .as_sorted_char_range();
        assert_eq!((usize::from(range.start), usize::from(range.end)), (8, 12));
        assert!(egui::Popup::is_id_open(
            &context,
            egui::Id::new((
                "selection-menu",
                &app.document.as_ref().ok_or("missing document")?.id
            ))
        ));
        assert!(app.frozen_selection.is_none());
        Ok(())
    }

    #[test]
    fn shift_f10_opens_selection_actions_without_changing_the_manuscript() -> AppResult<()> {
        let (_directory, mut app, context, mut frame) = fixture()?;
        render(
            &mut app,
            &context,
            &mut frame,
            vec![],
            egui::Modifiers::NONE,
        );
        select_second(&app, &context)?;
        render(
            &mut app,
            &context,
            &mut frame,
            vec![egui::Event::Key {
                key: egui::Key::F10,
                physical_key: Some(egui::Key::F10),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::SHIFT,
            }],
            egui::Modifiers::SHIFT,
        );
        let document = app.document.as_ref().ok_or("missing document")?;
        assert!(egui::Popup::is_id_open(
            &context,
            egui::Id::new(("selection-menu", &document.id))
        ));
        assert_eq!(app.workspace.document(&document.id)?.body, "🪶 echo, echo.");
        Ok(())
    }
}
