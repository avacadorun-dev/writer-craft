//! Presentation-only imported hierarchy; canonical structure stays in shared core storage.
use super::project_tree::TreeAction;
use super::*;
use std::collections::{HashMap, HashSet};
use studio_core::ProjectTreeNode;

pub(super) struct OutlineSelection<'a> {
    pub(super) document: Option<&'a str>,
    pub(super) node: Option<&'a str>,
}

struct OutlineNode {
    logical_id: String,
    document_id: Option<String>,
    title: String,
    kind: String,
    position: f64,
    ordinal: u64,
    parent: Option<usize>,
    children: Vec<usize>,
    mapped: bool,
}

#[derive(Default)]
pub(super) struct ProjectOutline {
    project_id: String,
    nodes: Vec<OutlineNode>,
    order: Vec<(usize, usize)>,
    mapped_documents: HashSet<String>,
    document_rows: HashMap<String, usize>,
    collapsed: HashSet<usize>,
    search_collapsed: HashSet<usize>,
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
        let mut hidden = HashSet::new();
        let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut pending = Vec::new();
        for item in source {
            if super::project_tree::trashed(item) {
                pending.push(item.logical_id.as_str());
            }
            if let Some(parent) = item.metadata["parentId"].as_str() {
                children.entry(parent).or_default().push(&item.logical_id);
            }
        }
        while let Some(id) = pending.pop() {
            if hidden.insert(id)
                && let Some(descendants) = children.get(id)
            {
                pending.extend(descendants.iter().copied());
            }
        }
        for item in source {
            if let Some(document) = &item.document_id {
                outline.mapped_documents.insert(document.clone());
            }
            if hidden.contains(item.logical_id.as_str()) {
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
                mapped: true,
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

    pub(super) fn refresh(&mut self, project: &str, source: &[ProjectTreeNode]) -> AppResult<()> {
        let collapsed: HashSet<_> = self
            .collapsed
            .iter()
            .map(|&index| self.nodes[index].logical_id.clone())
            .collect();
        let search_collapsed: HashSet<_> = self
            .search_collapsed
            .iter()
            .map(|&index| self.nodes[index].logical_id.clone())
            .collect();
        let mut next = Self::prepare(project, source)?;
        if self.project_id == project {
            next.collapsed = next
                .nodes
                .iter()
                .enumerate()
                .filter_map(|(index, node)| collapsed.contains(&node.logical_id).then_some(index))
                .collect();
            next.search_collapsed = next
                .nodes
                .iter()
                .enumerate()
                .filter_map(|(index, node)| {
                    search_collapsed.contains(&node.logical_id).then_some(index)
                })
                .collect();
            next.query.clone_from(&self.query);
            next.selected_document.clone_from(&self.selected_document);
        }
        *self = next;
        Ok(())
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
                    mapped: false,
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
                self.search_collapsed.remove(&index);
                cursor = self.nodes[index].parent;
            }
            self.dirty = true;
        }
        let query = query.trim().to_lowercase();
        if self.query != query {
            self.query = query;
            self.search_collapsed.clear();
            self.dirty = true;
        }
        if self.dirty {
            self.rebuild_visible();
            self.dirty = false;
        }
    }

    fn toggle(&mut self, index: usize) {
        let collapsed = if self.query.is_empty() {
            &mut self.collapsed
        } else {
            &mut self.search_collapsed
        };
        if !collapsed.remove(&index) {
            collapsed.insert(index);
        }
        self.dirty = true;
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
            if (if searching {
                &self.search_collapsed
            } else {
                &self.collapsed
            })
            .contains(&index)
            {
                hidden_below = Some(depth);
            }
        }
    }

    pub(super) fn show(
        &mut self,
        ui: &mut egui::Ui,
        documents: &[Document],
        selection: OutlineSelection<'_>,
        query: &str,
        language: Language,
        t: Tokens,
    ) -> Option<TreeAction> {
        self.synchronize(documents, selection.document, query);
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
            .max_height((ui.available_height() - 140.0).max(80.0))
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
                                let expanded = !(if self.query.is_empty() {
                                    &self.collapsed
                                } else {
                                    &self.search_collapsed
                                })
                                .contains(&index);
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
                                #[cfg(feature = "editor-preview")]
                                if super::capture_connected_writing::enabled(ui.ctx()) {
                                    super::capture_connected_writing::control(
                                        ui.ctx(),
                                        &format!("hierarchy.disclosure.{}", node.logical_id),
                                        response.rect,
                                    );
                                }
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
                            let active = selection.node == Some(node.logical_id.as_str())
                                || (selection.node.is_none()
                                    && node
                                        .document_id
                                        .as_deref()
                                        .is_some_and(|id| Some(id) == selection.document));
                            let response = ui
                                .allocate_ui_with_layout(
                                    egui::vec2((ui.available_width() - 32.0).max(40.0), 44.0),
                                    Layout::left_to_right(Align::Center),
                                    |ui| {
                                        nav_row(
                                            ui,
                                            if node.document_id.is_some() {
                                                Icon::Document
                                            } else {
                                                Icon::Folder
                                            },
                                            &node.title,
                                            active,
                                            t,
                                        )
                                    },
                                )
                                .inner;
                            response.context_menu(|ui| {
                                if let Some(action) = item_menu(ui, node, language) {
                                    chosen = Some(action);
                                }
                            });
                            let more =
                                icons::icon_button(ui, Icon::More, language.text("Item actions"));
                            egui::Popup::menu(&more).show(|ui| {
                                if let Some(action) = item_menu(ui, node, language) {
                                    chosen = Some(action);
                                }
                            });
                            #[cfg(feature = "editor-preview")]
                            if let Some(document) = &node.document_id
                                && super::capture_connected_writing::enabled(ui.ctx())
                            {
                                super::capture_connected_writing::control(
                                    ui.ctx(),
                                    &format!("hierarchy.document.{document}"),
                                    response.rect,
                                );
                            }
                            let right = response.has_focus()
                                && ui.input(|input| input.key_pressed(egui::Key::ArrowRight));
                            let left = response.has_focus()
                                && ui.input(|input| input.key_pressed(egui::Key::ArrowLeft));
                            let collapsed = (if self.query.is_empty() {
                                &self.collapsed
                            } else {
                                &self.search_collapsed
                            })
                            .contains(&index);
                            if !node.children.is_empty()
                                && ((right && collapsed) || (left && !collapsed))
                            {
                                toggle = Some(index);
                            }
                            if response.clicked() {
                                chosen = Some(if node.mapped {
                                    TreeAction::Open(node.logical_id.clone())
                                } else {
                                    TreeAction::OpenDocument(node.logical_id.clone(), false)
                                });
                            }
                            if node.mapped
                                && response.has_focus()
                                && ui.input(|input| input.key_pressed(egui::Key::F2))
                            {
                                chosen = Some(TreeAction::Rename(node.logical_id.clone()));
                            }
                            if node.mapped
                                && response.has_focus()
                                && ui.input(|input| input.key_pressed(egui::Key::Delete))
                            {
                                chosen = Some(TreeAction::Trash(node.logical_id.clone()));
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
            self.toggle(index);
            self.dirty = true;
            ui.ctx().request_repaint();
        }
        #[cfg(feature = "editor-preview")]
        if super::capture_connected_writing::enabled(ui.ctx()) {
            super::capture_connected_writing::observe(
                ui.ctx(),
                "hierarchy",
                serde_json::json!({
                    "project_id":self.project_id,"query":self.query,"selected_document":selection.document,
                    "visible":self.visible.iter().map(|&(index,depth)| {
                        let node=&self.nodes[index];
                        serde_json::json!({"logical_id":node.logical_id,"document_id":node.document_id,"title":node.title,"depth":depth,"expanded":!(if self.query.is_empty(){&self.collapsed}else{&self.search_collapsed}).contains(&index)})
                    }).collect::<Vec<_>>()
                }),
            );
        }
        chosen
    }
}

fn item_menu(ui: &mut egui::Ui, node: &OutlineNode, l: Language) -> Option<TreeAction> {
    let mut chosen = None;
    if ui.button(l.text("Open")).clicked() {
        chosen = Some(if node.mapped {
            TreeAction::Open(node.logical_id.clone())
        } else {
            TreeAction::OpenDocument(node.logical_id.clone(), false)
        });
    }
    if node.document_id.is_some() && ui.button(l.text("Consult and return")).clicked() {
        chosen = Some(if node.mapped {
            TreeAction::Reference(node.logical_id.clone())
        } else {
            TreeAction::OpenDocument(node.logical_id.clone(), true)
        });
    }
    if !node.mapped {
        ui.label(l.text("Open this retained document to edit its text or title. Its tree placement is not yet mapped."));
        if chosen.is_some() {
            ui.close();
        }
        return chosen;
    }
    if ui.button(l.text("Add inside…")).clicked() {
        chosen = Some(TreeAction::Create(Some(node.logical_id.clone())));
    }
    ui.separator();
    if ui.button(l.text("Rename…")).clicked() {
        chosen = Some(TreeAction::Rename(node.logical_id.clone()));
    }
    if ui.button(l.text("Move or reorder…")).clicked() {
        chosen = Some(TreeAction::Move(node.logical_id.clone()));
    }
    if ui.button(l.text("Move to Trash")).clicked() {
        chosen = Some(TreeAction::Trash(node.logical_id.clone()));
    }
    if chosen.is_some() {
        ui.close();
    }
    chosen
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
        outline.toggle(0);
        outline.synchronize(&documents, None, "echo");
        assert_eq!(labels(&outline), vec!["root"]);
        outline.synchronize(&documents, None, "");
        assert_eq!(labels(&outline), vec!["root"]);
        outline.synchronize(&documents, None, "echo");
        assert_eq!(outline.visible.len(), 3);
        outline.synchronize(&documents, Some("doc-b"), "");
        assert_eq!(outline.visible.len(), 3);
        assert_eq!(source[1].metadata["parentId"], "root");
        Ok(())
    }
    #[test]
    fn trashed_branches_never_leak_through_native_fallback_and_missing_parent_orphans_remain_visible()
    -> TestResult {
        let source = vec![
            node("trashed", None, 0.0, Some("trashed-doc"), true),
            node("child", Some("trashed"), 1.0, Some("child-doc"), false),
            node("orphan", Some("missing"), 2.0, Some("orphan-doc"), false),
        ];
        let mut outline = ProjectOutline::prepare("project", &source)?;
        let documents = [
            document("trashed-doc", "Hidden"),
            document("child-doc", "Child"),
            document("orphan-doc", "Orphan"),
            document("new-doc", "New native"),
        ];
        outline.synchronize(&documents, None, "");
        assert_eq!(labels(&outline), vec!["Orphan", "New native"]);
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
