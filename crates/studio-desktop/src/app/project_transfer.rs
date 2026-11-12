//! Explicit whole-project transfer over bounded canonical workers, separate from workspace backup.
use super::navigation_return::NavigationReturn;
use super::*;
use std::path::Path;
use studio_core::{
    PortableProjectScope, PortableProjectSummary, ProjectImportWorker, ProjectPortableExportWorker,
    ProjectTransferCancellation, VerifiedPortableProjectStream,
};
use studio_transfer::{
    CanonicalProjectArchive, CanonicalProjectFolder, ManuscriptEntry, ManuscriptPlan,
    ManuscriptSelection, MarkdownConversion, compile_manuscript,
    export_canonical_project_archive_file, export_canonical_project_folder, plan_manuscript,
};

#[derive(Default)]
pub(super) struct ProjectTransferState {
    visible: bool,
    view: Option<View>,
    origin: Option<NavigationReturn>,
    auxiliary_error: Option<String>,
    pending_open: Option<String>,
    notice: Option<&'static str>,
    /// Active manuscript nodes captured when the export dialog opened; the modal blocks editing.
    scope_nodes: Vec<ManuscriptScopeNode>,
    current_document: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum ExportKind {
    Manuscript,
    Archive,
    Folder,
}
/// Which part of the current project one compiled reader manuscript includes.
#[derive(Clone, Debug, PartialEq)]
enum ManuscriptScope {
    Project,
    Document(String),
    Subtree(String),
    Documents(Vec<String>),
}
#[derive(Clone, Copy, PartialEq)]
enum ScopeKind {
    Project,
    Document,
    Subtree,
    Documents,
}
/// One active node offered as a manuscript scope or checkbox row.
#[derive(Clone)]
struct ManuscriptScopeNode {
    id: String,
    path: String,
    folder: bool,
}
#[derive(Clone)]
struct ExportRequest {
    project: Project,
    kind: ExportKind,
    folder_name: String,
    manuscript: ManuscriptScope,
}
struct ManuscriptPreview {
    request: ExportRequest,
    plan: ManuscriptPlan,
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
    /// Pre-transfer checks finished; the actual export/restore starts only
    /// from an explicit user confirmation, never from reopening the dialog.
    armed: bool,
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
    Preview(ManuscriptPreview),
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
    Plan,
    Export,
    Restore,
}
enum Event {
    Checked(CheckedInput),
    Planned {
        request: ExportRequest,
        plan: ManuscriptPlan,
    },
    Exported(Context, ExportKind),
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
    Compiled,
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
                Some(View::Working(_))
                    | Some(View::Preparing(_))
                    | Some(View::Ready(_))
                    | Some(View::Preview(_))
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
            Some(View::Preview(preview)) => Context {
                title: preview.request.project.title.clone(),
                path: None,
            },
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
        let planning = matches!(job.kind, JobKind::Plan);
        self.view = Some(match event {
            Event::Checked(_input) if cancel_requested => View::Outcome(Outcome {
                context,
                kind: OutcomeKind::Cancelled,
                details: None,
                retry: Some(Retry::ChooseRestore),
            }),
            Event::Checked(input) => View::Ready(input),
            Event::Planned { request, .. } if cancel_requested => View::Outcome(Outcome {
                context,
                kind: OutcomeKind::Cancelled,
                details: None,
                retry: Some(Retry::Export(request)),
            }),
            Event::Planned { request, plan } => View::Preview(ManuscriptPreview { request, plan }),
            // A confirmed publication takes precedence over a later cancellation request.
            Event::Exported(context, kind) => View::Outcome(Outcome {
                context,
                kind: if kind == ExportKind::Manuscript {
                    OutcomeKind::Compiled
                } else {
                    OutcomeKind::Exported
                },
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
            Event::Failed { details: _, retry } if (checking || planning) && cancel_requested => {
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
                        for (kind,label) in [(ExportKind::Manuscript,"Manuscript Markdown (one file)"),(ExportKind::Archive,"Native project archive"),(ExportKind::Folder,"Readable project folder")]{
                            if ui.add_sized([ui.available_width(),components::FIELD_HEIGHT],egui::Button::new(l.text(label)).selected(request.kind==kind).wrap()).clicked(){request.kind=kind;}
                        }
                        ui.add_space(12.0);
                        if request.kind==ExportKind::Manuscript {
                            ui.label(l.text("One ordered Markdown manuscript for reading. Used images go to a sibling assets folder. This is not a project backup."));
                            ui.add_space(12.0);
                            manuscript_scope_editor(ui,l,&mut request.manuscript,&self.scope_nodes,self.current_document.as_deref(),t);
                        } else {
                            ui.label(l.text("Both formats preserve saved history, notes, original artifacts and untrusted recovery data. The folder also includes readable Markdown and JSON Canvas."));
                        }
                        ui.add_space(12.0);
                        if matches!(request.kind,ExportKind::Folder|ExportKind::Manuscript) {
                            ui.label(l.text("Folder name"));components::singleline(ui,&mut request.folder_name,l.text("Folder name"));
                            if validate_folder_name(&request.folder_name).is_err(){ui.colored_label(t.warning,l.text("Enter one folder name without separators or reserved filenames."));}
                            ui.label(RichText::new(l.text("Choose a parent folder. A new folder with this name will be created.")).color(t.muted));
                        }
                        ui.add_space(12.0);
                        if request.kind==ExportKind::Manuscript {
                            ui.label(RichText::new(l.text("Notes, annotations, histories, character research and project data stay in the project.")).color(t.muted));
                        } else {
                            ui.label(RichText::new(l.text("This transfers one complete project. Use workspace backup to copy every local project.")).color(t.muted));
                        }
                        ui.label(RichText::new(l.text("Choose a new path. Existing files and folders are never replaced.")).color(t.muted));
                    }
                    Some(View::Preview(preview))=>{
                        ui.label(RichText::new(l.text("Included order")).strong());ui.add_space(4.0);
                        egui::ScrollArea::vertical().id_salt("manuscript-included-order").max_height(220.0).show(ui,|ui|{
                            for entry in &preview.plan.entries {
                                ui.horizontal(|ui|{
                                    ui.add_space(f32::from(entry.heading_level.saturating_sub(1))*14.0);
                                    ui.label(RichText::new(manuscript_entry_heading(entry)).monospace());
                                });
                            }
                        });
                        ui.add_space(12.0);
                        let disclosures=manuscript_disclosures(&preview.plan,l);
                        if disclosures.is_empty() {
                            ui.label(RichText::new(l.text("No reader conversions beyond plain Markdown.")).color(t.muted));
                        } else {
                            ui.label(l.text("Reader conversions"));
                            for message in disclosures {ui.label(message);}
                        }
                        ui.add_space(12.0);
                        ui.label(RichText::new(l.text("Reader export is deliberately not a lossless project backup. The source project and original artifacts remain unchanged.")).color(t.muted));
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
                        if state.notes.is_empty()&&!state.armed{ui.spinner();ui.label(l.text("Saving current work before transfer…"));}
                        else if state.notes.is_empty() {
                            ui.label(l.text("Ready to transfer."));ui.add_space(8.0);
                            ui.label(RichText::new(l.text("Confirm to start; reopening this window never starts the transfer by itself.")).color(t.muted));
                        }
                        else {
                            ui.label(l.text("Review pending notes in this project before continuing."));ui.add_space(12.0);
                            for (id,title) in &state.notes {ui.horizontal_wrapped(|ui|{ui.label(title);if components::action(ui,l.text("Review notes"),true,None).clicked(){action=Some(Action::Review(id.clone()));}});}
                        }
                    }
                    Some(View::Working(job))=>{
                        ui.spinner();ui.label(l.text(if job.cancelling{"Cancelling transfer…"}else{match job.kind{JobKind::Check=>"Checking preserved project data…",JobKind::Plan=>"Previewing manuscript order…",JobKind::Export=>"Exporting project…",JobKind::Restore=>"Restoring verified project…"}}));
                    }
                    Some(View::Outcome(outcome))=>{
                        ui.label(RichText::new(l.text(match outcome.kind{OutcomeKind::Exported=>"Project exported and verified.",OutcomeKind::Compiled=>"Manuscript exported and verified.",OutcomeKind::Restored(_)=>"Independent project restored.",OutcomeKind::Cancelled=>"No project was exported or restored.",OutcomeKind::Unconfirmed=>"The transfer could not be confirmed."})).strong());
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
                        Some(View::Export(_)) | Some(View::Ready(_)) | Some(View::Preview(_)) => {
                            let label = match &self.view {
                                Some(View::Ready(_)) => "Restore project",
                                Some(View::Export(request))
                                    if request.kind == ExportKind::Manuscript =>
                                {
                                    "Preview order…"
                                }
                                _ => "Choose destination…",
                            };
                            let enabled = match &self.view {
                                Some(View::Export(request)) => {
                                    export_request_ready(request, &self.scope_nodes)
                                }
                                _ => true,
                            };
                            if components::action(ui, l.text(label), enabled, Some(t)).clicked() {
                                action = Some(Action::Continue);
                            }
                        }
                        Some(View::Preparing(state)) if !state.notes.is_empty() => {
                            if components::action(ui, l.text("Continue"), true, Some(t)).clicked() {
                                action = Some(Action::Continue);
                            }
                        }
                        Some(View::Preparing(state)) if state.armed => {
                            let label = if matches!(state.intent, Intent::Restore(_)) {
                                "Restore project"
                            } else {
                                "Choose destination…"
                            };
                            if components::action(ui, l.text(label), true, Some(t)).clicked() {
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
                            } else if matches!(
                                outcome.kind,
                                OutcomeKind::Exported | OutcomeKind::Compiled
                            ) && components::action(
                                ui,
                                l.text("Open output folder"),
                                true,
                                Some(t),
                            )
                            .clicked()
                            {
                                action = Some(Action::Reveal);
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
        let current_document = self
            .document
            .as_ref()
            .filter(|document| document.project_id == project.id)
            .map(|document| document.id.clone());
        let (scope_nodes, auxiliary_error) =
            match manuscript_scope_nodes(&self.workspace, &project.id) {
                Ok(nodes) => (nodes, None),
                Err(error) => (Vec::new(), Some(error.to_string())),
            };
        self.project_transfer.scope_nodes = scope_nodes;
        self.project_transfer.auxiliary_error = auxiliary_error;
        self.project_transfer.current_document = current_document.clone();
        let manuscript = match current_document {
            Some(id) => ManuscriptScope::Document(id),
            None => ManuscriptScope::Project,
        };
        self.project_transfer.view = Some(View::Export(ExportRequest {
            folder_name: portable_folder_name(&project.title),
            project,
            kind: ExportKind::Archive,
            manuscript,
        }));
        self.project_transfer.visible = true;
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
    /// Capture-only presentation override: open the manuscript export mode on the demo project.
    pub(super) fn capture_manuscript_export(&mut self, preview: bool) -> AppResult<()> {
        self.open_project_export();
        let Some(View::Export(mut request)) = self.project_transfer.view.take() else {
            return Ok(());
        };
        request.kind = ExportKind::Manuscript;
        if !preview {
            self.project_transfer.view = Some(View::Export(request));
            return Ok(());
        }
        request.manuscript = ManuscriptScope::Project;
        match plan_manuscript_job(
            &self.database,
            &request,
            &ProjectTransferCancellation::default(),
        ) {
            Event::Planned { request, plan } => {
                self.project_transfer.view =
                    Some(View::Preview(ManuscriptPreview { request, plan }));
            }
            Event::Failed { details, .. } => return Err(details.into()),
            _ => {
                self.project_transfer.view = Some(View::Export(request));
            }
        }
        Ok(())
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
        if let Some(action) = action
            && let Err(error) = self.transfer_action(action, ctx)
        {
            self.project_transfer.auxiliary_error = Some(error.to_string());
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
        if let Err(error) = self.continue_project_transfer(ctx, true) {
            self.project_transfer.auxiliary_error = Some(error.to_string());
        }
    }
    fn transfer_action(&mut self, action: Action, ctx: &egui::Context) -> AppResult<()> {
        match action {
            Action::Continue => {
                if let Some(View::Export(request)) = &self.project_transfer.view {
                    if matches!(request.kind, ExportKind::Folder | ExportKind::Manuscript) {
                        validate_folder_name(&request.folder_name)?;
                    }
                    if request.kind == ExportKind::Manuscript
                        && let Some(problem) = manuscript_scope_problem(
                            &request.manuscript,
                            &self.project_transfer.scope_nodes,
                        )
                    {
                        return Err(problem.into());
                    }
                }
                match self.project_transfer.view.take() {
                    Some(View::Export(request)) if request.kind == ExportKind::Manuscript => {
                        self.start_manuscript_plan(request, ctx);
                        return Ok(());
                    }
                    Some(View::Export(request)) => {
                        let project = Some(request.project.id.clone());
                        self.project_transfer.view = Some(View::Preparing(Prerequisites {
                            intent: Intent::Export(request),
                            project,
                            notes: Vec::new(),
                            armed: false,
                        }));
                    }
                    Some(View::Preview(preview)) => {
                        let project = Some(preview.request.project.id.clone());
                        self.project_transfer.view = Some(View::Preparing(Prerequisites {
                            intent: Intent::Export(preview.request),
                            project,
                            notes: Vec::new(),
                            armed: false,
                        }));
                    }
                    Some(View::Ready(input)) => {
                        let project = self.project.as_ref().map(|project| project.id.clone());
                        self.project_transfer.view = Some(View::Preparing(Prerequisites {
                            intent: Intent::Restore(input),
                            project,
                            notes: Vec::new(),
                            armed: false,
                        }));
                    }
                    other => self.project_transfer.view = other,
                }
                self.continue_project_transfer(ctx, false)?;
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
    fn continue_project_transfer(&mut self, ctx: &egui::Context, auto: bool) -> AppResult<()> {
        let view = self.project_transfer.view.take();
        let Some(View::Preparing(mut state)) = view else {
            self.project_transfer.view = view;
            return Ok(());
        };
        if state.armed && auto {
            // The checks already finished; only an explicit confirmation may start.
            self.project_transfer.view = Some(View::Preparing(state));
            return Ok(());
        }
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
            Ok(true) if auto => {
                // Reopening the dialog only arms the confirmed transfer; it never
                // starts an export or restore without an explicit user action.
                state.armed = true;
                self.project_transfer.auxiliary_error = None;
                self.project_transfer.view = Some(View::Preparing(state));
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
    fn start_manuscript_plan(&mut self, request: ExportRequest, ctx: &egui::Context) {
        let context = Context {
            title: request.project.title.clone(),
            path: None,
        };
        let worker_request = request.clone();
        let database = self.database.clone();
        let cancel = ProjectTransferCancellation::default();
        let worker_cancel = cancel.clone();
        let (sender, receiver) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let event = plan_manuscript_job(&database, &worker_request, &worker_cancel);
            let _ = sender.send(event);
            ctx.request_repaint();
        });
        self.project_transfer.auxiliary_error = None;
        self.project_transfer.view = Some(View::Working(Job {
            receiver,
            cancel,
            cancelling: false,
            context,
            kind: JobKind::Plan,
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
impl ExportRequest {
    fn selection(&self) -> ManuscriptSelection {
        match &self.manuscript {
            ManuscriptScope::Project => ManuscriptSelection::Project,
            ManuscriptScope::Document(id) => ManuscriptSelection::Document(id.clone()),
            ManuscriptScope::Subtree(id) => ManuscriptSelection::Subtree(id.clone()),
            ManuscriptScope::Documents(ids) => ManuscriptSelection::Documents(ids.clone()),
        }
    }
}
fn export_request_ready(request: &ExportRequest, nodes: &[ManuscriptScopeNode]) -> bool {
    if matches!(request.kind, ExportKind::Folder | ExportKind::Manuscript)
        && validate_folder_name(&request.folder_name).is_err()
    {
        return false;
    }
    request.kind != ExportKind::Manuscript
        || manuscript_scope_problem(&request.manuscript, nodes).is_none()
}
fn manuscript_scope_problem(
    scope: &ManuscriptScope,
    nodes: &[ManuscriptScopeNode],
) -> Option<&'static str> {
    match scope {
        ManuscriptScope::Project => None,
        ManuscriptScope::Document(id) => (id.is_empty()
            || !nodes.iter().any(|node| node.id == *id && !node.folder))
        .then_some("Open a document to include it."),
        ManuscriptScope::Subtree(id) => (id.is_empty() || !nodes.iter().any(|node| node.id == *id))
            .then_some("Choose a section."),
        ManuscriptScope::Documents(ids) => (ids.is_empty()
            || ids
                .iter()
                .any(|id| !nodes.iter().any(|node| node.id == *id && !node.folder)))
        .then_some("Select at least one document."),
    }
}
fn manuscript_scope_editor(
    ui: &mut egui::Ui,
    l: Language,
    scope: &mut ManuscriptScope,
    nodes: &[ManuscriptScopeNode],
    current_document: Option<&str>,
    t: Tokens,
) {
    let active = match scope {
        ManuscriptScope::Project => ScopeKind::Project,
        ManuscriptScope::Document(_) => ScopeKind::Document,
        ManuscriptScope::Subtree(_) => ScopeKind::Subtree,
        ManuscriptScope::Documents(_) => ScopeKind::Documents,
    };
    ui.label(l.text("Include"));
    ui.horizontal_wrapped(|ui| {
        for (kind, label) in [
            (ScopeKind::Project, "Whole project"),
            (ScopeKind::Document, "Current document"),
            (ScopeKind::Subtree, "Section"),
            (ScopeKind::Documents, "Selected documents"),
        ] {
            if ui
                .add(egui::Button::new(l.text(label)).selected(active == kind))
                .clicked()
                && active != kind
            {
                *scope = initial_scope(kind, nodes, current_document);
            }
        }
    });
    ui.add_space(6.0);
    match scope {
        ManuscriptScope::Project => {
            ui.label(
                RichText::new(l.text("Every active document in the project is included."))
                    .color(t.muted),
            );
        }
        ManuscriptScope::Document(id) => {
            match nodes.iter().find(|node| node.id == *id && !node.folder) {
                Some(node) => {
                    ui.label(&node.path);
                }
                None => {
                    ui.colored_label(t.warning, l.text("Open a document to include it."));
                }
            }
        }
        ManuscriptScope::Subtree(id) => {
            let selected = nodes
                .iter()
                .find(|node| node.id == *id)
                .map(|node| node.path.clone())
                .unwrap_or_else(|| l.text("Choose section").to_owned());
            egui::ComboBox::from_id_salt("manuscript-section")
                .selected_text(selected)
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for node in nodes {
                        ui.selectable_value(id, node.id.clone(), &node.path);
                    }
                });
        }
        ManuscriptScope::Documents(ids) => {
            if ids.is_empty() {
                ui.colored_label(t.warning, l.text("Select at least one document."));
            }
            egui::ScrollArea::vertical()
                .id_salt("manuscript-documents")
                .max_height(180.0)
                .show(ui, |ui| {
                    for node in nodes.iter().filter(|node| !node.folder) {
                        let mut checked = ids.contains(&node.id);
                        if ui.checkbox(&mut checked, &node.path).changed() {
                            if checked {
                                ids.push(node.id.clone());
                            } else {
                                ids.retain(|id| id != &node.id);
                            }
                        }
                    }
                });
        }
    }
}
fn initial_scope(
    kind: ScopeKind,
    nodes: &[ManuscriptScopeNode],
    current_document: Option<&str>,
) -> ManuscriptScope {
    let current = current_document
        .filter(|id| nodes.iter().any(|node| node.id == *id && !node.folder))
        .map(str::to_owned);
    match kind {
        ScopeKind::Project => ManuscriptScope::Project,
        ScopeKind::Document => ManuscriptScope::Document(current.unwrap_or_default()),
        ScopeKind::Subtree => ManuscriptScope::Subtree(
            nodes
                .iter()
                .find(|node| node.folder)
                .or_else(|| nodes.first())
                .map(|node| node.id.clone())
                .unwrap_or_default(),
        ),
        ScopeKind::Documents => ManuscriptScope::Documents(current.into_iter().collect()),
    }
}
fn manuscript_entry_heading(entry: &ManuscriptEntry) -> String {
    format!(
        "{} {}",
        "#".repeat(usize::from(entry.heading_level)),
        entry.title
    )
}
fn manuscript_disclosures(plan: &ManuscriptPlan, l: Language) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for conversion in &plan.conversions {
        let message = l.text(disclosure_message(conversion));
        let line = match plan
            .entries
            .iter()
            .find(|entry| entry.node_id == conversion.resource_id)
        {
            Some(entry) if !entry.title.is_empty() => format!("• {}: {message}", entry.title),
            _ => format!("• {message}"),
        };
        if seen.insert(line.clone()) {
            out.push(line);
        }
    }
    out
}
fn disclosure_message(conversion: &MarkdownConversion) -> &'static str {
    match conversion.feature.as_str() {
        "reader-export-is-not-a-canonical-project-backup" => {
            "This is a reading copy, not a project backup."
        }
        "source-frontmatter-omitted-from-reader-export" => {
            "Source frontmatter is not part of the reader manuscript."
        }
        "document-link-outside-export-scope" => {
            "A link points to a document outside this manuscript."
        }
        "external-or-opaque-image-source-retained"
        | "unresolved-or-application-link-in-companion" => {
            "An original source stays as a link instead of copied content."
        }
        "formula-reader-support-required" | "formula-description-in-companion" => {
            "A formula stays as LaTeX source for the reader to render."
        }
        "mermaid-reader-support-required" => {
            "A Mermaid diagram stays as source for the reader to render."
        }
        "callout-reader-extension"
        | "block-identity-html-anchor"
        | "inline-code-linebreak-reading-normalization"
        | "scene-break-reading-separator" => {
            "Some blocks use reading extensions or normalized layout."
        }
        "ordered-list-numbering-style-in-companion"
        | "code-language-label-in-companion"
        | "image-display-size-in-companion"
        | "extended-link-attributes-in-companion"
        | "table-widths-and-cell-metadata-in-companion" => {
            "Display details stay in companion metadata."
        }
        "markdown-table-first-row-as-header" | "complex-table-html-reading-view" => {
            "A table becomes HTML or first-row-header Markdown."
        }
        _ => "This formatting is represented approximately in Markdown.",
    }
}
fn manuscript_scope_nodes(
    workspace: &Workspace,
    project: &str,
) -> AppResult<Vec<ManuscriptScopeNode>> {
    let nodes = workspace.project_tree(project)?;
    let documents = workspace.list_documents(project)?;
    let titles = documents
        .iter()
        .map(|document| (document.id.as_str(), document.title.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    let mut by_parent: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();
    for (index, node) in nodes.iter().enumerate() {
        by_parent
            .entry(node.metadata["parentId"].as_str().unwrap_or("").to_owned())
            .or_default()
            .push(index);
    }
    let position =
        |node: &studio_core::ProjectTreeNode| node.metadata["position"].as_f64().unwrap_or(0.0);
    for indices in by_parent.values_mut() {
        indices.sort_by(|a, b| {
            position(&nodes[*a])
                .total_cmp(&position(&nodes[*b]))
                .then(nodes[*a].ordinal.cmp(&nodes[*b].ordinal))
        });
    }
    let mut out = Vec::new();
    let mut stack = by_parent
        .get("")
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .rev()
        .map(|index| (index, String::new()))
        .collect::<Vec<_>>();
    while let Some((index, parent_path)) = stack.pop() {
        let node = &nodes[index];
        if node.metadata["trashedAt"]
            .as_str()
            .is_some_and(|timestamp| !timestamp.is_empty())
        {
            continue;
        }
        let title = node
            .document_id
            .as_deref()
            .and_then(|document| titles.get(document))
            .cloned()
            .unwrap_or_else(|| node.metadata["title"].as_str().unwrap_or("").to_owned());
        let path = if parent_path.is_empty() {
            title.clone()
        } else {
            format!("{parent_path} / {title}")
        };
        out.push(ManuscriptScopeNode {
            id: node.logical_id.clone(),
            path: path.clone(),
            folder: node.metadata["currentRevision"].as_u64().unwrap_or(0) == 0,
        });
        for child in by_parent
            .get(&node.logical_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .rev()
        {
            stack.push((child, path.clone()));
        }
    }
    Ok(out)
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
        if request.kind == ExportKind::Manuscript {
            let workspace = Workspace::open(database)?;
            let snapshot = workspace.project_export_snapshot(&request.project.id)?;
            compile_manuscript(
                &snapshot,
                database,
                &destination,
                &request.selection(),
                cancel,
            )?;
            return Ok(());
        }
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
            ExportKind::Manuscript => {}
        }
        Ok(())
    })();
    match result {
        Ok(()) => Event::Exported(context, request.kind),
        Err(error) => Event::Failed {
            details: error.to_string(),
            retry: Retry::Export(request),
        },
    }
}
fn plan_manuscript_job(
    database: &Path,
    request: &ExportRequest,
    cancel: &ProjectTransferCancellation,
) -> Event {
    let result = (|| -> AppResult<ManuscriptPlan> {
        let cancelled = || -> AppResult<()> {
            if cancel.is_cancelled() {
                return Err("operation cancelled; existing data is unchanged".into());
            }
            Ok(())
        };
        let workspace = Workspace::open(database)?;
        cancelled()?;
        let snapshot = workspace.project_export_snapshot(&request.project.id)?;
        cancelled()?;
        Ok(plan_manuscript(&snapshot, &request.selection())?)
    })();
    match result {
        Ok(plan) => Event::Planned {
            request: request.clone(),
            plan,
        },
        Err(error) => Event::Failed {
            details: error.to_string(),
            retry: Retry::Export(request.clone()),
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
            manuscript: ManuscriptScope::Project,
        }
    }
    fn dialog_frame(
        ctx: &egui::Context,
        state: &mut ProjectTransferState,
        language: Language,
        time: f64,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 800.0),
                )),
                focused: true,
                time: Some(time),
                events,
                ..Default::default()
            },
            |root| {
                let ctx = root.ctx().clone();
                let tokens = crate::theme::tokens(&ctx);
                let _ = state.dialog(&ctx, language, tokens);
            },
        );
        output.textures_delta.clear();
        output
    }
    fn painted_text(output: &egui::FullOutput) -> String {
        fn visit(shape: &egui::epaint::Shape, out: &mut String) {
            match shape {
                egui::epaint::Shape::Text(text) => {
                    out.push_str(&text.galley.job.text);
                    out.push('\n');
                }
                egui::epaint::Shape::Vec(shapes) => {
                    for shape in shapes {
                        visit(shape, out);
                    }
                }
                _ => {}
            }
        }
        let mut out = String::new();
        for shape in &output.shapes {
            visit(&shape.shape, &mut out);
        }
        out
    }
    fn painted_center(output: &egui::FullOutput, label: &str) -> Option<egui::Pos2> {
        fn visit(shape: &egui::epaint::Shape, label: &str, found: &mut Option<egui::Pos2>) {
            match shape {
                egui::epaint::Shape::Text(text) if text.galley.job.text == label => {
                    *found = Some(text.visual_bounding_rect().center());
                }
                egui::epaint::Shape::Vec(shapes) => {
                    for shape in shapes {
                        visit(shape, label, found);
                    }
                }
                _ => {}
            }
        }
        let mut found = None;
        for shape in &output.shapes {
            visit(&shape.shape, label, &mut found);
        }
        found
    }
    fn click(
        ctx: &egui::Context,
        state: &mut ProjectTransferState,
        language: Language,
        point: egui::Pos2,
    ) {
        let press = |pressed| egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        dialog_frame(
            ctx,
            state,
            language,
            1.0,
            vec![egui::Event::PointerMoved(point), press(true)],
        );
        dialog_frame(ctx, state, language, 1.1, vec![press(false)]);
    }
    fn manuscript_request(project: &Project) -> ExportRequest {
        ExportRequest {
            project: project.clone(),
            kind: ExportKind::Manuscript,
            folder_name: "Chosen manuscript".into(),
            manuscript: ManuscriptScope::Project,
        }
    }
    #[test]
    fn manuscript_format_choice_should_be_reachable_and_render_every_locale() -> TestResult {
        let (_directory, mut app, _project) = fixture()?;
        app.open_project_export();
        let ctx = egui::Context::default();
        // The first pass sizes the centered modal; the second paints its content.
        let _first = dialog_frame(
            &ctx,
            &mut app.project_transfer,
            Language::English,
            0.0,
            vec![],
        );
        let english = dialog_frame(
            &ctx,
            &mut app.project_transfer,
            Language::English,
            0.05,
            vec![],
        );
        let painted = painted_text(&english);
        let point = painted_center(&english, "Manuscript Markdown (one file)")
            .ok_or_else(|| format!("manuscript format is not reachable: {painted}"))?;
        click(&ctx, &mut app.project_transfer, Language::English, point);
        let Some(View::Export(request)) = app.project_transfer.view.as_ref() else {
            return Err("export view was replaced".into());
        };
        assert_eq!(request.kind, ExportKind::Manuscript);
        for language in Language::ALL {
            let output = dialog_frame(&ctx, &mut app.project_transfer, language, 2.0, vec![]);
            let text = painted_text(&output);
            assert!(
                !text.contains("⟦missing translation⟧"),
                "{language:?} is missing a manuscript export translation"
            );
            assert!(
                text.contains(language.text("Include")),
                "{language:?} does not render the manuscript scope selector"
            );
        }
        Ok(())
    }
    #[test]
    fn manuscript_export_should_write_one_ordered_document_and_verify_it() -> TestResult {
        let (directory, app, project) = fixture()?;
        let revision = app.workspace.project_structure(&project.id)?.revision;
        app.workspace.create_project_tree_node(
            &project.id,
            revision,
            None,
            "document",
            "Second chapter",
        )?;
        let request = manuscript_request(&project);
        if let Event::Planned { plan, .. } = plan_manuscript_job(
            &app.database,
            &request,
            &ProjectTransferCancellation::default(),
        ) {
            for entry in &plan.entries {
                eprintln!(
                    "entry {} {} {}",
                    entry.node_id, entry.title, entry.heading_level
                );
            }
        }
        for node in app.workspace.project_tree(&project.id)? {
            eprintln!(
                "node {} ordinal={} meta={}",
                node.logical_id, node.ordinal, node.metadata
            );
        }
        let destination = directory.path().join("Manuscript");
        let context = Context {
            title: project.title.clone(),
            path: Some(destination.clone()),
        };
        let event = export_project_job(
            &app.database,
            request,
            destination.clone(),
            &ProjectTransferCancellation::default(),
            context,
        );
        assert!(
            matches!(event, Event::Exported(_, ExportKind::Manuscript)),
            "a compiled manuscript is acknowledged as a manuscript"
        );
        let body = std::fs::read_to_string(destination.join(format!("{}.md", project.title)))?;
        let first = body.find("Saved original").ok_or("first chapter missing")?;
        let second = body
            .find("Second chapter")
            .ok_or("second chapter missing")?;
        assert!(
            first < second,
            "chapters follow the project tree order: {body}"
        );
        assert!(!destination.join(".incomplete").exists());
        Ok(())
    }
    #[test]
    fn manuscript_plan_should_disclose_reading_scope_before_writing() -> TestResult {
        let (directory, app, project) = fixture()?;
        let request = manuscript_request(&project);
        let destination = directory.path().join("Never created");
        let Event::Planned { plan, .. } = plan_manuscript_job(
            &app.database,
            &request,
            &ProjectTransferCancellation::default(),
        ) else {
            return Err("plan_missing".into());
        };
        assert_eq!(plan.entries.len(), 1);
        assert_eq!(plan.entries[0].title, "Writing");
        assert!(!destination.exists());
        let disclosures = manuscript_disclosures(&plan, Language::German);
        assert!(
            disclosures
                .iter()
                .any(|line| line.contains("Dies ist eine Lesefassung, keine Projektsicherung.")),
            "the plan names the reader scope before writing: {disclosures:?}"
        );
        Ok(())
    }
    #[test]
    fn manuscript_scope_nodes_should_follow_the_tree_and_reject_empty_choices() -> TestResult {
        let (_directory, app, project) = fixture()?;
        let nodes = manuscript_scope_nodes(&app.workspace, &project.id)?;
        let writing = nodes
            .iter()
            .find(|node| node.path == "Writing" && !node.folder)
            .ok_or("scope node missing")?;
        assert_eq!(
            initial_scope(ScopeKind::Documents, &nodes, Some(&writing.id)),
            ManuscriptScope::Documents(vec![writing.id.clone()])
        );
        assert_eq!(
            initial_scope(ScopeKind::Document, &nodes, None),
            ManuscriptScope::Document(String::new())
        );
        assert!(
            manuscript_scope_problem(&ManuscriptScope::Documents(Vec::new()), &nodes).is_some()
        );
        assert!(
            manuscript_scope_problem(&ManuscriptScope::Document("foreign".into()), &nodes)
                .is_some()
        );
        assert!(manuscript_scope_problem(&ManuscriptScope::Project, &nodes).is_none());
        assert!(!export_request_ready(
            &ExportRequest {
                folder_name: "../outside".into(),
                ..manuscript_request(&project)
            },
            &nodes
        ));
        assert!(export_request_ready(&manuscript_request(&project), &nodes));
        Ok(())
    }
    #[test]
    fn manuscript_preview_should_show_included_order_and_localized_disclosures() -> TestResult {
        let (_directory, mut app, project) = fixture()?;
        let request = manuscript_request(&project);
        let Event::Planned { plan, .. } = plan_manuscript_job(
            &app.database,
            &request,
            &ProjectTransferCancellation::default(),
        ) else {
            return Err("plan_missing".into());
        };
        app.project_transfer.visible = true;
        app.project_transfer.view = Some(View::Preview(ManuscriptPreview {
            request: request.clone(),
            plan: plan.clone(),
        }));
        let ctx = egui::Context::default();
        let _first = dialog_frame(
            &ctx,
            &mut app.project_transfer,
            Language::German,
            3.0,
            vec![],
        );
        let output = dialog_frame(
            &ctx,
            &mut app.project_transfer,
            Language::German,
            3.05,
            vec![],
        );
        let text = painted_text(&output);
        assert!(
            text.contains("# Writing"),
            "the preview names the chapter heading"
        );
        assert!(
            text.contains("Lesekonvertierungen"),
            "disclosures are localized"
        );
        assert!(!text.contains("⟦missing translation⟧"));
        for language in Language::ALL {
            let output = dialog_frame(&ctx, &mut app.project_transfer, language, 4.0, vec![]);
            let text = painted_text(&output);
            assert!(
                text.contains(language.text("Reader conversions")),
                "{language:?} is missing the preview disclosure heading"
            );
            assert!(!text.contains("⟦missing translation⟧"));
        }
        Ok(())
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
            Event::Exported(..)
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
    fn reopening_armed_transfer_waits_for_explicit_confirmation() -> TestResult {
        let (directory, mut app, project) = fixture()?;
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
            Event::Exported(..)
        ));
        let Event::Checked(input) =
            check_input_job(path, false, &ProjectTransferCancellation::default())
        else {
            return Err("checked_input_missing".into());
        };
        app.project_transfer.visible = true;
        app.project_transfer.view = Some(View::Preparing(Prerequisites {
            intent: Intent::Restore(input),
            project: None,
            notes: Vec::new(),
            armed: true,
        }));
        let before = app.workspace.list_projects()?.len();
        let ctx = egui::Context::default();
        // Merely reopening the transfer window must never start the restore.
        app.poll_project_transfer(&ctx);
        assert!(matches!(
            &app.project_transfer.view,
            Some(View::Preparing(state)) if state.armed
        ));
        assert_eq!(app.workspace.list_projects()?.len(), before);
        // The explicit confirmation starts it.
        app.transfer_action(Action::Continue, &ctx)?;
        assert!(
            app.project_transfer
                .view
                .as_ref()
                .is_some_and(|view| matches!(view, View::Working(_)))
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
