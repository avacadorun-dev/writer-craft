//! One bounded background clipboard worker; native document mutation stays on the UI thread.
use std::fmt;
use std::sync::{
    Arc, Mutex, TryLockError,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc::{self, Receiver, SyncSender, TrySendError},
};

use super::project_clipboard::{ProjectClipboard, ProjectClipboardError};
use craft_editor_egui::{
    EditorSession,
    clipboard::{
        ClipboardBundle, ClipboardError, ClipboardProvider, ClipboardRequest, CutGuard, PasteGuard,
        PreparedPaste, WriteAck, WriteTicket,
    },
};
use craft_editor_model::Change;

const QUEUED_JOBS: usize = 2;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SubmitFailureKind {
    Busy,
    Closed,
    EpochExhausted,
}
/// Failed submission returns the exact one-shot request so the host can preserve or retry it.
pub(super) struct SubmitFailure {
    pub kind: SubmitFailureKind,
    pub request: ClipboardRequest,
}
impl fmt::Debug for SubmitFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubmitFailure")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}
#[derive(Debug)]
pub(super) enum JobError {
    Project(ProjectClipboardError),
    Transport(ClipboardError),
    Editor(craft_editor_egui::Error),
}
impl fmt::Display for JobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Project(error) => error.fmt(f),
            Self::Transport(error) => error.fmt(f),
            Self::Editor(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for JobError {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ClipboardIntent {
    Copy,
    Cut,
    Paste,
}
pub(super) fn request_intent(request: &ClipboardRequest) -> ClipboardIntent {
    match request {
        ClipboardRequest::Copy { .. } => ClipboardIntent::Copy,
        ClipboardRequest::Cut { .. } => ClipboardIntent::Cut,
        ClipboardRequest::Paste { .. } => ClipboardIntent::Paste,
    }
}
pub(super) fn captured_bundle(request: &ClipboardRequest) -> Arc<ClipboardBundle> {
    match request {
        ClipboardRequest::Copy { bundle, .. }
        | ClipboardRequest::Cut { bundle, .. }
        | ClipboardRequest::Paste { bundle, .. } => bundle.clone(),
    }
}
/// UI-owned result. Failed/cancelled jobs retain the exact original captured bytes for recovery.
pub(super) struct CompletionResult {
    pub intent: ClipboardIntent,
    pub scope: ProjectClipboard,
    pub captured: Arc<ClipboardBundle>,
    pub outcome: ClipboardOutcome,
}
pub(super) enum ClipboardOutcome {
    Copied,
    Cut(Change),
    Pasted(Change),
    Cancelled,
    Failed(JobError),
}
pub(super) struct ClipboardCompletion {
    intent: ClipboardIntent,
    scope: ProjectClipboard,
    captured: Arc<ClipboardBundle>,
    cancel_epoch: u64,
    write_epoch: Option<u64>,
    prepared: Prepared,
}
enum Prepared {
    Copied,
    Cut(CutGuard, WriteAck),
    Paste(PasteGuard, PreparedPaste),
    Cancelled,
    Failed(JobError),
}
#[derive(Default)]
struct Epochs {
    writes: AtomicU64,
    cancel: AtomicU64,
    // Only the final cancellation/start handshake is locked. OS and project I/O stay outside.
    publication_fence: Mutex<()>,
    active_write: AtomicBool,
}
struct ActiveWrite<'a>(&'a AtomicBool);
impl Drop for ActiveWrite<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
impl Epochs {
    fn begin_write(
        &self,
        cancel: u64,
        write: Option<u64>,
    ) -> std::result::Result<Option<ActiveWrite<'_>>, ClipboardError> {
        let _fence = self
            .publication_fence
            .lock()
            .map_err(|_| ClipboardError::Unavailable)?;
        if !current(self, cancel, write) {
            return Ok(None);
        }
        self.active_write.store(true, Ordering::Release);
        Ok(Some(ActiveWrite(&self.active_write)))
    }
}
struct NativeOutput {
    command: egui::OutputCommand,
    epoch: Option<u64>,
    error: Option<SubmitFailureKind>,
}
struct Job {
    request: ClipboardRequest,
    scope: ProjectClipboard,
    provider: Arc<dyn ClipboardProvider>,
    cancel_epoch: u64,
    write_epoch: Option<u64>,
    context: egui::Context,
}
/// Application-lifetime serial worker. Drop closes channels without joining/blocking the UI.
pub(super) struct ClipboardJobs {
    sender: SyncSender<Job>,
    receiver: Receiver<ClipboardCompletion>,
    #[cfg(test)]
    ready: std::collections::VecDeque<ClipboardCompletion>,
    epochs: Arc<Epochs>,
    native_output: Option<NativeOutput>,
}
impl ClipboardJobs {
    pub fn new() -> std::io::Result<Self> {
        let (sender, requests) = mpsc::sync_channel::<Job>(QUEUED_JOBS);
        let (results, receiver) = mpsc::sync_channel(QUEUED_JOBS);
        let epochs = Arc::new(Epochs::default());
        let worker_epochs = epochs.clone();
        std::thread::Builder::new()
            .name("writercraft-clipboard".into())
            .spawn(move || {
                while let Ok(job) = requests.recv() {
                    let context = job.context.clone();
                    let completion = execute(job, &worker_epochs);
                    if results.send(completion).is_err() {
                        break;
                    }
                    context.request_repaint();
                }
            })?;
        Ok(Self {
            sender,
            receiver,
            #[cfg(test)]
            ready: std::collections::VecDeque::new(),
            epochs,
            native_output: None,
        })
    }
    /// A newer rich Copy/Cut supersedes older pending writes. The host must also call
    /// cancel_pending before another widget's native Copy/Cut intent or project/window close.
    /// Paste is ordered and guarded independently; no paste intent is silently coalesced.
    pub fn submit(
        &self,
        request: ClipboardRequest,
        scope: ProjectClipboard,
        provider: Arc<dyn ClipboardProvider>,
        context: &egui::Context,
    ) -> std::result::Result<(), Box<SubmitFailure>> {
        let write_epoch = if matches!(
            &request,
            ClipboardRequest::Copy { .. } | ClipboardRequest::Cut { .. }
        ) {
            match advance(&self.epochs.writes) {
                Some(epoch) => Some(epoch),
                None => {
                    return Err(Box::new(SubmitFailure {
                        kind: SubmitFailureKind::EpochExhausted,
                        request,
                    }));
                }
            }
        } else {
            None
        };
        let job = Job {
            request,
            scope,
            provider,
            context: context.clone(),
            write_epoch,
            cancel_epoch: self.epochs.cancel.load(Ordering::Acquire),
        };
        self.sender.try_send(job).map_err(|error| {
            Box::new(match error {
                TrySendError::Full(job) => SubmitFailure {
                    kind: SubmitFailureKind::Busy,
                    request: job.request,
                },
                TrySendError::Disconnected(job) => SubmitFailure {
                    kind: SubmitFailureKind::Closed,
                    request: job.request,
                },
            })
        })
    }
    /// Cancel queued work before its native write when possible, and invalidate all completed
    /// Cut/Paste jobs even when their exact document revision/selection has not otherwise changed.
    pub fn cancel_pending(&self) -> std::result::Result<(), SubmitFailureKind> {
        advance(&self.epochs.cancel).ok_or(SubmitFailureKind::EpochExhausted)?;
        // Independent widget Copy is immutable and has no document destination. Modal/project
        // cancellation must not repeatedly discard it. Only a newer Copy/Cut intent supersedes it.
        Ok(())
    }
    /// Retain the exact native widget command until older rich writes have actually returned.
    /// This does not write or acknowledge the clipboard. Newer Copy intents replace older ones.
    pub fn defer_native_output(&mut self, command: egui::OutputCommand) {
        self.native_output = Some(NativeOutput {
            command,
            epoch: None,
            error: None,
        });
        self.progress_native_output();
    }
    /// Try the small publication handshake; never wait on worker or OS I/O.
    pub fn progress_native_output(&mut self) {
        let Some(pending) = self.native_output.as_mut() else {
            return;
        };
        if pending.error.is_some() {
            return;
        }
        if let Some(epoch) = pending.epoch {
            if self.epochs.writes.load(Ordering::Acquire) != epoch {
                self.native_output = None;
            }
            return;
        }
        let _fence = match self.epochs.publication_fence.try_lock() {
            Ok(fence) => fence,
            Err(TryLockError::WouldBlock) => return,
            Err(TryLockError::Poisoned(_)) => {
                pending.error = Some(SubmitFailureKind::Closed);
                return;
            }
        };
        let epoch = advance(&self.epochs.cancel).and_then(|_| advance(&self.epochs.writes));
        match epoch {
            Some(epoch) => pending.epoch = Some(epoch),
            None => pending.error = Some(SubmitFailureKind::EpochExhausted),
        }
    }
    pub fn has_pending_native_output(&self) -> bool {
        self.native_output.is_some()
    }
    pub fn native_output_error(&self) -> Option<SubmitFailureKind> {
        self.native_output
            .as_ref()
            .and_then(|pending| pending.error)
    }
    pub fn retry_native_output(&mut self) {
        if let Some(pending) = &mut self.native_output {
            pending.error = None;
        }
        self.progress_native_output();
    }
    pub fn dismiss_native_output(&mut self) {
        self.native_output = None;
    }
    /// Return an output only after the serial worker's prior write ended and no newer Copy won.
    /// The host must actually re-emit this value; taking it is not a native write ACK.
    pub fn take_ready_native_output(&mut self) -> Option<egui::OutputCommand> {
        self.progress_native_output();
        let pending = self.native_output.take()?;
        if pending
            .epoch
            .is_some_and(|epoch| self.epochs.writes.load(Ordering::Acquire) != epoch)
        {
            return None;
        }
        if pending.error.is_none()
            && pending.epoch.is_some()
            && !self.epochs.active_write.load(Ordering::Acquire)
        {
            Some(pending.command)
        } else {
            self.native_output = Some(pending);
            None
        }
    }
    pub fn take_completed(&mut self) -> Vec<ClipboardCompletion> {
        #[cfg(test)]
        {
            let mut ready: Vec<_> = self.ready.drain(..).collect();
            ready.extend(self.receiver.try_iter());
            ready
        }
        #[cfg(not(test))]
        {
            self.receiver.try_iter().collect()
        }
    }
    #[cfg(test)]
    pub(super) fn wait_ready(&mut self) -> std::result::Result<(), mpsc::RecvTimeoutError> {
        let completed = self
            .receiver
            .recv_timeout(std::time::Duration::from_secs(10))?;
        self.ready.push_back(completed);
        Ok(())
    }
    /// Apply only on the UI thread through the live session's one-shot guard. An absent session,
    /// superseding Copy/Cut intent or cancellation cannot delete or retarget writing.
    pub fn complete(
        &self,
        completion: ClipboardCompletion,
        session: Option<&mut EditorSession>,
    ) -> CompletionResult {
        let current = current(
            &self.epochs,
            completion.cancel_epoch,
            completion.write_epoch,
        );
        let outcome = if !current {
            ClipboardOutcome::Cancelled
        } else {
            match completion.prepared {
                Prepared::Copied => ClipboardOutcome::Copied,
                Prepared::Cancelled => ClipboardOutcome::Cancelled,
                Prepared::Failed(error) => ClipboardOutcome::Failed(error),
                Prepared::Cut(guard, ack) => match session {
                    Some(session) => match session.finish_cut(guard, ack) {
                        Ok(change) => ClipboardOutcome::Cut(change),
                        Err(error) => ClipboardOutcome::Failed(JobError::Editor(error)),
                    },
                    None => ClipboardOutcome::Cancelled,
                },
                Prepared::Paste(guard, prepared) => match session {
                    Some(session) => match session.finish_prepared_paste(guard, prepared) {
                        Ok(change) => ClipboardOutcome::Pasted(change),
                        Err(error) => ClipboardOutcome::Failed(JobError::Editor(error)),
                    },
                    None => ClipboardOutcome::Cancelled,
                },
            }
        };
        CompletionResult {
            intent: completion.intent,
            scope: completion.scope,
            captured: completion.captured,
            outcome,
        }
    }
}
fn advance(epoch: &AtomicU64) -> Option<u64> {
    epoch
        .try_update(Ordering::AcqRel, Ordering::Acquire, |value| {
            value.checked_add(1)
        })
        .ok()?
        .checked_add(1)
}
fn current(epochs: &Epochs, cancel: u64, write: Option<u64>) -> bool {
    epochs.cancel.load(Ordering::Acquire) == cancel
        && write.is_none_or(|write| epochs.writes.load(Ordering::Acquire) == write)
}
fn publish(
    ticket: WriteTicket,
    provider: &dyn ClipboardProvider,
    bundle: &ClipboardBundle,
    epochs: &Epochs,
    cancel: u64,
    write: Option<u64>,
) -> std::result::Result<Option<WriteAck>, ClipboardError> {
    let Some(_activity) = epochs.begin_write(cancel, write)? else {
        return Ok(None);
    };
    ticket.write(provider, bundle).map(Some)
}
fn execute(job: Job, epochs: &Epochs) -> ClipboardCompletion {
    let captured = captured_bundle(&job.request);
    let intent = request_intent(&job.request);
    let scope = job.scope.clone();
    let prepared = if !current(epochs, job.cancel_epoch, job.write_epoch) {
        Prepared::Cancelled
    } else {
        match job.request {
            ClipboardRequest::Copy { bundle, ticket } => match job.scope.prepare_copy(&bundle) {
                Ok(outbound) if current(epochs, job.cancel_epoch, job.write_epoch) => {
                    match publish(
                        ticket,
                        job.provider.as_ref(),
                        &outbound,
                        epochs,
                        job.cancel_epoch,
                        job.write_epoch,
                    ) {
                        Ok(Some(_)) => Prepared::Copied,
                        Ok(None) => Prepared::Cancelled,
                        Err(error) => Prepared::Failed(JobError::Transport(error)),
                    }
                }
                Ok(_) => Prepared::Cancelled,
                Err(error) => Prepared::Failed(JobError::Project(error)),
            },
            ClipboardRequest::Cut {
                bundle,
                ticket,
                guard,
            } => match job.scope.prepare_copy(&bundle) {
                Ok(outbound) if current(epochs, job.cancel_epoch, job.write_epoch) => {
                    match publish(
                        ticket,
                        job.provider.as_ref(),
                        &outbound,
                        epochs,
                        job.cancel_epoch,
                        job.write_epoch,
                    ) {
                        Ok(Some(ack)) => Prepared::Cut(guard, ack),
                        Ok(None) => Prepared::Cancelled,
                        Err(error) => Prepared::Failed(JobError::Transport(error)),
                    }
                }
                Ok(_) => Prepared::Cancelled,
                Err(error) => Prepared::Failed(JobError::Project(error)),
            },
            ClipboardRequest::Paste { context, guard, .. } => {
                match job.scope.prepare_paste(&context) {
                    Ok(prepared) if current(epochs, job.cancel_epoch, job.write_epoch) => {
                        Prepared::Paste(guard, prepared)
                    }
                    Ok(_) => Prepared::Cancelled,
                    Err(error) => Prepared::Failed(JobError::Project(error)),
                }
            }
        }
    };
    ClipboardCompletion {
        intent,
        scope,
        captured,
        cancel_epoch: job.cancel_epoch,
        write_epoch: job.write_epoch,
        prepared,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_editor_egui::clipboard::ClipboardResult;
    use craft_editor_egui::{DocPoint, Selection};
    use craft_editor_model::{Document, Revision, ScalarOffset};
    use std::sync::{Condvar, atomic::AtomicUsize};
    use std::time::Duration;
    use studio_core::Workspace;
    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
    const LIMIT: usize = 1_000_000;
    fn editor() -> std::result::Result<EditorSession, craft_editor_egui::Error> {
        let mut editor = EditorSession::new(Document::plain("p", "ABCDE")?, Revision(0));
        editor.set_selection(Selection {
            anchor: DocPoint {
                block_id: "p".into(),
                offset: ScalarOffset(1),
            },
            head: DocPoint {
                block_id: "p".into(),
                offset: ScalarOffset(4),
            },
        })?;
        Ok(editor)
    }
    fn scope() -> std::result::Result<ProjectClipboard, studio_core::StudioError> {
        let workspace = Workspace::memory()?;
        let project = workspace.create_project("Synthetic clipboard")?;
        Ok(ProjectClipboard::new(
            workspace.project_images(&project.id)?,
            LIMIT,
        ))
    }
    #[derive(Default)]
    struct Provider {
        bundle: Mutex<ClipboardBundle>,
        fail: bool,
        writes: AtomicUsize,
    }
    impl ClipboardProvider for Provider {
        fn read(&self) -> ClipboardResult<ClipboardBundle> {
            Err(ClipboardError::Read)
        }
        fn write(&self, bundle: &ClipboardBundle) -> ClipboardResult<()> {
            self.writes.fetch_add(1, Ordering::Relaxed);
            if self.fail {
                return Err(ClipboardError::Write);
            }
            *self.bundle.lock().map_err(|_| ClipboardError::Write)? = bundle.clone();
            Ok(())
        }
        fn max_bytes(&self) -> usize {
            LIMIT
        }
    }
    fn completed(
        jobs: &ClipboardJobs,
    ) -> std::result::Result<ClipboardCompletion, mpsc::RecvTimeoutError> {
        jobs.receiver.recv_timeout(Duration::from_secs(10))
    }
    #[test]
    fn failed_native_write_preserves_cut_document_and_retains_exact_captured_data() -> TestResult {
        let jobs = ClipboardJobs::new()?;
        let mut session = editor()?;
        let original = session.document().clone();
        let request = session.begin_cut()?.ok_or("missing cut")?;
        jobs.submit(
            request,
            scope()?,
            Arc::new(Provider {
                fail: true,
                ..Default::default()
            }),
            &egui::Context::default(),
        )
        .map_err(|_| "submit failed")?;
        let result = jobs.complete(completed(&jobs)?, Some(&mut session));
        assert!(matches!(
            result.outcome,
            ClipboardOutcome::Failed(JobError::Transport(ClipboardError::Write))
        ));
        assert_eq!(result.captured.plain, "BCD");
        assert_eq!(session.document(), &original);
        assert!(!session.can_undo());
        Ok(())
    }
    #[test]
    fn worker_prepares_marker_rules_before_ui_completion_with_exact_raw_capture_and_one_undo()
    -> TestResult {
        let jobs = ClipboardJobs::new()?;
        let mut session = editor()?;
        let original = session.document().clone();
        jobs.submit(
            session.begin_paste(ClipboardBundle {
                plain: "**bold**".into(),
                ..Default::default()
            })?,
            scope()?,
            Arc::new(Provider::default()),
            &egui::Context::default(),
        )
        .map_err(|_| "submit failed")?;
        let completion = completed(&jobs)?;
        let Prepared::Paste(_, prepared) = &completion.prepared else {
            return Err("worker did not prepare paste".into());
        };
        assert_eq!(prepared.source().plain, "**bold**");
        assert_eq!(
            prepared
                .rich_slice()
                .ok_or("missing prepared canonical")?
                .content[0]
                .inline_text(),
            "bold"
        );
        assert_eq!(session.document(), &original);
        let result = jobs.complete(completion, Some(&mut session));
        assert!(matches!(result.outcome, ClipboardOutcome::Pasted(_)));
        assert_eq!(session.document().block_text("p")?, "AboldE");
        session.undo()?;
        assert_eq!(session.document(), &original);
        assert!(!session.can_undo());
        Ok(())
    }
    #[test]
    fn two_paste_intents_are_returned_in_order_with_explicit_stale_failure_not_silent_loss()
    -> TestResult {
        let jobs = ClipboardJobs::new()?;
        let mut session = editor()?;
        for text in ["first", "second"] {
            let request = session.begin_paste(ClipboardBundle {
                plain: text.into(),
                ..Default::default()
            })?;
            jobs.submit(
                request,
                scope()?,
                Arc::new(Provider::default()),
                &egui::Context::default(),
            )
            .map_err(|_| "submit failed")?;
        }
        let first = jobs.complete(completed(&jobs)?, Some(&mut session));
        assert!(matches!(first.outcome, ClipboardOutcome::Pasted(_)));
        let after_first = session.document().clone();
        let second = jobs.complete(completed(&jobs)?, Some(&mut session));
        assert!(matches!(
            second.outcome,
            ClipboardOutcome::Failed(JobError::Editor(craft_editor_egui::Error::Clipboard(
                ClipboardError::StaleRequest
            )))
        ));
        assert_eq!(second.captured.plain, "second");
        assert_eq!(session.document(), &after_first);
        assert_eq!(session.document().block_text("p")?, "AfirstE");
        Ok(())
    }
    #[test]
    fn cancellation_invalidates_a_successfully_acknowledged_cut_before_document_completion()
    -> TestResult {
        let jobs = ClipboardJobs::new()?;
        let mut session = editor()?;
        let original = session.document().clone();
        jobs.submit(
            session.begin_cut()?.ok_or("missing cut")?,
            scope()?,
            Arc::new(Provider::default()),
            &egui::Context::default(),
        )
        .map_err(|_| "submit failed")?;
        let completion = completed(&jobs)?;
        jobs.cancel_pending().map_err(|_| "cancel failed")?;
        let result = jobs.complete(completion, Some(&mut session));
        assert!(matches!(result.outcome, ClipboardOutcome::Cancelled));
        assert_eq!(session.document(), &original);
        assert_eq!(result.captured.plain, "BCD");
        Ok(())
    }
    struct BlockingProvider {
        inner: Provider,
        entered: SyncSender<()>,
        released: Mutex<bool>,
        ready: Condvar,
    }
    impl ClipboardProvider for BlockingProvider {
        fn read(&self) -> ClipboardResult<ClipboardBundle> {
            self.inner.read()
        }
        fn write(&self, bundle: &ClipboardBundle) -> ClipboardResult<()> {
            if self.inner.writes.load(Ordering::Relaxed) == 0 {
                self.entered.send(()).map_err(|_| ClipboardError::Write)?;
                let mut released = self.released.lock().map_err(|_| ClipboardError::Write)?;
                while !*released {
                    released = self
                        .ready
                        .wait(released)
                        .map_err(|_| ClipboardError::Write)?;
                }
            }
            self.inner.write(bundle)
        }
        fn max_bytes(&self) -> usize {
            LIMIT
        }
    }
    #[test]
    fn rapid_newer_copy_wins_native_write_order_and_obsolete_cut_cannot_delete() -> TestResult {
        let jobs = ClipboardJobs::new()?;
        let mut session = editor()?;
        let original = session.document().clone();
        let (entered, waiting) = mpsc::sync_channel(1);
        let provider = Arc::new(BlockingProvider {
            inner: Provider::default(),
            entered,
            released: Mutex::new(false),
            ready: Condvar::new(),
        });
        jobs.submit(
            session.begin_cut()?.ok_or("missing cut")?,
            scope()?,
            provider.clone(),
            &egui::Context::default(),
        )
        .map_err(|_| "submit failed")?;
        waiting.recv_timeout(Duration::from_secs(10))?;
        let mut newer = editor()?;
        newer.insert_plain_text("NEW")?;
        newer.set_selection(Selection {
            anchor: DocPoint {
                block_id: "p".into(),
                offset: ScalarOffset(0),
            },
            head: DocPoint {
                block_id: "p".into(),
                offset: ScalarOffset(5),
            },
        })?;
        jobs.submit(
            newer.begin_copy()?.ok_or("missing copy")?,
            scope()?,
            provider.clone(),
            &egui::Context::default(),
        )
        .map_err(|_| "submit failed")?;
        *provider.released.lock().map_err(|_| "mock lock")? = true;
        provider.ready.notify_all();
        let cut = jobs.complete(completed(&jobs)?, Some(&mut session));
        let copy = jobs.complete(completed(&jobs)?, Some(&mut session));
        assert!(matches!(cut.outcome, ClipboardOutcome::Cancelled));
        assert!(matches!(copy.outcome, ClipboardOutcome::Copied));
        assert_eq!(session.document(), &original);
        assert_eq!(
            provider.inner.bundle.lock().map_err(|_| "mock lock")?.plain,
            "ANEWE"
        );
        Ok(())
    }
    fn blocked_write(
        jobs: &ClipboardJobs,
        session: &EditorSession,
        fail: bool,
    ) -> TestResultWith<(Arc<BlockingProvider>, Receiver<()>)> {
        let (entered, waiting) = mpsc::sync_channel(1);
        let provider = Arc::new(BlockingProvider {
            inner: Provider {
                fail,
                ..Default::default()
            },
            entered,
            released: Mutex::new(false),
            ready: Condvar::new(),
        });
        jobs.submit(
            session.begin_cut()?.ok_or("missing cut")?,
            scope()?,
            provider.clone(),
            &egui::Context::default(),
        )
        .map_err(|_| "submit failed")?;
        Ok((provider, waiting))
    }
    type TestResultWith<T> = std::result::Result<T, Box<dyn std::error::Error>>;
    #[test]
    fn widget_text_output_waits_for_actual_old_write_and_document_cancellation_does_not_cancel_it()
    -> TestResult {
        let mut jobs = ClipboardJobs::new()?;
        let mut session = editor()?;
        let original = session.document().clone();
        let (provider, entered) = blocked_write(&jobs, &session, false)?;
        entered.recv_timeout(Duration::from_secs(10))?;
        jobs.defer_native_output(egui::OutputCommand::CopyText("dialog exact 😀".into()));
        assert!(jobs.take_ready_native_output().is_none());
        for _ in 0..3 {
            jobs.cancel_pending().map_err(|_| "cancel failed")?;
        }
        assert!(jobs.has_pending_native_output());
        *provider.released.lock().map_err(|_| "mock lock")? = true;
        provider.ready.notify_all();
        let result = jobs.complete(completed(&jobs)?, Some(&mut session));
        assert!(matches!(result.outcome, ClipboardOutcome::Cancelled));
        let Some(egui::OutputCommand::CopyText(text)) = jobs.take_ready_native_output() else {
            return Err("widget command lost".into());
        };
        assert_eq!(text, "dialog exact 😀");
        assert_eq!(session.document(), &original);
        assert!(!session.can_undo());
        // Returning/re-emitting a widget command is not an OS write acknowledgement.
        assert_eq!(
            provider.inner.bundle.lock().map_err(|_| "mock lock")?.plain,
            "BCD"
        );
        assert!(!jobs.has_pending_native_output());
        assert!(jobs.take_ready_native_output().is_none());
        Ok(())
    }
    #[test]
    fn rapid_widget_image_copy_keeps_exact_latest_pixels_and_old_write_failure_does_not_clear_clipboard()
    -> TestResult {
        let mut jobs = ClipboardJobs::new()?;
        let session = editor()?;
        let (provider, entered) = blocked_write(&jobs, &session, true)?;
        entered.recv_timeout(Duration::from_secs(10))?;
        provider.inner.bundle.lock().map_err(|_| "mock lock")?.plain =
            "previous owned clipboard".into();
        let first = egui::ColorImage::from_rgba_unmultiplied([1, 1], &[255, 0, 0, 255]);
        let latest =
            egui::ColorImage::from_rgba_unmultiplied([2, 1], &[0, 255, 0, 255, 0, 0, 255, 128]);
        let pixels = latest.pixels.clone();
        jobs.defer_native_output(egui::OutputCommand::CopyImage(first));
        jobs.defer_native_output(egui::OutputCommand::CopyImage(latest));
        assert!(jobs.take_ready_native_output().is_none());
        *provider.released.lock().map_err(|_| "mock lock")? = true;
        provider.ready.notify_all();
        let _completion = completed(&jobs)?;
        let Some(egui::OutputCommand::CopyImage(image)) = jobs.take_ready_native_output() else {
            return Err("latest image lost".into());
        };
        assert_eq!(image.size, [2, 1]);
        assert_eq!(image.pixels, pixels);
        assert_eq!(
            provider.inner.bundle.lock().map_err(|_| "mock lock")?.plain,
            "previous owned clipboard"
        );
        assert!(jobs.take_ready_native_output().is_none());
        Ok(())
    }
    #[test]
    fn newer_rich_copy_supersedes_waiting_widget_output_in_publication_order() -> TestResult {
        let mut jobs = ClipboardJobs::new()?;
        let session = editor()?;
        let (provider, entered) = blocked_write(&jobs, &session, false)?;
        entered.recv_timeout(Duration::from_secs(10))?;
        jobs.defer_native_output(egui::OutputCommand::CopyText("older widget".into()));
        let mut newer = editor()?;
        newer.insert_plain_text("fresh")?;
        newer.set_selection(Selection {
            anchor: DocPoint {
                block_id: "p".into(),
                offset: ScalarOffset(0),
            },
            head: DocPoint {
                block_id: "p".into(),
                offset: ScalarOffset(7),
            },
        })?;
        jobs.submit(
            newer.begin_copy()?.ok_or("missing copy")?,
            scope()?,
            provider.clone(),
            &egui::Context::default(),
        )
        .map_err(|_| "submit failed")?;
        assert!(jobs.take_ready_native_output().is_none());
        *provider.released.lock().map_err(|_| "mock lock")? = true;
        provider.ready.notify_all();
        let _old = completed(&jobs)?;
        let current = jobs.complete(completed(&jobs)?, None);
        assert!(matches!(current.outcome, ClipboardOutcome::Copied));
        assert!(jobs.take_ready_native_output().is_none());
        assert_eq!(
            provider.inner.bundle.lock().map_err(|_| "mock lock")?.plain,
            "AfreshE"
        );
        assert_eq!(provider.inner.writes.load(Ordering::Relaxed), 2);
        Ok(())
    }
    #[test]
    fn busy_publication_handshake_is_nonblocking_and_retains_widget_output_for_next_frame()
    -> TestResult {
        let mut jobs = ClipboardJobs::new()?;
        let epochs = jobs.epochs.clone();
        let fence = epochs
            .publication_fence
            .lock()
            .map_err(|_| "fence poisoned")?;
        jobs.defer_native_output(egui::OutputCommand::CopyText("keep exact".into()));
        assert!(jobs.has_pending_native_output());
        assert!(jobs.take_ready_native_output().is_none());
        drop(fence);
        let Some(egui::OutputCommand::CopyText(text)) = jobs.take_ready_native_output() else {
            return Err("capture lost after retry".into());
        };
        assert_eq!(text, "keep exact");
        Ok(())
    }
    #[test]
    fn publication_error_retains_exact_widget_command_until_retry_or_explicit_dismiss() -> TestResult
    {
        let mut jobs = ClipboardJobs::new()?;
        jobs.epochs.cancel.store(u64::MAX, Ordering::Release);
        jobs.defer_native_output(egui::OutputCommand::CopyText("recover exact".into()));
        assert_eq!(
            jobs.native_output_error(),
            Some(SubmitFailureKind::EpochExhausted)
        );
        assert!(jobs.has_pending_native_output());
        assert!(jobs.take_ready_native_output().is_none());
        jobs.epochs.cancel.store(0, Ordering::Release);
        jobs.retry_native_output();
        let Some(egui::OutputCommand::CopyText(text)) = jobs.take_ready_native_output() else {
            return Err("retry lost capture".into());
        };
        assert_eq!(text, "recover exact");
        jobs.defer_native_output(egui::OutputCommand::CopyText(
            "dismiss only explicitly".into(),
        ));
        jobs.dismiss_native_output();
        assert!(!jobs.has_pending_native_output());
        Ok(())
    }
    #[test]
    fn full_queue_returns_exact_request_for_recovery_instead_of_dropping_it() -> TestResult {
        let jobs = ClipboardJobs::new()?;
        let session = editor()?;
        // These paste snapshots were legitimately captured before the later Copy intent.
        // A new Paste while Copy is pending is separately refused by ClipboardIntentGate.
        let mut earlier = Vec::new();
        for text in ["queued", "queued", "retain me"] {
            earlier.push(session.begin_paste(ClipboardBundle {
                plain: text.into(),
                ..Default::default()
            })?);
        }
        let retained = earlier.pop().ok_or("missing retained paste")?;
        let (entered, waiting) = mpsc::sync_channel(1);
        let provider = Arc::new(BlockingProvider {
            inner: Provider::default(),
            entered,
            released: Mutex::new(false),
            ready: Condvar::new(),
        });
        jobs.submit(
            session.begin_copy()?.ok_or("missing copy")?,
            scope()?,
            provider.clone(),
            &egui::Context::default(),
        )
        .map_err(|_| "submit failed")?;
        waiting.recv_timeout(Duration::from_secs(10))?;
        for request in earlier {
            jobs.submit(
                request,
                scope()?,
                provider.clone(),
                &egui::Context::default(),
            )
            .map_err(|_| "submit failed")?;
        }
        let result = jobs.submit(
            retained,
            scope()?,
            provider.clone(),
            &egui::Context::default(),
        );
        *provider.released.lock().map_err(|_| "mock lock")? = true;
        provider.ready.notify_all();
        let Err(failure) = result else {
            return Err("queue did not preserve paste".into());
        };
        let SubmitFailure {
            kind: SubmitFailureKind::Busy,
            request: ClipboardRequest::Paste { bundle, .. },
        } = *failure
        else {
            return Err("queue returned another failure".into());
        };
        assert_eq!(bundle.plain, "retain me");
        Ok(())
    }
}
