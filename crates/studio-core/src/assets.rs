//! Owned, immutable project images and a scoped source for native render workers.
//!
//! Imports store exact encoded bytes, never a reference to the author's file.
//! Canonical import/snapshot preservation is separate from validated authoring:
//! unresolved historical image attributes are not rewritten by this module.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::{Arc, Mutex};

use craft_editor_model::{Document as RichTree, Node};
use image::{ImageFormat, ImageReader};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use super::{Result, StudioError, Workspace, require_project};

/// Existing Writers Studio image encoded-byte limit: 12 MiB.
pub const MAX_IMAGE_BYTES: usize = 12 * 1024 * 1024;
/// Existing maximum width and height; no additional pixel-area limit is imposed.
pub const MAX_IMAGE_DIMENSION: u32 = 20_000;

/// Durable project-owned image metadata. Encoded bytes remain separate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageAsset {
    /// Stable application-owned media identity.
    pub id: String,
    /// Owning project; identities never authorize cross-project access.
    pub project_id: String,
    /// MIME type derived from the image header, not its filename.
    pub media_type: String,
    /// Display-only basename with controls and parent paths removed.
    pub original_name: String,
    /// SHA-256 of the exact original encoded bytes.
    pub sha256: String,
    /// Exact encoded length.
    pub byte_size: u64,
    /// Intrinsic header width.
    pub width: u32,
    /// Intrinsic header height.
    pub height: u32,
    /// Creation time in Unix seconds.
    pub created_at_unix: i64,
}
impl ImageAsset {
    /// Native resource key. It is an identity, not a URL to fetch.
    pub fn source(&self) -> String {
        format!("asset://{}", self.id)
    }
}

/// Exact verified bytes and metadata transferred through a project-scoped media command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageTransfer {
    /// Original reference key. It is a mapping key, never access authority.
    pub source_id: String,
    /// Display-only original name.
    pub original_name: String,
    /// Header-derived MIME type.
    pub media_type: String,
    /// SHA-256 of the exact encoded bytes.
    pub sha256: String,
    /// Declared encoded byte length, validated before import.
    pub byte_size: u64,
    /// Header width.
    pub width: u32,
    /// Header height.
    pub height: u32,
    /// Exact original encoded bytes, not decoded pixels or a filesystem path.
    pub bytes: Vec<u8>,
}
/// Result of one validated bulk import, preserving the caller's reference mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedImage {
    /// Original mapping key, including when the destination assigned a fresh identity.
    pub source_id: String,
    /// Durable destination-owned image.
    pub asset: ImageAsset,
}
/// Cloneable, fixed-project media capability for bounded background jobs.
///
/// This shares only the connection and scope, not Workspace replay/session state. Every command
/// rechecks project ownership. A foreign source ID never authorizes a global database lookup.
#[derive(Clone)]
pub struct ProjectImages {
    connection: Arc<Mutex<Connection>>,
    project_id: String,
}
impl ProjectImages {
    /// Fixed owning project; clipboard origin hints cannot change this scope.
    pub fn project_id(&self) -> &str {
        &self.project_id
    }
    /// Read only explicitly selected identities, with exact header/checksum verification.
    pub fn export_images(&self, ids: &[String], max_bytes: usize) -> Result<Vec<ImageTransfer>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StudioError::LockUnavailable)?;
        require_project(&connection, &self.project_id)?;
        let mut seen = HashSet::new();
        let mut total = 0usize;
        let mut output = Vec::with_capacity(ids.len());
        for id in ids {
            if !seen.insert(id) {
                continue;
            }
            let metadata = read_reference_metadata(&connection, &self.project_id, id)?;
            total = total
                .checked_add(usize::try_from(metadata.byte_size).map_err(|_| transfer_invalid())?)
                .filter(|total| *total <= max_bytes)
                .ok_or_else(transfer_invalid)?;
            let (asset, bytes) = read_verified_image(&connection, &self.project_id, &metadata.id)?;
            if asset != metadata {
                return Err(StudioError::CorruptData(
                    "Image changed while its transfer was being captured".into(),
                ));
            }
            output.push(ImageTransfer {
                source_id: id.clone(),
                original_name: asset.original_name,
                media_type: asset.media_type,
                sha256: asset.sha256,
                byte_size: asset.byte_size,
                width: asset.width,
                height: asset.height,
                bytes,
            });
        }
        Ok(output)
    }
    /// Validate every descriptor/header before any mutation, then import one atomic batch.
    ///
    /// An existing identity is reused only inside the destination project and only for identical
    /// verified bytes. Cross-project transfers receive fresh IDs; existing assets are immutable.
    /// A stale document guard can leave unreferenced durable resources, but cannot change writing.
    pub fn import_images(
        &self,
        images: &[ImageTransfer],
        max_bytes: usize,
    ) -> Result<Vec<ImportedImage>> {
        validate_image_transfers(images, max_bytes)?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StudioError::LockUnavailable)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_project(&transaction, &self.project_id)?;
        let mut output = Vec::with_capacity(images.len());
        for image in images {
            let existing =
                match read_reference_metadata(&transaction, &self.project_id, &image.source_id) {
                    Ok(metadata) => {
                        let (asset, bytes) =
                            read_verified_image(&transaction, &self.project_id, &metadata.id)?;
                        (bytes == image.bytes).then_some(asset)
                    }
                    Err(StudioError::NotFound { .. }) => None,
                    Err(error) => return Err(error),
                };
            let asset = if let Some(asset) = existing {
                asset
            } else {
                let id = uuid::Uuid::new_v4().to_string();
                let extension = match image.media_type.as_str() {
                    "image/png" => ".png",
                    "image/jpeg" => ".jpg",
                    "image/gif" => ".gif",
                    _ => ".webp",
                };
                let filename = safe_filename(&image.original_name, extension);
                let byte_size = i64::try_from(image.byte_size).map_err(|_| transfer_invalid())?;
                transaction.execute("INSERT INTO project_assets(id,project_id,media_type,original_name,sha256,byte_size,width,height,data) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                    params![id, self.project_id, image.media_type, filename, image.sha256, byte_size, image.width, image.height, image.bytes])?;
                read_image_metadata(&transaction, &self.project_id, &id)?
            };
            output.push(ImportedImage {
                source_id: image.source_id.clone(),
                asset,
            });
        }
        transaction.commit()?;
        Ok(output)
    }
}
/// Validate a complete portable image batch without database, filesystem or network access.
/// Duplicate mapping keys, original header/metadata mismatches and total-byte overflow all fail
/// before a caller may publish or import these bytes.
pub fn validate_image_transfers(images: &[ImageTransfer], max_bytes: usize) -> Result<()> {
    let mut seen = HashSet::new();
    let mut total = 0usize;
    for image in images {
        if !is_valid_image_reference_id(&image.source_id) || !seen.insert(&image.source_id) {
            return Err(transfer_invalid());
        }
        total = total
            .checked_add(image.bytes.len())
            .filter(|total| *total <= max_bytes)
            .ok_or_else(transfer_invalid)?;
        let inspection = inspect_image(&image.bytes)?;
        if image.byte_size != image.bytes.len() as u64
            || image.sha256 != checksum(&image.bytes)
            || image.media_type != inspection.media_type
            || image.width != inspection.width
            || image.height != inspection.height
        {
            return Err(transfer_invalid());
        }
    }
    Ok(())
}
fn transfer_invalid() -> StudioError {
    StudioError::Validation(
        "Image transfer descriptors, encoded bytes or total byte budget are invalid".into(),
    )
}

/// Why a preserved canonical image cannot currently resolve in a snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageAssetFailureKind {
    /// Unsupported source, absent bytes or another project's identity.
    UnresolvedReference,
    /// Stored image metadata could not be validated.
    InvalidMetadata,
}

/// A missing-image diagnostic, without copying private URLs or image data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageAssetFailure {
    /// Originating stable block identity, if supplied by the canonical document.
    pub block_id: Option<String>,
    /// Referenced asset identity, if present.
    pub asset_id: Option<String>,
    /// Classification for host-localized diagnostics.
    pub kind: ImageAssetFailureKind,
}

/// Immutable project-scoped resource snapshot. Bytes are read and verified lazily
/// on the render worker, avoiding a document-wide copy of every original image.
#[derive(Clone)]
pub struct ImageAssetSnapshot {
    connection: Arc<Mutex<Connection>>,
    project_id: String,
    assets: HashMap<String, ImageAsset>,
    failures: Vec<ImageAssetFailure>,
}
impl ImageAssetSnapshot {
    /// Project whose media this snapshot may access.
    pub fn project_id(&self) -> &str {
        &self.project_id
    }
    /// Preserved unresolved references; native rendering still emits per-node fallbacks.
    pub fn failures(&self) -> &[ImageAssetFailure] {
        &self.failures
    }
    /// Metadata captured for an authorized canonical reference.
    pub fn metadata(&self, id: &str) -> Option<&ImageAsset> {
        self.assets.get(id)
    }
    /// Load only a captured image with the exact src/assetId binding. No ambient
    /// filesystem, HTTP or legacy API request is performed. Bit rot, changed
    /// metadata or a missing row fails explicitly without substituting an image.
    pub fn load(&self, node: &Node) -> Result<Arc<Vec<u8>>> {
        let id = reference_id(node)?;
        let expected = self.assets.get(id).ok_or_else(|| unavailable(id))?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| StudioError::LockUnavailable)?;
        let (asset, bytes) = read_verified_image(&connection, &self.project_id, &expected.id)?;
        if &asset != expected {
            return Err(StudioError::CorruptData(
                "Image changed after its renderer snapshot was captured".into(),
            ));
        }
        Ok(Arc::new(bytes))
    }
}

impl Workspace {
    /// Create a fixed-project media handle for background clipboard/import work.
    pub fn project_images(&self, project_id: &str) -> Result<ProjectImages> {
        let connection = self.connection()?;
        require_project(&connection, project_id)?;
        Ok(ProjectImages {
            connection: self.connection.clone(),
            project_id: project_id.into(),
        })
    }

    /// Import exact PNG/JPEG/GIF/WebP bytes. The image is durable before any
    /// document insertion; importing alone never changes canonical writing.
    /// Header inspection matches the original DecodeConfig contract, and does
    /// not claim that every encoded pixel/frame has been rendered successfully.
    pub fn import_image(
        &self,
        project_id: &str,
        filename: &str,
        declared_media_type: Option<&str>,
        bytes: &[u8],
    ) -> Result<ImageAsset> {
        let inspected = inspect_image(bytes)?;
        if declared_media_type
            .is_some_and(|declared| !declared.trim().eq_ignore_ascii_case(inspected.media_type))
        {
            return Err(StudioError::Validation(
                "Declared image type does not match the encoded image".into(),
            ));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let filename = safe_filename(filename, inspected.extension);
        let checksum = checksum(bytes);
        let byte_size = i64::try_from(bytes.len())
            .map_err(|_| StudioError::Validation("Image byte size is out of range".into()))?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_project(&transaction, project_id)?;
        transaction.execute("INSERT INTO project_assets(id,project_id,media_type,original_name,sha256,byte_size,width,height,data) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)", params![id, project_id, inspected.media_type, filename, checksum, byte_size, inspected.width, inspected.height, bytes])?;
        let asset = read_image_metadata(&transaction, project_id, &id)?;
        transaction.commit()?;
        Ok(asset)
    }

    /// Explicit local-author import. Only a regular file is read, with a bounded
    /// actual read; subsequent rendering does not retain or reopen this path.
    pub fn import_image_file(
        &self,
        project_id: &str,
        path: impl AsRef<Path>,
    ) -> Result<ImageAsset> {
        let path = path.as_ref();
        if !std::fs::metadata(path)?.is_file() {
            return Err(StudioError::Validation(
                "Choose a regular image file".into(),
            ));
        }
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(StudioError::Validation(
                "Choose a regular image file".into(),
            ));
        }
        if metadata.len() > MAX_IMAGE_BYTES as u64 {
            return Err(StudioError::Validation(
                "Image exceeds the 12 MiB limit".into(),
            ));
        }
        let mut bytes = Vec::new();
        file.take(MAX_IMAGE_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        let filename = path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_default();
        self.import_image(project_id, &filename, None, &bytes)
    }

    /// Read image metadata only within its owning project.
    pub fn image_asset(&self, project_id: &str, id: &str) -> Result<ImageAsset> {
        let connection = self.connection()?;
        require_project(&connection, project_id)?;
        read_image_metadata(&connection, project_id, id)
    }

    /// Read exact immutable bytes after checking their length, checksum, MIME and
    /// intrinsic dimensions. This method never fetches an asset from its source.
    pub fn image_asset_bytes(&self, project_id: &str, id: &str) -> Result<Vec<u8>> {
        let connection = self.connection()?;
        require_project(&connection, project_id)?;
        read_verified_image(&connection, project_id, id).map(|(_, bytes)| bytes)
    }

    /// Prepare an owned authoring node after verifying durable media. Caller IDs,
    /// alternative text and caption are validated through the canonical model.
    /// Existing imported nodes are never passed through this fresh-node builder.
    pub fn image_node(
        &self,
        project_id: &str,
        id: &str,
        block_id: &str,
        alt: &str,
        caption: &str,
    ) -> Result<Node> {
        let connection = self.connection()?;
        require_project(&connection, project_id)?;
        let (asset, _) = read_verified_image(&connection, project_id, id)?;
        let node = Node::block(
            "imageBlock",
            Map::from_iter([
                ("blockId".into(), json!(block_id)),
                ("assetId".into(), json!(asset.id)),
                ("src".into(), json!(asset.source())),
                ("alt".into(), json!(alt)),
                ("caption".into(), json!(caption)),
                ("width".into(), Value::Null),
                ("height".into(), Value::Null),
            ]),
            vec![],
        );
        RichTree::from_root(Node::block(
            "doc",
            Map::from_iter([("schemaVersion".into(), json!(2))]),
            vec![node.clone()],
        ))
        .map_err(|error| StudioError::Validation(format!("Invalid image block: {error}")))?;
        Ok(node)
    }

    /// Capture only this document's referenced, project-owned image metadata.
    /// Missing/opaque imported references are recorded rather than rewritten or
    /// mistaken for successful migration. Full byte checks happen in load().
    pub fn image_asset_snapshot(
        &self,
        project_id: &str,
        document: &RichTree,
    ) -> Result<ImageAssetSnapshot> {
        let connection = self.connection()?;
        require_project(&connection, project_id)?;
        let mut snapshot = ImageAssetSnapshot {
            connection: self.connection.clone(),
            project_id: project_id.into(),
            assets: HashMap::new(),
            failures: vec![],
        };
        let mut seen = HashSet::new();
        let mut stack = vec![document.root()];
        while let Some(node) = stack.pop() {
            stack.extend(node.children());
            if node.node_type != "imageBlock" {
                continue;
            }
            let id = match reference_id(node) {
                Ok(id) => id,
                Err(_) => {
                    snapshot
                        .failures
                        .push(failure(node, ImageAssetFailureKind::UnresolvedReference));
                    continue;
                }
            };
            if !seen.insert(id.to_owned()) {
                continue;
            }
            match read_reference_metadata(&connection, project_id, id) {
                Ok(asset) => {
                    snapshot.assets.insert(id.into(), asset);
                }
                Err(StudioError::NotFound { .. }) => snapshot
                    .failures
                    .push(failure(node, ImageAssetFailureKind::UnresolvedReference)),
                Err(StudioError::Validation(_) | StudioError::CorruptData(_)) => snapshot
                    .failures
                    .push(failure(node, ImageAssetFailureKind::InvalidMetadata)),
                Err(error) => return Err(error),
            }
        }
        Ok(snapshot)
    }
}

fn failure(node: &Node, kind: ImageAssetFailureKind) -> ImageAssetFailure {
    ImageAssetFailure {
        block_id: node.block_id().map(str::to_owned),
        asset_id: node.string_attr("assetId").map(str::to_owned),
        kind,
    }
}
fn unavailable(id: &str) -> StudioError {
    StudioError::NotFound {
        kind: "Image asset",
        id: id.into(),
    }
}
/// Whether a canonical image identity fits the original project's opaque-ID bound.
///
/// This is syntax validation only. An identity grants no resource access; callers must use a
/// fixed owning project's alias/asset capability and verify exact bytes and src bindings.
pub fn is_valid_image_reference_id(id: &str) -> bool {
    !id.trim().is_empty() && id.len() <= 512
}

fn reference_id(node: &Node) -> Result<&str> {
    let invalid =
        || StudioError::Validation("Image reference does not identify an owned local image".into());
    if node.node_type != "imageBlock" {
        return Err(invalid());
    }
    let id = node
        .string_attr("assetId")
        .filter(|id| is_valid_image_reference_id(id))
        .ok_or_else(invalid)?;
    let source = node.string_attr("src").ok_or_else(invalid)?;
    if source != format!("asset://{id}") && source != format!("/api/media/{id}/content") {
        return Err(invalid());
    }
    Ok(id)
}

struct ImageInspection {
    media_type: &'static str,
    extension: &'static str,
    width: u32,
    height: u32,
}
fn inspect_image(bytes: &[u8]) -> Result<ImageInspection> {
    if bytes.is_empty() {
        return Err(StudioError::Validation("Choose a nonempty image".into()));
    }
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(StudioError::Validation(
            "Image exceeds the 12 MiB limit".into(),
        ));
    }
    let format = image::guess_format(bytes).map_err(|_| {
        StudioError::Validation(
            "Image header is invalid or unsupported; choose PNG, JPEG, GIF or WebP".into(),
        )
    })?;
    let (media_type, extension) = match format {
        ImageFormat::Png => ("image/png", ".png"),
        ImageFormat::Jpeg => ("image/jpeg", ".jpg"),
        ImageFormat::Gif => ("image/gif", ".gif"),
        ImageFormat::WebP => ("image/webp", ".webp"),
        _ => {
            return Err(StudioError::Validation(
                "Choose PNG, JPEG, GIF or WebP".into(),
            ));
        }
    };
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    reader.limits(limits);
    let (width, height) = reader.into_dimensions().map_err(|_| {
        StudioError::Validation("Image header is invalid or dimensions exceed 20000 pixels".into())
    })?;
    if width == 0 || height == 0 || width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
        return Err(StudioError::Validation(
            "Image dimensions must be between 1 and 20000 pixels".into(),
        ));
    }
    Ok(ImageInspection {
        media_type,
        extension,
        width,
        height,
    })
}
fn safe_filename(value: &str, extension: &str) -> String {
    let normalized = value.replace('\\', "/");
    let basename = normalized.rsplit('/').next().unwrap_or_default();
    let cleaned = basename
        .chars()
        .filter(|character| !character.is_control())
        .collect::<String>();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() || trimmed == "." {
        format!("image{extension}")
    } else {
        trimmed.chars().take(255).collect()
    }
}
fn checksum(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
/// Resolve a canonical logical identity once, inside its fixed project. Native authoring assets
/// have no source alias and retain their UUID fallback; aliases never authorize another project.
fn read_reference_metadata(
    connection: &Connection,
    project_id: &str,
    id: &str,
) -> Result<ImageAsset> {
    if !is_valid_image_reference_id(id) {
        return Err(unavailable(id));
    }
    let native: Option<String> = connection.query_row(
        "SELECT native_id FROM project_resource_aliases WHERE project_id=?1 AND resource_kind='media' AND logical_id=?2",
        params![project_id, id], |row| row.get(0),
    ).optional()?;
    read_image_metadata(connection, project_id, native.as_deref().unwrap_or(id))
}
fn read_image_metadata(connection: &Connection, project_id: &str, id: &str) -> Result<ImageAsset> {
    if uuid::Uuid::parse_str(id).is_err() {
        return Err(unavailable(id));
    }
    let asset = connection.query_row("SELECT id,project_id,media_type,original_name,sha256,byte_size,COALESCE(width,0),COALESCE(height,0),created_at FROM project_assets WHERE id=?1 AND project_id=?2", params![id,project_id], |row| Ok(ImageAsset {
        id:row.get(0)?,project_id:row.get(1)?,media_type:row.get(2)?,original_name:row.get(3)?,sha256:row.get(4)?,byte_size:u64::try_from(row.get::<_,i64>(5)?).map_err(|error|rusqlite::Error::FromSqlConversionFailure(5,rusqlite::types::Type::Integer,Box::new(error)))?,width:row.get(6)?,height:row.get(7)?,created_at_unix:row.get(8)?,
    })).optional()?.ok_or_else(|| unavailable(id))?;
    if !["image/png", "image/jpeg", "image/gif", "image/webp"].contains(&asset.media_type.as_str())
    {
        return Err(StudioError::Validation(
            "Asset is not a supported image".into(),
        ));
    }
    if asset.byte_size == 0
        || asset.byte_size > MAX_IMAGE_BYTES as u64
        || asset.width == 0
        || asset.width > MAX_IMAGE_DIMENSION
        || asset.height == 0
        || asset.height > MAX_IMAGE_DIMENSION
        || asset.sha256.len() != 64
        || !asset.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(StudioError::CorruptData(
            "Stored image metadata is invalid".into(),
        ));
    }
    Ok(asset)
}
fn read_verified_image(
    connection: &Connection,
    project_id: &str,
    id: &str,
) -> Result<(ImageAsset, Vec<u8>)> {
    let asset = read_image_metadata(connection, project_id, id)?;
    let bytes = connection
        .query_row(
            "SELECT data FROM project_assets WHERE id=?1 AND project_id=?2 AND length(data)=?3 AND length(data)<=?4",
            params![id, project_id, i64::try_from(asset.byte_size).map_err(|_| StudioError::CorruptData("Image byte size is out of range".into()))?, i64::try_from(MAX_IMAGE_BYTES).map_err(|_| StudioError::Validation("Image byte limit is out of range".into()))?],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()?
        .ok_or_else(|| StudioError::CorruptData("Stored image byte length does not match its metadata".into()))?;
    if bytes.len() as u64 != asset.byte_size
        || !checksum(&bytes).eq_ignore_ascii_case(&asset.sha256)
    {
        return Err(StudioError::CorruptData(
            "Stored image bytes failed their size/checksum verification".into(),
        ));
    }
    let inspected = inspect_image(&bytes)
        .map_err(|_| StudioError::CorruptData("Stored image header is invalid".into()))?;
    if inspected.media_type != asset.media_type
        || inspected.width != asset.width
        || inspected.height != asset.height
    {
        return Err(StudioError::CorruptData(
            "Stored image header does not match its metadata".into(),
        ));
    }
    Ok((asset, bytes))
}

#[cfg(test)]
mod alias_tests {
    use super::*;
    use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
    type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
    fn png(color: u8) -> TestResult<Vec<u8>> {
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(2, 3, Rgb([color, 80, 120])));
        let mut bytes = std::io::Cursor::new(Vec::new());
        image.write_to(&mut bytes, ImageFormat::Png)?;
        Ok(bytes.into_inner())
    }
    fn alias(workspace: &Workspace, project: &str, logical: &str, native: &str) -> TestResult {
        workspace.connection()?.execute(
            "INSERT INTO project_resource_aliases(project_id,resource_kind,logical_id,native_id) VALUES(?1,'media',?2,?3) ON CONFLICT(project_id,resource_kind,logical_id) DO UPDATE SET native_id=excluded.native_id",
            params![project,logical,native],
        )?;
        Ok(())
    }
    fn tree(logical: &str) -> TestResult<RichTree> {
        Ok(RichTree::from_json(
            json!({"type":"doc","attrs":{"schemaVersion":2},"content":[
                {"type":"imageBlock","attrs":{"blockId":"picture","assetId":logical,"src":format!("/api/media/{logical}/content"),"alt":"Original","caption":"Keep","vendor":{"retained":true}}}
            ]}),
        )?)
    }
    #[test]
    fn opaque_image_aliases_remain_project_scoped_and_transfer_the_original_mapping_key()
    -> TestResult {
        let workspace = Workspace::memory()?;
        let first = workspace.create_project("First")?;
        let second = workspace.create_project("Second")?;
        let first_bytes = png(10)?;
        let second_bytes = png(240)?;
        let first_asset = workspace.import_image(&first.id, "first.png", None, &first_bytes)?;
        let second_asset = workspace.import_image(&second.id, "second.png", None, &second_bytes)?;
        let logical = "original image 作者";
        alias(&workspace, &first.id, logical, &first_asset.id)?;
        alias(&workspace, &second.id, logical, &second_asset.id)?;
        let document = tree(logical)?;
        let original = document.to_json()?;
        let node = document.node(&craft_editor_model::NodePath(vec![0]))?;
        let first_snapshot = workspace.image_asset_snapshot(&first.id, &document)?;
        let second_snapshot = workspace.image_asset_snapshot(&second.id, &document)?;
        assert!(first_snapshot.failures().is_empty());
        assert_eq!(first_snapshot.metadata(logical), Some(&first_asset));
        assert_eq!(first_snapshot.load(node)?.as_ref(), &first_bytes);
        assert_eq!(second_snapshot.load(node)?.as_ref(), &second_bytes);
        let transfer = workspace
            .project_images(&first.id)?
            .export_images(&[logical.into()], MAX_IMAGE_BYTES)?;
        assert_eq!(transfer[0].source_id, logical);
        assert_eq!(transfer[0].bytes, first_bytes);
        let destination = workspace.create_project("Destination")?;
        let imported = workspace
            .project_images(&destination.id)?
            .import_images(&transfer, MAX_IMAGE_BYTES)?;
        assert_eq!(imported[0].source_id, logical);
        assert_ne!(imported[0].asset.id, first_asset.id);
        assert_eq!(
            workspace.image_asset_bytes(&destination.id, &imported[0].asset.id)?,
            first_bytes
        );
        assert_eq!(document.to_json()?, original);
        Ok(())
    }
    #[test]
    fn captured_image_capability_keeps_its_verified_native_identity_after_alias_rebinding()
    -> TestResult {
        let workspace = Workspace::memory()?;
        let project = workspace.create_project("Images")?;
        let original_bytes = png(10)?;
        let newer_bytes = png(240)?;
        let original =
            workspace.import_image(&project.id, "original.png", None, &original_bytes)?;
        let newer = workspace.import_image(&project.id, "newer.png", None, &newer_bytes)?;
        alias(&workspace, &project.id, "image", &original.id)?;
        let document = tree("image")?;
        let node = document.node(&craft_editor_model::NodePath(vec![0]))?;
        let captured = workspace.image_asset_snapshot(&project.id, &document)?;
        alias(&workspace, &project.id, "image", &newer.id)?;
        assert_eq!(captured.load(node)?.as_ref(), &original_bytes);
        let fresh = workspace.image_asset_snapshot(&project.id, &document)?;
        assert_eq!(fresh.load(node)?.as_ref(), &newer_bytes);
        Ok(())
    }
    #[test]
    fn foreign_alias_targets_and_mismatched_src_bindings_never_authorize_a_read() -> TestResult {
        let workspace = Workspace::memory()?;
        let first = workspace.create_project("First")?;
        let other = workspace.create_project("Other")?;
        let asset = workspace.import_image(&other.id, "foreign.png", None, &png(200)?)?;
        alias(&workspace, &first.id, "image", &asset.id)?;
        let document = tree("image")?;
        let snapshot = workspace.image_asset_snapshot(&first.id, &document)?;
        assert_eq!(snapshot.failures().len(), 1);
        assert!(
            snapshot
                .load(document.node(&craft_editor_model::NodePath(vec![0]))?)
                .is_err()
        );
        assert!(
            workspace
                .project_images(&first.id)?
                .export_images(&["image".into()], MAX_IMAGE_BYTES)
                .is_err()
        );
        assert!(
            workspace
                .project_images(&first.id)?
                .export_images(std::slice::from_ref(&asset.id), MAX_IMAGE_BYTES)
                .is_err()
        );
        let mut forged = tree("image")?.to_json()?;
        forged["content"][0]["attrs"]["src"] = json!(format!("asset://{}", asset.id));
        let forged = RichTree::from_json(forged)?;
        let snapshot = workspace.image_asset_snapshot(&other.id, &forged)?;
        assert_eq!(snapshot.failures().len(), 1);
        Ok(())
    }
}
