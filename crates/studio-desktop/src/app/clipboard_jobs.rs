//! One bounded background clipboard worker; native document mutation stays on the UI thread.
use std::fmt;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
    mpsc::{self, Receiver, SyncSender, TrySendError},
};

use super::project_clipboard::{ProjectClipboard, ProjectClipboardError};
use craft_editor_egui::{
    EditorSession,
    clipboard::{
        ClipboardBundle, ClipboardError, ClipboardProvider, ClipboardRequest, CutGuard, PasteGuard,
        WriteAck,
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
/// UI-owned result. Failed/cancelled jobs retain the exact original captured bytes for recovery.
pub(super) struct CompletionResult {
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
    captured: Arc<ClipboardBundle>,
    cancel_epoch: u64,
    write_epoch: Option<u64>,
    prepared: Prepared,
}
enum Prepared {
    Copied,
    Cut(CutGuard, WriteAck),
    Paste(PasteGuard, ClipboardBundle, usize),
    Cancelled,
    Failed(JobError),
}
#[derive(Default)]
struct Epochs {
    writes: AtomicU64,
    cancel: AtomicU64,
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
    epochs: Arc<Epochs>,
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
            epochs,
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
    ) -> std::result::Result<(), SubmitFailure> {
        let write_epoch = if matches!(
            &request,
            ClipboardRequest::Copy { .. } | ClipboardRequest::Cut { .. }
        ) {
            match advance(&self.epochs.writes) {
                Some(epoch) => Some(epoch),
                None => {
                    return Err(SubmitFailure {
                        kind: SubmitFailureKind::EpochExhausted,
                        request,
                    });
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
        self.sender.try_send(job).map_err(|error| match error {
            TrySendError::Full(job) => SubmitFailure {
                kind: SubmitFailureKind::Busy,
                request: job.request,
            },
            TrySendError::Disconnected(job) => SubmitFailure {
                kind: SubmitFailureKind::Closed,
                request: job.request,
            },
        })
    }
    /// Cancel queued work before its native write when possible, and invalidate all completed
    /// Cut/Paste jobs even when their exact document revision/selection has not otherwise changed.
    pub fn cancel_pending(&self) -> std::result::Result<(), SubmitFailureKind> {
        advance(&self.epochs.cancel).ok_or(SubmitFailureKind::EpochExhausted)?;
        advance(&self.epochs.writes).ok_or(SubmitFailureKind::EpochExhausted)?;
        Ok(())
    }
    pub fn take_completed(&self) -> Vec<ClipboardCompletion> {
        self.receiver.try_iter().collect()
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
                Prepared::Paste(guard, bundle, max_bytes) => match session {
                    Some(session) => match session.finish_paste(guard, &bundle, max_bytes) {
                        Ok(change) => ClipboardOutcome::Pasted(change),
                        Err(error) => ClipboardOutcome::Failed(JobError::Editor(error)),
                    },
                    None => ClipboardOutcome::Cancelled,
                },
            }
        };
        CompletionResult {
            captured: completion.captured,
            outcome,
        }
    }
}
fn advance(epoch: &AtomicU64) -> Option<u64> {
    epoch
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
            value.checked_add(1)
        })
        .ok()?
        .checked_add(1)
}
fn current(epochs: &Epochs, cancel: u64, write: Option<u64>) -> bool {
    epochs.cancel.load(Ordering::Acquire) == cancel
        && write.is_none_or(|write| epochs.writes.load(Ordering::Acquire) == write)
}
fn execute(job: Job, epochs: &Epochs) -> ClipboardCompletion {
    let captured = match &job.request {
        ClipboardRequest::Copy { bundle, .. }
        | ClipboardRequest::Cut { bundle, .. }
        | ClipboardRequest::Paste { bundle, .. } => bundle.clone(),
    };
    let prepared = if !current(epochs, job.cancel_epoch, job.write_epoch) {
        Prepared::Cancelled
    } else {
        match job.request {
            ClipboardRequest::Copy { bundle, ticket } => match job.scope.prepare_copy(&bundle) {
                Ok(outbound) if current(epochs, job.cancel_epoch, job.write_epoch) => {
                    match ticket.write(job.provider.as_ref(), &outbound) {
                        Ok(_) => Prepared::Copied,
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
                    match ticket.write(job.provider.as_ref(), &outbound) {
                        Ok(ack) => Prepared::Cut(guard, ack),
                        Err(error) => Prepared::Failed(JobError::Transport(error)),
                    }
                }
                Ok(_) => Prepared::Cancelled,
                Err(error) => Prepared::Failed(JobError::Project(error)),
            },
            ClipboardRequest::Paste { bundle, guard } => match job.scope.prepare_paste(&bundle) {
                Ok(outbound) if current(epochs, job.cancel_epoch, job.write_epoch) => {
                    Prepared::Paste(guard, outbound, job.provider.max_bytes())
                }
                Ok(_) => Prepared::Cancelled,
                Err(error) => Prepared::Failed(JobError::Project(error)),
            },
        }
    };
    ClipboardCompletion {
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
    use std::sync::{Condvar, Mutex, atomic::AtomicUsize};
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
    #[test]
    fn full_queue_returns_exact_request_for_recovery_instead_of_dropping_it() -> TestResult {
        let jobs = ClipboardJobs::new()?;
        let session = editor()?;
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
        for _ in 0..QUEUED_JOBS {
            jobs.submit(
                session.begin_paste(ClipboardBundle {
                    plain: "queued".into(),
                    ..Default::default()
                })?,
                scope()?,
                provider.clone(),
                &egui::Context::default(),
            )
            .map_err(|_| "submit failed")?;
        }
        let result = jobs.submit(
            session.begin_paste(ClipboardBundle {
                plain: "retain me".into(),
                ..Default::default()
            })?,
            scope()?,
            provider.clone(),
            &egui::Context::default(),
        );
        *provider.released.lock().map_err(|_| "mock lock")? = true;
        provider.ready.notify_all();
        let Err(SubmitFailure {
            kind: SubmitFailureKind::Busy,
            request: ClipboardRequest::Paste { bundle, .. },
        }) = result
        else {
            return Err("queue did not preserve paste".into());
        };
        assert_eq!(bundle.plain, "retain me");
        Ok(())
    }
}
