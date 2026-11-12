//! One ordered reader manuscript compiled from the selected project documents.
//!
//! This is a final-manuscript reading export, not a lossless project backup. It reuses the
//! portable renderer, writes an explicitly incomplete destination marker first, copies only the
//! referenced original media into an adjacent folder, and verifies every payload before it
//! removes that marker.
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;
use studio_core::{
    ProjectAttachment, ProjectExportSnapshot, ProjectTransferCancellation, is_workspace_data_path,
};

use super::markdown_render::{MarkdownRenderer, escape_text};
use super::portable_markdown::{
    DigestWriter, MarkdownConversion, PortableExportFile, artifact_extension, check_cancel,
    ensure_parent, ordered_tree, required, sync_directory, verify_file, write_new,
};
use super::portable_paths::{NameAllocator, percent_encode};
use super::{Result, TransferError};

/// Which manuscript documents one reader export includes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManuscriptSelection {
    /// Every active node of the project, in tree order.
    Project,
    /// One content-bearing document identity.
    Document(String),
    /// A folder or document identity and every descendant, in tree order.
    Subtree(String),
    /// Explicit distinct content-bearing document identities, emitted in tree order.
    Documents(Vec<String>),
}

/// One planned section or chapter heading in the single compiled document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManuscriptEntry {
    /// Logical project node identity.
    pub node_id: String,
    /// Node title exactly as stored, never translated.
    pub title: String,
    /// `folder` for a section heading, `document` for a rendered chapter.
    pub kind: String,
    /// Heading level 1-6 derived from included ancestor depth.
    pub heading_level: u8,
}

/// Inclusion/order preview plus the reading conversions known before writing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManuscriptPlan {
    /// Included sections and documents in actual tree order.
    pub entries: Vec<ManuscriptEntry>,
    /// Known reader conversions, including scope-specific disclosures.
    pub conversions: Vec<MarkdownConversion>,
}

/// Verified members of a completed reader export, relative to its new destination root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledManuscriptReceipt {
    /// Exact user-selected new directory.
    pub destination: PathBuf,
    /// The single reader document, relative to the destination root.
    pub document: String,
    /// The adjacent original-artifact folder, when the manuscript references media.
    pub assets_directory: Option<String>,
    /// Verified payloads with their actual SHA-256 and byte length.
    pub files: Vec<PortableExportFile>,
    /// Included sections and documents in actual order.
    pub entries: Vec<ManuscriptEntry>,
    /// Every disclosed reading conversion, scoped to its resource.
    pub conversions: Vec<MarkdownConversion>,
}

/// Bounded worker progress for the single compiled document and its assets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledManuscriptProgress {
    /// Relative payload currently about to be written; user content is not translated.
    pub path: String,
    /// Payloads written so far, excluding the incomplete marker.
    pub completed: usize,
}

struct PreparedCompilation {
    entries: Vec<ManuscriptEntry>,
    documents: Vec<(String, u8)>,
    anchors: HashMap<String, String>,
    asset_paths: HashMap<String, String>,
    attachments: Vec<ProjectAttachment>,
    conversions: BTreeSet<MarkdownConversion>,
    stem: String,
    document_path: String,
}

/// Resolve actual inclusion and order without reading or writing any destination.
///
/// Included revisions are rendered in memory so the preview names every reader conversion that
/// compilation would disclose. Nothing is written and no cancellation is reported.
///
/// # Errors
/// Returns an error for an unknown or empty selection, a non-content document, a broken tree or
/// a document that references a missing captured artifact.
pub fn plan_manuscript(
    snapshot: &ProjectExportSnapshot,
    selection: &ManuscriptSelection,
) -> Result<ManuscriptPlan> {
    let mut prepared = prepare(snapshot, selection)?;
    render_manuscript(
        snapshot,
        &mut prepared,
        &ProjectTransferCancellation::default(),
    )?;
    Ok(ManuscriptPlan {
        entries: prepared.entries,
        conversions: prepared.conversions.into_iter().collect(),
    })
}

/// Compile one ordered Markdown manuscript and its adjacent original artifacts.
///
/// The destination must be a new directory. A `.incomplete` marker is written before any payload
/// and removed only after every file verifies and syncs. Cancellation, missing artifacts,
/// collisions or write failures leave the marker in place; no completion is reported.
///
/// # Errors
/// Returns an error for an unsafe destination, invalid selection, missing artifact, missing
/// revision head or any I/O, cancellation or verification failure.
pub fn compile_manuscript(
    snapshot: &ProjectExportSnapshot,
    source_database: &Path,
    destination: &Path,
    selection: &ManuscriptSelection,
    cancel: &ProjectTransferCancellation,
) -> Result<CompiledManuscriptReceipt> {
    compile_manuscript_with_progress(
        snapshot,
        source_database,
        destination,
        selection,
        cancel,
        &mut |_| {},
    )
}

/// Compile with a quick progress callback. Cancellation is checked again after every callback.
/// The callback grants no mutation authority and should not perform blocking work.
///
/// # Errors
/// Returns the same failures as [`compile_manuscript`].
pub fn compile_manuscript_with_progress(
    snapshot: &ProjectExportSnapshot,
    source_database: &Path,
    destination: &Path,
    selection: &ManuscriptSelection,
    cancel: &ProjectTransferCancellation,
    progress: &mut dyn FnMut(CompiledManuscriptProgress),
) -> Result<CompiledManuscriptReceipt> {
    if is_workspace_data_path(destination, source_database)? {
        return Err(TransferError::Invalid(
            "Choose a new export folder away from workspace databases, journals and preferences"
                .into(),
        ));
    }
    check_cancel(cancel)?;
    let mut prepared = prepare(snapshot, selection)?;
    if destination.exists() {
        return Err(TransferError::Invalid(
            "Choose a new export folder; an existing file or directory is never overwritten".into(),
        ));
    }
    std::fs::create_dir(destination)?;
    let marker = destination.join(".incomplete");
    let mut marker_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&marker)?;
    marker_file.write_all(
        b"WriterCraft manuscript export is incomplete. Do not treat this folder as a finished manuscript.\n",
    )?;
    marker_file.sync_all()?;
    drop(marker_file);

    let mut directories = HashSet::new();
    let mut files = Vec::new();
    check_cancel(cancel)?;
    progress(CompiledManuscriptProgress {
        path: prepared.document_path.clone(),
        completed: 0,
    });
    let manuscript = render_manuscript(snapshot, &mut prepared, cancel)?;
    files.push(write_new(
        destination,
        &prepared.document_path,
        manuscript.as_bytes(),
        &mut directories,
        cancel,
    )?);
    for attachment in &prepared.attachments {
        check_cancel(cancel)?;
        let path = prepared
            .asset_paths
            .get(&attachment.logical_id)
            .ok_or_else(|| TransferError::Invalid("Compiled asset path is missing".into()))?;
        progress(CompiledManuscriptProgress {
            path: path.clone(),
            completed: files.len(),
        });
        check_cancel(cancel)?;
        ensure_parent(destination, path, &mut directories)?;
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination.join(path))?;
        let mut writer = DigestWriter::new(file);
        snapshot
            .assets()
            .copy_to(attachment, &mut writer, cancel)
            .map_err(|error| TransferError::Invalid(error.to_string()))?;
        files.push(writer.finish(path)?);
    }
    for file in &files {
        verify_file(destination, file, cancel)?;
    }
    check_cancel(cancel)?;
    let mut sync_paths = directories.iter().collect::<Vec<_>>();
    sync_paths.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for relative in sync_paths {
        sync_directory(&destination.join(relative))?;
    }
    sync_directory(destination)?;
    std::fs::remove_file(marker)?;
    sync_directory(
        destination
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new(".")),
    )?;
    Ok(CompiledManuscriptReceipt {
        destination: destination.to_path_buf(),
        document: prepared.document_path,
        assets_directory: prepared
            .attachments
            .first()
            .map(|_| format!("{}-assets", prepared.stem)),
        files,
        entries: prepared.entries,
        conversions: prepared.conversions.into_iter().collect(),
    })
}

fn render_manuscript(
    snapshot: &ProjectExportSnapshot,
    prepared: &mut PreparedCompilation,
    cancel: &ProjectTransferCancellation,
) -> Result<String> {
    let manifest = snapshot.manifest();
    let nodes = array(manifest, "nodes");
    let revisions = array(manifest, "documentRevisions");
    let project_id = manifest["project"]["id"].as_str().unwrap_or("");
    let by_id: HashMap<&str, &Value> = nodes
        .iter()
        .filter_map(|node| node["id"].as_str().map(|id| (id, node)))
        .collect();
    let mut extra = BTreeSet::new();
    let mut chapters: HashMap<String, String> = HashMap::new();
    for (id, level) in &prepared.documents {
        check_cancel(cancel)?;
        let node = by_id
            .get(id.as_str())
            .ok_or_else(|| TransferError::Invalid("Included document is missing".into()))?;
        let revision_number = node["currentRevision"].as_u64().unwrap_or(0);
        let revision = revisions
            .iter()
            .find(|revision| {
                revision["nodeId"].as_str() == Some(id)
                    && revision["revision"].as_u64() == Some(revision_number)
            })
            .ok_or_else(|| TransferError::Invalid("Export document head is missing".into()))?;
        let mut renderer = MarkdownRenderer {
            current: &prepared.document_path,
            project: project_id,
            nodes: &HashMap::new(),
            anchors: &prepared.anchors,
            heading_offset: *level,
            block_anchors: false,
            assets: &prepared.asset_paths,
            conversions: BTreeSet::new(),
        };
        let body = renderer.render(&revision["content"])?;
        for feature in renderer.conversions {
            extra.insert(MarkdownConversion {
                resource_id: id.clone(),
                feature,
            });
        }
        let mut chapter = format!(
            "<a id=\"{}\"></a>\n\n{} {}\n\n",
            document_anchor(id),
            "#".repeat(usize::from(*level)),
            escape_text(node["title"].as_str().unwrap_or(""))
        );
        if node["sourceFrontmatter"]
            .as_str()
            .is_some_and(|frontmatter| !frontmatter.trim().is_empty())
        {
            extra.insert(MarkdownConversion {
                resource_id: id.clone(),
                feature: "source-frontmatter-omitted-from-reader-export".into(),
            });
        }
        chapter.push_str(&body);
        chapters.insert(id.clone(), chapter);
    }
    let mut sections = Vec::new();
    for entry in &prepared.entries {
        if let Some(chapter) = chapters.remove(&entry.node_id) {
            sections.push(chapter);
        } else if entry.kind == "folder" {
            sections.push(format!(
                "{} {}",
                "#".repeat(usize::from(entry.heading_level)),
                escape_text(&entry.title)
            ));
        }
    }
    let mut out = String::new();
    for section in sections {
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(section.trim_end());
        out.push('\n');
    }
    prepared.conversions.extend(extra);
    Ok(out)
}

fn prepare(
    snapshot: &ProjectExportSnapshot,
    selection: &ManuscriptSelection,
) -> Result<PreparedCompilation> {
    let manifest = snapshot.manifest();
    let all = array(manifest, "nodes");
    // Deleted nodes keep their captured title and metadata; a reader manuscript must never
    // resurrect them merely because they still exist in the canonical snapshot.
    let nodes = all
        .iter()
        .filter(|node| !is_trashed(node))
        .cloned()
        .collect::<Vec<_>>();
    let nodes = nodes.as_slice();
    let ordered = ordered_tree(nodes, false)?;
    let by_id: HashMap<&str, usize> = ordered
        .iter()
        .filter_map(|index| nodes[*index]["id"].as_str().map(|id| (id, *index)))
        .collect();
    let included = selected_identities(nodes, &by_id, selection)?;
    if included.is_empty() {
        return Err(TransferError::Invalid(
            "Select at least one content-bearing manuscript document".into(),
        ));
    }
    let included_folders: HashSet<&str> = included
        .iter()
        .filter(|id| {
            by_id
                .get(id.as_str())
                .is_some_and(|index| nodes[*index]["currentRevision"].as_u64().unwrap_or(0) == 0)
        })
        .map(String::as_str)
        .collect();
    let mut entries = Vec::new();
    let mut documents = Vec::new();
    for index in &ordered {
        let node = &nodes[*index];
        let id = required(node, "id")?;
        if !included.contains(id) {
            continue;
        }
        let folder = node["currentRevision"].as_u64().unwrap_or(0) == 0;
        let level = heading_level(nodes, id, &included_folders).ok_or_else(|| {
            TransferError::Invalid("Included manuscript depth cannot be resolved".into())
        })?;
        entries.push(ManuscriptEntry {
            node_id: id.to_owned(),
            title: node["title"].as_str().unwrap_or("").to_owned(),
            kind: if folder { "folder" } else { "document" }.into(),
            heading_level: level,
        });
        if !folder {
            documents.push((id.to_owned(), level));
        }
    }
    let anchors = documents
        .iter()
        .map(|(id, _)| (id.clone(), format!("#{}", document_anchor(id))))
        .collect::<HashMap<_, _>>();
    let stem = manuscript_stem(
        manifest["project"]["title"]
            .as_str()
            .unwrap_or("Manuscript"),
    );
    let asset_ids = referenced_assets(snapshot, &documents)?;
    let mut allocator = NameAllocator::default();
    let mut asset_paths = HashMap::new();
    let mut attachments = Vec::new();
    for attachment in snapshot.attachments() {
        if attachment.resource_kind != "media" || !asset_ids.contains(&attachment.logical_id) {
            continue;
        }
        let extension = artifact_extension(attachment);
        let title = attachment
            .original_name
            .rsplit_once('.')
            .filter(|(_, ext)| extension.eq_ignore_ascii_case(&format!(".{ext}")))
            .map(|(stem, _)| stem)
            .unwrap_or(&attachment.original_name);
        let name = allocator.name(
            &format!("artifact:media:{}", attachment.logical_id),
            title,
            &extension,
        )?;
        asset_paths.insert(
            attachment.logical_id.clone(),
            format!("{stem}-assets/{name}"),
        );
        attachments.push(attachment.clone());
    }
    let missing = asset_ids
        .iter()
        .filter(|id| !asset_paths.contains_key(*id))
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(TransferError::Invalid(format!(
            "Included writing references missing project artifacts: {}",
            missing.join(", ")
        )));
    }
    let mut conversions = BTreeSet::new();
    conversions.insert(MarkdownConversion {
        resource_id: "project".into(),
        feature: "reader-export-is-not-a-canonical-project-backup".into(),
    });
    Ok(PreparedCompilation {
        entries,
        documents,
        anchors,
        asset_paths,
        attachments,
        conversions,
        document_path: format!("{stem}.md"),
        stem,
    })
}

fn selected_identities(
    nodes: &[Value],
    by_id: &HashMap<&str, usize>,
    selection: &ManuscriptSelection,
) -> Result<HashSet<String>> {
    match selection {
        ManuscriptSelection::Project => Ok(nodes
            .iter()
            .filter_map(|node| node["id"].as_str().map(str::to_owned))
            .collect()),
        ManuscriptSelection::Document(id) => {
            let index = by_id.get(id.as_str()).ok_or_else(|| {
                TransferError::Invalid("The selected document is not in this project".into())
            })?;
            if nodes[*index]["currentRevision"].as_u64().unwrap_or(0) == 0 {
                return Err(TransferError::Invalid(
                    "Select a content-bearing document, not a folder".into(),
                ));
            }
            Ok(HashSet::from([id.clone()]))
        }
        ManuscriptSelection::Subtree(id) => {
            if !by_id.contains_key(id.as_str()) {
                return Err(TransferError::Invalid(
                    "The selected section is not in this project".into(),
                ));
            }
            let mut included = HashSet::new();
            let mut stack = vec![id.as_str()];
            while let Some(current) = stack.pop() {
                if !included.insert(current.to_owned()) {
                    return Err(TransferError::Invalid(
                        "The project tree contains a repeated identity".into(),
                    ));
                }
                for node in nodes {
                    if node["parentId"].as_str() == Some(current)
                        && let Some(child) = node["id"].as_str()
                    {
                        stack.push(child);
                    }
                }
            }
            Ok(included)
        }
        ManuscriptSelection::Documents(ids) => {
            let selected: HashSet<String> = ids.iter().cloned().collect();
            if selected.is_empty() || selected.len() != ids.len() {
                return Err(TransferError::Invalid(
                    "Select one or more distinct documents".into(),
                ));
            }
            for id in &selected {
                let index = by_id.get(id.as_str()).ok_or_else(|| {
                    TransferError::Invalid("A selected document is not in this project".into())
                })?;
                if nodes[*index]["currentRevision"].as_u64().unwrap_or(0) == 0 {
                    return Err(TransferError::Invalid(
                        "Select content-bearing documents, not folders".into(),
                    ));
                }
            }
            Ok(selected)
        }
    }
}

fn heading_level(nodes: &[Value], id: &str, included_folders: &HashSet<&str>) -> Option<u8> {
    let by_id: HashMap<&str, &Value> = nodes
        .iter()
        .filter_map(|node| node["id"].as_str().map(|id| (id, node)))
        .collect();
    let mut level = 1_u8;
    let mut parent = by_id.get(id).and_then(|node| node["parentId"].as_str());
    while let Some(parent_id) = parent {
        if included_folders.contains(parent_id) {
            level = level.saturating_add(1).min(6);
        }
        parent = by_id
            .get(parent_id)
            .and_then(|node| node["parentId"].as_str());
    }
    Some(level)
}

fn referenced_assets(
    snapshot: &ProjectExportSnapshot,
    documents: &[(String, u8)],
) -> Result<HashSet<String>> {
    let manifest = snapshot.manifest();
    let revisions = array(manifest, "documentRevisions");
    let nodes = array(manifest, "nodes");
    let mut assets = HashSet::new();
    for (id, _) in documents {
        let Some(node) = nodes.iter().find(|node| node["id"].as_str() == Some(id)) else {
            return Err(TransferError::Invalid(
                "Included document is missing".into(),
            ));
        };
        let current = node["currentRevision"].as_u64().unwrap_or(0);
        let revision = revisions
            .iter()
            .find(|revision| {
                revision["nodeId"].as_str() == Some(id)
                    && revision["revision"].as_u64() == Some(current)
            })
            .ok_or_else(|| TransferError::Invalid("Export document head is missing".into()))?;
        collect_asset_ids(&revision["content"], &mut assets);
    }
    Ok(assets)
}

fn collect_asset_ids(value: &Value, assets: &mut HashSet<String>) {
    if let Some(id) = value["attrs"]["assetId"]
        .as_str()
        .filter(|id| !id.is_empty())
    {
        assets.insert(id.to_owned());
    }
    if let Some(id) = value["attrs"]["src"].as_str().and_then(asset_id) {
        assets.insert(id.to_owned());
    }
    for mark in array(value, "marks") {
        if let Some(id) = mark["attrs"]["href"].as_str().and_then(asset_id) {
            assets.insert(id.to_owned());
        }
    }
    for child in array(value, "content") {
        collect_asset_ids(child, assets);
    }
}

fn asset_id(source: &str) -> Option<&str> {
    let source = source.split('#').next().unwrap_or(source);
    source.strip_prefix("asset://").or_else(|| {
        source
            .strip_prefix("/api/media/")
            .and_then(|s| s.strip_suffix("/content"))
    })
}

fn document_anchor(id: &str) -> String {
    format!("document-{}", percent_encode(id))
}

fn manuscript_stem(title: &str) -> String {
    let mut stem: String = title
        .trim()
        .chars()
        .map(|character| {
            if character.is_control() || "/\\<>:\"|?*".contains(character) {
                '-'
            } else {
                character
            }
        })
        .collect();
    stem = stem.trim_end_matches(['.', ' ']).to_owned();
    while stem.chars().count() > 80 {
        stem.pop();
    }
    stem = stem.trim_end_matches(['.', ' ']).to_owned();
    if stem.is_empty() || stem == "." || stem == ".." {
        return "manuscript".into();
    }
    if stem.starts_with('.') {
        stem.insert(0, '_');
    }
    let reserved = stem.split('.').next().unwrap_or("").to_ascii_uppercase();
    if matches!(reserved.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((reserved.starts_with("COM") || reserved.starts_with("LPT"))
            && reserved.len() == 4
            && matches!(reserved.as_bytes()[3], b'1'..=b'9'))
    {
        stem.insert(0, '_');
    }
    stem
}

fn is_trashed(node: &Value) -> bool {
    node["trashedAt"]
        .as_str()
        .is_some_and(|timestamp| !timestamp.is_empty())
}

fn array<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value[key].as_array().map(Vec::as_slice).unwrap_or(&[])
}
