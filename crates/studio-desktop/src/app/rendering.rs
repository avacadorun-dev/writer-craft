//! Progressive preparation of exact native atoms; prose never waits for media readiness.
use super::*;
use craft_editor_egui::{
    layout::AtomSize,
    renderers::{AtomPlaceholder, AtomPlaceholderState, AtomResolver, ResolvedAtom},
};
use craft_editor_model::{Document as RichDocument, Node, NodePath, Revision};
use craft_editor_render::{FallbackLabels, PreparedCatalog, RenderOptions, Renderer};
use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

const CACHE_BYTES: usize = 64 * 1024 * 1024;
const ACKNOWLEDGEMENTS_PER_FRAME: usize = 4;
const BATCH_TIME: Duration = Duration::from_secs(15);
static WORKER_BUSY: AtomicBool = AtomicBool::new(false);

#[derive(Clone, PartialEq)]
struct PresentationKey {
    font_size: u32,
    pixel_scale: u32,
    foreground: [u8; 4],
    background: [u8; 4],
    language: Language,
}
struct RenderJob {
    epoch: u64,
    receiver: mpsc::Receiver<WorkerEvent>,
    cancelled: Arc<AtomicBool>,
}
impl Drop for RenderJob {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}
enum WorkerEvent {
    Atom {
        identity: String,
        result: craft_editor_render::Result<PreparedCatalog>,
    },
    Finished,
}
struct WorkerPermit;
impl WorkerPermit {
    fn acquire() -> Option<Self> {
        WORKER_BUSY
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Self)
    }
}
impl Drop for WorkerPermit {
    fn drop(&mut self) {
        WORKER_BUSY.store(false, Ordering::Release);
    }
}

pub(super) struct RenderHost<'a> {
    pub language: Language,
    pub tokens: Tokens,
    pub font_size: f32,
    pub pixel_scale: f32,
    pub workspace: &'a Workspace,
    pub project_id: &'a str,
}

enum AtomState {
    Pending,
    Ready(PreparedCatalog),
    Failed(craft_editor_render::Error),
}
struct CacheEntry {
    state: AtomState,
    last_used: u64,
}
impl CacheEntry {
    fn bytes(&self) -> usize {
        match &self.state {
            AtomState::Ready(catalog) => catalog.encoded_bytes(),
            AtomState::Pending | AtomState::Failed(_) => 0,
        }
    }
    fn failed(&self) -> bool {
        match &self.state {
            AtomState::Failed(_) => true,
            AtomState::Ready(catalog) => !catalog.failures().is_empty(),
            AtomState::Pending => false,
        }
    }
}

struct KnownGeometry {
    size: AtomSize,
    font_size: f32,
}

/// Immutable during a frame, apart from a nonauthoritative viewport hint. No I/O in resolution.
struct RenderSurface {
    entries: HashMap<String, CacheEntry>,
    dimensions: HashMap<String, KnownGeometry>,
    paths: HashMap<NodePath, String>,
    visible: Mutex<Option<Vec<NodePath>>>,
    generation: u64,
    language: Language,
    font_size: f32,
    clock: u64,
    cache_bytes: usize,
}
impl Default for RenderSurface {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            dimensions: HashMap::new(),
            paths: HashMap::new(),
            visible: Mutex::new(None),
            generation: 0,
            language: Language::default(),
            font_size: 20.0,
            clock: 0,
            cache_bytes: CACHE_BYTES,
        }
    }
}
impl RenderSurface {
    fn visible_ids(&self) -> Vec<String> {
        // The hint owns no source or authority. A poisoned hint can safely be recovered.
        let visible = self
            .visible
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut seen = HashSet::new();
        visible
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter_map(|path| self.paths.get(path))
            .filter(|identity| seen.insert(identity.as_str()))
            .cloned()
            .collect()
    }
    fn touch_visible(&mut self) {
        self.clock = self.clock.saturating_add(1);
        for identity in self.visible_ids() {
            if let Some(entry) = self.entries.get_mut(&identity) {
                entry.last_used = self.clock;
            }
        }
    }
    fn bytes(&self) -> usize {
        self.entries.values().map(CacheEntry::bytes).sum()
    }
    fn retain_geometry(&mut self, identity: &str, node: &Node, catalog: &PreparedCatalog) {
        if let Ok(image) = catalog.resolve(node, 20_000.0)
            && image.size.width.is_finite()
            && image.size.height.is_finite()
            && image.size.width > 0.0
            && image.size.height > 0.0
        {
            self.dimensions.insert(
                identity.into(),
                KnownGeometry {
                    size: image.size,
                    font_size: self.font_size,
                },
            );
        }
    }
    fn acknowledge(
        &mut self,
        identity: &str,
        result: craft_editor_render::Result<PreparedCatalog>,
    ) {
        if !self.entries.contains_key(identity) {
            return;
        }
        let state = match result {
            Ok(catalog) => {
                let incoming = catalog.encoded_bytes();
                let visible: HashSet<_> = self.visible_ids().into_iter().collect();
                // Evict derived images, never reject or alter canonical document content.
                // An individual atom retains its renderer's own input/pixel/output bounds.
                while self.bytes().saturating_add(incoming) > self.cache_bytes {
                    let evict = self
                        .entries
                        .iter()
                        .filter(|(key, entry)| key.as_str() != identity && entry.bytes() > 0)
                        .min_by_key(|(key, entry)| (visible.contains(*key), entry.last_used))
                        .map(|(key, _)| key.clone());
                    let Some(evict) = evict else { break };
                    if let Some(entry) = self.entries.get_mut(&evict) {
                        entry.state = AtomState::Pending;
                    }
                }
                AtomState::Ready(catalog)
            }
            Err(error) => AtomState::Failed(error),
        };
        if let Some(entry) = self.entries.get_mut(identity) {
            entry.state = state;
            entry.last_used = self.clock;
        }
        self.generation = self.generation.saturating_add(1);
    }
    fn visible_settled(&self) -> bool {
        if self
            .visible
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .is_none()
        {
            return self.entries.is_empty();
        }
        self.visible_ids().iter().all(|identity| {
            self.entries
                .get(identity)
                .is_none_or(|entry| !matches!(entry.state, AtomState::Pending))
        })
    }
}
impl AtomResolver for RenderSurface {
    fn resolve(&self, node: &Node, width: f32) -> craft_editor_egui::Result<ResolvedAtom> {
        let identity = serde_json::to_string(node)
            .map_err(|_| craft_editor_egui::Error::AtomRendering("atom_identity_failed".into()))?;
        match self.entries.get(&identity).map(|entry| &entry.state) {
            Some(AtomState::Ready(catalog)) => catalog.resolve(node, width),
            Some(AtomState::Failed(error)) => {
                Err(craft_editor_egui::Error::AtomRendering(error.to_string()))
            }
            Some(AtomState::Pending) | None => Err(craft_editor_egui::Error::AtomRendering(
                "atom_pending".into(),
            )),
        }
    }
    fn placeholder(&self, node: &Node, available_width: f32) -> Option<AtomPlaceholder> {
        let identity = serde_json::to_string(node).ok()?;
        let failed = match self.entries.get(&identity).map(|entry| &entry.state) {
            Some(AtomState::Ready(_)) => return None,
            Some(AtomState::Failed(_)) => true,
            Some(AtomState::Pending) | None => false,
        };
        let inline = node.node_type == "inlineMath";
        let label = if !failed {
            self.language.text(if inline {
                "Rendering…"
            } else {
                "Rendering document blocks…"
            })
        } else {
            self.language.text(match node.node_type.as_str() {
                "mermaidBlock" => "Diagram could not be rendered",
                "imageBlock" => "Image could not be loaded",
                _ => "Formula could not be rendered",
            })
        };
        let size = self.dimensions.get(&identity).map_or_else(
            || AtomSize {
                width: available_width.clamp(1.0, if inline { 230.0 } else { 520.0 }),
                height: if inline {
                    (self.font_size * 1.6).max(28.0)
                } else {
                    116.0
                },
            },
            |known| {
                // Formula em-size changes make this an explicit provisional estimate, never a
                // claim of newly shaped output. Authored image/diagram dimensions are not em units.
                let em = if matches!(node.node_type.as_str(), "inlineMath" | "mathBlock")
                    && known.font_size.is_finite()
                    && known.font_size > 0.0
                    && self.font_size.is_finite()
                    && self.font_size > 0.0
                {
                    self.font_size / known.font_size
                } else {
                    1.0
                };
                let width = known.size.width * em;
                let fit = (available_width / width).min(1.0);
                AtomSize {
                    width: width * fit,
                    height: known.size.height * em * fit,
                }
            },
        );
        Some(AtomPlaceholder {
            size,
            state: if failed {
                AtomPlaceholderState::Failed
            } else {
                AtomPlaceholderState::Pending
            },
            label: label.into(),
            detail: source_preview(node),
        })
    }
    fn prioritize(&self, visible: &[NodePath]) {
        *self
            .visible
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(visible.to_vec());
    }
    fn generation(&self) -> u64 {
        self.generation
    }
}

/// One global bounded renderer, progressive exact-atom acknowledgements and a disposable LRU cache.
#[derive(Default)]
pub(super) struct RenderState {
    source_revision: Option<Revision>,
    atom_source: Vec<u8>,
    presentation: Option<PresentationKey>,
    nodes: HashMap<String, Node>,
    surface: RenderSurface,
    job: Option<RenderJob>,
    epoch: u64,
}
impl RenderState {
    pub fn failed(&self) -> bool {
        self.surface.entries.values().any(CacheEntry::failed)
    }
    pub fn retry(&mut self) {
        for entry in self
            .surface
            .entries
            .values_mut()
            .filter(|entry| entry.failed())
        {
            entry.state = AtomState::Pending;
        }
        self.surface.generation = self.surface.generation.saturating_add(1);
    }
    pub fn catalog(&self) -> &impl AtomResolver {
        &self.surface
    }
    fn poll(&mut self) {
        // Even fast renderer acknowledgements yield back to native input after bounded work.
        for _ in 0..ACKNOWLEDGEMENTS_PER_FRAME {
            let Some(job) = &self.job else { break };
            let epoch = job.epoch;
            let event = job.receiver.try_recv();
            match event {
                Ok(WorkerEvent::Atom { identity, result }) if epoch == self.epoch => {
                    if let (Some(node), Ok(catalog)) = (self.nodes.get(&identity), &result) {
                        self.surface.retain_geometry(&identity, node, catalog);
                    }
                    self.surface.acknowledge(&identity, result);
                }
                Ok(WorkerEvent::Atom { .. }) => {}
                Ok(WorkerEvent::Finished) => {
                    self.job = None;
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.job = None;
                    if epoch == self.epoch {
                        for entry in self.surface.entries.values_mut() {
                            if matches!(entry.state, AtomState::Pending) {
                                entry.state =
                                    AtomState::Failed(craft_editor_render::Error::Identity);
                            }
                        }
                        self.surface.generation = self.surface.generation.saturating_add(1);
                    }
                    break;
                }
            }
        }
    }
    fn synchronize(
        &mut self,
        document: &RichDocument,
        revision: Revision,
        presentation: PresentationKey,
    ) -> AppResult<()> {
        let source_changed = if self.source_revision != Some(revision) {
            let source = atom_source(document)?;
            let changed = source != self.atom_source;
            self.atom_source = source;
            self.source_revision = Some(revision);
            let mut atoms = vec![];
            collect_atoms(document.root(), &mut vec![], &mut atoms);
            self.nodes.clear();
            self.surface.paths.clear();
            for (path, node) in atoms {
                let identity = serde_json::to_string(node)?;
                self.surface.paths.insert(path, identity.clone());
                self.nodes.insert(identity, node.clone());
            }
            changed
        } else {
            false
        };
        let presentation_changed = self.presentation.as_ref() != Some(&presentation);
        if source_changed || presentation_changed {
            self.epoch = self
                .epoch
                .checked_add(1)
                .ok_or("Renderer epoch exhausted")?;
            if let Some(job) = &self.job {
                job.cancelled.store(true, Ordering::Release);
            }
            if presentation_changed {
                self.surface.entries.clear();
            }
            self.surface
                .entries
                .retain(|key, _| self.nodes.contains_key(key));
            self.surface
                .dimensions
                .retain(|key, _| self.nodes.contains_key(key));
            for identity in self.nodes.keys() {
                self.surface
                    .entries
                    .entry(identity.clone())
                    .or_insert(CacheEntry {
                        state: AtomState::Pending,
                        last_used: self.surface.clock,
                    });
            }
            self.surface.language = presentation.language;
            self.surface.font_size = f32::from_bits(presentation.font_size);
            self.surface.generation = self.surface.generation.saturating_add(1);
            self.presentation = Some(presentation);
        }
        Ok(())
    }
    pub fn update(
        &mut self,
        document: &RichDocument,
        revision: Revision,
        host: RenderHost<'_>,
    ) -> AppResult<bool> {
        let RenderHost {
            language,
            tokens: t,
            font_size,
            pixel_scale,
            workspace,
            project_id,
        } = host;
        let key = PresentationKey {
            font_size: font_size.to_bits(),
            pixel_scale: pixel_scale.to_bits(),
            foreground: t.text.to_array(),
            background: t.editor.to_array(),
            language,
        };
        self.synchronize(document, revision, key.clone())?;
        self.surface.touch_visible();
        self.poll();
        if self.job.is_none() {
            let requests: Vec<_> = self
                .surface
                .visible_ids()
                .into_iter()
                .filter(|identity| {
                    self.surface
                        .entries
                        .get(identity)
                        .is_some_and(|entry| matches!(entry.state, AtomState::Pending))
                })
                .filter_map(|identity| {
                    self.nodes
                        .get(&identity)
                        .cloned()
                        .map(|node| (identity, node))
                })
                .collect();
            if !requests.is_empty()
                && let Some(permit) = WorkerPermit::acquire()
            {
                // The snapshot captures metadata only. Loading/verification stays on the worker.
                // An unavailable project snapshot becomes an explicit per-image fallback; diagrams
                // and formulas do not depend on image storage and continue preparing normally.
                let snapshot = if requests
                    .iter()
                    .any(|(_, node)| node.node_type == "imageBlock")
                {
                    workspace.image_asset_snapshot(project_id, document).ok()
                } else {
                    None
                };
                let assets = OwnedAssets { snapshot };
                let options = RenderOptions {
                    font_size,
                    pixel_scale,
                    foreground: key.foreground,
                    background: key.background,
                    dark: egui::ecolor::Hsva::from(t.editor).v < 0.5,
                    font_faces: vec![cjk_font()],
                    fallback_labels: FallbackLabels {
                        diagram: language.text("Diagram could not be rendered").into(),
                        formula: language.text("Formula could not be rendered").into(),
                        image: language.text("Image could not be loaded").into(),
                    },
                    ..Default::default()
                };
                let epoch = self.epoch;
                let cancelled = Arc::new(AtomicBool::new(false));
                let worker_cancel = Arc::clone(&cancelled);
                let (sender, receiver) = mpsc::sync_channel(1);
                let failed_requests: Vec<_> = requests
                    .iter()
                    .map(|(identity, _)| identity.clone())
                    .collect();
                let started = std::thread::Builder::new()
                    .name("writercraft-native-media".into())
                    .spawn(move || {
                        let _permit = permit;
                        run_batch(requests, options, assets, epoch, worker_cancel, sender);
                    });
                if let Err(error) = started {
                    // Keep a failed atom state until explicit Retry. An OS worker failure must
                    // not reopen an error dialog or attempt a new thread on every native frame.
                    for identity in failed_requests {
                        self.surface
                            .acknowledge(&identity, Err(craft_editor_render::Error::ResourceLimit));
                    }
                    return Err(error.into());
                }
                self.job = Some(RenderJob {
                    epoch,
                    receiver,
                    cancelled,
                });
            }
        }
        Ok(self.surface.visible_settled())
    }
}
fn run_batch(
    requests: Vec<(String, Node)>,
    options: RenderOptions,
    assets: OwnedAssets,
    epoch: u64,
    cancelled: Arc<AtomicBool>,
    sender: mpsc::SyncSender<WorkerEvent>,
) {
    if cancelled.load(Ordering::Acquire) {
        return;
    }
    let started = Instant::now();
    let renderer = Renderer::new(options);
    for (index, (identity, node)) in requests.into_iter().enumerate() {
        if cancelled.load(Ordering::Acquire) || (index > 0 && started.elapsed() >= BATCH_TIME) {
            break;
        }
        let result = match &renderer {
            Ok(renderer) => renderer.prepare_atom(&node, &assets, epoch),
            Err(error) => Err(error.clone()),
        };
        if cancelled.load(Ordering::Acquire)
            || sender.send(WorkerEvent::Atom { identity, result }).is_err()
        {
            return;
        }
    }
    let _ = sender.send(WorkerEvent::Finished);
}
fn cjk_font() -> Arc<Vec<u8>> {
    static FONT: OnceLock<Arc<Vec<u8>>> = OnceLock::new();
    Arc::clone(FONT.get_or_init(|| {
        Arc::new(include_bytes!("../../../../assets/fonts/NotoSansCJKjp-Regular.otf").to_vec())
    }))
}
fn collect_atoms<'a>(
    node: &'a Node,
    path: &mut Vec<usize>,
    output: &mut Vec<(NodePath, &'a Node)>,
) {
    if matches!(
        node.node_type.as_str(),
        "imageBlock" | "mermaidBlock" | "mathBlock" | "inlineMath"
    ) {
        output.push((NodePath(path.clone()), node));
    }
    for (index, child) in node.children().iter().enumerate() {
        path.push(index);
        collect_atoms(child, path, output);
        path.pop();
    }
}
fn atom_source(document: &RichDocument) -> AppResult<Vec<u8>> {
    let mut atoms = vec![];
    collect_atoms(document.root(), &mut vec![], &mut atoms);
    Ok(serde_json::to_vec(
        &atoms.iter().map(|(_, node)| node).collect::<Vec<_>>(),
    )?)
}
fn source_preview(node: &Node) -> String {
    let source = node
        .string_attr("code")
        .or_else(|| node.string_attr("latex"))
        .or_else(|| node.string_attr("alt"))
        .unwrap_or_default();
    let mut preview = String::new();
    let mut lines = 1;
    let mut chars = source.chars();
    for _ in 0..180 {
        let Some(character) = chars.next() else {
            return preview;
        };
        if character == '\n' {
            lines += 1;
            if lines > 3 {
                preview.push('…');
                return preview;
            }
        }
        preview.push(character);
    }
    if chars.next().is_some() {
        preview.push('…');
    }
    preview
}

/// Only application-owned bytes and the explicit bundled showcase resource are visible.
struct OwnedAssets {
    snapshot: Option<studio_core::ImageAssetSnapshot>,
}
impl craft_editor_render::AssetSource for OwnedAssets {
    fn load(&self, node: &Node) -> craft_editor_render::Result<Arc<Vec<u8>>> {
        if node.string_attr("assetId") == Some("showcase-lighthouse")
            && node.string_attr("src") == Some("asset://showcase-lighthouse")
        {
            return Ok(Arc::new(
                include_bytes!("../../../../assets/demo/lighthouse.svg").to_vec(),
            ));
        }
        self.snapshot
            .as_ref()
            .ok_or(craft_editor_render::Error::AssetUnavailable)?
            .load(node)
            .map_err(|_| craft_editor_render::Error::AssetUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_editor_egui::{DocPoint, EditorSession, Selection, canvas::CanvasStyle};
    use craft_editor_model::ScalarOffset;
    use serde_json::json;

    fn fixture(first: &str, second: &str, prose: &str) -> AppResult<RichDocument> {
        Ok(RichDocument::from_json(
            json!({"type":"doc","attrs":{"schemaVersion":4},"content":[
                {"type":"paragraph","attrs":{"blockId":"prose"},"content":[{"type":"text","text":prose}]},
                {"type":"mathBlock","attrs":{"blockId":"first","latex":first,"description":"First formula","caption":"Preserved caption"}},
                {"type":"mathBlock","attrs":{"blockId":"second","latex":second,"description":"Second formula"}},
                {"type":"paragraph","attrs":{"blockId":"last"},"content":[{"type":"text","text":"After😀"}]}
            ]}),
        )?)
    }
    fn key() -> PresentationKey {
        PresentationKey {
            font_size: 20.0_f32.to_bits(),
            pixel_scale: 1.0_f32.to_bits(),
            foreground: [35, 39, 47, 255],
            background: [250, 250, 248, 255],
            language: Language::English,
        }
    }
    fn identity(document: &RichDocument, index: usize) -> AppResult<String> {
        Ok(serde_json::to_string(&document.root().children()[index])?)
    }
    fn prepared(node: &Node) -> AppResult<PreparedCatalog> {
        Ok(Renderer::new(RenderOptions::default())?.prepare_atom(
            node,
            &craft_editor_render::AssetMap::default(),
            1,
        )?)
    }
    fn pending_job(state: &mut RenderState) -> mpsc::SyncSender<WorkerEvent> {
        let (sender, receiver) = mpsc::sync_channel(1);
        state.job = Some(RenderJob {
            epoch: state.epoch,
            receiver,
            cancelled: Arc::new(AtomicBool::new(false)),
        });
        sender
    }

    #[test]
    fn prose_edit_does_not_invalidate_unchanged_diagram_output() -> AppResult<()> {
        let before = fixture("x^2", "y^2", "Before")?;
        let after = fixture("x^2", "y^2", "After")?;
        let mut state = RenderState::default();
        state.synchronize(&before, Revision(0), key())?;
        state.surface.acknowledge(
            &identity(&before, 1)?,
            Ok(prepared(&before.root().children()[1])?),
        );
        let generation = state.surface.generation;
        let epoch = state.epoch;
        state.synchronize(&after, Revision(1), key())?;
        assert_eq!(state.epoch, epoch);
        assert_eq!(state.surface.generation, generation);
        assert!(
            state
                .catalog()
                .resolve(&after.root().children()[1], 500.0)
                .is_ok()
        );
        assert_eq!(atom_source(&before)?, atom_source(&after)?);
        Ok(())
    }
    #[test]
    fn changed_atom_preserves_acknowledged_output_for_other_exact_atoms() -> AppResult<()> {
        let before = fixture("x^2", "y^2", "Prose")?;
        let after = fixture("x^2", "z^2", "Prose")?;
        let mut state = RenderState::default();
        state.synchronize(&before, Revision(0), key())?;
        state.surface.acknowledge(
            &identity(&before, 1)?,
            Ok(prepared(&before.root().children()[1])?),
        );
        state.surface.acknowledge(
            &identity(&before, 2)?,
            Ok(prepared(&before.root().children()[2])?),
        );
        state.synchronize(&after, Revision(1), key())?;
        assert!(
            state
                .catalog()
                .resolve(&after.root().children()[1], 500.0)
                .is_ok()
        );
        assert_eq!(
            state
                .catalog()
                .placeholder(&after.root().children()[2], 500.0)
                .map(|status| status.state),
            Some(AtomPlaceholderState::Pending)
        );
        assert!(!state.surface.entries.contains_key(&identity(&before, 2)?));
        Ok(())
    }
    #[test]
    fn changed_atom_attributes_invalidate_its_actual_media_identity() -> AppResult<()> {
        let before = fixture("x^2", "y^2", "Prose")?;
        let mut content = before.to_json()?;
        content["content"][1]["attrs"]["caption"] = json!("A new caption");
        let after = RichDocument::from_json(content)?;
        assert_ne!(atom_source(&before)?, atom_source(&after)?);
        Ok(())
    }
    #[test]
    fn a_worker_acknowledges_one_atom_before_the_rest_of_its_batch_finishes() -> AppResult<()> {
        let document = fixture("x^2", "y^2", "Prose")?;
        let mut state = RenderState::default();
        state.synchronize(&document, Revision(0), key())?;
        let sender = pending_job(&mut state);
        sender.send(WorkerEvent::Atom {
            identity: identity(&document, 1)?,
            result: Ok(prepared(&document.root().children()[1])?),
        })?;
        state.poll();
        assert!(
            state
                .catalog()
                .resolve(&document.root().children()[1], 500.0)
                .is_ok()
        );
        assert!(
            state
                .catalog()
                .resolve(&document.root().children()[2], 500.0)
                .is_err()
        );
        assert!(state.job.is_some());
        Ok(())
    }
    #[test]
    fn stale_worker_output_cannot_replace_changed_source_or_theme() -> AppResult<()> {
        for change_theme in [false, true] {
            let before = fixture("x^2", "y^2", "Prose")?;
            let after = if change_theme {
                before.clone()
            } else {
                fixture("z^2", "y^2", "Prose")?
            };
            let mut state = RenderState::default();
            state.synchronize(&before, Revision(0), key())?;
            let sender = pending_job(&mut state);
            let cancellation = Arc::clone(&state.job.as_ref().ok_or("missing job")?.cancelled);
            let mut next_key = key();
            if change_theme {
                next_key.foreground = [255, 255, 255, 255];
            }
            state.synchronize(&after, Revision(1), next_key)?;
            assert!(cancellation.load(Ordering::Acquire));
            sender.send(WorkerEvent::Atom {
                identity: identity(&before, 1)?,
                result: Ok(prepared(&before.root().children()[1])?),
            })?;
            state.poll();
            assert!(
                state
                    .catalog()
                    .resolve(&after.root().children()[1], 500.0)
                    .is_err()
            );
        }
        Ok(())
    }
    #[test]
    fn lru_eviction_keeps_valid_source_pending_for_lazy_rerender_instead_of_rejecting_it()
    -> AppResult<()> {
        let document = fixture("x^2", "y^2", "Prose")?;
        let canonical = document.to_json()?;
        let mut state = RenderState::default();
        state.synchronize(&document, Revision(0), key())?;
        let first = prepared(&document.root().children()[1])?;
        let second = prepared(&document.root().children()[2])?;
        state.surface.cache_bytes = first.encoded_bytes().max(second.encoded_bytes());
        state
            .surface
            .acknowledge(&identity(&document, 1)?, Ok(first));
        state.surface.prioritize(&[NodePath(vec![2])]);
        state.surface.touch_visible();
        state
            .surface
            .acknowledge(&identity(&document, 2)?, Ok(second));
        assert!(
            state
                .catalog()
                .resolve(&document.root().children()[2], 500.0)
                .is_ok()
        );
        assert_eq!(
            state
                .catalog()
                .placeholder(&document.root().children()[1], 500.0)
                .map(|status| status.state),
            Some(AtomPlaceholderState::Pending)
        );
        assert!(!state.failed());
        assert_eq!(document.to_json()?, canonical);
        Ok(())
    }
    #[test]
    fn acknowledged_bounds_survive_theme_and_lru_invalidation_without_stale_pixels() -> AppResult<()>
    {
        let document = fixture("x^2", "y^2", "Prose")?;
        for theme in [true, false] {
            let mut state = RenderState::default();
            state.synchronize(&document, Revision(0), key())?;
            let node = &document.root().children()[1];
            let id = identity(&document, 1)?;
            let first = prepared(node)?;
            state.surface.retain_geometry(&id, node, &first);
            state.surface.acknowledge(&id, Ok(first));
            let acknowledged = state.catalog().resolve(node, 500.0)?.size;
            if theme {
                let mut changed = key();
                changed.foreground = [255, 255, 255, 255];
                state.synchronize(&document, Revision(0), changed)?;
            } else {
                let second = prepared(&document.root().children()[2])?;
                state.surface.cache_bytes = second.encoded_bytes();
                state
                    .surface
                    .acknowledge(&identity(&document, 2)?, Ok(second));
            }
            let pending = state
                .catalog()
                .placeholder(node, 500.0)
                .ok_or("missing pending atom")?;
            assert_eq!(pending.state, AtomPlaceholderState::Pending);
            assert_eq!(
                (pending.size.width, pending.size.height),
                (acknowledged.width, acknowledged.height)
            );
            assert!(state.catalog().resolve(node, 500.0).is_err());
            let mut larger = key();
            larger.font_size = 40.0_f32.to_bits();
            state.synchronize(&document, Revision(0), larger)?;
            let estimated = state
                .catalog()
                .placeholder(node, 500.0)
                .ok_or("missing em estimate")?;
            assert!((estimated.size.width - acknowledged.width * 2.0).abs() < 0.01);
            assert!((estimated.size.height - acknowledged.height * 2.0).abs() < 0.01);
        }
        Ok(())
    }
    #[test]
    fn renderer_failure_and_retry_are_scoped_to_the_failed_atom() -> AppResult<()> {
        let document = fixture("x^2", "y^2", "Prose")?;
        let mut state = RenderState::default();
        state.synchronize(&document, Revision(0), key())?;
        state.surface.acknowledge(
            &identity(&document, 1)?,
            Ok(prepared(&document.root().children()[1])?),
        );
        state.surface.acknowledge(
            &identity(&document, 2)?,
            Err(craft_editor_render::Error::FormulaRender),
        );
        assert!(state.failed());
        assert_eq!(
            state
                .catalog()
                .placeholder(&document.root().children()[2], 500.0)
                .map(|status| status.state),
            Some(AtomPlaceholderState::Failed)
        );
        state.retry();
        assert!(
            state
                .catalog()
                .resolve(&document.root().children()[1], 500.0)
                .is_ok()
        );
        assert_eq!(
            state
                .catalog()
                .placeholder(&document.root().children()[2], 500.0)
                .map(|status| status.state),
            Some(AtomPlaceholderState::Pending)
        );
        Ok(())
    }
    #[test]
    fn dropping_a_document_cancels_its_worker_without_blocking_the_native_thread() -> AppResult<()>
    {
        let document = fixture("x", "y", "Prose")?;
        let mut state = RenderState::default();
        state.synchronize(&document, Revision(0), key())?;
        let _sender = pending_job(&mut state);
        let cancelled = Arc::clone(&state.job.as_ref().ok_or("missing job")?.cancelled);
        drop(state);
        assert!(cancelled.load(Ordering::Acquire));
        Ok(())
    }
    #[test]
    fn placeholder_source_preview_is_unicode_safe_explicitly_shortened_and_noncanonical()
    -> AppResult<()> {
        let source = "字😀".repeat(120);
        let document = fixture(&source, "y", "Prose")?;
        let canonical = document.to_json()?;
        let preview = source_preview(&document.root().children()[1]);
        assert!(preview.ends_with('…'));
        assert_eq!(preview.chars().count(), 181);
        assert_eq!(document.to_json()?, canonical);
        Ok(())
    }
    #[test]
    fn application_resolver_keeps_real_native_prose_input_available_after_media_failure()
    -> AppResult<()> {
        for failed in [false, true] {
            let document = fixture("x", "y", "Prose")?;
            let mut state = RenderState::default();
            state.synchronize(&document, Revision(0), key())?;
            if failed {
                state.surface.acknowledge(
                    &identity(&document, 1)?,
                    Err(craft_editor_render::Error::FormulaRender),
                );
            }
            let mut session = EditorSession::new(document.clone(), Revision(0));
            let point = DocPoint {
                block_id: "last".into(),
                offset: ScalarOffset(0),
            };
            session.set_selection(Selection {
                anchor: point.clone(),
                head: point,
            })?;
            let context = egui::Context::default();
            for typing in [false, true] {
                let mut errors = Vec::new();
                if typing {
                    context.memory_mut(|memory| memory.request_focus(session.options.id));
                }
                let mut output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(700.0, 650.0),
                        )),
                        events: if typing {
                            vec![egui::Event::Text("Я".into())]
                        } else {
                            vec![]
                        },
                        focused: true,
                        ..Default::default()
                    },
                    |root| {
                        egui::CentralPanel::default().show(root, |ui| {
                            errors.extend(
                                session
                                    .show_canvas_with_resolver(
                                        ui,
                                        &CanvasStyle::default(),
                                        state.catalog(),
                                    )
                                    .errors,
                            );
                        });
                    },
                );
                output.textures_delta.clear();
                assert!(errors.is_empty(), "{errors:?}");
            }
            assert_eq!(session.document().block_text("last")?, "ЯAfter😀");
            assert_eq!(
                session.document().root().children()[1],
                document.root().children()[1]
            );
        }
        Ok(())
    }
}
