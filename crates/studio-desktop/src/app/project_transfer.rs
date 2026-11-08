//! Explicit whole-project transfer over bounded canonical workers, separate from workspace backup.
use super::navigation_return::NavigationReturn;
use super::*;
use std::path::Path;
use studio_core::{
    PortableProjectScope, PortableProjectSummary, ProjectImportWorker, ProjectPortableExportWorker,
    ProjectTransferCancellation, VerifiedPortableProjectStream,
};
use studio_transfer::{
    CanonicalProjectArchive, CanonicalProjectFolder, export_canonical_project_archive_file,
    export_canonical_project_folder,
};

#[derive(Default)]
pub(super) struct ProjectTransferState {
    visible: bool,
    view: Option<View>,
    origin: Option<NavigationReturn>,
    auxiliary_error: Option<String>,
    pending_open: Option<String>,
    notice: Option<&'static str>,
}
#[derive(Clone, Copy, PartialEq)]
enum ExportKind {
    Archive,
    Folder,
}
#[derive(Clone)]
struct ExportRequest {
    project: Project,
    kind: ExportKind,
    folder_name: String,
}
struct CheckedInput {
    path: PathBuf,
    summary: PortableProjectSummary,
    stream: Box<VerifiedPortableProjectStream>,
}
enum Intent {
    Export(ExportRequest),
    Restore(CheckedInput),
}
struct Prerequisites {
    intent: Intent,
    project: Option<String>,
    notes: Vec<(String, String)>,
}
#[derive(Clone)]
struct Context {
    title: String,
    path: Option<PathBuf>,
}
enum View {
    Export(ExportRequest),
    ChooseRestore,
    Ready(CheckedInput),
    Preparing(Prerequisites),
    Working(Job),
    Outcome(Outcome),
}
struct Job {
    receiver: mpsc::Receiver<Event>,
    cancel: ProjectTransferCancellation,
    cancelling: bool,
    context: Context,
    kind: JobKind,
}
#[derive(Clone, Copy)]
enum JobKind {
    Check,
    Export,
    Restore,
}
enum Event {
    Checked(CheckedInput),
    Exported(Context),
    Restored {
        context: Context,
        project_id: String,
    },
    Failed {
        details: String,
        retry: Retry,
    },
}
enum Retry {
    Export(ExportRequest),
    Restore(CheckedInput),
    ChooseRestore,
}
struct Outcome {
    context: Context,
    kind: OutcomeKind,
    details: Option<String>,
    retry: Option<Retry>,
}
enum OutcomeKind {
    Exported,
    Restored(String),
    Cancelled,
    Unconfirmed,
}
enum Action {
    Continue,
    ChooseArchive,
    ChooseFolder,
    Cancel,
    Review(String),
    Retry,
    OpenRestored(String),
    Reveal,
    Done,
}
impl ProjectTransferState {
    pub(super) fn modal_open(&self) -> bool {
        self.visible && self.view.is_some()
    }
    pub(super) fn busy(&self) -> bool {
        self.pending_open.is_some()
            || matches!(self.view, Some(View::Working(_)))
            || (self.visible
                && matches!(&self.view,Some(View::Preparing(state)) if state.notes.is_empty()))
    }
    pub(super) fn has_state(&self) -> bool {
        self.view.is_some()
    }
    fn active(&self) -> bool {
        self.pending_open.is_some()
            || matches!(
                self.view,
                Some(View::Working(_)) | Some(View::Preparing(_)) | Some(View::Ready(_))
            )
    }
    fn show(&mut self) {
        self.visible = self.view.is_some();
    }
    fn context(&self) -> Context {
        match &self.view {
            Some(View::Export(request)) => Context {
                title: request.project.title.clone(),
                path: None,
            },
            Some(View::Ready(input)) => Context {
                title: input.summary.title.clone(),
                path: Some(input.path.clone()),
            },
            Some(View::Preparing(state)) => intent_context(&state.intent),
            Some(View::Working(job)) => job.context.clone(),
            Some(View::Outcome(outcome)) => outcome.context.clone(),
            _ => Context {
                title: String::new(),
                path: None,
            },
        }
    }
    fn poll(&mut self, ctx: &egui::Context) {
        let Some(View::Working(job)) = self.view.as_ref() else {
            return;
        };
        let event = match job.receiver.try_recv() {
            Ok(event) => event,
            Err(mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(Duration::from_millis(40));
                return;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                let context = job.context.clone();
                self.view = Some(View::Outcome(Outcome {
                    context,
                    kind: OutcomeKind::Unconfirmed,
                    details: Some("Transfer worker stopped before acknowledgement".into()),
                    retry: None,
                }));
                return;
            }
        };
        let context = job.context.clone();
        let cancel_requested = job.cancelling;
        let checking = matches!(job.kind, JobKind::Check);
        self.view = Some(match event {
            Event::Checked(_input) if cancel_requested => View::Outcome(Outcome {
                context,
                kind: OutcomeKind::Cancelled,
                details: None,
                retry: Some(Retry::ChooseRestore),
            }),
            Event::Checked(input) => View::Ready(input),
            // A confirmed publication takes precedence over a later cancellation request.
            Event::Exported(context) => View::Outcome(Outcome {
                context,
                kind: OutcomeKind::Exported,
                details: None,
                retry: None,
            }),
            Event::Restored {
                context,
                project_id,
            } => View::Outcome(Outcome {
                context,
                kind: OutcomeKind::Restored(project_id),
                details: None,
                retry: None,
            }),
            Event::Failed { details: _, retry } if checking && cancel_requested => {
                View::Outcome(Outcome {
                    context,
                    kind: OutcomeKind::Cancelled,
                    details: None,
                    retry: Some(retry),
                })
            }
            Event::Failed { details, retry } => View::Outcome(Outcome {
                context,
                kind: OutcomeKind::Unconfirmed,
                details: Some(details),
                retry: Some(retry),
            }),
        });
    }
    fn dialog(&mut self, ctx: &egui::Context, l: Language, t: Tokens) -> Option<Action> {
        if !self.modal_open() {
            return None;
        }
        let mut action = None;
        let context = self.context();
        let busy = self.busy();
        let response = components::modal(
            ctx,
            l,
            "project-transfer",
            l.text("Project transfer…"),
            620.0,
            |ui| {
                egui::ScrollArea::vertical().max_height((ctx.content_rect().height()-260.0).max(80.0)).show(ui,|ui|{
                if !context.title.is_empty(){ui.label(RichText::new(l.format("Whole project: {project}",&[("project",context.title.clone())])).size(18.0).strong());ui.add_space(16.0);}
                match self.view.as_mut() {
                    Some(View::Export(request))=>{
                        ui.label(l.text("Choose export format"));ui.add_space(8.0);
                        for (kind,label) in [(ExportKind::Archive,"Native project archive"),(ExportKind::Folder,"Readable project folder")]{
                            if ui.add_sized([ui.available_width(),components::FIELD_HEIGHT],egui::Button::new(l.text(label)).selected(request.kind==kind).wrap()).clicked(){request.kind=kind;}
                        }
                        ui.add_space(12.0);
                        ui.label(l.text("Both formats preserve saved history, notes, original artifacts and untrusted recovery data. The folder also includes readable Markdown and JSON Canvas."));
                        ui.add_space(12.0);
                        if request.kind==ExportKind::Folder {
                            ui.label(l.text("Folder name"));components::singleline(ui,&mut request.folder_name,l.text("Folder name"));
                            if validate_folder_name(&request.folder_name).is_err(){ui.colored_label(t.warning,l.text("Enter one folder name without separators or reserved filenames."));}
                            ui.label(RichText::new(l.text("Choose a parent folder. A new folder with this name will be created.")).color(t.muted));
                        }
                        ui.add_space(12.0);
                        ui.label(RichText::new(l.text("This transfers one complete project. Use workspace backup to copy every local project.")).color(t.muted));
                        ui.label(RichText::new(l.text("Choose a new path. Existing files and folders are never replaced.")).color(t.muted));
                    }
                    Some(View::ChooseRestore)=>{
                        ui.label(l.text("Restore an independent project from a native archive or a readable folder with its canonical companion."));
                        ui.add_space(16.0);
                        if components::action(ui,l.text("Choose archive…"),true,None).clicked(){action=Some(Action::ChooseArchive);}
                        if components::action(ui,l.text("Choose folder…"),true,None).clicked(){action=Some(Action::ChooseFolder);}
                    }
                    Some(View::Ready(input))=>{
                        if input.summary.scope!=PortableProjectScope::CompleteProject {ui.colored_label(t.warning,l.text("Only complete native project restoration is available in this build."));}
                        egui::Grid::new("native-project-summary").spacing([24.0,12.0]).show(ui,|ui|{
                            for (label,count) in [("Project items",input.summary.node_count),("Boards",input.summary.canvas_count),("Artifacts",input.summary.attachment_count)] {ui.label(l.text(label));ui.label(count.to_string());ui.end_row();}
                        });
                        ui.add_space(16.0);
                        ui.label(l.text("Preserved data is checked. Supported content is validated before a new project is created; preserved data may have limited native editing."));
                        ui.add_space(12.0);
                        ui.label(RichText::new(l.text("Reading files that have changed cannot be restored as an unchanged canonical backup.")).color(t.muted));
                    }
                    Some(View::Preparing(state))=>{
                        if state.notes.is_empty(){ui.spinner();ui.label(l.text("Saving current work before transfer…"));}
                        else {
                            ui.label(l.text("Review pending notes in this project before continuing."));ui.add_space(12.0);
                            for (id,title) in &state.notes {ui.horizontal_wrapped(|ui|{ui.label(title);if components::action(ui,l.text("Review notes"),true,None).clicked(){action=Some(Action::Review(id.clone()));}});}
                        }
                    }
                    Some(View::Working(job))=>{
                        ui.spinner();ui.label(l.text(if job.cancelling{"Cancelling transfer…"}else{match job.kind{JobKind::Check=>"Checking preserved project data…",JobKind::Export=>"Exporting project…",JobKind::Restore=>"Restoring verified project…"}}));
                    }
                    Some(View::Outcome(outcome))=>{
                        ui.label(RichText::new(l.text(match outcome.kind{OutcomeKind::Exported=>"Project exported and verified.",OutcomeKind::Restored(_)=>"Independent project restored.",OutcomeKind::Cancelled=>"No project was exported or restored.",OutcomeKind::Unconfirmed=>"The transfer could not be confirmed."})).strong());
                        if matches!(outcome.kind,OutcomeKind::Unconfirmed){ui.label(l.text("Check the selected destination or project library before retrying."));}
                        if let Some(details)=&outcome.details {ui.collapsing(l.text("Technical details"),|ui|{ui.add(egui::Label::new(details).wrap());});}
                    }
                    None=>{}
                }
                if let Some(path)=&context.path {ui.add_space(12.0);ui.add(egui::Label::new(path.to_string_lossy()).wrap());}
                if let Some(notice)=self.notice {ui.label(RichText::new(l.text(notice)).color(t.muted));}
                if self.pending_open.is_some(){ui.spinner();ui.label(l.text("Saving current work before transfer…"));}
                if let Some(error)=&self.auxiliary_error {ui.add_space(12.0);ui.colored_label(t.warning,l.text("Attention needed"));ui.collapsing(l.text("Technical details"),|ui|{ui.add(egui::Label::new(error).wrap());});}
            });
                components::footer(ui, |ui| {
                    match &self.view {
                        Some(View::Export(_)) | Some(View::Ready(_)) => {
                            if components::action(
                                ui,
                                l.text(if matches!(self.view, Some(View::Ready(_))) {
                                    "Restore project"
                                } else {
                                    "Choose destination…"
                                }),
                                !matches!(&self.view,Some(View::Export(request)) if request.kind==ExportKind::Folder && validate_folder_name(&request.folder_name).is_err()),
                                Some(t),
                            )
                            .clicked()
                            {
                                action = Some(Action::Continue);
                            }
                        }
                        Some(View::Preparing(state)) if !state.notes.is_empty() => {
                            if components::action(ui, l.text("Continue"), true, Some(t)).clicked() {
                                action = Some(Action::Continue);
                            }
                        }
                        Some(View::Outcome(outcome)) => {
                            if let OutcomeKind::Restored(id) = &outcome.kind {
                                if components::action(
                                    ui,
                                    l.text("Open restored project"),
                                    true,
                                    Some(t),
                                )
                                .clicked()
                                {
                                    action = Some(Action::OpenRestored(id.clone()));
                                }
                            } else if matches!(outcome.kind, OutcomeKind::Exported) {
                                if components::action(
                                    ui,
                                    l.text("Open output folder"),
                                    true,
                                    Some(t),
                                )
                                .clicked()
                                {
                                    action = Some(Action::Reveal);
                                }
                            }
                            if outcome.retry.is_some()
                                && components::action(ui, l.text("Retry"), true, None).clicked()
                            {
                                action = Some(Action::Retry);
                            }
                        }
                        _ => {}
                    }
                    let cancelling =
                        matches!(&self.view,Some(View::Working(job)) if job.cancelling);
                    if components::action(
                        ui,
                        l.text(if matches!(self.view, Some(View::Outcome(_))) {
                            "Done"
                        } else {
                            "Cancel"
                        }),
                        !cancelling,
                        None,
                    )
                    .clicked()
                    {
                        action = Some(if matches!(self.view, Some(View::Outcome(_))) {
                            Action::Done
                        } else {
                            Action::Cancel
                        });
                    }
                });
            },
        );
        if response.should_close() {
            action = Some(if busy { Action::Cancel } else { Action::Done });
        }
        action
    }
}
impl StudioApp {
    pub(super) fn open_project_export(&mut self) {
        if self.project_transfer.active() {
            self.project_transfer.show();
            return;
        }
        let Some(project) = self.project.clone() else {
            return;
        };
        self.project_transfer.view = Some(View::Export(ExportRequest {
            folder_name: portable_folder_name(&project.title),
            project,
            kind: ExportKind::Archive,
        }));
        self.project_transfer.visible = true;
        self.project_transfer.auxiliary_error = None;
        self.project_transfer.notice = None;
        self.settings_open = false;
    }
    pub(super) fn open_project_restore(&mut self) {
        if self.project_transfer.active() {
            self.project_transfer.show();
            return;
        }
        self.project_transfer.view = Some(View::ChooseRestore);
        self.project_transfer.visible = true;
        self.project_transfer.auxiliary_error = None;
        self.project_transfer.notice = None;
        self.settings_open = false;
    }
    pub(super) fn show_project_transfer(&mut self) {
        self.settings_open = false;
        self.project_transfer.show();
    }
    pub(super) fn project_transfer_settings(&mut self, ui: &mut egui::Ui, t: Tokens) {
        let l = self.language;
        ui.add_space(24.0);
        ui.separator();
        ui.add_space(12.0);
        ui.label(RichText::new(l.text("Project transfer…")).strong());
        ui.label(RichText::new(l.text("This transfers one complete project. Use workspace backup to copy every local project.")).color(t.muted));
        ui.horizontal_wrapped(|ui| {
            if components::action(ui, l.text("Export project…"), self.project.is_some(), None)
                .clicked()
            {
                self.open_project_export();
            }
            if components::action(ui, l.text("Restore project…"), true, None).clicked() {
                self.open_project_restore();
            }
            if self.project_transfer.has_state()
                && components::action(ui, l.text("Project transfer…"), true, None).clicked()
            {
                self.show_project_transfer();
            }
        });
    }
    pub(super) fn project_transfer_dialog(&mut self, ctx: &egui::Context, t: Tokens) {
        let action = self.project_transfer.dialog(ctx, self.language, t);
        if let Some(action) = action {
            if let Err(error) = self.transfer_action(action, ctx) {
                self.project_transfer.auxiliary_error = Some(error.to_string());
            }
        }
    }
    pub(super) fn poll_project_transfer(&mut self, ctx: &egui::Context) {
        self.project_transfer.poll(ctx);
        if let Some(id) = self.project_transfer.pending_open.take() {
            let result = (|| -> AppResult<bool> {
                self.save_explicit()?;
                #[cfg(feature = "editor-preview")]
                if !self.flush_canvas_for_transfer(None)? {
                    return Ok(false);
                }
                let project = self
                    .workspace
                    .list_projects()?
                    .into_iter()
                    .find(|project| project.id == id)
                    .ok_or("restored_project_missing")?;
                self.open_project(project)?;
                self.projects = self.workspace.list_projects()?;
                self.project_transfer.visible = false;
                Ok(true)
            })();
            match result {
                Ok(false) => {
                    self.project_transfer.pending_open = Some(id);
                    ctx.request_repaint_after(Duration::from_millis(40));
                }
                Err(error) => self.project_transfer.auxiliary_error = Some(error.to_string()),
                Ok(true) => {}
            }
        }
        if !self.project_transfer.visible
            || !matches!(self.project_transfer.view, Some(View::Preparing(_)))
        {
            return;
        }
        if let Err(error) = self.continue_project_transfer(ctx) {
            self.project_transfer.auxiliary_error = Some(error.to_string());
        }
    }
    fn transfer_action(&mut self, action: Action, ctx: &egui::Context) -> AppResult<()> {
        match action {
            Action::Continue => {
                if let Some(View::Export(request)) = &self.project_transfer.view
                    && request.kind == ExportKind::Folder
                {
                    validate_folder_name(&request.folder_name)?;
                }
                match self.project_transfer.view.take() {
                    Some(View::Export(request)) => {
                        let project = Some(request.project.id.clone());
                        self.project_transfer.view = Some(View::Preparing(Prerequisites {
                            intent: Intent::Export(request),
                            project,
                            notes: Vec::new(),
                        }));
                    }
                    Some(View::Ready(input)) => {
                        let project = self.project.as_ref().map(|project| project.id.clone());
                        self.project_transfer.view = Some(View::Preparing(Prerequisites {
                            intent: Intent::Restore(input),
                            project,
                            notes: Vec::new(),
                        }));
                    }
                    other => self.project_transfer.view = other,
                }
                self.continue_project_transfer(ctx)?;
            }
            Action::ChooseArchive | Action::ChooseFolder => {
                let path = if matches!(action, Action::ChooseArchive) {
                    rfd::FileDialog::new()
                        .set_title(self.language.text("Choose archive…"))
                        .add_filter("WriterCraft", &["writercraft"])
                        .pick_file()
                } else {
                    rfd::FileDialog::new()
                        .set_title(self.language.text("Choose folder…"))
                        .pick_folder()
                };
                if let Some(path) = path {
                    self.start_transfer_check(path, matches!(action, Action::ChooseFolder), ctx);
                } else {
                    self.transfer_cancelled(None);
                }
            }
            Action::Cancel => {
                if let Some(View::Working(job)) = self.project_transfer.view.as_mut() {
                    job.cancelling = true;
                    job.cancel.cancel();
                } else {
                    self.return_from_transfer_review()?;
                    self.transfer_cancelled(None);
                }
            }
            Action::Done => {
                self.project_transfer.pending_open = None;
                self.return_from_transfer_review()?;
                self.project_transfer.visible = false;
            }
            Action::Review(id) => {
                self.review_transfer_notes(&id)?;
            }
            Action::Retry => {
                if let Some(View::Outcome(mut outcome)) = self.project_transfer.view.take() {
                    self.project_transfer.auxiliary_error = None;
                    self.project_transfer.view = match outcome.retry.take() {
                        Some(Retry::Export(request)) => Some(View::Export(request)),
                        Some(Retry::Restore(input)) => Some(View::Ready(input)),
                        Some(Retry::ChooseRestore) => Some(View::ChooseRestore),
                        None => Some(View::Outcome(outcome)),
                    };
                }
            }
            Action::OpenRestored(id) => {
                self.project_transfer.pending_open = Some(id);
                ctx.request_repaint();
            }
            Action::Reveal => {
                if let Some(path) = self.project_transfer.context().path {
                    reveal_output(&path)?;
                }
            }
        }
        Ok(())
    }
    fn continue_project_transfer(&mut self, ctx: &egui::Context) -> AppResult<()> {
        let view = self.project_transfer.view.take();
        let Some(View::Preparing(mut state)) = view else {
            self.project_transfer.view = view;
            return Ok(());
        };
        let preparation = (|| -> AppResult<bool> {
            self.save_explicit()?;
            #[cfg(feature = "editor-preview")]
            if !self.flush_canvas_for_transfer(None)? {
                return Ok(false);
            }
            state.notes = self.pending_transfer_notes(state.project.as_deref())?;
            if !state.notes.is_empty() {
                return Ok(false);
            }
            self.return_from_transfer_review()?;
            #[cfg(feature = "editor-preview")]
            if !self.flush_canvas_for_transfer(state.project.as_deref())? {
                return Ok(false);
            }
            Ok(true)
        })();
        match preparation {
            Ok(false) => {
                self.project_transfer.view = Some(View::Preparing(state));
                ctx.request_repaint_after(Duration::from_millis(40));
            }
            Err(error) => {
                self.project_transfer.view = Some(View::Preparing(state));
                return Err(error);
            }
            Ok(true) => {
                self.project_transfer.auxiliary_error = None;
                match state.intent {
                    Intent::Export(request) => {
                        let exists = self
                            .workspace
                            .list_projects()?
                            .iter()
                            .any(|project| project.id == request.project.id);
                        if !exists {
                            self.project_transfer.view = Some(View::Export(request));
                            return Err("selected_project_no_longer_exists".into());
                        }
                        let destination = if request.kind == ExportKind::Archive {
                            rfd::FileDialog::new()
                                .set_title(self.language.text("Choose destination…"))
                                .set_file_name(format!(
                                    "{}.writercraft",
                                    portable_folder_name(&request.project.title)
                                ))
                                .add_filter("WriterCraft", &["writercraft"])
                                .save_file()
                        } else {
                            rfd::FileDialog::new()
                                .set_title(self.language.text("Choose a parent folder…"))
                                .pick_folder()
                                .map(|parent| parent.join(&request.folder_name))
                        };
                        if let Some(destination) = destination {
                            self.start_project_export(request, destination, ctx);
                        } else {
                            self.project_transfer.view = Some(View::Export(request.clone()));
                            self.transfer_cancelled(Some(Retry::Export(request)));
                        }
                    }
                    Intent::Restore(input) => self.start_project_restore(input, ctx),
                }
            }
        }
        Ok(())
    }
    fn pending_transfer_notes(&self, project: Option<&str>) -> AppResult<Vec<(String, String)>> {
        #[cfg(feature = "editor-preview")]
        if let Some(project) = project {
            return scoped_pending_notes(&self.workspace, project, &self.notes.pending_documents());
        }
        #[cfg(not(feature = "editor-preview"))]
        let _ = project;
        Ok(Vec::new())
    }
    fn review_transfer_notes(&mut self, id: &str) -> AppResult<()> {
        let Some(View::Preparing(state)) = self.project_transfer.view.as_ref() else {
            return Ok(());
        };
        if !state.notes.iter().any(|(document, _)| document == id) {
            return Err("pending_note_is_outside_transfer_scope".into());
        }
        let document = self.workspace.document(id)?;
        if state.project.as_deref() != Some(document.project_id.as_str()) {
            return Err("pending_note_is_outside_transfer_scope".into());
        }
        if self.project_transfer.origin.is_none() {
            self.project_transfer.origin = Some(NavigationReturn::capture(self)?);
        }
        let project = self
            .workspace
            .list_projects()?
            .into_iter()
            .find(|project| project.id == document.project_id)
            .ok_or("pending_note_project_missing")?;
        if self.project.as_ref().map(|current| current.id.as_str()) != Some(project.id.as_str()) {
            self.open_project(project)?;
        }
        self.open_document(document)?;
        #[cfg(feature = "editor-preview")]
        {
            self.notes.select_pending_draft(id);
            self.inspector = Inspector::Notes;
            self.inspector_visible = true;
            self.prefer_inspector = true;
            self.focus_mode = false;
        }
        self.project_transfer.visible = false;
        Ok(())
    }
    fn return_from_transfer_review(&mut self) -> AppResult<()> {
        let Some(origin) = self.project_transfer.origin.take() else {
            return Ok(());
        };
        match origin.restore(self) {
            Ok(true) => Ok(()),
            Ok(false) => {
                self.status = "Original writing changed; its latest saved version is open.".into();
                self.project_transfer.notice =
                    Some("Original writing changed; its latest saved version is open.");
                Ok(())
            }
            Err(error) => {
                self.project_transfer.origin = Some(origin);
                Err(error)
            }
        }
    }
    fn transfer_cancelled(&mut self, retry: Option<Retry>) {
        let context = self.project_transfer.context();
        self.project_transfer.view = Some(View::Outcome(Outcome {
            context,
            kind: OutcomeKind::Cancelled,
            details: None,
            retry,
        }));
    }
    fn start_transfer_check(&mut self, path: PathBuf, folder: bool, ctx: &egui::Context) {
        let context = Context {
            title: String::new(),
            path: Some(path.clone()),
        };
        let cancel = ProjectTransferCancellation::default();
        let worker_cancel = cancel.clone();
        let (sender, receiver) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let event = check_input_job(path, folder, &worker_cancel);
            let _ = sender.send(event);
            ctx.request_repaint();
        });
        self.project_transfer.view = Some(View::Working(Job {
            receiver,
            cancel,
            cancelling: false,
            context,
            kind: JobKind::Check,
        }));
    }
    fn start_project_export(
        &mut self,
        request: ExportRequest,
        destination: PathBuf,
        ctx: &egui::Context,
    ) {
        let context = Context {
            title: request.project.title.clone(),
            path: Some(destination.clone()),
        };
        let worker_context = context.clone();
        let database = self.database.clone();
        let cancel = ProjectTransferCancellation::default();
        let worker_cancel = cancel.clone();
        let (sender, receiver) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let event = export_project_job(
                &database,
                request,
                destination,
                &worker_cancel,
                worker_context,
            );
            let _ = sender.send(event);
            ctx.request_repaint();
        });
        self.project_transfer.view = Some(View::Working(Job {
            receiver,
            cancel,
            cancelling: false,
            context,
            kind: JobKind::Export,
        }));
    }
    fn start_project_restore(&mut self, input: CheckedInput, ctx: &egui::Context) {
        let context = Context {
            title: input.summary.title.clone(),
            path: Some(input.path.clone()),
        };
        let worker_context = context.clone();
        let database = self.database.clone();
        let cancel = ProjectTransferCancellation::default();
        let worker_cancel = cancel.clone();
        let (sender, receiver) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let event = restore_project_job(&database, input, &worker_cancel, worker_context);
            let _ = sender.send(event);
            ctx.request_repaint();
        });
        self.project_transfer.view = Some(View::Working(Job {
            receiver,
            cancel,
            cancelling: false,
            context,
            kind: JobKind::Restore,
        }));
    }
}
fn intent_context(intent: &Intent) -> Context {
    match intent {
        Intent::Export(request) => Context {
            title: request.project.title.clone(),
            path: None,
        },
        Intent::Restore(input) => Context {
            title: input.summary.title.clone(),
            path: Some(input.path.clone()),
        },
    }
}
fn portable_folder_name(title: &str) -> String {
    let mut title = title
        .trim()
        .chars()
        .map(|c| {
            if c.is_control() || "/\\<>:\"|?*".contains(c) {
                '-'
            } else {
                c
            }
        })
        .collect::<String>();
    while title.len() > 180 {
        title.pop();
    }
    let title = title.trim_end_matches(['.', ' ']);
    if title.is_empty() || matches!(title, "." | "..") {
        "WriterCraft project".into()
    } else {
        let candidate = format!("{title}-export");
        if reserved_folder_name(&candidate) {
            format!("_{candidate}")
        } else {
            candidate
        }
    }
}
fn validate_folder_name(name: &str) -> AppResult<()> {
    if name.trim() != name
        || name.is_empty()
        || name.len() > 240
        || matches!(name, "." | "..")
        || name
            .chars()
            .any(|c| c.is_control() || "/\\<>:\"|?*".contains(c))
        || name.ends_with(['.', ' '])
        || reserved_folder_name(name)
    {
        return Err("folder_name_must_be_one_portable_component".into());
    }
    Ok(())
}
fn reveal_output(path: &Path) -> AppResult<()> {
    let folder = if path.is_dir() {
        path
    } else {
        path.parent().ok_or("missing_export_parent")?
    };
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = std::process::Command::new("explorer.exe");
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut command = std::process::Command::new("xdg-open");
    command.arg(folder).spawn()?;
    Ok(())
}

#[cfg(any(test, feature = "editor-preview"))]
fn scoped_pending_notes(
    workspace: &Workspace,
    project: &str,
    pending: &[String],
) -> AppResult<Vec<(String, String)>> {
    let pending = pending.iter().collect::<std::collections::HashSet<_>>();
    Ok(workspace
        .list_documents(project)?
        .into_iter()
        .filter(|document| pending.contains(&document.id))
        .map(|document| (document.id, document.title))
        .collect())
}
fn check_input_job(path: PathBuf, folder: bool, cancel: &ProjectTransferCancellation) -> Event {
    let result = (|| -> AppResult<CheckedInput> {
        let stream = if folder {
            CanonicalProjectFolder::open_with_cancellation(&path, cancel)?.preflight(cancel)?
        } else {
            CanonicalProjectArchive::open_with_cancellation(&path, cancel)?.preflight(cancel)?
        };
        let summary = stream.project_summary()?;
        if summary.scope != PortableProjectScope::CompleteProject {
            return Err("selected_native_restore_is_not_available".into());
        }
        Ok(CheckedInput {
            path,
            summary,
            stream: Box::new(stream),
        })
    })();
    match result {
        Ok(input) => Event::Checked(input),
        Err(error) => Event::Failed {
            details: error.to_string(),
            retry: Retry::ChooseRestore,
        },
    }
}
fn export_project_job(
    database: &Path,
    request: ExportRequest,
    destination: PathBuf,
    cancel: &ProjectTransferCancellation,
    context: Context,
) -> Event {
    let result = (|| -> AppResult<()> {
        let prepared = ProjectPortableExportWorker::open(database)?.prepare_named(
            &request.project.id,
            &request.project.title,
            cancel,
        )?;
        match request.kind {
            ExportKind::Archive => {
                export_canonical_project_archive_file(&prepared, database, &destination, cancel)?;
            }
            ExportKind::Folder => {
                export_canonical_project_folder(&prepared, database, &destination, cancel)?;
            }
        }
        Ok(())
    })();
    match result {
        Ok(()) => Event::Exported(context),
        Err(error) => Event::Failed {
            details: error.to_string(),
            retry: Retry::Export(request),
        },
    }
}
fn restore_project_job(
    database: &Path,
    input: CheckedInput,
    cancel: &ProjectTransferCancellation,
    context: Context,
) -> Event {
    let result = (|| -> AppResult<String> {
        Ok(ProjectImportWorker::open(database)?
            .restore_stream(&input.stream, cancel)?
            .project_id)
    })();
    match result {
        Ok(project_id) => Event::Restored {
            context,
            project_id,
        },
        Err(error) => Event::Failed {
            details: error.to_string(),
            retry: Retry::Restore(input),
        },
    }
}

fn reserved_folder_name(name: &str) -> bool {
    let base = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((base.starts_with("COM") || base.starts_with("LPT"))
            && base.len() == 4
            && matches!(base.as_bytes()[3], b'1'..=b'9'))
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult = AppResult<()>;
    fn fixture() -> AppResult<(tempfile::TempDir, StudioApp, Project)> {
        let directory = tempfile::tempdir()?;
        let creation = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = StudioApp::new(
            &creation,
            directory.path().join("transfer.sqlite"),
            false,
            None,
        )?;
        let project = app
            .workspace
            .create_project("Synthetic transfer Автор 😀")?;
        let content = serde_json::json!({"type":"doc","attrs":{"schemaVersion":1},"content":[{"type":"paragraph","attrs":{"blockId":"p"},"content":[{"type":"text","text":"Saved original 😀"}]}]});
        app.workspace
            .create_rich_document(&project.id, "Writing", content)?;
        app.open_project(project.clone())?;
        Ok((directory, app, project))
    }
    fn request(project: &Project, kind: ExportKind) -> ExportRequest {
        ExportRequest {
            project: project.clone(),
            kind,
            folder_name: "Chosen folder 😀".into(),
        }
    }
    #[test]
    fn actual_export_acknowledgement_should_win_over_late_cancellation() -> TestResult {
        let (directory, app, project) = fixture()?;
        let path = directory.path().join("Project.writercraft");
        let context = Context {
            title: project.title.clone(),
            path: Some(path.clone()),
        };
        let event = export_project_job(
            &app.database,
            request(&project, ExportKind::Archive),
            path.clone(),
            &ProjectTransferCancellation::default(),
            context.clone(),
        );
        let (sender, receiver) = mpsc::channel();
        sender.send(event).map_err(|_| "event_send")?;
        let cancel = ProjectTransferCancellation::default();
        cancel.cancel();
        let mut state = ProjectTransferState {
            visible: true,
            view: Some(View::Working(Job {
                receiver,
                cancel,
                cancelling: true,
                context,
                kind: JobKind::Export,
            })),
            ..Default::default()
        };
        state.poll(&egui::Context::default());
        assert!(matches!(
            state.view,
            Some(View::Outcome(Outcome {
                kind: OutcomeKind::Exported,
                ..
            }))
        ));
        CanonicalProjectArchive::open(&path)?.preflight(&ProjectTransferCancellation::default())?;
        Ok(())
    }
    #[test]
    fn confirmed_restore_should_use_private_checked_bytes_after_source_path_is_replaced()
    -> TestResult {
        let (directory, app, project) = fixture()?;
        let path = directory.path().join("Source.writercraft");
        let context = Context {
            title: project.title.clone(),
            path: Some(path.clone()),
        };
        assert!(matches!(
            export_project_job(
                &app.database,
                request(&project, ExportKind::Archive),
                path.clone(),
                &ProjectTransferCancellation::default(),
                context.clone()
            ),
            Event::Exported(_)
        ));
        let Event::Checked(input) =
            check_input_job(path.clone(), false, &ProjectTransferCancellation::default())
        else {
            return Err("checked_input_missing".into());
        };
        std::fs::write(&path, b"Unrelated replacement after verified preview")?;
        let database = directory.path().join("Restored.sqlite");
        let Event::Restored { project_id, .. } = restore_project_job(
            &database,
            input,
            &ProjectTransferCancellation::default(),
            context,
        ) else {
            return Err("restore_ack_missing".into());
        };
        let workspace = Workspace::open(&database)?;
        assert_ne!(project_id, project.id);
        assert_eq!(
            workspace.list_documents(&project_id)?[0].body,
            "Saved original 😀"
        );
        assert_eq!(
            std::fs::read(path)?,
            b"Unrelated replacement after verified preview"
        );
        Ok(())
    }
    #[test]
    fn folder_collision_should_preserve_unrelated_data_and_exact_retry_input() -> TestResult {
        let (directory, app, project) = fixture()?;
        let destination = directory.path().join("Existing");
        std::fs::create_dir(&destination)?;
        std::fs::write(destination.join("README.md"), b"Unrelated existing data")?;
        let context = Context {
            title: project.title.clone(),
            path: Some(destination.clone()),
        };
        let Event::Failed {
            retry: Retry::Export(retry),
            ..
        } = export_project_job(
            &app.database,
            request(&project, ExportKind::Folder),
            destination.clone(),
            &ProjectTransferCancellation::default(),
            context,
        )
        else {
            return Err("collision_not_refused".into());
        };
        assert_eq!(retry.project.id, project.id);
        assert_eq!(retry.folder_name, "Chosen folder 😀");
        assert_eq!(
            std::fs::read(destination.join("README.md"))?,
            b"Unrelated existing data"
        );
        assert!(!destination.join(".incomplete").exists());
        Ok(())
    }
    #[test]
    fn invalid_folder_input_should_remain_in_its_configuration_without_opening_a_picker()
    -> TestResult {
        let (_directory, mut app, project) = fixture()?;
        let mut request = request(&project, ExportKind::Folder);
        request.folder_name = "../outside".into();
        app.project_transfer.view = Some(View::Export(request));
        assert!(
            app.transfer_action(Action::Continue, &egui::Context::default())
                .is_err()
        );
        let Some(View::Export(retained)) = app.project_transfer.view.as_ref() else {
            return Err("configured_input_lost".into());
        };
        assert_eq!(retained.folder_name, "../outside");
        assert_eq!(retained.project.id, project.id);
        Ok(())
    }
    #[test]
    fn pending_note_review_should_query_only_actual_owned_documents_without_saving_foreign_writing()
    -> TestResult {
        let (_directory, app, project) = fixture()?;
        let owned = app.workspace.list_documents(&project.id)?.remove(0);
        let other = app.workspace.create_project("Unrelated")?;
        let foreign = app
            .workspace
            .create_document(&other.id, "Foreign", "Untouched writing")?;
        let notes = scoped_pending_notes(
            &app.workspace,
            &project.id,
            &[
                owned.id.clone(),
                foreign.id.clone(),
                "unavailable-foreign-handle".into(),
            ],
        )?;
        assert_eq!(notes, vec![(owned.id, owned.title)]);
        assert_eq!(app.workspace.document(&foreign.id)?, foreign);
        Ok(())
    }
}
