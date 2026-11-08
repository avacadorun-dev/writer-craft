//! Clipboard frame orchestration and explicit, non-destructive recovery.
use super::clipboard_jobs::{
    self, ClipboardIntent, ClipboardOutcome, CompletionResult, JobError, SubmitFailureKind,
};
use super::project_clipboard::{ProjectClipboard, ProjectClipboardError};
use super::*;
use craft_editor_egui::clipboard::{ClipboardBundle, ClipboardError, ClipboardRequest};
use std::{collections::VecDeque, sync::Arc};

const RECOVERY_BACKPRESSURE: usize = 4;
struct CapturedItem {
    bundle: Arc<ClipboardBundle>,
    scope: Option<ProjectClipboard>,
    message: &'static str,
}
/// Bounded-producer recovery queue. Input backpressure starts at four retained items; existing
/// engine/worker queues can add only their already bounded in-flight captures after that point.
pub(super) struct ClipboardRecovery {
    items: VecDeque<CapturedItem>,
}
const WORKING: &str = "Preparing clipboard content…";
const PENDING: &str = "Copy is still finishing. Try Paste again shortly.";
const BUSY: &str = "The clipboard is busy. Your captured content is preserved.";
const STALE: &str = "The selection changed. Paste again at the intended position.";
const CANCELLED: &str = "Clipboard action cancelled. Captured content is preserved.";
const RECOVERY_FULL: &str = "Resolve captured clipboard content before another clipboard action.";

pub(super) fn message(error: &ClipboardError) -> &'static str {
    match error {
        ClipboardError::Unavailable => {
            "The system clipboard is unavailable. Reopen the application and try again."
        }
        ClipboardError::Read => "The clipboard could not be read. Copy the content again.",
        ClipboardError::Write | ClipboardError::InvalidAcknowledgement => {
            "The clipboard could not be updated. Your text was not cut."
        }
        ClipboardError::ResourceLimit => {
            "The clipboard content is too large. Copy a smaller selection."
        }
        ClipboardError::InvalidSlice => "The clipboard content is invalid. Copy the content again.",
        ClipboardError::StaleRequest | ClipboardError::InvalidPasteContext => STALE,
        ClipboardError::Busy => BUSY,
        ClipboardError::WritePending => PENDING,
    }
}
fn job_message(error: &JobError) -> &'static str {
    match error {
        JobError::Transport(error)
        | JobError::Editor(craft_editor_egui::Error::Clipboard(error)) => message(error),
        JobError::Project(ProjectClipboardError::Clipboard(error)) => message(error),
        JobError::Project(ProjectClipboardError::InvalidCapsule) => {
            "Clipboard image data is invalid. Copy the content again."
        }
        JobError::Project(ProjectClipboardError::ExternalImagesRequired(images)) => {
            if images.is_empty() {
                "Clipboard image data is invalid. Copy the content again."
            } else {
                "Clipboard images need importing. Captured content is preserved."
            }
        }
        JobError::Project(ProjectClipboardError::Assets(_)) => {
            "Clipboard images could not be prepared. Original content is preserved."
        }
        JobError::Project(ProjectClipboardError::Canonical(_)) => {
            "The clipboard content is invalid. Copy the content again."
        }
        JobError::Editor(craft_editor_egui::Error::CompositionActive) => {
            "Finish composing text before retrying the clipboard action."
        }
        JobError::Editor(_) => {
            "The clipboard action could not be applied. Captured content is preserved."
        }
    }
}
fn is_clipboard_event(event: &egui::Event) -> bool {
    matches!(
        event,
        egui::Event::Copy | egui::Event::Cut | egui::Event::Paste(_)
    )
}
impl StudioApp {
    fn clipboard_scope(&self) -> studio_core::Result<ProjectClipboard> {
        let document = self.document.as_ref().ok_or_else(|| {
            studio_core::StudioError::Validation("No active clipboard project".into())
        })?;
        let max_bytes = self
            .clipboard
            .as_ref()
            .map_or(0, |provider| provider.max_bytes());
        Ok(ProjectClipboard::new(
            self.workspace.project_images(&document.project_id)?,
            max_bytes,
        ))
    }
    fn retain_clipboard(
        &mut self,
        bundle: Arc<ClipboardBundle>,
        scope: Option<ProjectClipboard>,
        message: &'static str,
    ) {
        let recovery = self
            .clipboard_recovery
            .get_or_insert_with(|| ClipboardRecovery {
                items: VecDeque::new(),
            });
        recovery.items.push_back(CapturedItem {
            bundle,
            scope,
            message,
        });
        self.status = message.into();
    }
    /// Call before shortcuts/input and before applying any background completion.
    pub(super) fn clipboard_frame_start(&mut self, ctx: &egui::Context) {
        self.clipboard_jobs.progress_native_output();
        let rich_focused = !self.library
            && !self.modal_open()
            && self
                .rich
                .as_ref()
                .is_some_and(|rich| ctx.memory(|memory| memory.has_focus(rich.session.options.id)));
        let foreign_copy = !rich_focused
            && ctx.input(|input| {
                input
                    .events
                    .iter()
                    .any(|event| matches!(event, egui::Event::Copy | egui::Event::Cut))
            });
        if self.modal_open()
            || foreign_copy
            || ctx.input(|input| input.viewport().close_requested())
        {
            self.cancel_clipboard_work();
        }
        let preserve = self
            .clipboard_recovery
            .as_ref()
            .is_some_and(|recovery| recovery.items.len() >= RECOVERY_BACKPRESSURE);
        let pending = self.clipboard_intents.has_pending_write()
            || self.clipboard_jobs.has_pending_native_output();
        if (preserve && rich_focused) || pending {
            let mut refused = false;
            ctx.input_mut(|input| {
                input.events.retain(|event| {
                    let reject = (preserve && rich_focused && is_clipboard_event(event))
                        || (pending && matches!(event, egui::Event::Paste(_)));
                    refused |= reject;
                    !reject
                })
            });
            if refused {
                self.status = if preserve { RECOVERY_FULL } else { PENDING }.into();
            }
        }
        let allow_mutation = !self.library && !self.modal_open() && self.can_edit_manuscript();
        for completion in self.clipboard_jobs.take_completed() {
            let session = if allow_mutation {
                self.rich.as_mut().map(|rich| &mut rich.session)
            } else {
                None
            };
            let result = self.clipboard_jobs.complete(completion, session);
            self.finish_clipboard_result(result);
        }
    }
    fn finish_clipboard_result(&mut self, result: CompletionResult) {
        match result.outcome {
            ClipboardOutcome::Copied => self.status = "Content copied".into(),
            ClipboardOutcome::Cut(change) | ClipboardOutcome::Pasted(change) => {
                if change.before_revision != change.after_revision {
                    // The normal rich canvas drains the pending change/proof and writes its draft.
                    self.dirty = Some(Instant::now());
                    self.status = "Saving…".into();
                }
            }
            ClipboardOutcome::Cancelled => {
                // Superseding Copy intentionally replaces an earlier read-only publication.
                // Cut/Paste preserve their capture because they represented a document intent.
                if result.intent != ClipboardIntent::Copy {
                    self.retain_clipboard(result.captured, Some(result.scope), CANCELLED);
                }
            }
            ClipboardOutcome::Failed(error) => {
                self.retain_clipboard(result.captured, Some(result.scope), job_message(&error))
            }
        }
    }
    /// Call after all canvas/dialog UI. A later native text-copy command wins over an earlier
    /// rich job even when the command was produced by a button rather than a keyboard event.
    pub(super) fn clipboard_frame_end(&mut self, ctx: &egui::Context) {
        let ordinary_copy = ctx.output(|output| {
            output.commands.iter().any(|command| {
                matches!(
                    command,
                    egui::OutputCommand::CopyText(_) | egui::OutputCommand::CopyImage(_)
                )
            })
        });
        let needs_ordering = self.clipboard_intents.has_pending_write()
            || self.clipboard_jobs.has_pending_native_output();
        if ordinary_copy || self.modal_open() {
            self.cancel_clipboard_work();
            if ordinary_copy && needs_ordering {
                let captured = ctx.output_mut(|output| {
                    let mut latest = None;
                    for command in std::mem::take(&mut output.commands) {
                        if matches!(
                            &command,
                            egui::OutputCommand::CopyText(_) | egui::OutputCommand::CopyImage(_)
                        ) {
                            // Native clipboard semantics retain the latest Copy intent in order.
                            latest = Some(command);
                        } else {
                            output.commands.push(command);
                        }
                    }
                    latest
                });
                if let Some(command) = captured {
                    self.clipboard_jobs.defer_native_output(command);
                    self.status = WORKING.into();
                }
            }
        } else {
            let requests = self
                .rich
                .as_mut()
                .map(|rich| rich.session.take_clipboard_requests())
                .unwrap_or_default();
            for request in requests {
                let scope = self.clipboard_scope().ok();
                self.submit_clipboard_request(request, scope, ctx);
            }
        }
        if let Some(command) = self.clipboard_jobs.take_ready_native_output() {
            // Preserve the framework's exact native TextEdit/image output path. This is actual
            // re-emission, not a fabricated ClipboardProvider write acknowledgement.
            ctx.output_mut(|output| output.commands.push(command));
            self.status = "Content copied".into();
        } else if self.clipboard_jobs.native_output_error().is_some() {
            self.status = "The clipboard worker stopped. Captured content is preserved.".into();
        }
    }
    fn submit_clipboard_request(
        &mut self,
        request: ClipboardRequest,
        scope: Option<ProjectClipboard>,
        ctx: &egui::Context,
    ) {
        let captured = clipboard_jobs::captured_bundle(&request);
        let Some(provider) = self.clipboard.clone() else {
            self.retain_clipboard(captured, scope, message(&ClipboardError::Unavailable));
            return;
        };
        let Some(scope) = scope else {
            self.retain_clipboard(
                captured,
                None,
                "Clipboard images could not be prepared. Original content is preserved.",
            );
            return;
        };
        match self
            .clipboard_jobs
            .submit(request, scope.clone(), provider, ctx)
        {
            Ok(()) => {
                self.status = WORKING.into();
            }
            Err(failure) => {
                let msgid = match failure.kind {
                    SubmitFailureKind::Busy => BUSY,
                    SubmitFailureKind::Closed | SubmitFailureKind::EpochExhausted => {
                        "The clipboard worker stopped. Captured content is preserved."
                    }
                };
                self.retain_clipboard(
                    clipboard_jobs::captured_bundle(&failure.request),
                    Some(scope),
                    msgid,
                );
                // Dropping this rejected request releases its pending write ticket without ACK.
            }
        }
    }
    /// Call before changing project/document or closing. Cancellation never joins the worker.
    pub(super) fn cancel_clipboard_work(&mut self) {
        if self.clipboard_jobs.cancel_pending().is_err() {
            self.status = "The clipboard worker stopped. Captured content is preserved.".into();
        }
        let requests = if let Some(rich) = self.rich.as_mut() {
            let requests = rich.session.take_clipboard_requests();
            if rich.session.cancel_clipboard_requests().is_err() {
                self.status = "The clipboard worker stopped. Captured content is preserved.".into();
            }
            requests
        } else {
            vec![]
        };
        if requests.is_empty() {
            return;
        }
        // Scope creation is needed only for actual captured work, never on every modal frame.
        let scope = self.clipboard_scope().ok();
        for request in requests {
            if clipboard_jobs::request_intent(&request) != ClipboardIntent::Copy {
                self.retain_clipboard(
                    clipboard_jobs::captured_bundle(&request),
                    scope.clone(),
                    CANCELLED,
                );
            }
        }
    }
    /// Render as a separate bounded bottom panel above the existing status bar, not inside its
    /// fixed-height row. Actions are explicit new intents; a failed Cut is never silently retried.
    pub(super) fn clipboard_recovery_actions(&mut self, ui: &mut egui::Ui) {
        if self.clipboard_jobs.native_output_error().is_some() {
            let l = self.language;
            let tokens = theme::tokens(ui.ctx());
            let mut retry = false;
            let mut dismiss = false;
            egui::Panel::bottom("native-widget-clipboard-recovery")
                .resizable(false)
                .frame(
                    egui::Frame::new()
                        .fill(tokens.panel)
                        .inner_margin(egui::Margin::symmetric(16, 10)),
                )
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            l.text("The clipboard worker stopped. Captured content is preserved."),
                        );
                        retry = ui
                            .add_enabled(
                                !self.modal_open(),
                                egui::Button::new(l.text("Copy captured content"))
                                    .min_size(egui::vec2(0.0, 32.0)),
                            )
                            .clicked();
                        dismiss = ui
                            .add(
                                egui::Button::new(l.text("Dismiss captured content"))
                                    .min_size(egui::vec2(0.0, 32.0)),
                            )
                            .clicked();
                    });
                });
            if retry {
                self.clipboard_jobs.retry_native_output();
                self.status = WORKING.into();
            }
            if dismiss {
                self.clipboard_jobs.dismiss_native_output();
            }
        }
        let Some(recovery) = &self.clipboard_recovery else {
            return;
        };
        let Some(front) = recovery.items.front() else {
            self.clipboard_recovery = None;
            return;
        };
        let msgid = front.message;
        let count = recovery.items.len();
        let can_copy = front.scope.is_some() && self.clipboard.is_some() && !self.modal_open();
        let can_paste = !self.library
            && !self.modal_open()
            && self.rich.is_some()
            && self.can_edit_manuscript();
        let l = self.language;
        let t = theme::tokens(ui.ctx());
        let mut action = None;
        egui::Panel::bottom("clipboard-recovery")
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(t.panel)
                    .inner_margin(egui::Margin::symmetric(16, 10)),
            )
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(l.text(msgid));
                    if count > 1 {
                        ui.label(l.format(
                            "Captured clipboard items: {count}",
                            &[("count", count.to_string())],
                        ));
                    }
                    if ui
                        .add_enabled(
                            can_copy,
                            egui::Button::new(l.text("Copy captured content"))
                                .min_size(egui::vec2(0.0, 32.0)),
                        )
                        .clicked()
                    {
                        action = Some(0);
                    }
                    if ui
                        .add_enabled(
                            can_paste,
                            egui::Button::new(l.text("Paste captured content here"))
                                .min_size(egui::vec2(0.0, 32.0)),
                        )
                        .clicked()
                    {
                        action = Some(1);
                    }
                    if ui
                        .add(
                            egui::Button::new(l.text("Dismiss captured content"))
                                .min_size(egui::vec2(0.0, 32.0)),
                        )
                        .clicked()
                    {
                        action = Some(2);
                    }
                });
            });
        if let Some(action) = action {
            self.retry_captured_clipboard(action, ui.ctx());
        }
    }
    fn retry_captured_clipboard(&mut self, action: u8, ctx: &egui::Context) {
        let Some(item) = self
            .clipboard_recovery
            .as_mut()
            .and_then(|recovery| recovery.items.pop_front())
        else {
            return;
        };
        if action == 1 && !self.can_edit_manuscript() {
            self.retain_clipboard(item.bundle, item.scope, "Read only");
            return;
        }
        if action != 2 {
            let result = if action == 0 {
                self.clipboard_intents
                    .copy_request((*item.bundle).clone())
                    .map_err(craft_editor_egui::Error::from)
            } else if let Some(rich) = self.rich.as_ref() {
                rich.session.begin_paste((*item.bundle).clone())
            } else {
                Err(craft_editor_egui::Error::InvalidSelection)
            };
            match result {
                Ok(request) => {
                    let scope = if action == 0 {
                        item.scope
                    } else {
                        self.clipboard_scope().ok()
                    };
                    self.submit_clipboard_request(request, scope, ctx);
                }
                Err(error) => self.retain_clipboard(
                    item.bundle,
                    item.scope,
                    job_message(&JobError::Editor(error)),
                ),
            }
        }
        if self
            .clipboard_recovery
            .as_ref()
            .is_some_and(|recovery| recovery.items.is_empty())
        {
            self.clipboard_recovery = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_editor_egui::{DocPoint, Selection};
    use craft_editor_model::{Document as RichDocument, ScalarOffset};
    fn fixture() -> AppResult<(tempfile::TempDir, StudioApp, egui::Context)> {
        let directory = tempfile::tempdir()?;
        let context = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(context.clone());
        let mut app = StudioApp::new(&cc, directory.path().join("clipboard.sqlite"), false, None)?;
        let project = app.workspace.create_project("Clipboard")?;
        app.workspace.create_rich_document(
            &project.id,
            "Writing",
            RichDocument::plain("p", "ABCDE")?.to_json()?,
        )?;
        app.open_project(project)?;
        let rich = app.rich.as_mut().ok_or("missing session")?;
        rich.session
            .set_clipboard_intent_gate(app.clipboard_intents.clone())?;
        rich.session
            .set_clipboard_mode(craft_editor_egui::clipboard::ClipboardMode::Deferred)?;
        rich.session.set_selection(Selection {
            anchor: DocPoint {
                block_id: "p".into(),
                offset: ScalarOffset(1),
            },
            head: DocPoint {
                block_id: "p".into(),
                offset: ScalarOffset(4),
            },
        })?;
        Ok((directory, app, context))
    }
    #[test]
    fn deferred_widget_output_blocks_paste_until_the_exact_command_is_reemitted() -> AppResult<()> {
        let (_directory, mut app, context) = fixture()?;
        let input = egui::Id::new("deferred-native-input");
        context.memory_mut(|memory| memory.request_focus(input));
        app.clipboard_jobs
            .defer_native_output(egui::OutputCommand::CopyText("exact widget 😀".into()));
        let mut text = String::new();
        let mut frame = context.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Paste("WRONG OLD BUFFER".into())],
                focused: true,
                ..Default::default()
            },
            |root| {
                app.clipboard_frame_start(root.ctx());
                egui::CentralPanel::default().show(root, |ui| {
                    ui.add(egui::TextEdit::singleline(&mut text).id(input));
                });
                app.clipboard_frame_end(root.ctx());
            },
        );
        frame.textures_delta.clear();
        assert!(text.is_empty());
        assert!(frame.platform_output.commands.iter().any(|command|matches!(command,egui::OutputCommand::CopyText(text) if text=="exact widget 😀")));
        assert!(!app.clipboard_jobs.has_pending_native_output());
        assert_eq!(app.status, "Content copied");
        Ok(())
    }
    #[test]
    fn ordinary_widget_copy_keeps_native_output_path_and_latest_deferred_image_is_exact()
    -> AppResult<()> {
        let (_directory, mut app, context) = fixture()?;
        let mut ordinary = context.run_ui(egui::RawInput::default(), |root| {
            root.ctx().copy_text("ordinary exact".into());
            app.clipboard_frame_end(root.ctx());
        });
        ordinary.textures_delta.clear();
        assert_eq!(
            ordinary.platform_output.commands,
            vec![egui::OutputCommand::CopyText("ordinary exact".into())]
        );
        assert!(!app.clipboard_jobs.has_pending_native_output());
        app.clipboard_jobs
            .defer_native_output(egui::OutputCommand::CopyText("older intent".into()));
        let image =
            egui::ColorImage::from_rgba_unmultiplied([2, 1], &[255, 0, 0, 255, 0, 255, 0, 128]);
        let expected = image.clone();
        let mut latest = context.run_ui(egui::RawInput::default(), |root| {
            root.ctx().copy_image(image.clone());
            app.clipboard_frame_end(root.ctx());
        });
        latest.textures_delta.clear();
        assert_eq!(
            latest.platform_output.commands,
            vec![egui::OutputCommand::CopyImage(expected)]
        );
        assert!(!app.clipboard_jobs.has_pending_native_output());
        Ok(())
    }
    #[test]
    fn pending_rich_copy_refuses_dialog_paste_before_old_os_text_can_replace_its_input()
    -> AppResult<()> {
        let (_directory, mut app, context) = fixture()?;
        let captured = app
            .rich
            .as_ref()
            .ok_or("missing session")?
            .session
            .begin_copy()?
            .ok_or("missing copy")?;
        let input_id = egui::Id::new("native-dialog");
        context.memory_mut(|memory| memory.request_focus(input_id));
        let mut text = String::new();
        let mut frame = context.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Paste("WRONG OLD BUFFER".into())],
                focused: true,
                ..Default::default()
            },
            |root| {
                app.clipboard_frame_start(root.ctx());
                egui::CentralPanel::default().show(root, |ui| {
                    ui.add(egui::TextEdit::singleline(&mut text).id(input_id));
                });
            },
        );
        frame.textures_delta.clear();
        assert!(text.is_empty());
        assert_eq!(app.status, PENDING);
        assert!(app.error.is_none());
        assert_eq!(
            app.rich
                .as_ref()
                .ok_or("missing session")?
                .session
                .document()
                .block_text("p")?,
            "ABCDE"
        );
        drop(captured);
        assert!(!app.clipboard_intents.has_pending_write());
        Ok(())
    }
    #[test]
    fn cancelling_queued_cut_preserves_capture_and_source_without_worker_or_os_access()
    -> AppResult<()> {
        let (_directory, mut app, context) = fixture()?;
        let id = app
            .rich
            .as_ref()
            .ok_or("missing session")?
            .session
            .options
            .id;
        context.memory_mut(|memory| memory.request_focus(id));
        let mut frame = context.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Cut],
                focused: true,
                ..Default::default()
            },
            |root| {
                egui::CentralPanel::default().show(root, |ui| {
                    if let Some(rich) = &mut app.rich {
                        rich.session.show_canvas(ui, &Default::default());
                    }
                });
            },
        );
        frame.textures_delta.clear();
        app.cancel_clipboard_work();
        let rich = app.rich.as_ref().ok_or("missing session")?;
        assert_eq!(rich.session.document().block_text("p")?, "ABCDE");
        assert!(!rich.session.can_undo());
        let retained = app
            .clipboard_recovery
            .as_ref()
            .and_then(|recovery| recovery.items.front())
            .ok_or("missing preserved cut")?;
        assert_eq!(retained.bundle.plain, "BCD");
        assert!(!app.clipboard_intents.has_pending_write());
        assert!(app.error.is_none());
        Ok(())
    }
    #[test]
    fn private_failure_details_never_become_visible_status_or_modal_content() -> AppResult<()> {
        let (_directory, mut app, _context) = fixture()?;
        let bundle = Arc::new(ClipboardBundle {
            plain: "private source phrase".into(),
            ..Default::default()
        });
        let scope = app.clipboard_scope()?;
        let result = CompletionResult {
            intent: ClipboardIntent::Paste,
            scope,
            captured: bundle.clone(),
            outcome: ClipboardOutcome::Failed(JobError::Project(ProjectClipboardError::Assets(
                studio_core::StudioError::Validation("private path and source details".into()),
            ))),
        };
        app.finish_clipboard_result(result);
        assert!(!app.status.contains("private"));
        assert!(app.error.is_none());
        assert!(Arc::ptr_eq(
            &bundle,
            &app.clipboard_recovery
                .as_ref()
                .and_then(|recovery| recovery.items.front())
                .ok_or("missing recovery")?
                .bundle
        ));
        Ok(())
    }
    #[test]
    fn clipboard_recovery_and_error_messages_resolve_in_all_eight_catalogs() {
        let errors = [
            ClipboardError::Unavailable,
            ClipboardError::Read,
            ClipboardError::Write,
            ClipboardError::ResourceLimit,
            ClipboardError::InvalidSlice,
            ClipboardError::StaleRequest,
            ClipboardError::InvalidPasteContext,
            ClipboardError::InvalidAcknowledgement,
            ClipboardError::Busy,
            ClipboardError::WritePending,
        ];
        for language in Language::ALL {
            for error in errors {
                assert_ne!(language.text(message(&error)), "⟦missing translation⟧");
            }
            for key in [
                WORKING,
                PENDING,
                BUSY,
                STALE,
                CANCELLED,
                RECOVERY_FULL,
                "Content copied",
                "Clipboard image data is invalid. Copy the content again.",
                "Clipboard images need importing. Captured content is preserved.",
                "Clipboard images could not be prepared. Original content is preserved.",
                "Finish composing text before retrying the clipboard action.",
                "The clipboard action could not be applied. Captured content is preserved.",
                "The clipboard worker stopped. Captured content is preserved.",
                "Captured clipboard items: {count}",
                "Copy captured content",
                "Paste captured content here",
                "Dismiss captured content",
            ] {
                assert_ne!(
                    language.text(key),
                    "⟦missing translation⟧",
                    "{}: {key}",
                    language.code()
                );
            }
        }
    }
}

#[cfg(test)]
mod completion_save_tests {
    use super::*;
    use craft_editor_egui::clipboard::{ClipboardProvider, ClipboardResult};
    use craft_editor_egui::{DocPoint, Selection};
    use craft_editor_model::{Document as RichDocument, ScalarOffset};
    use std::sync::Mutex;
    #[derive(Default)]
    struct Provider {
        bytes: Mutex<ClipboardBundle>,
    }
    impl ClipboardProvider for Provider {
        fn read(&self) -> ClipboardResult<ClipboardBundle> {
            Err(ClipboardError::Read)
        }
        fn write(&self, bundle: &ClipboardBundle) -> ClipboardResult<()> {
            *self.bytes.lock().map_err(|_| ClipboardError::Write)? = bundle.clone();
            Ok(())
        }
        fn max_bytes(&self) -> usize {
            1_000_000
        }
    }
    fn complete_save_reopen_undo(cut: bool) -> AppResult<()> {
        let directory = tempfile::tempdir()?;
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = StudioApp::new(
            &cc,
            directory.path().join("clipboard-save.sqlite"),
            false,
            None,
        )?;
        let project = app.workspace.create_project("Clipboard save")?;
        let original = RichDocument::plain("p", "ABCDE")?;
        let document =
            app.workspace
                .create_rich_document(&project.id, "Writing", original.to_json()?)?;
        let anchor = app.workspace.capture_annotation_anchor(
            &document.id,
            document.revision,
            "p",
            ScalarOffset(4),
            ScalarOffset(5),
            "E",
        )?;
        let note = app.workspace.create_annotation(
            &document.id,
            document.revision,
            anchor,
            "Keep this tail note",
        )?;
        let provider = Arc::new(Provider::default());
        app.clipboard = Some(provider.clone());
        app.open_project(project)?;
        let rich = app.rich.as_mut().ok_or("missing session")?;
        rich.session.set_selection(Selection {
            anchor: DocPoint {
                block_id: "p".into(),
                offset: ScalarOffset(1),
            },
            head: DocPoint {
                block_id: "p".into(),
                offset: ScalarOffset(4),
            },
        })?;
        let request = if cut {
            rich.session.begin_cut()?.ok_or("missing cut")?
        } else {
            rich.session.begin_paste(ClipboardBundle {
                plain: "X".into(),
                ..Default::default()
            })?
        };
        app.submit_clipboard_request(request, Some(app.clipboard_scope()?), &ctx);
        app.clipboard_jobs.wait_ready()?;
        // No canvas or note decoration paint runs between worker completion and durable save.
        app.clipboard_frame_start(&ctx);
        assert!(app.dirty.is_some());
        let expected_text = if cut { "AE" } else { "AXE" };
        let expected_start = if cut { 1 } else { 2 };
        let expected = app
            .rich
            .as_ref()
            .ok_or("missing session")?
            .session
            .document()
            .to_json()?;
        app.save()?;
        let rich = app.rich.as_mut().ok_or("missing session")?;
        assert!(rich.session.take_pending_changes().is_empty());
        assert_eq!(rich.pending_proof_count(), 0);
        let reopened = Workspace::open(&app.database)?;
        let saved = reopened.document(&document.id)?;
        assert_eq!(saved.body, expected_text);
        assert_eq!(saved.rich_json, Some(expected));
        assert_eq!(saved.revision, document.revision + 1);
        let shifted = reopened.annotation(&note.id)?;
        assert_eq!(shifted.anchor.start, ScalarOffset(expected_start));
        assert_eq!(shifted.anchor.end, ScalarOffset(expected_start + 1));
        assert_eq!(shifted.anchor.exact_text, "E");
        assert_eq!(shifted.anchor_status, studio_core::AnchorStatus::Attached);
        assert!(reopened.rich_draft(&document.id)?.is_none());
        drop(reopened);
        rich.session.undo()?;
        assert!(!rich.session.can_undo());
        app.dirty = Some(Instant::now());
        app.save()?;
        let reopened = Workspace::open(&app.database)?;
        let restored = reopened.document(&document.id)?;
        assert_eq!(restored.rich_json, Some(original.to_json()?));
        assert_eq!(restored.body, "ABCDE");
        let restored_note = reopened.annotation(&note.id)?;
        assert_eq!(restored_note.anchor, note.anchor);
        assert_eq!(
            restored_note.anchor_status,
            studio_core::AnchorStatus::Attached
        );
        assert_eq!(restored_note.note, note.note);
        assert!(
            app.rich
                .as_mut()
                .ok_or("missing session")?
                .session
                .take_pending_changes()
                .is_empty()
        );
        assert_eq!(
            app.rich
                .as_ref()
                .ok_or("missing session")?
                .pending_proof_count(),
            0
        );
        Ok(())
    }
    #[test]
    fn worker_paste_save_before_canvas_paint_reopens_exact_content_and_maps_note_then_one_undo_restores()
    -> AppResult<()> {
        complete_save_reopen_undo(false)
    }
    #[test]
    fn acknowledged_worker_cut_save_before_canvas_paint_reopens_exact_content_and_maps_note_then_one_undo_restores()
    -> AppResult<()> {
        complete_save_reopen_undo(true)
    }
}
