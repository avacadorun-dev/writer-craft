//! Presentation-only imported hierarchy; canonical structure stays in shared core storage.
use super::*;
use std::collections::{HashMap, HashSet};
use studio_core::ProjectTreeNode;

struct OutlineNode {
    logical_id: String,
    document_id: Option<String>,
    title: String,
    kind: String,
    position: f64,
    ordinal: u64,
    parent: Option<usize>,
    children: Vec<usize>,
}

#[derive(Default)]
pub(super) struct ProjectOutline {
    project_id: String,
    nodes: Vec<OutlineNode>,
    order: Vec<(usize, usize)>,
    mapped_documents: HashSet<String>,
    document_rows: HashMap<String, usize>,
    collapsed: HashSet<usize>,
    visible: Vec<(usize, usize)>,
    query: String,
    selected_document: Option<String>,
    dirty: bool,
}

impl ProjectOutline {
    pub(super) fn prepare(project: &str, source: &[ProjectTreeNode]) -> AppResult<Self> {
        let mut outline = Self {
            project_id: project.into(),
            dirty: true,
            ..Self::default()
        };
        let mut by_id = HashMap::new();
        let mut parents = Vec::new();
        for item in source {
            if let Some(document) = &item.document_id {
                outline.mapped_documents.insert(document.clone());
            }
            if item.metadata["trashedAt"]
                .as_str()
                .is_some_and(|s| !s.is_empty())
            {
                continue;
            }
            let index = outline.nodes.len();
            if by_id.insert(item.logical_id.as_str(), index).is_some() {
                return Err(studio_core::StudioError::CorruptData(
                    "Duplicate project navigation identity".into(),
                )
                .into());
            }
            let position = item.metadata["position"]
                .as_f64()
                .filter(|n| n.is_finite())
                .ok_or_else(|| {
                    studio_core::StudioError::CorruptData(
                        "Invalid project navigation position".into(),
                    )
                })?;
            let title = item.metadata["title"].as_str().ok_or_else(|| {
                studio_core::StudioError::CorruptData("Missing project navigation title".into())
            })?;
            parents.push(item.metadata["parentId"].as_str());
            outline.nodes.push(OutlineNode {
                logical_id: item.logical_id.clone(),
                document_id: item.document_id.clone(),
                title: title.into(),
                kind: item.metadata["kind"].as_str().unwrap_or_default().into(),
                position,
                ordinal: item.ordinal,
                parent: None,
                children: Vec::new(),
            });
            if let Some(document) = &item.document_id {
                outline.document_rows.insert(document.clone(), index);
            }
        }
        let mut sorted: Vec<_> = (0..outline.nodes.len()).collect();
        sorted.sort_by(|&a, &b| {
            let (a, b) = (&outline.nodes[a], &outline.nodes[b]);
            a.position
                .total_cmp(&b.position)
                .then_with(|| a.title.cmp(&b.title))
                .then(a.ordinal.cmp(&b.ordinal))
                .then_with(|| a.logical_id.cmp(&b.logical_id))
        });
        let mut roots = Vec::new();
        for &index in &sorted {
            let parent = parents[index].and_then(|id| by_id.get(id).copied());
            if let Some(parent) = parent {
                outline.nodes[index].parent = Some(parent);
                outline.nodes[parent].children.push(index);
            } else {
                roots.push(index);
            }
        }
        let mut visited = vec![false; outline.nodes.len()];
        // Process real roots first, then any damaged cyclic component exactly once.
        for root in roots.into_iter().chain(sorted) {
            if visited[root] {
                continue;
            }
            let mut pending = vec![(root, 0_usize)];
            while let Some((index, depth)) = pending.pop() {
                if std::mem::replace(&mut visited[index], true) {
                    continue;
                }
                outline.order.push((index, depth));
                pending.extend(
                    outline.nodes[index]
                        .children
                        .iter()
                        .rev()
                        .map(|&child| (child, depth + 1)),
                );
            }
        }
        Ok(outline)
    }

    fn synchronize(&mut self, documents: &[Document], selected: Option<&str>, query: &str) {
        for document in documents
            .iter()
            .filter(|document| document.project_id == self.project_id)
        {
            if let Some(&index) = self.document_rows.get(&document.id) {
                if self.nodes[index].title != document.title {
                    self.nodes[index].title.clone_from(&document.title);
                    self.dirty = true;
                }
            } else if !self.mapped_documents.contains(&document.id) {
                let index = self.nodes.len();
                self.nodes.push(OutlineNode {
                    logical_id: document.id.clone(),
                    document_id: Some(document.id.clone()),
                    title: document.title.clone(),
                    kind: String::new(),
                    position: 0.0,
                    ordinal: 0,
                    parent: None,
                    children: Vec::new(),
                });
                self.order.push((index, 0));
                self.document_rows.insert(document.id.clone(), index);
                self.dirty = true;
            }
        }
        if self.selected_document.as_deref() != selected {
            self.selected_document = selected.map(str::to_owned);
            let mut cursor = selected
                .and_then(|id| self.document_rows.get(id).copied())
                .and_then(|index| self.nodes[index].parent);
            let mut visited = HashSet::new();
            while let Some(index) = cursor {
                if !visited.insert(index) {
                    break;
                }
                self.collapsed.remove(&index);
                cursor = self.nodes[index].parent;
            }
            self.dirty = true;
        }
        let query = query.trim().to_lowercase();
        if self.query != query {
            self.query = query;
            self.dirty = true;
        }
        if self.dirty {
            self.rebuild_visible();
            self.dirty = false;
        }
    }

    fn rebuild_visible(&mut self) {
        let searching = !self.query.is_empty();
        let mut include = vec![!searching; self.nodes.len()];
        if searching {
            for start in 0..self.nodes.len() {
                if !self.nodes[start].title.to_lowercase().contains(&self.query) {
                    continue;
                }
                let mut cursor = Some(start);
                while let Some(index) = cursor {
                    if std::mem::replace(&mut include[index], true) {
                        break;
                    }
                    cursor = self.nodes[index].parent;
                }
            }
        }
        self.visible.clear();
        let mut hidden_below = None;
        for &(index, depth) in &self.order {
            if hidden_below.is_some_and(|ancestor| depth > ancestor) {
                continue;
            }
            hidden_below = None;
            if !include[index] {
                continue;
            }
            self.visible.push((index, depth));
            if !searching && self.collapsed.contains(&index) {
                hidden_below = Some(depth);
            }
        }
    }

    pub(super) fn show(
        &mut self,
        ui: &mut egui::Ui,
        documents: &[Document],
        selected: Option<&str>,
        query: &str,
        language: Language,
        t: Tokens,
    ) -> Option<String> {
        self.synchronize(documents, selected, query);
        let mut chosen = None;
        let mut toggle = None;
        let count = self.visible.len();
        if count == 0 && !self.query.is_empty() {
            ui.label(RichText::new(language.text("No matching documents")).color(t.muted));
        }
        let row_spacing = ui.spacing().item_spacing.y;
        ui.spacing_mut().item_spacing.y = 0.0;
        egui::ScrollArea::vertical()
            .id_salt(("project-hierarchy", &self.project_id))
            .auto_shrink([false, false])
            .max_height((ui.available_height() - 90.0).max(80.0))
            .show_rows(ui, 44.0, count, |ui, range| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for ordinal in range {
                    let (index, depth) = self.visible[ordinal];
                    let node = &self.nodes[index];
                    ui.push_id((&self.project_id, &node.logical_id), |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            // Bound indentation so even a deeply nested custom structure retains a readable target.
                            ui.add_space((depth.min(4) * 16) as f32);
                            if node.children.is_empty() {
                                ui.add_space(32.0);
                            } else {
                                let expanded =
                                    !self.collapsed.contains(&index) || !self.query.is_empty();
                                let label = language.format(
                                    if expanded {
                                        "Collapse {title}"
                                    } else {
                                        "Expand {title}"
                                    },
                                    &[("title", node.title.clone())],
                                );
                                let response = ui.add(
                                    egui::Button::new("")
                                        .frame(false)
                                        .min_size(egui::vec2(32.0, 40.0)),
                                );
                                response.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::Button,
                                        ui.is_enabled(),
                                        &label,
                                    )
                                });
                                icons::paint_disclosure(
                                    ui.painter(),
                                    egui::Rect::from_center_size(
                                        response.rect.center(),
                                        egui::Vec2::splat(18.0),
                                    ),
                                    expanded,
                                    t.muted,
                                );
                                if response.clicked() {
                                    toggle = Some(index);
                                }
                                response.on_hover_text(label);
                            }
                            let active = node
                                .document_id
                                .as_deref()
                                .is_some_and(|id| Some(id) == selected);
                            let response = nav_row(
                                ui,
                                if node.document_id.is_some() {
                                    Icon::Document
                                } else {
                                    Icon::Folder
                                },
                                &node.title,
                                active,
                                t,
                            );
                            let right = response.has_focus()
                                && ui.input(|input| input.key_pressed(egui::Key::ArrowRight));
                            let left = response.has_focus()
                                && ui.input(|input| input.key_pressed(egui::Key::ArrowLeft));
                            if (right && self.collapsed.contains(&index))
                                || (left && !self.collapsed.contains(&index))
                            {
                                toggle = Some(index);
                            }
                            if response.clicked() {
                                if let Some(document) = &node.document_id {
                                    chosen = Some(document.clone());
                                } else if !node.children.is_empty() {
                                    toggle = Some(index);
                                }
                            }
                            let response = if node.document_id.is_none() && node.children.is_empty()
                            {
                                response.on_hover_text(
                                    language.text("This section has no saved text yet."),
                                )
                            } else {
                                response
                            };
                            if !node.kind.is_empty() {
                                response.on_hover_text(&node.kind);
                            }
                        });
                    });
                }
            });
        ui.spacing_mut().item_spacing.y = row_spacing;
        if let Some(index) = toggle {
            if !self.collapsed.remove(&index) {
                self.collapsed.insert(index);
            }
            self.dirty = true;
            ui.ctx().request_repaint();
        }
        chosen
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn node(
        id: &str,
        parent: Option<&str>,
        position: f64,
        document: Option<&str>,
        trashed: bool,
    ) -> ProjectTreeNode {
        ProjectTreeNode {
            logical_id: id.into(),
            document_id: document.map(str::to_owned),
            ordinal: 0,
            metadata: json!({"title":id,"kind":"custom","parentId":parent,"position":position,"trashedAt":if trashed {Some("2026-11-09")}else{None}}),
        }
    }
    fn document(id: &str, title: &str) -> Document {
        Document {
            id: id.into(),
            project_id: "project".into(),
            title: title.into(),
            body: String::new(),
            revision: 1,
            read_only: false,
            source_json: None,
            rich_json: None,
        }
    }
    fn labels(outline: &ProjectOutline) -> Vec<&str> {
        outline
            .visible
            .iter()
            .map(|&(index, _)| outline.nodes[index].title.as_str())
            .collect()
    }
    #[test]
    fn nested_search_preserves_collapsed_state_and_exact_duplicate_name_handles() -> TestResult {
        let source = vec![
            node("root", None, 0.0, None, false),
            node("second", Some("root"), 2.0, Some("doc-b"), false),
            node("first", Some("root"), 1.0, Some("doc-a"), false),
        ];
        let mut outline = ProjectOutline::prepare("project", &source)?;
        let documents = [document("doc-a", "Echo"), document("doc-b", "Echo")];
        outline.collapsed.insert(0);
        outline.synchronize(&documents, None, "");
        assert_eq!(labels(&outline), vec!["root"]);
        outline.synchronize(&documents, None, "echo");
        assert_eq!(labels(&outline), vec!["root", "Echo", "Echo"]);
        assert_eq!(
            outline.nodes[outline.visible[1].0].document_id.as_deref(),
            Some("doc-a")
        );
        assert_eq!(
            outline.nodes[outline.visible[2].0].document_id.as_deref(),
            Some("doc-b")
        );
        outline.synchronize(&documents, None, "");
        assert_eq!(labels(&outline), vec!["root"]);
        outline.synchronize(&documents, Some("doc-b"), "");
        assert_eq!(outline.visible.len(), 3);
        assert_eq!(source[1].metadata["parentId"], "root");
        Ok(())
    }
    #[test]
    fn trashed_mappings_never_leak_through_native_fallback_and_active_orphans_remain_visible()
    -> TestResult {
        let source = vec![
            node("trashed", None, 0.0, Some("trashed-doc"), true),
            node("child", Some("trashed"), 1.0, Some("child-doc"), false),
        ];
        let mut outline = ProjectOutline::prepare("project", &source)?;
        let documents = [
            document("trashed-doc", "Hidden"),
            document("child-doc", "Child"),
            document("new-doc", "New native"),
        ];
        outline.synchronize(&documents, None, "");
        assert_eq!(labels(&outline), vec!["Child", "New native"]);
        assert_eq!(outline.visible[0].1, 0);
        outline.synchronize(&documents, None, "");
        assert_eq!(outline.visible.len(), 2);
        Ok(())
    }
    #[test]
    fn cyclic_and_deep_source_uses_iterative_unique_traversal() -> TestResult {
        let cyclic = vec![
            node("a", Some("b"), 0.0, None, false),
            node("b", Some("a"), 1.0, None, false),
        ];
        let mut outline = ProjectOutline::prepare("project", &cyclic)?;
        outline.synchronize(&[], None, "b");
        assert_eq!(outline.order.len(), 2);
        assert_eq!(outline.visible.len(), 2);
        let deep: Vec<_> = (0..20_000)
            .map(|i| {
                node(
                    &i.to_string(),
                    (i > 0).then(|| (i - 1).to_string()).as_deref(),
                    i as f64,
                    None,
                    false,
                )
            })
            .collect();
        let mut outline = ProjectOutline::prepare("project", &deep)?;
        outline.synchronize(&[], None, "19999");
        assert_eq!(outline.visible.len(), 20_000);
        assert_eq!(outline.visible.last().map(|item| item.1), Some(19_999));
        Ok(())
    }
    #[test]
    fn current_titles_and_new_native_documents_do_not_duplicate_or_accept_foreign_context()
    -> TestResult {
        let mut outline =
            ProjectOutline::prepare("project", &[node("source", None, 0.0, Some("doc"), false)])?;
        let mut foreign = document("foreign", "Wrong project");
        foreign.project_id = "other".into();
        let documents = [document("doc", "Renamed"), document("new", "New"), foreign];
        outline.synchronize(&documents, None, "renamed");
        assert_eq!(labels(&outline), vec!["Renamed"]);
        outline.synchronize(&documents, None, "");
        assert_eq!(outline.visible.len(), 2);
        assert_eq!(outline.nodes.len(), 2);
        Ok(())
    }
}
