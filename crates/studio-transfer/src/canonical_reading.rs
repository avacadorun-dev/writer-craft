//! Bounded current reader views consume the same prepared project as native canonical objects.
use std::collections::{BTreeSet, HashMap, HashSet};
use std::io::Cursor;

use serde_json::Value;
use studio_core::{
    MAX_PORTABLE_SHARD_BYTES, PortableObjectReference, PortableProjectHeader,
    PreparedPortableProject, ProjectAttachment, ProjectReadingCanvas, ProjectReadingNode,
    ProjectTransferCancellation, visit_portable_shard,
};

use super::markdown_render::{MarkdownRenderer, escape_text, fenced};
use super::portable_markdown::{
    MarkdownConversion, artifact_extension, resource_path, rewrite_canvas_resources,
};
use super::portable_paths::{NameAllocator, relative_link};
use super::{Result, TransferError};

pub(super) struct ReadingArtifact {
    pub(super) descriptor: ProjectAttachment,
    pub(super) object: PortableObjectReference,
    pub(super) path: String,
}
/// Only current titles/order/paths are materialized. Prose, graphs and histories are not arrays.
pub(super) struct ReadingPlan {
    pub(super) project: String,
    pub(super) title: String,
    pub(super) scope: String,
    pub(super) nodes: Vec<ProjectReadingNode>,
    pub(super) artifacts: Vec<ReadingArtifact>,
    canvases: Vec<ProjectReadingCanvas>,
    ordered: Vec<usize>,
    children: HashMap<String, Vec<usize>>,
    node_paths: HashMap<String, String>,
    canvas_paths: HashMap<String, String>,
    asset_paths: HashMap<String, String>,
    conversions: BTreeSet<MarkdownConversion>,
}
impl ReadingPlan {
    pub(super) fn new(
        prepared: &PreparedPortableProject,
        cancel: &ProjectTransferCancellation,
    ) -> Result<Self> {
        let header = PortableProjectHeader::parse(prepared.root_bytes()).map_err(core_error)?;
        let mut project = None;
        visit_records(prepared, "project", cancel, &mut |record| {
            if project.is_some() {
                return invalid("Captured project identity is not unique");
            }
            project = Some((
                string(record, "logicalId")?.to_owned(),
                string(record, "title")?.to_owned(),
            ));
            Ok(())
        })?;
        let (project, title) = project
            .ok_or_else(|| TransferError::Invalid("Captured project identity is missing".into()))?;
        let mut nodes = Vec::new();
        prepared
            .visit_reading_nodes(&mut |node| {
                nodes.push(node);
                Ok(())
            })
            .map_err(core_error)?;
        let (ordered, children) = ordered_tree(&nodes)?;
        let mut allocator = NameAllocator::default();
        let mut node_paths = HashMap::new();
        let mut folders: HashMap<String, String> = HashMap::new();
        let mut conversions = BTreeSet::new();
        for index in &ordered {
            check_cancel(cancel)?;
            let node = &nodes[*index];
            let base = allocator.name(&format!("node:{}", node.logical_id), &node.title, "")?;
            let parent = node
                .parent_id
                .as_ref()
                .and_then(|id| folders.get(id))
                .map(String::as_str)
                .unwrap_or("documents");
            let container = node.current_revision == 0 || children.contains_key(&node.logical_id);
            let folder = format!("{parent}/{base}");
            let path = if container {
                format!("{folder}/index.md")
            } else {
                format!("{folder}.md")
            };
            let (folder, path) = if path.len() > 2048 {
                conversions.insert(conversion(
                    &node.logical_id,
                    "long-path-reading-layout-shortened",
                ));
                (
                    format!("documents/_long_paths/{base}"),
                    format!("documents/_long_paths/{base}.md"),
                )
            } else {
                (folder, path)
            };
            if container {
                folders.insert(node.logical_id.clone(), folder);
            }
            if node.read_only_preview {
                conversions.insert(conversion(
                    &node.logical_id,
                    "opaque-source-read-only-preview",
                ));
            }
            node_paths.insert(node.logical_id.clone(), path);
        }
        let mut artifacts = Vec::new();
        let mut asset_paths = HashMap::new();
        let mut asset_ids = HashSet::new();
        visit_records(prepared, "assets", cancel, &mut |record| {
            let object = object(&record["object"])?;
            let kind = string(record, "resourceKind")?;
            if !matches!(kind, "media" | "audio") {
                return invalid("Captured artifact kind is invalid");
            }
            let logical = string(record, "logicalId")?;
            let identity = format!("{kind}:{logical}");
            if !asset_ids.insert(identity.clone()) {
                return invalid("Captured artifact identity is not unique");
            }
            let descriptor = ProjectAttachment {
                resource_kind: kind.into(),
                logical_id: logical.into(),
                path: String::new(),
                sha256: object.sha256().into(),
                byte_size: object.byte_size(),
                media_type: string(record, "mediaType")?.into(),
                original_name: string(record, "originalName")?.into(),
                width: None,
                height: None,
            };
            let extension = artifact_extension(&descriptor);
            let title = descriptor
                .original_name
                .rsplit_once('.')
                .filter(|(_, ext)| extension.eq_ignore_ascii_case(&format!(".{ext}")))
                .map(|(stem, _)| stem)
                .unwrap_or(&descriptor.original_name);
            let name = allocator.name(&format!("artifact:{identity}"), title, &extension)?;
            let path = format!("assets/{name}");
            if kind == "media" {
                asset_paths.insert(logical.into(), path.clone());
            }
            artifacts.push(ReadingArtifact {
                descriptor,
                object,
                path,
            });
            Ok(())
        })?;
        let mut canvases = Vec::new();
        prepared
            .visit_reading_canvases(&mut |board| {
                canvases.push(board);
                Ok(())
            })
            .map_err(core_error)?;
        let mut canvas_paths = HashMap::new();
        for board in &canvases {
            let base = allocator.name(&format!("canvas:{}", board.logical_id), &board.title, "")?;
            canvas_paths.insert(board.logical_id.clone(), format!("canvases/{base}.canvas"));
            conversions.insert(conversion(
                &board.logical_id,
                "canvas-extensions-and-original-paths-in-companion",
            ));
        }
        Ok(Self {
            project,
            title,
            scope: header.scope().as_str().into(),
            nodes,
            artifacts,
            canvases,
            ordered,
            children,
            node_paths,
            canvas_paths,
            asset_paths,
            conversions,
        })
    }
    /// Each callback owns one bounded current content projection, not all manuscript contents.
    pub(super) fn visit_files(
        &self,
        prepared: &PreparedPortableProject,
        cancel: &ProjectTransferCancellation,
        visitor: &mut dyn FnMut(&str, &[u8], &BTreeSet<MarkdownConversion>) -> Result<()>,
    ) -> Result<()> {
        for index in &self.ordered {
            check_cancel(cancel)?;
            let node = &self.nodes[*index];
            let path = &self.node_paths[&node.logical_id];
            let mut conversions = BTreeSet::new();
            let mut text = if node.current_revision > 0 {
                let reading = prepared
                    .reading_document(&node.logical_id)
                    .map_err(core_error)?;
                let body = if let Some(content) = &reading.rich_content {
                    let mut renderer = MarkdownRenderer {
                        current: path,
                        project: &self.project,
                        nodes: &self.node_paths,
                        assets: &self.asset_paths,
                        conversions: BTreeSet::new(),
                    };
                    let text = renderer.render(content)?;
                    conversions.extend(
                        renderer
                            .conversions
                            .into_iter()
                            .map(|feature| conversion(&node.logical_id, &feature)),
                    );
                    text
                } else {
                    conversions.insert(conversion(&node.logical_id, "plain-text-reading-literal"));
                    fenced(&reading.body, "text")
                };
                format!("{}{body}\n", reading.source_frontmatter)
            } else {
                format!("# {}\n", escape_text(&node.title))
            };
            if node.read_only_preview {
                text.insert_str(0,"> Read-only preview. The complete original remains in the canonical companion.\n\n");
            }
            if let Some(children) = self.children.get(&node.logical_id) {
                text.push_str("\n## Writing in this section\n\n");
                for child in children {
                    let child = &self.nodes[*child];
                    text.push_str(&format!(
                        "- [{}](<{}>)\n",
                        escape_text(&child.title),
                        relative_link(path, &self.node_paths[&child.logical_id])?
                    ));
                }
            }
            visitor(path, text.as_bytes(), &conversions)?;
        }
        for board in &self.canvases {
            check_cancel(cancel)?;
            let path = &self.canvas_paths[&board.logical_id];
            let mut graph = prepared
                .reading_canvas(&board.logical_id)
                .map_err(core_error)?;
            let mut conversions = BTreeSet::new();
            if array(&graph, "nodes").iter().any(|node| {
                node["type"].as_str() == Some("file")
                    && node["x-writers-studio"]
                        .get("resource")
                        .is_none_or(Value::is_null)
            }) {
                conversions.insert(conversion(
                    &board.logical_id,
                    "unresolved-canvas-file-source-retained-in-companion",
                ));
            }
            rewrite_canvas_resources(
                &mut graph,
                path,
                &self.node_paths,
                &self.canvas_paths,
                &self.asset_paths,
            )?;
            visitor(path, &serde_json::to_vec_pretty(&graph)?, &conversions)?;
            let reading_path = path
                .strip_suffix(".canvas")
                .map(|base| format!("{base}.md"))
                .ok_or_else(|| TransferError::Invalid("Captured Canvas path is invalid".into()))?;
            let mut text = format!(
                "# {}\n\n[Open JSON Canvas](<{}>)\n\n",
                escape_text(&board.title),
                relative_link(&reading_path, path)?
            );
            for node in array(&graph, "nodes") {
                if let Some(content) = node["text"].as_str() {
                    text.push_str(&fenced(content, "text"));
                    text.push_str("\n\n");
                }
                if let Some(resource) = node["x-writers-studio"]
                    .get("resource")
                    .filter(|value| !value.is_null())
                {
                    let target = resource_path(
                        resource,
                        &self.node_paths,
                        &self.canvas_paths,
                        &self.asset_paths,
                    )?;
                    text.push_str(&format!(
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
                    text.push_str(&format!(
                        "- File source retained in canonical companion: {}\n",
                        escape_text(node["file"].as_str().unwrap_or(""))
                    ));
                }
                if let Some(url) = node["url"].as_str() {
                    text.push_str(&format!("- {}\n", escape_text(url)));
                }
            }
            visitor(&reading_path, text.as_bytes(), &BTreeSet::new())?;
        }
        self.visit_index_files(cancel, visitor)
    }
    pub(super) fn conversions(&self) -> &BTreeSet<MarkdownConversion> {
        &self.conversions
    }
    fn visit_index_files(
        &self,
        cancel: &ProjectTransferCancellation,
        visitor: &mut dyn FnMut(&str, &[u8], &BTreeSet<MarkdownConversion>) -> Result<()>,
    ) -> Result<()> {
        // A large project index is independently paged. README and completion root stay small.
        let mut page = String::new();
        let mut page_index = 1_u64;
        let mut write_line = |line: String| -> Result<()> {
            check_cancel(cancel)?;
            if page.len() + line.len() + 80 > MAX_PORTABLE_SHARD_BYTES && !page.is_empty() {
                page.push_str(&format!(
                    "\n[Next index page](<index-{:06}.md>)\n",
                    page_index + 1
                ));
                visitor(
                    &format!("index/index-{page_index:06}.md"),
                    page.as_bytes(),
                    &BTreeSet::new(),
                )?;
                page.clear();
                page_index += 1;
            }
            page.push_str(&line);
            Ok(())
        };
        for index in &self.ordered {
            let node = &self.nodes[*index];
            write_line(format!(
                "- Writing: [{}](<{}>)\n",
                escape_text(&node.title),
                relative_link("index/index-000001.md", &self.node_paths[&node.logical_id])?
            ))?;
        }
        for board in &self.canvases {
            let path = &self.canvas_paths[&board.logical_id];
            let reading = format!("{}.md", path.strip_suffix(".canvas").unwrap_or(path));
            write_line(format!(
                "- Board: [{}](<{}>)\n",
                escape_text(&board.title),
                relative_link("index/index-000001.md", &reading)?
            ))?;
        }
        for asset in &self.artifacts {
            write_line(format!(
                "- Original artifact: [{}](<{}>)\n",
                escape_text(&asset.descriptor.original_name),
                relative_link("index/index-000001.md", &asset.path)?
            ))?;
        }
        if page.is_empty() {
            page.push_str("This captured project has no writing, boards or artifacts.\n");
        }
        visitor(
            &format!("index/index-{page_index:06}.md"),
            page.as_bytes(),
            &BTreeSet::new(),
        )?;
        let readme = format!(
            "# {}\n\nA {} captured project. [Browse writing, boards and original artifacts](<index/index-000001.md>).\n\nMarkdown and JSON Canvas files are reading views. The native [canonical companion](<.writercraft/project.json>) and its verified objects retain saved histories, notes, metadata and untrusted recovery evidence. Recovery evidence does not authorize replay or Apply. The original encoded artifact bytes are unchanged.\n\n[Export completion and conversion reports](<.writercraft/export.json>) describe exact byte identities and reader differences. Mermaid, math and callouts require a compatible Markdown reader. Changing reading files does not silently change canonical project content. Restore the complete verified folder through WriterCraft; the old Studio app does not read this native format.\n",
            escape_text(&self.title),
            self.scope
        );
        visitor("README.md", readme.as_bytes(), &BTreeSet::new())
    }
}
fn ordered_tree(nodes: &[ProjectReadingNode]) -> Result<(Vec<usize>, HashMap<String, Vec<usize>>)> {
    let known = nodes
        .iter()
        .map(|node| node.logical_id.as_str())
        .collect::<HashSet<_>>();
    if known.len() != nodes.len() {
        return invalid("Captured reading node identity is not unique");
    }
    let mut children: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, node) in nodes.iter().enumerate() {
        if node
            .parent_id
            .as_deref()
            .is_some_and(|id| !known.contains(id))
        {
            return invalid("Captured reading node has an orphan parent");
        }
        children
            .entry(node.parent_id.clone().unwrap_or_default())
            .or_default()
            .push(index);
    }
    for indices in children.values_mut() {
        indices.sort_by(|a, b| {
            nodes[*a]
                .position
                .total_cmp(&nodes[*b].position)
                .then(nodes[*a].ordinal.cmp(&nodes[*b].ordinal))
        });
    }
    let mut stack = children.get("").cloned().unwrap_or_default();
    stack.reverse();
    let mut output = Vec::with_capacity(nodes.len());
    let mut seen = HashSet::new();
    while let Some(index) = stack.pop() {
        if !seen.insert(index) {
            return invalid("Captured reading tree has a cycle");
        }
        output.push(index);
        if let Some(items) = children.get(&nodes[index].logical_id) {
            stack.extend(items.iter().rev().copied());
        }
    }
    if output.len() != nodes.len() {
        return invalid("Captured reading tree has an unreachable cycle");
    }
    children.remove("");
    Ok((output, children))
}
fn visit_records(
    prepared: &PreparedPortableProject,
    domain: &str,
    cancel: &ProjectTransferCancellation,
    visitor: &mut dyn FnMut(&Value) -> Result<()>,
) -> Result<()> {
    let header = PortableProjectHeader::parse(prepared.root_bytes()).map_err(core_error)?;
    for component in header
        .components()
        .iter()
        .filter(|component| component.domain() == domain)
    {
        check_cancel(cancel)?;
        let mut bytes = Vec::new();
        prepared
            .copy_object(component.object(), &mut bytes, cancel)
            .map_err(core_error)?;
        // One metadata page is at most 1 MiB; payload objects are never collected here.
        let mut records = Vec::new();
        visit_portable_shard(component, &mut Cursor::new(bytes), cancel, &mut |record| {
            records.push(serde_json::from_str::<Value>(record.raw_json())?);
            Ok(())
        })
        .map_err(core_error)?;
        for record in records {
            visitor(&record)?;
        }
    }
    Ok(())
}
fn object(value: &Value) -> Result<PortableObjectReference> {
    PortableObjectReference::new(
        string(value, "sha256")?,
        value["byteSize"]
            .as_u64()
            .ok_or_else(|| TransferError::Invalid("Captured object size is invalid".into()))?,
    )
    .map_err(core_error)
}
fn conversion(resource: &str, feature: &str) -> MarkdownConversion {
    MarkdownConversion {
        resource_id: resource.into(),
        feature: feature.into(),
    }
}
fn array<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value[key].as_array().map(Vec::as_slice).unwrap_or(&[])
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .ok_or_else(|| TransferError::Invalid(format!("Captured reading field {key} is missing")))
}
fn check_cancel(cancel: &ProjectTransferCancellation) -> Result<()> {
    if cancel.is_cancelled() {
        return invalid("Portable reading export cancelled");
    }
    Ok(())
}
fn core_error(error: studio_core::StudioError) -> TransferError {
    TransferError::Invalid(error.to_string())
}
fn invalid<T>(message: &str) -> Result<T> {
    Err(TransferError::Invalid(message.into()))
}
