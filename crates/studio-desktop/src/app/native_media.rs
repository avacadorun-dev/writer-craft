//! Bounded project-owned preview workers; no asset/database/decoder work occurs during paint.
use super::*;
use craft_media_egui::RasterView;
use craft_media_preview::{
    PdfPageOptions, PreviewBudget, PreviewCancellation, PreviewRequest, PreviewSource,
    PreviewWarning, RasterFrame, render_image, render_pdf_page,
};
use std::collections::HashMap;
use std::sync::{
    Arc,
    mpsc::{SyncSender, TrySendError},
};
use studio_core::ProjectTransferCancellation;

const MAX_PENDING: usize = 16;
const MAX_ENTRIES: usize = 256;
const MAX_TEXTURE_BYTES: u64 = 96 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct MediaKey {
    project: String,
    resource: String,
    page: usize,
    target: [u32; 2],
}
#[derive(Clone, Default)]
struct Cancellation {
    preview: PreviewCancellation,
    transfer: ProjectTransferCancellation,
}
impl Cancellation {
    fn cancel(&self) {
        self.preview.cancel();
        self.transfer.cancel();
    }
}
struct LoadJob {
    key: MediaKey,
    ticket: u64,
    cancel: Cancellation,
    context: egui::Context,
}
#[derive(Default)]
struct PaintOutput {
    pages: Option<usize>,
    pointer_captured: bool,
    keyboard_captured: bool,
}
struct LoadedMedia {
    frame: RasterFrame,
    name: String,
}
struct Completion {
    key: MediaKey,
    ticket: u64,
    result: Result<LoadedMedia, String>,
}
enum Entry {
    Pending {
        ticket: u64,
        cancel: Cancellation,
    },
    Ready {
        view: RasterView,
        name: String,
        pages: Option<usize>,
        warnings: Vec<PreviewWarning>,
        bytes: u64,
        used: u64,
    },
    Failed {
        message: String,
        used: u64,
    },
}
/// One bounded actor and texture cache per native resource context. Dropping it cancels work.
pub(super) struct MediaCache {
    sender: SyncSender<LoadJob>,
    receiver: mpsc::Receiver<Completion>,
    entries: HashMap<MediaKey, Entry>,
    sequence: u64,
}
impl Drop for MediaCache {
    fn drop(&mut self) {
        for entry in self.entries.values() {
            if let Entry::Pending { cancel, .. } = entry {
                cancel.cancel();
            }
        }
    }
}
impl MediaCache {
    pub(super) fn new(database: PathBuf) -> Self {
        let (sender, jobs) = mpsc::sync_channel::<LoadJob>(MAX_PENDING);
        let (results, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut cached: Option<(String, String, PreviewSource, String)> = None;
            while let Ok(job) = jobs.recv() {
                let result = load(&database, &job, &mut cached).map_err(|error| error.to_string());
                let context = job.context;
                if results
                    .send(Completion {
                        key: job.key,
                        ticket: job.ticket,
                        result,
                    })
                    .is_err()
                {
                    break;
                }
                context.request_repaint();
            }
        });
        Self {
            sender,
            receiver,
            entries: HashMap::new(),
            sequence: 0,
        }
    }
    pub(super) fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(completion) = self.receiver.try_recv() {
            let current = matches!(self.entries.get(&completion.key),Some(Entry::Pending {ticket,..}) if *ticket==completion.ticket);
            if !current {
                continue;
            }
            self.sequence = self.sequence.saturating_add(1);
            let used = self.sequence;
            let entry = match completion.result {
                Ok(loaded) => {
                    let mut view = RasterView::default();
                    view.request(loaded.frame.request().clone());
                    match view.accept(ctx, &loaded.frame) {
                        Ok(()) => {
                            let [width, height] = loaded.frame.dimensions();
                            Entry::Ready {
                                view,
                                name: loaded.name,
                                pages: loaded.frame.page().map(|(_, pages)| pages),
                                warnings: loaded.frame.warnings().to_vec(),
                                bytes: u64::from(width) * u64::from(height) * 4,
                                used,
                            }
                        }
                        Err(error) => Entry::Failed {
                            message: error.to_string(),
                            used,
                        },
                    }
                }
                Err(message) => Entry::Failed { message, used },
            };
            self.entries.insert(completion.key, entry);
        }
        self.trim();
    }
    fn trim(&mut self) {
        loop {
            let texture_bytes: u64 = self
                .entries
                .values()
                .filter_map(|entry| {
                    if let Entry::Ready { bytes, .. } = entry {
                        Some(*bytes)
                    } else {
                        None
                    }
                })
                .sum();
            if self.entries.len() <= MAX_ENTRIES && texture_bytes <= MAX_TEXTURE_BYTES {
                break;
            }
            let oldest = self
                .entries
                .iter()
                .filter_map(|(key, entry)| match entry {
                    Entry::Ready { used, .. } | Entry::Failed { used, .. } => {
                        Some((key.clone(), *used))
                    }
                    Entry::Pending { .. } => None,
                })
                .min_by_key(|(_, used)| *used)
                .map(|(key, _)| key);
            let Some(key) = oldest else { break };
            self.entries.remove(&key);
        }
    }
    fn ensure(&mut self, ctx: &egui::Context, key: &MediaKey) {
        self.sequence = self.sequence.saturating_add(1);
        if let Some(entry) = self.entries.get_mut(key) {
            match entry {
                Entry::Ready { used, .. } | Entry::Failed { used, .. } => *used = self.sequence,
                Entry::Pending { .. } => {}
            }
            return;
        }
        if self
            .entries
            .values()
            .filter(|entry| matches!(entry, Entry::Pending { .. }))
            .count()
            >= MAX_PENDING
        {
            ctx.request_repaint_after(Duration::from_millis(50));
            return;
        }
        let cancel = Cancellation::default();
        let job = LoadJob {
            key: key.clone(),
            ticket: self.sequence,
            cancel: cancel.clone(),
            context: ctx.clone(),
        };
        match self.sender.try_send(job) {
            Ok(()) => {
                self.entries.insert(
                    key.clone(),
                    Entry::Pending {
                        ticket: self.sequence,
                        cancel,
                    },
                );
            }
            Err(TrySendError::Full(_)) => {
                ctx.request_repaint_after(Duration::from_millis(50));
            }
            Err(TrySendError::Disconnected(_)) => {
                self.entries.insert(
                    key.clone(),
                    Entry::Failed {
                        message: "Media preview worker stopped; reopen the board to retry".into(),
                        used: self.sequence,
                    },
                );
            }
        }
        self.trim();
    }
    fn key(project: &str, resource: &str, page: usize, size: Vec2) -> MediaKey {
        let bucket = |side: f32| {
            if !side.is_finite() {
                return 256;
            }
            ((side.clamp(64.0, 2048.0) / 64.0).ceil() as u32 * 64).clamp(64, 2048)
        };
        MediaKey {
            project: project.into(),
            resource: resource.into(),
            page,
            target: [bucket(size.x), bucket(size.y)],
        }
    }
    pub(super) fn card(
        &mut self,
        ui: &mut egui::Ui,
        project: &str,
        resource: &str,
        size: Vec2,
        l: Language,
        t: Tokens,
    ) -> craft_canvas_egui::NodeBodyResponse {
        let output = self.paint(
            ui,
            &Self::key(project, resource, 0, size),
            size,
            l,
            t,
            false,
        );
        craft_canvas_egui::NodeBodyResponse {
            pointer_captured: output.pointer_captured,
            keyboard_captured: output.keyboard_captured,
        }
    }
    fn paint(
        &mut self,
        ui: &mut egui::Ui,
        key: &MediaKey,
        size: Vec2,
        l: Language,
        t: Tokens,
        details: bool,
    ) -> PaintOutput {
        self.ensure(ui.ctx(), key);
        let mut retry = false;
        let mut output = PaintOutput::default();
        match self.entries.get(key) {
            Some(Entry::Ready {
                view,
                pages: count,
                warnings,
                ..
            }) => {
                output.pages = *count;
                let _ = view.show(ui, size);
                if !warnings.is_empty() {
                    ui.colored_label(t.warning,l.text("Some media content could not be previewed. The original file is preserved."));
                }
            }
            Some(Entry::Failed { message, .. }) => {
                ui.colored_label(
                    t.warning,
                    l.text("Preview unavailable. The original file is preserved."),
                );
                let button = ui.button(l.text("Retry rendering"));
                retry = button.clicked();
                output.pointer_captured = button.hovered();
                output.keyboard_captured = button.has_focus();
                if details {
                    ui.collapsing(l.text("Technical details"), |ui| {
                        ui.label(message);
                    });
                }
            }
            _ => {
                ui.spinner();
                ui.label(l.text("Loading media preview…"));
                ui.ctx().request_repaint_after(Duration::from_millis(50));
            }
        }
        if retry {
            self.entries.remove(key);
        }
        output
    }
    pub(super) fn viewer(
        &mut self,
        ctx: &egui::Context,
        project: &str,
        viewer: &mut MediaViewer,
        l: Language,
        t: Tokens,
    ) -> bool {
        self.poll(ctx);
        let mut close = false;
        let max_height = (ctx.content_rect().height() - 250.0).clamp(100.0, 700.0);
        let available = Vec2::new(
            (ctx.content_rect().width() - 150.0).clamp(200.0, 960.0),
            max_height,
        );
        let key = Self::key(project, &viewer.resource, viewer.page, available);
        let name = match self.entries.get(&key) {
            Some(Entry::Ready { name, .. }) => name.clone(),
            _ => l.text("Media preview").into(),
        };
        let response =
            components::modal(ctx, l, "project-media-preview", &name, available.x, |ui| {
                let output = self.paint(ui, &key, available, l, t, true);
                components::footer(ui, |ui| {
                    close = components::action(ui, l.text("Done"), true, Some(t)).clicked();
                    if let Some(pages) = output.pages {
                        if components::action(
                            ui,
                            l.text("Next page"),
                            viewer.page + 1 < pages,
                            None,
                        )
                        .clicked()
                        {
                            viewer.page += 1;
                        }
                        if components::action(ui, l.text("Previous page"), viewer.page > 0, None)
                            .clicked()
                        {
                            viewer.page -= 1;
                        }
                        ui.label(l.format(
                            "Page {page} of {pages}",
                            &[
                                ("page", (viewer.page + 1).to_string()),
                                ("pages", pages.to_string()),
                            ],
                        ));
                    }
                });
            });
        close || response.should_close()
    }
}
/// Explicit selected project attachment; every page change gets a distinct native load ticket.
pub(super) struct MediaViewer {
    pub(super) resource: String,
    page: usize,
}
impl MediaViewer {
    pub(super) fn new(resource: String) -> Self {
        Self { resource, page: 0 }
    }
}
fn load(
    database: &std::path::Path,
    job: &LoadJob,
    cached: &mut Option<(String, String, PreviewSource, String)>,
) -> AppResult<LoadedMedia> {
    job.cancel.preview.check()?;
    let same = cached.as_ref().is_some_and(|(project, resource, _, _)| {
        project == &job.key.project && resource == &job.key.resource
    });
    if !same {
        // Release the previous encoded source before reading another accepted large original.
        *cached = None;
        let workspace = Workspace::open(database)?;
        let (attachment, source) =
            workspace.project_media_source(&job.key.project, &job.key.resource)?;
        let bytes = source.load(&attachment.logical_id, &job.cancel.transfer)?;
        job.cancel.preview.check()?;
        let length = usize::try_from(attachment.byte_size).map_err(|_| {
            studio_core::StudioError::Validation("Media byte size exceeds native capacity".into())
        })?;
        let source = PreviewSource::verified(
            Arc::new(bytes),
            length,
            &attachment.sha256,
            &attachment.media_type,
            &PreviewBudget::default(),
        )?;
        *cached = Some((
            job.key.project.clone(),
            job.key.resource.clone(),
            source,
            attachment.original_name,
        ));
    }
    let (_, _, source, name) = cached.as_ref().ok_or_else(|| {
        studio_core::StudioError::Validation("Media source was not acknowledged".into())
    })?;
    let request = PreviewRequest::new(source)?;
    let budget = PreviewBudget::default();
    let frame = if source.stamp().media_type == "application/pdf" {
        render_pdf_page(
            source,
            &request,
            &PdfPageOptions {
                page_index: job.key.page,
                target: job.key.target,
                ..Default::default()
            },
            &budget,
            &job.cancel.preview,
        )?
    } else {
        render_image(
            source,
            &request,
            job.key.target,
            &budget,
            &job.cancel.preview,
        )?
    };
    job.cancel.preview.check()?;
    Ok(LoadedMedia {
        frame,
        name: name.clone(),
    })
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "Synthetic cancellation/queue contracts fail immediately"
)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::io::{Cursor, Read};
    use studio_core::{
        ImportAssetSource, ProjectAttachment, ProjectImportWorker, ValidatedProjectImport,
    };
    type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
    struct Asset(Vec<u8>);
    impl ImportAssetSource for Asset {
        fn open<'a>(&'a mut self, _: &ProjectAttachment) -> std::io::Result<Box<dyn Read + 'a>> {
            Ok(Box::new(Cursor::new(self.0.as_slice())))
        }
    }
    fn imported(database: &std::path::Path, bytes: &[u8], mime: &str) -> AppResult<String> {
        let source = Workspace::memory()?;
        let project = source.create_project("Synthetic native media")?;
        let mut manifest = source
            .project_export_snapshot(&project.id)?
            .manifest()
            .clone();
        manifest["media"] = serde_json::json!([{
            "id":"same-alias", "path":"media/original.bin", "sha256":format!("{:x}",Sha256::digest(bytes)),
            "byteSize":bytes.len(), "mediaType":mime, "originalName":"original.bin", "width":null,"height":null
        }]);
        let validated = ValidatedProjectImport::parse(&serde_json::to_vec(&manifest)?)?;
        let imported = ProjectImportWorker::open(database)?.import(
            &validated,
            &mut Asset(bytes.to_vec()),
            &ProjectTransferCancellation::default(),
        )?;
        Ok(imported.project_id)
    }
    fn job(project: &str) -> LoadJob {
        LoadJob {
            key: MediaKey {
                project: project.into(),
                resource: "same-alias".into(),
                page: 0,
                target: [4, 4],
            },
            ticket: 1,
            cancel: Cancellation::default(),
            context: egui::Context::default(),
        }
    }
    fn cache_without_worker() -> (
        MediaCache,
        mpsc::Receiver<LoadJob>,
        mpsc::Sender<Completion>,
    ) {
        let (sender, jobs) = mpsc::sync_channel(MAX_PENDING);
        let (results, receiver) = mpsc::channel();
        (
            MediaCache {
                sender,
                receiver,
                entries: HashMap::new(),
                sequence: 0,
            },
            jobs,
            results,
        )
    }
    #[test]
    fn actual_image_worker_should_resolve_only_its_project_and_keep_original_encoded_bytes()
    -> TestResult {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("workspace.sqlite");
        let image = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            8,
            4,
            image::Rgb([25, 80, 160]),
        ));
        let mut encoded = Cursor::new(Vec::new());
        image.write_to(&mut encoded, image::ImageFormat::Png)?;
        let bytes = encoded.into_inner();
        let project = imported(&database, &bytes, "image/png")?;
        let other = imported(&database, b"not a PNG", "application/x-synthetic")?;
        let mut cached = None;
        let loaded = load(&database, &job(&project), &mut cached)?;
        assert_eq!(loaded.frame.dimensions(), [4, 2]);
        assert_eq!(&loaded.frame.rgba()[..4], &[25, 80, 160, 255]);
        assert_eq!(
            cached.as_ref().map(|(_, _, source, _)| source.bytes()),
            Some(bytes.as_slice())
        );
        assert!(load(&database, &job(&other), &mut cached).is_err());
        let workspace = Workspace::open(&database)?;
        let (descriptor, source) = workspace.project_media_source(&project, "same-alias")?;
        assert_eq!(
            source.load(
                &descriptor.logical_id,
                &ProjectTransferCancellation::default()
            )?,
            bytes
        );
        Ok(())
    }
    fn pdf() -> Vec<u8> {
        let content = "1 0 0 rg 0 0 100 50 re f\n";
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 50] /Contents 4 0 R >>".to_owned(),
            format!(
                "<< /Length {} >>\nstream\n{}endstream",
                content.len(),
                content
            ),
        ];
        let mut bytes = b"%PDF-1.7\n".to_vec();
        let mut offsets = vec![0];
        for (index, object) in objects.iter().enumerate() {
            offsets.push(bytes.len());
            bytes
                .extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
        }
        let xref = bytes.len();
        bytes.extend_from_slice(
            format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes(),
        );
        for offset in &offsets[1..] {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                offsets.len(),
                xref
            )
            .as_bytes(),
        );
        bytes
    }
    #[test]
    fn pdf_worker_should_render_real_page_pixels_and_refuse_an_out_of_range_page() -> TestResult {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("workspace.sqlite");
        let project = imported(&database, &pdf(), "application/pdf")?;
        let mut request = job(&project);
        request.key.target = [100, 100];
        let mut cached = None;
        let loaded = load(&database, &request, &mut cached)?;
        assert_eq!(loaded.frame.page(), Some((0, 1)));
        let [width, height] = loaded.frame.dimensions();
        let center = ((height / 2 * width + width / 2) * 4) as usize;
        assert_eq!(&loaded.frame.rgba()[center..center + 4], &[255, 0, 0, 255]);
        request.key.page = 1;
        assert!(load(&database, &request, &mut cached).is_err());
        Ok(())
    }
    #[test]
    fn cancelled_worker_should_refuse_before_opening_any_database() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("not-created.sqlite");
        let request = job("missing");
        request.cancel.cancel();
        assert!(load(&database, &request, &mut None).is_err());
        assert!(!database.exists());
    }
    #[test]
    fn stale_ticket_or_other_project_completion_should_not_replace_pending_request() -> TestResult {
        let (mut cache, _jobs, results) = cache_without_worker();
        let current = job("current");
        let key = current.key.clone();
        cache.entries.insert(
            key.clone(),
            Entry::Pending {
                ticket: 2,
                cancel: current.cancel.clone(),
            },
        );
        results.send(Completion {
            key: key.clone(),
            ticket: 1,
            result: Err("stale".into()),
        })?;
        results.send(Completion {
            key: job("foreign").key,
            ticket: 2,
            result: Err("foreign".into()),
        })?;
        cache.poll(&egui::Context::default());
        assert!(matches!(
            cache.entries.get(&key),
            Some(Entry::Pending { ticket: 2, .. })
        ));
        results.send(Completion {
            key: key.clone(),
            ticket: 2,
            result: Err("current failure".into()),
        })?;
        cache.poll(&egui::Context::default());
        assert!(
            matches!(cache.entries.get(&key),Some(Entry::Failed {message,..}) if message=="current failure")
        );
        Ok(())
    }
    #[test]
    fn media_queue_should_bound_all_outstanding_work_and_cancel_it_on_drop() {
        let (mut cache, jobs, _results) = cache_without_worker();
        let ctx = egui::Context::default();
        for index in 0..64 {
            cache.ensure(
                &ctx,
                &MediaKey {
                    project: "p".into(),
                    resource: index.to_string(),
                    page: 0,
                    target: [64, 64],
                },
            );
        }
        assert_eq!(cache.entries.len(), MAX_PENDING);
        let queued = jobs.try_recv().unwrap();
        drop(cache);
        assert!(queued.cancel.preview.is_cancelled());
        assert!(queued.cancel.transfer.is_cancelled());
    }
    #[test]
    fn preview_targets_should_quantize_finite_dimensions_and_bound_invalid_geometry() {
        assert_eq!(
            MediaCache::key("p", "r", 0, Vec2::new(65.0, 127.0)).target,
            [128, 128]
        );
        assert_eq!(
            MediaCache::key("p", "r", 0, Vec2::new(f32::NAN, 1e9)).target,
            [256, 2048]
        );
    }
}
