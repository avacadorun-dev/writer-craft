//! Background preparation of real native rich-block media, separate from canonical editing.
use super::*;
use craft_editor_model::{Document as RichDocument, Node, Revision};
use craft_editor_render::{AssetMap, FallbackLabels, PreparedCatalog, RenderOptions, Renderer};
use std::sync::{Arc, OnceLock};

#[derive(Clone, PartialEq)]
struct RenderKey {
    atoms: Arc<Vec<u8>>,
    font_size: u32,
    pixel_scale: u32,
    foreground: [u8; 4],
    background: [u8; 4],
    language: Language,
}
struct RenderJob {
    key: RenderKey,
    receiver: mpsc::Receiver<craft_editor_render::Result<PreparedCatalog>>,
}

/// One bounded worker at a time; a late result cannot replace a newer source/theme request.
#[derive(Default)]
pub(super) struct RenderState {
    source_revision: Option<Revision>,
    atom_source: Arc<Vec<u8>>,
    desired: Option<RenderKey>,
    acknowledged: Option<RenderKey>,
    job: Option<RenderJob>,
    catalog: Option<PreparedCatalog>,
    generation: u64,
    error: Option<String>,
}

impl RenderState {
    pub fn failed(&self) -> bool {
        self.error.is_some()
    }
    pub fn retry(&mut self) {
        self.error = None;
    }

    pub fn catalog(&self) -> Option<&PreparedCatalog> {
        (self.desired == self.acknowledged)
            .then_some(self.catalog.as_ref())
            .flatten()
    }

    pub fn update(
        &mut self,
        document: &RichDocument,
        revision: Revision,
        language: Language,
        t: Tokens,
        font_size: f32,
        pixel_scale: f32,
    ) -> AppResult<bool> {
        if self.source_revision != Some(revision) {
            let source = atom_source(document)?;
            if self.atom_source.as_ref() != &source {
                self.atom_source = Arc::new(source);
            }
            self.source_revision = Some(revision);
        }
        if self.atom_source.as_slice() == b"[]" {
            self.desired = None;
            self.acknowledged = None;
            self.catalog = None;
            self.error = None;
            // An existing worker may finish, but no result is used for this atom-free surface.
            if self
                .job
                .as_ref()
                .is_some_and(|job| job.receiver.try_recv().is_ok())
            {
                self.job = None;
            }
            return Ok(true);
        }
        let key = RenderKey {
            atoms: self.atom_source.clone(),
            font_size: font_size.to_bits(),
            pixel_scale: pixel_scale.to_bits(),
            foreground: t.text.to_array(),
            background: t.editor.to_array(),
            language,
        };
        if self.desired.as_ref() != Some(&key) {
            self.desired = Some(key.clone());
            self.error = None;
        }
        if let Some(job) = &self.job {
            let result = match job.receiver.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err(craft_editor_render::Error::Identity))
                }
                Err(mpsc::TryRecvError::Empty) => None,
            };
            if let Some(result) = result {
                let job = self.job.take().ok_or("Missing renderer worker")?;
                if self.desired.as_ref() == Some(&job.key) {
                    match result {
                        Ok(catalog) => {
                            self.catalog = Some(catalog);
                            self.acknowledged = Some(job.key);
                            self.error = None;
                        }
                        Err(error) => {
                            self.error = Some(error.to_string());
                        }
                    }
                }
            }
        }
        if let Some(error) = &self.error {
            return Err(error.clone().into());
        }
        if self.acknowledged.as_ref() == Some(&key) {
            return Ok(true);
        }
        if self.job.is_none() {
            self.generation = self
                .generation
                .checked_add(1)
                .ok_or("Renderer generation exhausted")?;
            let generation = self.generation;
            let document = document.clone();
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
            let (sender, receiver) = mpsc::channel();
            std::thread::Builder::new()
                .name("writercraft-native-media".into())
                .spawn(move || {
                    let mut assets = AssetMap::default();
                    assets.insert(
                        "showcase-lighthouse",
                        Arc::new(include_bytes!("../../../../assets/demo/lighthouse.svg").to_vec()),
                    );
                    let result = Renderer::new(options).and_then(|renderer| {
                        renderer.prepare_document(&document, &assets, generation)
                    });
                    // The owner may have switched documents. Dropping its result is intentional.
                    let _ = sender.send(result);
                })?;
            self.job = Some(RenderJob { key, receiver });
        }
        Ok(false)
    }
}

fn cjk_font() -> Arc<Vec<u8>> {
    static FONT: OnceLock<Arc<Vec<u8>>> = OnceLock::new();
    Arc::clone(FONT.get_or_init(|| {
        Arc::new(include_bytes!("../../../../assets/fonts/NotoSansCJKjp-Regular.otf").to_vec())
    }))
}

fn atom_source(document: &RichDocument) -> AppResult<Vec<u8>> {
    fn visit<'a>(node: &'a Node, atoms: &mut Vec<&'a Node>) {
        if matches!(
            node.node_type.as_str(),
            "imageBlock" | "mermaidBlock" | "mathBlock" | "inlineMath"
        ) {
            atoms.push(node);
        }
        for child in node.children() {
            visit(child, atoms);
        }
    }
    let mut atoms = vec![];
    visit(document.root(), &mut atoms);
    Ok(serde_json::to_vec(&atoms)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn prose_edit_does_not_invalidate_unchanged_diagram_output() -> AppResult<()> {
        let make = |text: &str| {
            RichDocument::from_json(json!({"type":"doc","attrs":{"schemaVersion":4},"content":[
                {"type":"paragraph","attrs":{"blockId":"prose"},"content":[{"type":"text","text":text}]},
                {"type":"mermaidBlock","attrs":{"blockId":"diagram","code":"flowchart LR\nA-->B","caption":"A path"}}
            ]}))
        };
        assert_eq!(
            atom_source(&make("Before")?)?,
            atom_source(&make("After")?)?
        );
        Ok(())
    }

    #[test]
    fn changed_atom_attributes_invalidate_its_actual_media_identity() -> AppResult<()> {
        let make = |caption: &str| {
            RichDocument::from_json(json!({"type":"doc","attrs":{"schemaVersion":4},"content":[
                {"type":"imageBlock","attrs":{"blockId":"image","src":"asset://test","assetId":"test","alt":"A scene","caption":caption}}
            ]}))
        };
        assert_ne!(
            atom_source(&make("First caption")?)?,
            atom_source(&make("Second caption")?)?
        );
        Ok(())
    }
}
