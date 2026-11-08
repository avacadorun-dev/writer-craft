//! Portable reading files and an explicitly complete or selected canonical companion.
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use studio_core::{
    ProjectAttachment, ProjectExportSnapshot, ProjectTransferCancellation, is_workspace_data_path,
};

use super::markdown_render::{MarkdownRenderer, escape_text, fenced};
use super::portable_paths::{NameAllocator, relative_file_path, relative_link, safe_relative};
use super::portable_selection::selected_companion;
use super::{Result, TransferError};

/// Explicit scope; a selected-document export never carries unrelated project writing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkdownExportSelection {
    /// Saved documents, all histories/metadata/Canvas/artifacts and native recovery evidence.
    Project,
    /// Only these project-scoped logical document identities and their related evidence.
    Documents(Vec<String>),
}
/// A verified output member relative to the exclusively created destination root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableExportFile {
    /// Safe relative Unicode path, never an absolute machine location.
    pub path: String,
    /// SHA-256 of the actual flushed bytes.
    pub sha256: String,
    /// Actual encoded byte length.
    pub byte_size: u64,
}
/// One documented Markdown reading-view conversion; canonical semantics remain separate.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownConversion {
    /// Logical source resource, or project for overall scope information.
    pub resource_id: String,
    /// Stable English conversion identifier for the portable report.
    pub feature: String,
}
/// Receipt exists only after all files verify and the incomplete marker is removed.
#[derive(Debug, Clone, Serialize)]
pub struct MarkdownExportReceipt {
    /// Exact user-selected new directory.
    pub destination: PathBuf,
    /// Explicit complete-project or selected-documents scope.
    pub scope: String,
    /// Verified payloads; the final completion manifest does not hash itself.
    pub files: Vec<PortableExportFile>,
    /// Every known reading conversion, including reader extensions and scoped omissions.
    pub conversions: Vec<MarkdownConversion>,
}

struct ExportPlan {
    manifest: Value,
    companion: Vec<u8>,
    node_paths: HashMap<String, String>,
    canvas_paths: HashMap<String, String>,
    asset_paths: HashMap<String, String>,
    artifact_paths: HashMap<String, String>,
    attachments: Vec<ProjectAttachment>,
    ordered_nodes: Vec<usize>,
    conversions: BTreeSet<MarkdownConversion>,
    scope: String,
}

/// Export one captured snapshot beneath a new exclusive directory, with no existing-file merge.
///
/// `source_database` is the actual selected workspace path from the native entrypoint. Reserved
/// database/journal/preferences destinations are refused before any creation. On interruption,
/// `.incomplete` identifies the owned partial output; no recursive deletion touches user paths.
/// This completion-manifest protocol does not claim atomic directory replacement or universal
/// power-loss durability. Run it on a dedicated worker, not the desktop Workspace save mutex.
pub fn export_markdown_folder(
    snapshot: &ProjectExportSnapshot,
    source_database: &Path,
    destination: &Path,
    selection: &MarkdownExportSelection,
    cancel: &ProjectTransferCancellation,
) -> Result<MarkdownExportReceipt> {
    export_markdown_folder_with_progress(
        snapshot,
        source_database,
        destination,
        selection,
        cancel,
        &mut |_| {},
    )
}

/// Bounded worker progress; a native adapter should forward it without blocking the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownExportProgress {
    /// Relative payload currently about to be written; user content is not translated.
    pub path: String,
    /// Payloads written so far, excluding the incomplete marker.
    pub completed: usize,
}
/// Export with a quick progress callback. Cancellation is checked again after every callback.
/// The callback grants no mutation authority and should not perform blocking work.
pub fn export_markdown_folder_with_progress(
    snapshot: &ProjectExportSnapshot,
    source_database: &Path,
    destination: &Path,
    selection: &MarkdownExportSelection,
    cancel: &ProjectTransferCancellation,
    progress: &mut dyn FnMut(MarkdownExportProgress),
) -> Result<MarkdownExportReceipt> {
    if is_workspace_data_path(destination, source_database)? {
        return invalid(
            "Choose a new export folder away from workspace databases, journals and preferences",
        );
    }
    check_cancel(cancel)?;
    let mut plan = plan_export(snapshot, selection)?;
    check_cancel(cancel)?;
    std::fs::create_dir(destination)?;
    let marker = destination.join(".incomplete");
    let mut marker_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&marker)?;
    marker_file.write_all(
        b"WriterCraft export is incomplete. Do not treat this directory as a completed backup.\n",
    )?;
    marker_file.sync_all()?;
    drop(marker_file);
    let mut directories = HashSet::from([PathBuf::new()]);
    let mut files = Vec::new();
    let project_id = plan.manifest["project"]["id"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    let nodes = array(&plan.manifest, "nodes");
    let revisions = array(&plan.manifest, "documentRevisions");
    for index in &plan.ordered_nodes {
        check_cancel(cancel)?;
        let node = &nodes[*index];
        let id = required(node, "id")?;
        let path = plan
            .node_paths
            .get(id)
            .ok_or_else(|| TransferError::Invalid("Export node path is missing".into()))?;
        progress(MarkdownExportProgress {
            path: path.clone(),
            completed: files.len(),
        });
        check_cancel(cancel)?;
        let current = node["currentRevision"].as_u64().unwrap_or(0);
        let body = if current > 0 {
            let revision = revisions
                .iter()
                .find(|revision| {
                    revision["nodeId"].as_str() == Some(id)
                        && revision["revision"].as_u64() == Some(current)
                })
                .ok_or_else(|| TransferError::Invalid("Export document head is missing".into()))?;
            let mut renderer = MarkdownRenderer {
                current: path,
                project: &project_id,
                nodes: &plan.node_paths,
                assets: &plan.asset_paths,
                conversions: BTreeSet::new(),
            };
            let text = renderer.render(&revision["content"])?;
            plan.conversions
                .extend(
                    renderer
                        .conversions
                        .into_iter()
                        .map(|feature| MarkdownConversion {
                            resource_id: id.to_owned(),
                            feature,
                        }),
                );
            let frontmatter = node["sourceFrontmatter"].as_str().unwrap_or("");
            format!("{frontmatter}{text}\n")
        } else {
            let mut body = format!(
                "# {}\n\n",
                escape_text(node["title"].as_str().unwrap_or(""))
            );
            for child in nodes
                .iter()
                .filter(|child| child["parentId"].as_str() == Some(id))
            {
                let child_id = required(child, "id")?;
                let target = plan
                    .node_paths
                    .get(child_id)
                    .ok_or_else(|| TransferError::Invalid("Export child path is missing".into()))?;
                body.push_str(&format!(
                    "- [{}](<{}>)\n",
                    escape_text(child["title"].as_str().unwrap_or("")),
                    relative_link(path, target)?
                ));
            }
            body
        };
        files.push(write_new(
            destination,
            path,
            body.as_bytes(),
            &mut directories,
            cancel,
        )?);
    }
    for attachment in &plan.attachments {
        check_cancel(cancel)?;
        let key = asset_key(&attachment.resource_kind, &attachment.logical_id);
        let path = plan
            .artifact_paths
            .get(&key)
            .ok_or_else(|| TransferError::Invalid("Export artifact path is missing".into()))?;
        progress(MarkdownExportProgress {
            path: path.clone(),
            completed: files.len(),
        });
        check_cancel(cancel)?;
        ensure_parent(destination, path, &mut directories)?;
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination.join(path))?;
        let mut writer = DigestWriter {
            file,
            digest: Sha256::new(),
            length: 0,
        };
        snapshot
            .assets()
            .copy_to(attachment, &mut writer, cancel)
            .map_err(core_error)?;
        files.push(writer.finish(path)?);
    }
    for canvas in array(&plan.manifest, "canvases") {
        check_cancel(cancel)?;
        let id = required(canvas, "id")?;
        let path = plan
            .canvas_paths
            .get(id)
            .ok_or_else(|| TransferError::Invalid("Export Canvas path is missing".into()))?;
        let current = canvas["currentRevision"].as_u64().unwrap_or(0);
        let revision = array(&plan.manifest, "canvasRevisions")
            .iter()
            .find(|revision| {
                revision["canvasId"].as_str() == Some(id)
                    && revision["revision"].as_u64() == Some(current)
            })
            .ok_or_else(|| TransferError::Invalid("Export Canvas head is missing".into()))?;
        let mut content = revision["content"].clone();
        if array(&content, "nodes").iter().any(|node| {
            node["type"].as_str() == Some("file")
                && node["x-writers-studio"]
                    .get("resource")
                    .is_none_or(Value::is_null)
        }) {
            plan.conversions.insert(MarkdownConversion {
                resource_id: id.to_owned(),
                feature: "unresolved-canvas-file-source-retained-in-companion".into(),
            });
        }
        rewrite_canvas_resources(
            &mut content,
            path,
            &plan.node_paths,
            &plan.canvas_paths,
            &plan.asset_paths,
        )?;
        files.push(write_new(
            destination,
            path,
            &serde_json::to_vec_pretty(&content)?,
            &mut directories,
            cancel,
        )?);
        let reading_path = path
            .strip_suffix(".canvas")
            .map(|s| format!("{s}.md"))
            .ok_or_else(|| TransferError::Invalid("Export Canvas extension is invalid".into()))?;
        let mut reading = format!(
            "# {}\n\n[Open JSON Canvas](<{}>)\n\n",
            escape_text(canvas["title"].as_str().unwrap_or("")),
            relative_link(&reading_path, path)?
        );
        for node in array(&revision["content"], "nodes") {
            if let Some(text) = node["text"].as_str() {
                reading.push_str(&fenced(text, "text"));
                reading.push_str("\n\n");
            }
            if let Some(resource) = node["x-writers-studio"]
                .get("resource")
                .filter(|value| !value.is_null())
            {
                let target = resource_path(
                    resource,
                    &plan.node_paths,
                    &plan.canvas_paths,
                    &plan.asset_paths,
                )?;
                reading.push_str(&format!(
                    "- [{}](<{}>)\n",
                    escape_text(node["file"].as_str().unwrap_or("Resource")),
                    relative_link(&reading_path, target)?
                ));
            }
            if node["type"].as_str() == Some("file")
                && node["x-writers-studio"]
                    .get("resource")
                    .is_none_or(Value::is_null)
            {
                reading.push_str(&format!(
                    "- File source retained in canonical companion: {}\n",
                    escape_text(node["file"].as_str().unwrap_or(""))
                ));
            }
            if let Some(url) = node["url"].as_str() {
                reading.push_str(&format!("- {}\n", escape_text(url)));
            }
        }
        files.push(write_new(
            destination,
            &reading_path,
            reading.as_bytes(),
            &mut directories,
            cancel,
        )?);
    }
    files.push(write_new(
        destination,
        ".writercraft/project.json",
        &plan.companion,
        &mut directories,
        cancel,
    )?);
    let readme = reading_index(&plan)?;
    files.push(write_new(
        destination,
        "README.md",
        readme.as_bytes(),
        &mut directories,
        cancel,
    )?);
    let conversions: Vec<MarkdownConversion> = plan.conversions.into_iter().collect();
    for file in &files {
        verify_file(destination, file, cancel)?;
    }
    check_cancel(cancel)?;
    let complete = serde_json::to_vec_pretty(
        &json!({"format":"writercraft-portable-export","formatVersion":1,"state":"complete","scope":plan.scope,"files":files,"conversions":conversions,"canonicalCompanion":".writercraft/project.json"}),
    )?;
    write_new(
        destination,
        ".writercraft/export.json",
        &complete,
        &mut directories,
        cancel,
    )?;
    let mut sync_paths = directories.iter().collect::<Vec<_>>();
    sync_paths.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for relative in sync_paths {
        sync_directory(&destination.join(relative))?;
    }
    sync_directory(
        destination
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new(".")),
    )?;
    check_cancel(cancel)?;
    std::fs::remove_file(marker)?;
    Ok(MarkdownExportReceipt {
        destination: destination.to_path_buf(),
        scope: plan.scope,
        files,
        conversions,
    })
}

fn plan_export(
    snapshot: &ProjectExportSnapshot,
    selection: &MarkdownExportSelection,
) -> Result<ExportPlan> {
    let (manifest, companion, asset_ids, audio_ids, outside_source_references, scope) =
        match selection {
            MarkdownExportSelection::Project => (
                snapshot.manifest().clone(),
                snapshot.canonical_manifest_bytes().map_err(core_error)?,
                None,
                None,
                false,
                "complete-project",
            ),
            MarkdownExportSelection::Documents(ids) => {
                let selected: HashSet<String> = ids.iter().cloned().collect();
                if selected.is_empty() || selected.len() != ids.len() {
                    return invalid("Select one or more distinct document identities");
                }
                let selected = selected_companion(snapshot, &selected)?;
                (
                    selected.manifest,
                    selected.bytes,
                    Some(selected.asset_ids),
                    Some(selected.audio_ids),
                    selected.outside_source_references,
                    "selected-documents",
                )
            }
        };
    let nodes = array(&manifest, "nodes");
    let ordered_nodes = ordered_tree(nodes, scope == "selected-documents")?;
    let mut allocator = NameAllocator::default();
    let mut node_paths = HashMap::new();
    let mut folders = HashMap::new();
    let mut conversions = BTreeSet::new();
    for index in &ordered_nodes {
        let node = &nodes[*index];
        let id = required(node, "id")?;
        let base = allocator.name(
            &format!("node:{id}"),
            node["title"].as_str().unwrap_or("untitled"),
            "",
        )?;
        let parent = node["parentId"]
            .as_str()
            .and_then(|id| folders.get(id))
            .map(String::as_str)
            .unwrap_or("documents");
        let container = node["currentRevision"].as_u64().unwrap_or(0) == 0
            || nodes
                .iter()
                .any(|child| child["parentId"].as_str() == Some(id));
        let folder = format!("{parent}/{base}");
        let path = if container {
            format!("{folder}/index.md")
        } else {
            format!("{folder}.md")
        };
        let (folder, path) = if path.len() > 2048 {
            conversions.insert(MarkdownConversion {
                resource_id: id.to_owned(),
                feature: "long-path-reading-layout-shortened".into(),
            });
            (
                format!("_long_paths/{base}"),
                format!("_long_paths/{base}.md"),
            )
        } else {
            (folder, path)
        };
        if container {
            folders.insert(id.to_owned(), folder);
        }
        node_paths.insert(id.to_owned(), path);
    }
    let attachments = snapshot
        .attachments()
        .iter()
        .filter(|attachment| match attachment.resource_kind.as_str() {
            "media" => asset_ids
                .as_ref()
                .is_none_or(|ids| ids.contains(&attachment.logical_id)),
            "audio" => audio_ids
                .as_ref()
                .is_none_or(|ids| ids.contains(&attachment.logical_id)),
            _ => false,
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut asset_paths = HashMap::new();
    let mut artifact_paths = HashMap::new();
    for attachment in &attachments {
        let key = asset_key(&attachment.resource_kind, &attachment.logical_id);
        let extension = artifact_extension(attachment);
        let title = attachment
            .original_name
            .rsplit_once('.')
            .filter(|(_, ext)| extension.eq_ignore_ascii_case(&format!(".{ext}")))
            .map(|(stem, _)| stem)
            .unwrap_or(&attachment.original_name);
        let base = allocator.name(&format!("artifact:{key}"), title, &extension)?;
        let path = format!("assets/{base}");
        artifact_paths.insert(key, path.clone());
        if attachment.resource_kind == "media" {
            asset_paths.insert(attachment.logical_id.clone(), path);
        }
    }
    if let Some(ids) = asset_ids
        && ids.iter().any(|id| !asset_paths.contains_key(id))
    {
        return invalid("A selected document references a missing project artifact");
    }
    let mut canvas_paths = HashMap::new();
    for canvas in array(&manifest, "canvases") {
        let id = required(canvas, "id")?;
        let base = allocator.name(
            &format!("canvas:{id}"),
            canvas["title"].as_str().unwrap_or("Canvas"),
            "",
        )?;
        canvas_paths.insert(id.to_owned(), format!("canvases/{base}.canvas"));
        conversions.insert(MarkdownConversion {
            resource_id: id.to_owned(),
            feature: "canvas-extensions-and-original-paths-in-companion".into(),
        });
    }
    record_opaque_previews(snapshot, &node_paths, &mut conversions)?;
    if outside_source_references {
        conversions.insert(MarkdownConversion {
            resource_id: "project".into(),
            feature: "out-of-scope-source-references-retained-opaque".into(),
        });
    }
    conversions.insert(MarkdownConversion {
        resource_id: "project".into(),
        feature: "canonical-companion-retains-history-metadata-notes-and-recovery".into(),
    });
    if scope == "selected-documents" {
        conversions.insert(MarkdownConversion {
            resource_id: "project".into(),
            feature: "selected-documents-is-not-a-complete-project-backup".into(),
        });
    }
    Ok(ExportPlan {
        manifest,
        companion,
        node_paths,
        canvas_paths,
        asset_paths,
        artifact_paths,
        attachments,
        ordered_nodes,
        conversions,
        scope: scope.into(),
    })
}
fn ordered_tree(nodes: &[Value], allow_outside_parents: bool) -> Result<Vec<usize>> {
    let known: HashSet<&str> = nodes
        .iter()
        .map(|node| required(node, "id"))
        .collect::<Result<_>>()?;
    let mut children: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, node) in nodes.iter().enumerate() {
        let parent = node["parentId"].as_str().unwrap_or("");
        if !parent.is_empty() && !known.contains(parent) && !allow_outside_parents {
            return invalid("Export tree has an orphan parent");
        }
        children
            .entry(if known.contains(parent) { parent } else { "" })
            .or_default()
            .push(index);
    }
    for indices in children.values_mut() {
        indices.sort_by(|a, b| {
            nodes[*a]["position"]
                .as_f64()
                .unwrap_or(0.0)
                .total_cmp(&nodes[*b]["position"].as_f64().unwrap_or(0.0))
                .then(a.cmp(b))
        });
    }
    let mut stack = children.get("").cloned().unwrap_or_default();
    stack.reverse();
    let mut output = Vec::with_capacity(nodes.len());
    let mut seen = HashSet::new();
    while let Some(index) = stack.pop() {
        if !seen.insert(index) {
            return invalid("Export tree has a cycle");
        }
        output.push(index);
        if let Some(items) = children.get(required(&nodes[index], "id")?) {
            stack.extend(items.iter().rev().copied());
        }
    }
    if output.len() != nodes.len() {
        return invalid("Export tree has an unreachable cycle");
    }
    Ok(output)
}
fn reading_index(plan: &ExportPlan) -> Result<String> {
    let mut out = format!(
        "# {}\n\nMarkdown is the reading view. The [canonical companion](<.writercraft/project.json>) retains records, histories, notes, metadata and recovery evidence. Project-owned artifacts keep their original bytes. External URLs remain external. Reader extensions and conversion details are listed in [the export manifest](<.writercraft/export.json>).\n\n",
        escape_text(
            plan.manifest["project"]["title"]
                .as_str()
                .unwrap_or("Project")
        )
    );
    if plan.scope == "selected-documents" {
        out.push_str("This export contains selected documents and related evidence. It is not a complete project backup.\n\n");
    }
    for index in &plan.ordered_nodes {
        let node = &array(&plan.manifest, "nodes")[*index];
        let id = required(node, "id")?;
        out.push_str(&format!(
            "- [{}](<{}>)\n",
            escape_text(node["title"].as_str().unwrap_or("")),
            relative_link(
                "README.md",
                plan.node_paths
                    .get(id)
                    .ok_or_else(|| TransferError::Invalid(
                        "Reading-index node is missing".into()
                    ))?
            )?
        ));
    }
    if !plan.canvas_paths.is_empty() {
        out.push_str("\n## Canvas\n\n");
        for canvas in array(&plan.manifest, "canvases") {
            let id = required(canvas, "id")?;
            let path = plan
                .canvas_paths
                .get(id)
                .ok_or_else(|| TransferError::Invalid("Reading-index Canvas is missing".into()))?;
            let reading = format!("{}.md", path.strip_suffix(".canvas").unwrap_or(path));
            out.push_str(&format!(
                "- [{}](<{}>)\n",
                escape_text(canvas["title"].as_str().unwrap_or("")),
                relative_link("README.md", &reading)?
            ));
        }
    }
    if !plan.attachments.is_empty() {
        out.push_str("\n## Original artifacts\n\n");
        for attachment in &plan.attachments {
            let path = plan
                .artifact_paths
                .get(&asset_key(
                    &attachment.resource_kind,
                    &attachment.logical_id,
                ))
                .ok_or_else(|| {
                    TransferError::Invalid("Reading-index artifact is missing".into())
                })?;
            out.push_str(&format!(
                "- [{}](<{}>)\n",
                escape_text(&attachment.original_name),
                relative_link("README.md", path)?
            ));
        }
    }
    Ok(out)
}
pub(super) fn rewrite_canvas_resources(
    content: &mut Value,
    current: &str,
    nodes: &HashMap<String, String>,
    canvases: &HashMap<String, String>,
    assets: &HashMap<String, String>,
) -> Result<()> {
    if let Some(items) = content["nodes"].as_array_mut() {
        for node in items {
            if node["type"].as_str() == Some("file") {
                if let Some(resource) = node["x-writers-studio"]
                    .get("resource")
                    .filter(|value| !value.is_null())
                {
                    let target = resource_path(resource, nodes, canvases, assets)?;
                    let resource = json!({"kind":resource["kind"],"id":resource["id"]});
                    node["file"] = Value::String(relative_file_path(current, target)?);
                    // Only a scoped resource identifier belongs to the reader extension. Authored
                    // private/unknown payloads and source paths remain solely canonical objects.
                    node["x-writers-studio"] = json!({"resource":resource});
                } else {
                    reading_placeholder(
                        node,
                        "Unresolved file reference. The original is retained in the WriterCraft canonical companion.",
                    );
                }
            } else {
                if let Some(fields) = node.as_object_mut() {
                    fields.remove("x-writers-studio");
                }
                let kind = node["type"].as_str().unwrap_or("");
                if !matches!(kind, "text" | "link" | "group")
                    || (kind == "link"
                        && !node["url"].as_str().is_some_and(|url| {
                            url.starts_with("https://") || url.starts_with("http://")
                        }))
                {
                    reading_placeholder(
                        node,
                        "Unsupported Canvas item. The original is retained in the WriterCraft canonical companion.",
                    );
                }
            }
            let kind = node["type"].as_str().unwrap_or("").to_owned();
            if let Some(fields) = node.as_object_mut() {
                fields.retain(|key, _| {
                    ["id", "type", "x", "y", "width", "height", "color"].contains(&key.as_str())
                        || match kind.as_str() {
                            "file" => {
                                ["file", "subpath", "x-writers-studio"].contains(&key.as_str())
                            }
                            "text" => key == "text",
                            "link" => key == "url",
                            "group" => key == "label",
                            _ => false,
                        }
                });
            }
        }
    }
    if let Some(edges) = content["edges"].as_array_mut() {
        for edge in edges {
            if let Some(fields) = edge.as_object_mut() {
                fields.retain(|key, _| {
                    [
                        "id", "fromNode", "fromSide", "toNode", "toSide", "fromEnd", "toEnd",
                        "color", "label",
                    ]
                    .contains(&key.as_str())
                });
            }
        }
    }
    if let Some(fields) = content.as_object_mut() {
        fields.retain(|key, _| matches!(key.as_str(), "nodes" | "edges"));
    }
    Ok(())
}
fn reading_placeholder(node: &mut Value, message: &str) {
    if let Some(fields) = node.as_object_mut() {
        for key in ["file", "subpath", "url", "x-writers-studio"] {
            fields.remove(key);
        }
    }
    node["type"] = json!("text");
    node["text"] = json!(message);
}
pub(super) fn resource_path<'a>(
    resource: &Value,
    nodes: &'a HashMap<String, String>,
    canvases: &'a HashMap<String, String>,
    assets: &'a HashMap<String, String>,
) -> Result<&'a str> {
    let id = required(resource, "id")?;
    match required(resource, "kind")? {
        "document" => nodes.get(id),
        "canvas" => canvases.get(id),
        "media" => assets.get(id),
        _ => None,
    }
    .map(String::as_str)
    .ok_or_else(|| {
        TransferError::Invalid("A Canvas resource is missing from the portable snapshot".into())
    })
}
fn asset_key(kind: &str, id: &str) -> String {
    format!("{kind}:{id}")
}
pub(super) fn artifact_extension(attachment: &ProjectAttachment) -> String {
    let known = extension(&attachment.media_type);
    if known != ".bin" {
        return known.into();
    }
    attachment
        .original_name
        .rsplit_once('.')
        .map(|(_, ext)| ext)
        .filter(|ext| {
            !ext.is_empty() && ext.len() <= 12 && ext.bytes().all(|b| b.is_ascii_alphanumeric())
        })
        .map(|ext| format!(".{ext}"))
        .unwrap_or_else(|| ".bin".into())
}
fn extension(mime: &str) -> &str {
    match mime {
        "image/png" => ".png",
        "image/jpeg" => ".jpg",
        "image/gif" => ".gif",
        "image/webp" => ".webp",
        "application/pdf" => ".pdf",
        "audio/mpeg" => ".mp3",
        "audio/mp4" => ".m4a",
        "audio/ogg" => ".ogg",
        "audio/wav" => ".wav",
        "audio/webm" | "video/webm" => ".webm",
        "video/mp4" => ".mp4",
        _ => ".bin",
    }
}
fn array<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value[key].as_array().map(Vec::as_slice).unwrap_or(&[])
}
fn required<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .ok_or_else(|| TransferError::Invalid(format!("Portable export field {key} is missing")))
}
fn invalid<T>(message: &str) -> Result<T> {
    Err(TransferError::Invalid(message.into()))
}
fn core_error(error: studio_core::StudioError) -> TransferError {
    TransferError::Invalid(error.to_string())
}
fn check_cancel(cancel: &ProjectTransferCancellation) -> Result<()> {
    if cancel.is_cancelled() {
        return invalid("Export cancelled; any new output remains explicitly incomplete");
    }
    Ok(())
}
fn ensure_parent(root: &Path, relative: &str, owned: &mut HashSet<PathBuf>) -> Result<()> {
    if !safe_relative(relative) {
        return invalid("Unsafe portable output path");
    }
    let path = Path::new(relative);
    let mut prefix = PathBuf::new();
    if let Some(parent) = path.parent() {
        for part in parent.components() {
            prefix.push(part.as_os_str());
            if !owned.contains(&prefix) {
                std::fs::create_dir(root.join(&prefix))?;
                owned.insert(prefix.clone());
            }
        }
    }
    Ok(())
}
fn write_new(
    root: &Path,
    relative: &str,
    bytes: &[u8],
    directories: &mut HashSet<PathBuf>,
    cancel: &ProjectTransferCancellation,
) -> Result<PortableExportFile> {
    check_cancel(cancel)?;
    ensure_parent(root, relative, directories)?;
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(relative))?;
    let mut writer = DigestWriter {
        file,
        digest: Sha256::new(),
        length: 0,
    };
    for chunk in bytes.chunks(64 * 1024) {
        check_cancel(cancel)?;
        writer.write_all(chunk)?;
    }
    writer.finish(relative)
}
struct DigestWriter {
    file: File,
    digest: Sha256,
    length: u64,
}
impl Write for DigestWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let written = self.file.write(bytes)?;
        self.digest.update(&bytes[..written]);
        self.length = self
            .length
            .checked_add(u64::try_from(written).map_err(std::io::Error::other)?)
            .ok_or_else(|| std::io::Error::other("Export byte count overflow"))?;
        Ok(written)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}
impl DigestWriter {
    fn finish(mut self, path: &str) -> Result<PortableExportFile> {
        self.flush()?;
        self.file.sync_all()?;
        Ok(PortableExportFile {
            path: path.into(),
            sha256: format!("{:x}", self.digest.finalize()),
            byte_size: self.length,
        })
    }
}
fn verify_file(
    root: &Path,
    expected: &PortableExportFile,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    let path = root.join(&expected.path);
    let metadata = std::fs::symlink_metadata(&path)?;
    if !metadata.file_type().is_file() || metadata.len() != expected.byte_size {
        return invalid("Export member identity or size changed before completion");
    }
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0; 64 * 1024];
    loop {
        check_cancel(cancel)?;
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    if format!("{:x}", digest.finalize()) != expected.sha256 {
        return invalid("Export member checksum changed before completion");
    }
    Ok(())
}
fn sync_directory(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        File::open(path)?.sync_all()?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

fn record_opaque_previews(
    snapshot: &ProjectExportSnapshot,
    exported_nodes: &HashMap<String, String>,
    conversions: &mut BTreeSet<MarkdownConversion>,
) -> Result<()> {
    let native = snapshot.native_state();
    for opaque in array(native, "opaqueDocuments") {
        let identity = required(opaque, "nativeDocumentId")?;
        let logical = native["documentAliases"][identity]
            .as_str()
            .ok_or_else(|| {
                TransferError::Invalid(
                    "Opaque preview is missing its captured logical identity".into(),
                )
            })?;
        if exported_nodes.contains_key(logical) {
            conversions.insert(MarkdownConversion {
                resource_id: logical.to_owned(),
                feature: "opaque-import-reading-preview-original-json-in-companion".into(),
            });
        }
    }
    Ok(())
}
