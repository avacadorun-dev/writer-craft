//! Project-owned image portability for immutable, off-thread clipboard jobs.
//!
//! The native provider stays application-owned. These functions never read the OS clipboard,
//! open arbitrary paths or fetch a URL. The capsule binds the exact canonical bytes, so a native
//! owner change cannot silently associate one selection with another selection's image bytes.
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use craft_editor_egui::clipboard::{
    AssetReference, ClipboardBundle, ClipboardFormat, ImageBinding, PasteContext, PreparedPaste,
    RichSlice,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use studio_core::{ImageTransfer, MAX_IMAGE_BYTES, ProjectImages, StudioError};

/// Registered attachment format; canonical rich JSON remains a separate engine format.
pub(super) const IMAGE_CAPSULE_MIME: &str = "application/vnd.writercraft.images+json";
/// Platform mapping used by the shared native provider on macOS.
pub(super) fn image_capsule_format() -> ClipboardFormat {
    ClipboardFormat {
        mime: IMAGE_CAPSULE_MIME.into(),
        macos_uti: "app.writercraft.clipboard-images-json".into(),
    }
}
/// A canonical remote reference that requires the host's explicit async import policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExternalImage {
    pub source: String,
}
/// Content-free diagnostics for host localization. Private image bytes are never displayed.
#[derive(Debug)]
pub(super) enum ProjectClipboardError {
    Clipboard(craft_editor_egui::clipboard::ClipboardError),
    Canonical(craft_editor_egui::Error),
    Assets(StudioError),
    InvalidCapsule,
    ExternalImagesRequired(Vec<ExternalImage>),
}
impl fmt::Display for ProjectClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Clipboard(_) => "clipboard_transport_failed",
            Self::Canonical(_) => "clipboard_invalid_rich_content",
            Self::Assets(_) => "clipboard_project_image_failed",
            Self::InvalidCapsule => "clipboard_invalid_image_capsule",
            Self::ExternalImagesRequired(_) => "clipboard_external_images_require_import",
        })
    }
}
impl std::error::Error for ProjectClipboardError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Clipboard(error) => Some(error),
            Self::Canonical(error) => Some(error),
            Self::Assets(error) => Some(error),
            _ => None,
        }
    }
}
impl From<craft_editor_egui::clipboard::ClipboardError> for ProjectClipboardError {
    fn from(value: craft_editor_egui::clipboard::ClipboardError) -> Self {
        Self::Clipboard(value)
    }
}
impl From<craft_editor_egui::Error> for ProjectClipboardError {
    fn from(value: craft_editor_egui::Error) -> Self {
        Self::Canonical(value)
    }
}
impl From<StudioError> for ProjectClipboardError {
    fn from(value: StudioError) -> Self {
        Self::Assets(value)
    }
}
type Result<T> = std::result::Result<T, ProjectClipboardError>;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capsule {
    version: u32,
    rich_sha256: String,
    /// A diagnostic hint only. It is never used for authorization or a database lookup.
    origin_project_id: String,
    images: Vec<EncodedImage>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EncodedImage {
    source_id: String,
    original_name: String,
    media_type: String,
    sha256: String,
    byte_size: u64,
    width: u32,
    height: u32,
    data_base64: String,
}
/// Cloneable job scope. Heavy database access, hashing and base64 work belongs on a host worker.
#[derive(Clone)]
pub(super) struct ProjectClipboard {
    images: ProjectImages,
    max_bytes: usize,
}
impl ProjectClipboard {
    pub fn new(images: ProjectImages, max_bytes: usize) -> Self {
        Self { images, max_bytes }
    }
    /// Add only images referenced by this exact selection; never include the rest of a project.
    pub fn prepare_copy(&self, captured: &ClipboardBundle) -> Result<ClipboardBundle> {
        captured.check_budget(self.max_bytes)?;
        let Some(rich) = &captured.rich else {
            if captured.formats.contains_key(IMAGE_CAPSULE_MIME) {
                return Err(ProjectClipboardError::InvalidCapsule);
            }
            return Ok(captured.clone());
        };
        let slice = RichSlice::decode(rich, self.max_bytes)?;
        let references = references(&slice)?;
        if capsule_transfers(captured, &references, self.max_bytes)?.is_some() {
            // Recovery re-publishes the already self-contained exact capture. Its origin/project
            // hint authorizes no database read and no destination import is required.
            return Ok(captured.clone());
        }
        let ids = references.owned.into_iter().collect::<Vec<_>>();
        let images = self.images.export_images(&ids, self.max_bytes)?;
        let mut outbound = captured.clone();
        // An old capsule can never survive replacement of the canonical selection.
        outbound.formats.remove(IMAGE_CAPSULE_MIME);
        if !images.is_empty() {
            let capsule = Capsule {
                version: 1,
                rich_sha256: digest(rich),
                origin_project_id: self.images.project_id().into(),
                images: images
                    .into_iter()
                    .map(|image| EncodedImage {
                        source_id: image.source_id,
                        original_name: image.original_name,
                        media_type: image.media_type,
                        sha256: image.sha256,
                        byte_size: image.byte_size,
                        width: image.width,
                        height: image.height,
                        data_base64: STANDARD.encode(&image.bytes),
                    })
                    .collect(),
            };
            outbound.formats.insert(
                IMAGE_CAPSULE_MIME.into(),
                serde_json::to_vec(&capsule).map_err(|_| ProjectClipboardError::InvalidCapsule)?,
            );
        }
        outbound.check_budget(self.max_bytes)?;
        Ok(outbound)
    }
    /// Prepare source rules once, validate the original capsule, and bind only host-authorized
    /// image references. The sealed engine result retains its exact original context/provenance.
    pub fn prepare_paste(&self, context: &PasteContext) -> Result<PreparedPaste> {
        let mut prepared = context.prepare(self.max_bytes)?;
        let captured = prepared.source();
        let source_references = references_from(prepared.source_asset_references())?;
        let needed_assets = prepared.asset_references();
        let needed = references_from(&needed_assets)?;
        let mut transfers = if let Some(images) =
            capsule_transfers(captured, &source_references, self.max_bytes)?
        {
            images
        } else {
            self.images.export_images(
                &needed.owned.iter().cloned().collect::<Vec<_>>(),
                self.max_bytes,
            )?
        };
        if !needed.external.is_empty() {
            return Err(ProjectClipboardError::ExternalImagesRequired(
                needed.external,
            ));
        }
        // Validate the entire source capsule before filtering resources made unnecessary by
        // source code/literal context. Foreign IDs never become destination lookup authority.
        transfers.retain(|transfer| needed.owned.contains(&transfer.source_id));
        if transfers.is_empty() {
            return Ok(prepared);
        }
        let imported = self.images.import_images(&transfers, self.max_bytes)?;
        let mapping: BTreeMap<_, _> = imported
            .into_iter()
            .map(|image| (image.source_id, image.asset))
            .collect();
        let mut bound = HashSet::new();
        let mut bindings = Vec::new();
        for original in needed_assets {
            if !bound.insert(original.clone()) {
                continue;
            }
            let id = original
                .asset_id
                .as_ref()
                .ok_or(ProjectClipboardError::InvalidCapsule)?;
            let asset = mapping
                .get(id)
                .ok_or(ProjectClipboardError::InvalidCapsule)?;
            bindings.push(ImageBinding {
                original,
                destination: AssetReference {
                    asset_id: Some(asset.id.clone()),
                    source: asset.source(),
                },
            });
        }
        prepared.bind_images(&bindings)?;
        Ok(prepared)
    }
}
#[derive(Default)]
struct References {
    owned: HashSet<String>,
    external: Vec<ExternalImage>,
}
fn capsule_transfers(
    captured: &ClipboardBundle,
    references: &References,
    max_bytes: usize,
) -> Result<Option<Vec<ImageTransfer>>> {
    let Some(bytes) = captured.formats.get(IMAGE_CAPSULE_MIME) else {
        return Ok(None);
    };
    let rich = captured
        .rich
        .as_deref()
        .ok_or(ProjectClipboardError::InvalidCapsule)?;
    let capsule: Capsule =
        serde_json::from_slice(bytes).map_err(|_| ProjectClipboardError::InvalidCapsule)?;
    if capsule.version != 1
        || capsule.rich_sha256 != digest(rich)
        || capsule.origin_project_id.len() > 255
        || capsule.images.len() != references.owned.len()
    {
        return Err(ProjectClipboardError::InvalidCapsule);
    }
    let mut seen = HashSet::new();
    let mut total = 0usize;
    let mut images = Vec::with_capacity(capsule.images.len());
    for image in capsule.images {
        if !references.owned.contains(&image.source_id)
            || !seen.insert(image.source_id.clone())
            || image.data_base64.len() > MAX_IMAGE_BYTES.div_ceil(3) * 4
            || image.original_name.len() > 4096
        {
            return Err(ProjectClipboardError::InvalidCapsule);
        }
        let bytes = STANDARD
            .decode(&image.data_base64)
            .map_err(|_| ProjectClipboardError::InvalidCapsule)?;
        total = total
            .checked_add(bytes.len())
            .filter(|total| *total <= max_bytes)
            .ok_or(ProjectClipboardError::InvalidCapsule)?;
        images.push(ImageTransfer {
            source_id: image.source_id,
            original_name: image.original_name,
            media_type: image.media_type,
            sha256: image.sha256,
            byte_size: image.byte_size,
            width: image.width,
            height: image.height,
            bytes,
        });
    }
    studio_core::validate_image_transfers(&images, max_bytes)
        .map_err(|_| ProjectClipboardError::InvalidCapsule)?;
    Ok(Some(images))
}
fn references(slice: &RichSlice) -> Result<References> {
    references_from(&slice.asset_references())
}
fn references_from(references: &[AssetReference]) -> Result<References> {
    let mut output = References::default();
    for reference in references {
        let source_id = reference.source.strip_prefix("asset://").or_else(|| {
            reference
                .source
                .strip_prefix("/api/media/")
                .and_then(|value| value.strip_suffix("/content"))
        });
        if let Some(id) = source_id {
            if reference.asset_id.as_deref() != Some(id)
                || !studio_core::is_valid_image_reference_id(id)
            {
                return Err(ProjectClipboardError::InvalidCapsule);
            }
            output.owned.insert(id.into());
        } else if reference.source.starts_with("https://")
            || reference.source.starts_with("http://")
        {
            if !output
                .external
                .iter()
                .any(|image| image.source == reference.source)
            {
                output.external.push(ExternalImage {
                    source: reference.source.clone(),
                });
            }
        } else {
            return Err(ProjectClipboardError::InvalidCapsule);
        }
    }
    Ok(output)
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

// Keep the job object sendable without exposing its scoped connection internals.
const _: fn() = || {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<ProjectClipboard>();
    send_sync::<Arc<ClipboardBundle>>();
};

#[cfg(test)]
mod tests {
    use super::*;
    use craft_editor_egui::{DocPoint, EditorSession, Selection};
    use craft_editor_model::{Document, Revision};
    use image::{DynamicImage, ImageFormat, RgbImage};
    use serde_json::{Value, json};
    use std::io::Cursor;
    use studio_core::Workspace;
    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
    const LIMIT: usize = 8 * 1024 * 1024;
    fn fixture(
        workspace: &Workspace,
    ) -> std::result::Result<(ProjectClipboard, ClipboardBundle, String), Box<dyn std::error::Error>>
    {
        let project = workspace.create_project("Private source")?;
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(RgbImage::new(2, 3)).write_to(&mut bytes, ImageFormat::Png)?;
        let asset = workspace.import_image(
            &project.id,
            "image.png",
            Some("image/png"),
            &bytes.into_inner(),
        )?;
        let node =
            workspace.image_node(&project.id, &asset.id, "image", "Description", "Caption")?;
        let mut value = serde_json::to_value(node)?;
        value["attrs"]["vendor"] = json!({"opaque":[null,true]});
        value["attrs"]["sourceRefs"] = json!(["synthetic-source"]);
        value["vendorNode"] = json!(null);
        let image = serde_json::from_value(value)?;
        let slice = RichSlice {
            content: vec![image],
            open_start: 0,
            open_end: 0,
        };
        Ok((
            ProjectClipboard::new(workspace.project_images(&project.id)?, LIMIT),
            slice.bundle()?,
            asset.id,
        ))
    }
    fn image_id(
        bundle: &ClipboardBundle,
    ) -> std::result::Result<String, Box<dyn std::error::Error>> {
        let slice = bundle.canonical_slice(LIMIT)?.ok_or("missing slice")?;
        Ok(slice.content[0]
            .string_attr("assetId")
            .ok_or("missing image")?
            .into())
    }
    fn prepare_bundle(
        scope: &ProjectClipboard,
        captured: &ClipboardBundle,
    ) -> Result<ClipboardBundle> {
        let mut session = EditorSession::new(
            Document::plain("target", "").map_err(craft_editor_egui::Error::from)?,
            Revision(0),
        );
        session.set_selection(Selection {
            anchor: DocPoint {
                block_id: "target".into(),
                offset: craft_editor_model::ScalarOffset(0),
            },
            head: DocPoint {
                block_id: "target".into(),
                offset: craft_editor_model::ScalarOffset(0),
            },
        })?;
        let craft_editor_egui::clipboard::ClipboardRequest::Paste { context, .. } =
            session.begin_paste(captured.clone())?
        else {
            return Err(ProjectClipboardError::InvalidCapsule);
        };
        let prepared = scope.prepare_paste(&context)?;
        if let Some(slice) = prepared.rich_slice() {
            return Ok(slice.bundle()?);
        }
        Ok(prepared.source().clone())
    }
    #[test]
    fn copy_capsule_contains_exact_selected_image_and_is_bound_to_exact_canonical_bytes()
    -> TestResult {
        let workspace = Workspace::memory()?;
        let (source, captured, id) = fixture(&workspace)?;
        let copied = source.prepare_copy(&captured)?;
        assert_eq!(copied.rich, captured.rich);
        assert_eq!(copied.plain, captured.plain);
        let capsule: Capsule = serde_json::from_slice(
            copied
                .formats
                .get(IMAGE_CAPSULE_MIME)
                .ok_or("missing capsule")?,
        )?;
        assert_eq!(capsule.images.len(), 1);
        assert_eq!(capsule.images[0].source_id, id);
        assert_eq!(
            capsule.rich_sha256,
            digest(captured.rich.as_deref().ok_or("missing canonical")?)
        );
        assert_eq!(
            STANDARD.decode(&capsule.images[0].data_base64)?,
            workspace.image_asset_bytes(&capsule.origin_project_id, &id)?
        );
        Ok(())
    }
    #[test]
    fn same_project_paste_reuses_owned_bytes_and_foreign_project_rebinds_only_image_identity()
    -> TestResult {
        let workspace = Workspace::memory()?;
        let (source, captured, id) = fixture(&workspace)?;
        let copied = source.prepare_copy(&captured)?;
        let same = prepare_bundle(&source, &copied)?;
        assert_eq!(image_id(&same)?, id);
        let project = workspace.create_project("Destination")?;
        let destination = ProjectClipboard::new(workspace.project_images(&project.id)?, LIMIT);
        let pasted = prepare_bundle(&destination, &copied)?;
        let new_id = image_id(&pasted)?;
        assert_ne!(new_id, id);
        let old: Value = serde_json::from_slice(captured.rich.as_deref().ok_or("missing source")?)?;
        let mut new: Value =
            serde_json::from_slice(pasted.rich.as_deref().ok_or("missing target")?)?;
        new["content"][0]["attrs"]["assetId"] = old["content"][0]["attrs"]["assetId"].clone();
        new["content"][0]["attrs"]["src"] = old["content"][0]["attrs"]["src"].clone();
        assert_eq!(new, old);
        assert_eq!(
            workspace.image_asset_bytes(&project.id, &new_id)?.len() as u64,
            workspace.image_asset(&project.id, &new_id)?.byte_size
        );
        assert!(!pasted.formats.contains_key(IMAGE_CAPSULE_MIME));
        Ok(())
    }
    #[test]
    fn external_marker_rules_and_owned_image_binding_share_one_sealed_preparation() -> TestResult {
        let workspace = Workspace::memory()?;
        let (scope, _, id) = fixture(&workspace)?;
        let captured = ClipboardBundle {
            html: Some(format!(
                "<p>**bold**</p><figure data-image-block=\"\" data-asset-id=\"{id}\"><img src=\"/api/media/{id}/content\" alt=\"Alt\"><figcaption>Caption</figcaption></figure>"
            )),
            plain: "**bold**\nImage".into(),
            ..Default::default()
        };
        let mut session = EditorSession::new(Document::plain("target", "leftright")?, Revision(0));
        let point = DocPoint {
            block_id: "target".into(),
            offset: craft_editor_model::ScalarOffset(4),
        };
        session.set_selection(Selection {
            anchor: point.clone(),
            head: point,
        })?;
        let original = session.document().clone();
        let craft_editor_egui::clipboard::ClipboardRequest::Paste { context, guard, .. } =
            session.begin_paste(captured.clone())?
        else {
            return Err("missing paste".into());
        };
        let prepared = scope.prepare_paste(&context)?;
        assert_eq!(prepared.source(), &captured);
        let slice = prepared.rich_slice().ok_or("missing rich slice")?;
        assert!(slice.content[0].children().iter().any(|node| {
            node.text.as_deref() == Some("bold")
                && node
                    .marks
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .any(|mark| mark.mark_type == "bold")
        }));
        assert_eq!(
            slice.asset_references()[0].asset_id.as_deref(),
            Some(id.as_str())
        );
        assert_eq!(slice.content[1].string_attr("caption"), Some("Caption"));
        session.finish_prepared_paste(guard, prepared)?;
        session.undo()?;
        assert_eq!(session.document(), &original);
        assert_eq!(
            session
                .selection()
                .map(|selection| selection.anchor.offset.0),
            Some(4)
        );
        Ok(())
    }
    #[test]
    fn missing_capsule_never_authorizes_foreign_uuid_but_existing_destination_owner_can_paste()
    -> TestResult {
        let workspace = Workspace::memory()?;
        let (source, captured, id) = fixture(&workspace)?;
        assert_eq!(image_id(&prepare_bundle(&source, &captured)?)?, id);
        let project = workspace.create_project("Other")?;
        let destination = ProjectClipboard::new(workspace.project_images(&project.id)?, LIMIT);
        assert!(prepare_bundle(&destination, &captured).is_err());
        assert!(workspace.image_asset(&project.id, &id).is_err());
        Ok(())
    }
    #[test]
    fn modified_canonical_or_asset_descriptors_are_refused_without_destination_imports()
    -> TestResult {
        let workspace = Workspace::memory()?;
        let (source, captured, _) = fixture(&workspace)?;
        let copied = source.prepare_copy(&captured)?;
        let project = workspace.create_project("Other")?;
        let destination = ProjectClipboard::new(workspace.project_images(&project.id)?, LIMIT);
        for field in [
            "rich_sha256",
            "byte_size",
            "media_type",
            "width",
            "height",
            "sha256",
            "data_base64",
            "duplicate",
            "extra",
        ] {
            let mut bad = copied.clone();
            let mut capsule: Value = serde_json::from_slice(
                bad.formats
                    .get(IMAGE_CAPSULE_MIME)
                    .ok_or("missing capsule")?,
            )?;
            match field {
                "rich_sha256" => capsule[field] = json!("0".repeat(64)),
                "byte_size" => capsule["images"][0][field] = json!(9999),
                "media_type" => capsule["images"][0][field] = json!("image/jpeg"),
                "width" | "height" => capsule["images"][0][field] = json!(9999),
                "sha256" => capsule["images"][0][field] = json!("0".repeat(64)),
                "data_base64" => capsule["images"][0][field] = json!("not valid !"),
                "duplicate" => {
                    let entry = capsule["images"][0].clone();
                    capsule["images"]
                        .as_array_mut()
                        .ok_or("missing images")?
                        .push(entry);
                }
                _ => capsule["unexpected"] = json!(true),
            }
            bad.formats
                .insert(IMAGE_CAPSULE_MIME.into(), serde_json::to_vec(&capsule)?);
            assert!(
                prepare_bundle(&destination, &bad).is_err(),
                "accepted {field}"
            );
        }
        Ok(())
    }
    #[test]
    fn origin_hint_is_never_authority_and_total_combined_budget_includes_capsule() -> TestResult {
        let workspace = Workspace::memory()?;
        let (source, captured, _) = fixture(&workspace)?;
        let mut copied = source.prepare_copy(&captured)?;
        let mut capsule: Value = serde_json::from_slice(
            copied
                .formats
                .get(IMAGE_CAPSULE_MIME)
                .ok_or("missing capsule")?,
        )?;
        capsule["origin_project_id"] = json!("untrusted origin hint");
        copied
            .formats
            .insert(IMAGE_CAPSULE_MIME.into(), serde_json::to_vec(&capsule)?);
        let project = workspace.create_project("Target")?;
        let destination = ProjectClipboard::new(workspace.project_images(&project.id)?, LIMIT);
        assert!(prepare_bundle(&destination, &copied).is_ok());
        let tight = ProjectClipboard::new(
            workspace.project_images(&project.id)?,
            captured.rich.as_ref().map_or(0, Vec::len),
        );
        assert!(tight.prepare_copy(&captured).is_err());
        assert!(prepare_bundle(&tight, &copied).is_err());
        Ok(())
    }
    #[test]
    fn partial_selection_export_does_not_include_unselected_project_assets() -> TestResult {
        let workspace = Workspace::memory()?;
        let (source, captured, _) = fixture(&workspace)?;
        let mut session = EditorSession::new(Document::plain("p", "Select this")?, Revision(0));
        session.set_selection(Selection {
            anchor: DocPoint {
                block_id: "p".into(),
                offset: craft_editor_model::ScalarOffset(0),
            },
            head: DocPoint {
                block_id: "p".into(),
                offset: craft_editor_model::ScalarOffset(6),
            },
        })?;
        let selected = session.copy_selection()?.ok_or("missing range")?;
        let copied = source.prepare_copy(&selected)?;
        assert!(!copied.formats.contains_key(IMAGE_CAPSULE_MIME));
        assert_eq!(copied.plain, "Select");
        assert!(
            source
                .prepare_copy(&captured)?
                .formats
                .contains_key(IMAGE_CAPSULE_MIME)
        );
        Ok(())
    }
    #[test]
    fn external_html_image_is_staged_for_explicit_import_instead_of_pasted_as_foreign_placeholder()
    -> TestResult {
        let workspace = Workspace::memory()?;
        let (source, _, _) = fixture(&workspace)?;
        let captured = ClipboardBundle {
            html: Some("<img src=\"https://example.test/image.png\" alt=\"Image\">".into()),
            ..Default::default()
        };
        let Err(ProjectClipboardError::ExternalImagesRequired(images)) =
            prepare_bundle(&source, &captured)
        else {
            return Err("external image was not staged".into());
        };
        assert_eq!(
            images,
            vec![ExternalImage {
                source: "https://example.test/image.png".into()
            }]
        );
        Ok(())
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    use image::{DynamicImage, ImageFormat, RgbImage};
    use std::io::Cursor;
    use studio_core::Workspace;
    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
    #[test]
    fn recovered_foreign_capsule_can_be_copied_exactly_without_destination_asset_lookup_or_import()
    -> TestResult {
        let source = Workspace::memory()?;
        let project = source.create_project("Source")?;
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(RgbImage::new(2, 3)).write_to(&mut bytes, ImageFormat::Png)?;
        let asset = source.import_image(
            &project.id,
            "original.png",
            Some("image/png"),
            &bytes.into_inner(),
        )?;
        let node = source.image_node(&project.id, &asset.id, "image", "Alt", "Caption")?;
        let captured = ProjectClipboard::new(source.project_images(&project.id)?, 1_000_000)
            .prepare_copy(
                &RichSlice {
                    content: vec![node],
                    open_start: 0,
                    open_end: 0,
                }
                .bundle()?,
            )?;
        let destination = Workspace::memory()?;
        let foreign = destination.create_project("Foreign")?;
        let scope = ProjectClipboard::new(destination.project_images(&foreign.id)?, 1_000_000);
        // The separate destination has no source asset. An attempted lookup would fail.
        assert!(destination.image_asset(&foreign.id, &asset.id).is_err());
        assert_eq!(scope.prepare_copy(&captured)?, captured);
        assert!(destination.image_asset(&foreign.id, &asset.id).is_err());
        let mut invalid = captured.clone();
        let data = invalid
            .formats
            .get_mut(IMAGE_CAPSULE_MIME)
            .ok_or("missing capsule")?;
        let mut value: serde_json::Value = serde_json::from_slice(data)?;
        value["images"][0]["width"] = serde_json::json!(1000);
        *data = serde_json::to_vec(&value)?;
        assert!(matches!(
            scope.prepare_copy(&invalid),
            Err(ProjectClipboardError::InvalidCapsule)
        ));
        Ok(())
    }
}

#[cfg(test)]
mod native_format_tests {
    use super::*;
    #[test]
    fn project_capsule_has_explicit_portable_mime_and_valid_macos_type_mapping() {
        let format = image_capsule_format();
        assert_eq!(format.mime, IMAGE_CAPSULE_MIME);
        assert_ne!(format.mime, format.macos_uti);
        assert!(format.macos_uti.contains('.'));
        assert!(format.macos_uti.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-')
        }));
    }
}
