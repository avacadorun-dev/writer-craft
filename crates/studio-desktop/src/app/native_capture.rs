//! Explicit engineering captures observe settled native output; they never certify interaction.
use super::*;
use image::ImageEncoder;
use serde::Serialize;
use serde_json::{Value, json};
use std::fs::OpenOptions;
use std::path::Path;

const STABLE_FRAMES: u8 = 3;
const DEADLINE: Duration = Duration::from_secs(38);
const READBACK_RETRY: Duration = Duration::from_millis(500);
const MAX_READBACK_REQUESTS: u8 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub(super) struct CaptureReadiness {
    pub generation: u64,
    pub scroll_offset_bits: u32,
    pub visible_atoms: usize,
    pub target_is_text: bool,
    pub visible_text_targets: usize,
    pub pending_atoms: usize,
    pub failed_atoms: usize,
    pub target_visible: bool,
    pub settled: bool,
}
impl Default for CaptureReadiness {
    fn default() -> Self {
        Self {
            generation: 0,
            scroll_offset_bits: 0,
            visible_atoms: 0,
            target_is_text: false,
            visible_text_targets: 0,
            pending_atoms: 0,
            failed_atoms: 0,
            target_visible: true,
            settled: true,
        }
    }
}

#[derive(Default)]
pub(super) struct CaptureProbe {
    #[cfg(feature = "editor-preview")]
    pub(super) connected: Option<super::capture_connected_writing::ConnectedCapture>,
    started: Option<Instant>,
    target: Option<String>,
    initial_document: Option<Value>,
    last: Option<CaptureReadiness>,
    stable_frames: u8,
    requested: bool,
    requested_at: Option<Instant>,
    readback_requests: u8,
    #[cfg(any(feature = "editor-preview", test))]
    navigated_generation: Option<u64>,
    #[cfg(any(feature = "editor-preview", test))]
    settled_navigation_generation: Option<u64>,
    #[cfg(any(feature = "editor-preview", test))]
    navigation_sample: Option<CaptureReadiness>,
    #[cfg(any(feature = "editor-preview", test))]
    navigation_stable_frames: u8,
}
impl CaptureProbe {
    #[cfg(feature = "editor-preview")]
    pub(super) fn target(&self) -> Option<&str> {
        self.target.as_deref()
    }
    #[cfg(any(feature = "editor-preview", test))]
    fn needs_navigation(&mut self, generation: u64) -> bool {
        if self.navigated_generation == Some(generation) {
            return false;
        }
        self.navigated_generation = Some(generation);
        self.navigation_sample = None;
        self.navigation_stable_frames = 0;
        true
    }
    #[cfg(any(feature = "editor-preview", test))]
    fn needs_settled_navigation(&mut self, readiness: CaptureReadiness) -> bool {
        if self.navigated_generation != Some(readiness.generation)
            || self.settled_navigation_generation == Some(readiness.generation)
            || readiness.target_visible
            || !readiness.settled
            || readiness.pending_atoms != 0
            || readiness.failed_atoms != 0
        {
            self.navigation_sample = None;
            self.navigation_stable_frames = 0;
            return false;
        }
        if self.navigation_sample == Some(readiness) {
            self.navigation_stable_frames = self.navigation_stable_frames.saturating_add(1);
        } else {
            self.navigation_sample = Some(readiness);
            self.navigation_stable_frames = 1;
        }
        if self.navigation_stable_frames < STABLE_FRAMES {
            return false;
        }
        self.settled_navigation_generation = Some(readiness.generation);
        true
    }
    fn readback_due(&self, now: Instant) -> bool {
        self.readback_requests < MAX_READBACK_REQUESTS
            && (!self.requested
                || self
                    .requested_at
                    .is_some_and(|sent| now.saturating_duration_since(sent) >= READBACK_RETRY))
    }
    fn acknowledge_readback_request(&mut self, now: Instant) {
        self.requested = true;
        self.requested_at = Some(now);
        self.readback_requests = self.readback_requests.saturating_add(1);
    }
    fn observe(&mut self, readiness: CaptureReadiness) -> bool {
        if readiness.settled && readiness.target_visible && self.last == Some(readiness) {
            self.stable_frames = self.stable_frames.saturating_add(1);
        } else {
            self.stable_frames = u8::from(readiness.settled && readiness.target_visible);
            self.requested = false;
            self.requested_at = None;
            self.readback_requests = 0;
        }
        self.last = Some(readiness);
        self.stable_frames >= STABLE_FRAMES
    }
}
fn state_path(capture: &Path) -> PathBuf {
    capture.with_extension("capture.json")
}
fn validate_output(path: &Path, database: &Path) -> AppResult<()> {
    if studio_core::is_workspace_data_path(path, database)? {
        return Err("protected_workspace_export_path".into());
    }
    if path.try_exists()? {
        return Err("capture_output_already_exists".into());
    }
    Ok(())
}
fn write_new_json(path: &Path, value: &Value) -> AppResult<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}
impl StudioApp {
    pub(super) fn initialize_capture(&mut self, demo: bool, ctx: &egui::Context) -> AppResult<()> {
        let path = self.capture.as_ref().ok_or("capture_path_missing")?;
        validate_output(path, &self.database)?;
        validate_output(&state_path(path), &self.database)?;
        self.capture_probe.started = Some(Instant::now());
        #[cfg(feature = "editor-preview")]
        {
            self.capture_probe.connected =
                super::capture_connected_writing::ConnectedCapture::initialize(self, demo, ctx)?;
        }
        #[cfg(not(feature = "editor-preview"))]
        {
            let _ = ctx;
            if std::env::var("WRITERCRAFT_CAPTURE_SCENARIO").is_ok_and(|value| !value.is_empty()) {
                return Err("connected_capture_requires_native_editor_feature".into());
            }
        }
        let target = std::env::var("WRITERCRAFT_CAPTURE_BLOCK")
            .ok()
            .filter(|id| !id.is_empty());
        if target.is_some()
            && std::env::var("WRITERCRAFT_CAPTURE_SCENARIO").is_ok_and(|value| !value.is_empty())
        {
            return Err("capture_block_and_connected_scenario_are_exclusive".into());
        }
        if target.is_some() && !demo {
            return Err("capture_target_requires_explicit_synthetic_demo".into());
        }
        #[cfg(feature = "editor-preview")]
        if target.is_some() && self.rich.is_none() {
            return Err("capture_target_requires_native_rich_document".into());
        }
        #[cfg(feature = "editor-preview")]
        if let Some(rich) = &mut self.rich {
            if let Some(id) = &target {
                rich.select_capture_target(id)?;
            }
            self.capture_probe.initial_document = Some(rich.session.document().to_json()?);
        }
        #[cfg(not(feature = "editor-preview"))]
        if target.is_some() {
            return Err("capture_target_requires_native_editor".into());
        }
        self.capture_probe.target = target;
        Ok(())
    }
    pub(super) fn prepare_capture_frame(&mut self) {
        if self.capture.is_none() || self.capture_probe.target.is_none() {
            return;
        }
        #[cfg(feature = "editor-preview")]
        if let Some(rich) = &mut self.rich {
            let readiness = rich.capture_readiness(self.capture_probe.target.as_deref());
            if self.capture_probe.needs_navigation(readiness.generation)
                || self.capture_probe.needs_settled_navigation(readiness)
            {
                // A progressive metric change can leave an earlier animated scroll at a
                // position that no longer shows the selected target. After actual native
                // render/scroll stability, allow one semantic retry per generation only
                // while that target is still unseen. Visible tall objects never retry;
                // repeated nearest-edge scrolling could alternate their top/bottom edges.
                rich.session.request_scroll_to_caret();
            }
        }
    }
    pub(super) fn advance_capture(&mut self, ctx: &egui::Context) {
        if self.capture.is_none() {
            return;
        }
        self.frames = self.frames.saturating_add(1);
        #[cfg(feature = "editor-preview")]
        let readiness = if let Some(mut connected) = self.capture_probe.connected.take() {
            let readiness = connected.readiness(self, ctx);
            self.capture_probe.connected = Some(connected);
            readiness
        } else {
            self.rich
                .as_ref()
                .map_or_else(CaptureReadiness::default, |rich| {
                    rich.capture_readiness(self.capture_probe.target.as_deref())
                })
        };
        #[cfg(not(feature = "editor-preview"))]
        let readiness = CaptureReadiness::default();
        let ready = self.capture_probe.observe(readiness) && self.frames >= 8;
        let image = ctx.input(|input| {
            input.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if ready
            && self.capture_probe.requested
            && let Some(image) = image
        {
            self.finish_capture(ctx, Some(&image), None);
        } else if self
            .capture_probe
            .started
            .is_some_and(|start| start.elapsed() >= DEADLINE)
        {
            self.finish_capture(
                ctx,
                None,
                Some(if ready {
                    "native_readback_did_not_arrive_before_deadline"
                } else {
                    "native_media_or_target_did_not_settle_before_deadline"
                }),
            );
        } else if ready && self.capture_probe.readback_due(Instant::now()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.capture_probe
                .acknowledge_readback_request(Instant::now());
        }
        if self.capture.is_some() {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }
    fn finish_capture(
        &mut self,
        ctx: &egui::Context,
        image: Option<&egui::ColorImage>,
        error: Option<&str>,
    ) {
        let Some(path) = self.capture.take() else {
            return;
        };
        let outcome = (|| -> AppResult<()> {
            validate_output(&path, &self.database)?;
            validate_output(&state_path(&path), &self.database)?;
            #[cfg(feature = "editor-preview")]
            let connected_capture = self
                .capture_probe
                .connected
                .as_ref()
                .map(|probe| probe.evidence());
            #[cfg(not(feature = "editor-preview"))]
            let connected_capture: Option<Value> = None;
            #[cfg(feature = "editor-preview")]
            let canonical_unchanged = self
                .rich
                .as_ref()
                .map(|rich| rich.session.document().to_json())
                .transpose()?
                == self.capture_probe.initial_document;
            #[cfg(not(feature = "editor-preview"))]
            let canonical_unchanged = self.capture_probe.initial_document.is_none();
            if let Some(image) = image {
                let file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)?;
                image::codecs::png::PngEncoder::new(&file).write_image(
                    image.as_raw(),
                    image.width() as u32,
                    image.height() as u32,
                    image::ExtendedColorType::Rgba8,
                )?;
                file.sync_all()?;
            }
            let viewport = ctx
                .input(|input| input.viewport().inner_rect)
                .map(|rect| [rect.width(), rect.height()]);
            let evidence = json!({
                "schema_version": 1, "status": if error.is_none() && image.is_some() { "captured" } else { "failed" },
                "error": error, "target_block": self.capture_probe.target,
                "target_kind": if connected_capture.is_some() { "connected" } else if self.capture_probe.target.is_none() { "viewport" } else if self.capture_probe.last.is_some_and(|state|state.target_is_text) { "text" } else { "opaque" },
                "frame": self.frames, "stable_frames": self.capture_probe.stable_frames,
                "readback_requests": self.capture_probe.readback_requests,
                "render_readiness": self.capture_probe.last,
                "canonical_unchanged": if connected_capture.is_some() { None } else { Some(canonical_unchanged) },
                "connected_capture": connected_capture,
                "observed_inner_size_points": viewport, "pixels_per_point": ctx.pixels_per_point(),
                "image_size_pixels": image.map(|image| [image.width(), image.height()]),
                "language": self.language.code(), "font_size": self.font_size,
                "editor_preferences": self.editor_preferences.values,
                "focus_mode": self.focus_mode, "settings_open": self.settings_open,
                "editor_acceptance": false, "native_interaction_acceptance": false,
            });
            write_new_json(&state_path(&path), &evidence)?;
            if connected_capture
                .as_ref()
                .is_some_and(|probe| probe["content_contract_satisfied"] != true)
            {
                return Err("connected_capture_content_contract_failed".into());
            }
            if connected_capture.is_none() && !canonical_unchanged {
                return Err("capture_navigation_changed_canonical_document".into());
            }
            Ok(())
        })();
        self.record_error(outcome);
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

/// A capture-only observer of the same measured visible text passed to normal decorations.
/// It delegates unchanged and never creates a diagnostic highlight or guesses visibility.
#[cfg(feature = "editor-preview")]
pub(super) struct VisibleTargetFrame<'a> {
    source: &'a mut dyn craft_editor_egui::decorations::DecorationSource,
    target: &'a str,
    pub visible: bool,
}
#[cfg(feature = "editor-preview")]
impl<'a> VisibleTargetFrame<'a> {
    pub fn new(
        source: &'a mut dyn craft_editor_egui::decorations::DecorationSource,
        target: &'a str,
    ) -> Self {
        Self {
            source,
            target,
            visible: false,
        }
    }
}
#[cfg(feature = "editor-preview")]
impl craft_editor_egui::decorations::DecorationSource for VisibleTargetFrame<'_> {
    fn decorations(
        &mut self,
        document: &craft_editor_model::Document,
        revision: craft_editor_model::Revision,
        pending: &[craft_editor_model::ReplayProof],
    ) -> craft_editor_egui::Result<Vec<craft_editor_egui::decorations::TextDecoration>> {
        self.source.decorations(document, revision, pending)
    }
    fn underlines_in_view(
        &mut self,
        document: &craft_editor_model::Document,
        revision: craft_editor_model::Revision,
        pending: &[craft_editor_model::ReplayProof],
        visible: &[craft_editor_egui::Selection],
    ) -> craft_editor_egui::Result<Vec<craft_editor_egui::decorations::TextUnderline>> {
        self.visible = visible.iter().any(|range| {
            range.anchor.block_id == self.target || range.head.block_id == self.target
        });
        self.source
            .underlines_in_view(document, revision, pending, visible)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn asynchronous_geometry_or_target_changes_restart_stability() {
        let mut probe = CaptureProbe::default();
        let ready = CaptureReadiness::default();
        assert!(!probe.observe(ready));
        assert!(!probe.observe(ready));
        assert!(probe.observe(ready));
        probe.requested = true;
        assert!(!probe.observe(CaptureReadiness {
            generation: 1,
            ..ready
        }));
        assert!(!probe.requested);
        assert!(!probe.observe(CaptureReadiness {
            target_visible: false,
            ..ready
        }));
        assert!(!probe.observe(CaptureReadiness {
            pending_atoms: 1,
            settled: false,
            ..ready
        }));
        assert!(!probe.observe(ready));
        assert!(!probe.observe(ready));
        assert!(probe.observe(ready));
    }
    #[test]
    fn missing_gpu_readback_retries_are_bounded_and_invalidated_by_new_geometry() {
        let mut probe = CaptureProbe::default();
        let ready = CaptureReadiness::default();
        probe.observe(ready);
        let start = Instant::now();
        for index in 0..MAX_READBACK_REQUESTS {
            let now = start + READBACK_RETRY * u32::from(index);
            assert!(probe.readback_due(now));
            probe.acknowledge_readback_request(now);
            assert!(!probe.readback_due(now + Duration::from_millis(1)));
        }
        assert!(!probe.readback_due(start + Duration::from_secs(30)));
        probe.observe(CaptureReadiness {
            generation: 1,
            ..ready
        });
        assert!(!probe.requested);
        assert_eq!(probe.readback_requests, 0);
        assert!(probe.readback_due(start + Duration::from_secs(31)));
    }
    #[test]
    fn an_oversized_target_is_not_repeatedly_scrolled_without_new_metrics() {
        let mut probe = CaptureProbe::default();
        assert!(probe.needs_navigation(0));
        for _ in 0..120 {
            assert!(!probe.needs_navigation(0));
        }
        assert!(probe.needs_navigation(1));
        for _ in 0..120 {
            assert!(!probe.needs_navigation(1));
        }
        assert!(probe.needs_navigation(2));
    }
    #[test]
    fn stable_unseen_target_gets_one_post_layout_navigation_per_generation() {
        let mut probe = CaptureProbe::default();
        let unseen = CaptureReadiness {
            generation: 7,
            scroll_offset_bits: 3061.162_f32.to_bits(),
            target_visible: false,
            ..CaptureReadiness::default()
        };
        assert!(probe.needs_navigation(7));
        assert!(!probe.needs_settled_navigation(unseen));
        assert!(!probe.needs_settled_navigation(unseen));
        assert!(probe.needs_settled_navigation(unseen));
        for _ in 0..120 {
            assert!(!probe.needs_navigation(7));
            assert!(!probe.needs_settled_navigation(unseen));
            assert!(!probe.observe(unseen));
        }
        assert_eq!(probe.stable_frames, 0);
        assert!(!probe.requested);
        assert!(probe.needs_navigation(8));
        let next = CaptureReadiness {
            generation: 8,
            ..unseen
        };
        assert!(!probe.needs_settled_navigation(next));
        assert!(!probe.needs_settled_navigation(next));
        assert!(probe.needs_settled_navigation(next));
    }
    #[test]
    fn post_layout_navigation_waits_for_valid_unchanged_geometry() {
        let unseen = CaptureReadiness {
            generation: 3,
            target_visible: false,
            ..CaptureReadiness::default()
        };
        for invalid in [
            CaptureReadiness {
                settled: false,
                ..unseen
            },
            CaptureReadiness {
                pending_atoms: 1,
                ..unseen
            },
            CaptureReadiness {
                failed_atoms: 1,
                ..unseen
            },
            CaptureReadiness {
                target_visible: true,
                ..unseen
            },
            CaptureReadiness {
                generation: 4,
                ..unseen
            },
        ] {
            let mut probe = CaptureProbe::default();
            assert!(probe.needs_navigation(3));
            assert!(!probe.needs_settled_navigation(unseen));
            assert!(!probe.needs_settled_navigation(unseen));
            assert!(!probe.needs_settled_navigation(invalid));
            assert!(!probe.needs_settled_navigation(unseen));
            assert!(!probe.needs_settled_navigation(unseen));
            assert!(probe.needs_settled_navigation(unseen));
        }
        let mut probe = CaptureProbe::default();
        assert!(probe.needs_navigation(3));
        for offset in 0..120 {
            assert!(!probe.needs_settled_navigation(CaptureReadiness {
                scroll_offset_bits: (offset as f32).to_bits(),
                ..unseen
            }));
        }
        assert!(!probe.needs_settled_navigation(unseen));
        assert!(!probe.needs_settled_navigation(unseen));
        assert!(probe.needs_settled_navigation(unseen));
    }
    #[test]
    fn visible_tall_targets_never_get_post_layout_navigation() {
        let mut probe = CaptureProbe::default();
        assert!(probe.needs_navigation(4));
        let visible = CaptureReadiness {
            generation: 4,
            visible_atoms: 1,
            ..CaptureReadiness::default()
        };
        for _ in 0..120 {
            assert!(!probe.needs_navigation(4));
            assert!(!probe.needs_settled_navigation(visible));
        }
    }
    #[test]
    fn capture_refuses_live_data_and_does_not_replace_existing_outputs() -> AppResult<()> {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("workspace.png");
        let _workspace = Workspace::open(&database)?;
        assert!(validate_output(&database, &database).is_err());
        assert!(validate_output(&directory.path().join("workspace.png-wal"), &database).is_err());
        assert!(validate_output(&directory.path().join("editor.json"), &database).is_err());
        let output = directory.path().join("evidence.png");
        std::fs::write(&output, b"preserve")?;
        assert!(validate_output(&output, &database).is_err());
        assert_eq!(std::fs::read(&output)?, b"preserve");
        assert!(validate_output(&directory.path().join("new.png"), &database).is_ok());
        Ok(())
    }
}
