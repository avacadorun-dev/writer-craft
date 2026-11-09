//! Presentation-only imported hierarchy; canonical structure stays in shared core storage.
use super::project_tree::{TreeAction, TreeDrop, allowed_for_kind};
use super::*;
use std::collections::{HashMap, HashSet};
use studio_core::{ImportedProjectStructure, ProjectTreeNode};

pub(super) struct OutlineSelection<'a> {
    pub(super) document: Option<&'a str>,
    pub(super) node: Option<&'a str>,
    pub(super) structure: Option<&'a ImportedProjectStructure>,
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
    source_index: Option<usize>,
}

const ROW_HEIGHT: f32 = 34.0;
const INDENT: f32 = 7.0;

#[derive(Clone)]
pub(super) struct OutlineDrag {
    pub(super) project: String,
    pub(super) original: ProjectTreeNode,
    pub(super) structure: ImportedProjectStructure,
    subtree_height: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DropZone {
    Above,
    Inside,
    Below,
}

fn drop_zone(rect: egui::Rect, pointer: egui::Pos2) -> DropZone {
    let edge = rect.height() * 0.25;
    if pointer.y < rect.top() + edge {
        DropZone::Above
    } else if pointer.y > rect.bottom() - edge {
        DropZone::Below
    } else {
        DropZone::Inside
    }
}

fn edge_scroll(viewport: egui::Rect, pointer: egui::Pos2, dt: f32) -> f32 {
    if !viewport.contains(pointer) {
        return 0.0;
    }
    let edge = 28.0_f32.min(viewport.height() / 3.0);
    if edge <= 0.0 {
        return 0.0;
    }
    let speed = ((viewport.top() + edge - pointer.y) / edge).clamp(0.0, 1.0)
        - ((pointer.y - viewport.bottom() + edge) / edge).clamp(0.0, 1.0);
    speed * 420.0 * dt.clamp(0.0, 0.05)
}

#[derive(Default)]
pub(super) struct ProjectOutline {
    project_id: String,
    source: Vec<ProjectTreeNode>,
    source_rows: HashMap<String, usize>,
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
    hover_expand: Option<(String, f64)>,
}

impl ProjectOutline {
    pub(super) fn prepare(project: &str, source: &[ProjectTreeNode]) -> AppResult<Self> {
        let mut outline = Self {
            project_id: project.into(),
            source: source.to_vec(),
            dirty: true,
            ..Self::default()
        };
        let mut by_id = HashMap::new();
        let mut parents = Vec::new();
        let mut hidden = HashSet::new();
        let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut pending = Vec::new();
        for (index, item) in source.iter().enumerate() {
            if outline
                .source_rows
                .insert(item.logical_id.clone(), index)
                .is_some()
            {
                return Err(studio_core::StudioError::CorruptData(
                    "Duplicate project navigation identity".into(),
                )
                .into());
            }
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
        for (source_index, item) in source.iter().enumerate() {
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
                source_index: Some(source_index),
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
                if let Some(source) = self.nodes[index].source_index {
                    self.source[source].metadata["title"] = serde_json::json!(document.title);
                    self.source[source].metadata["currentRevision"] =
                        serde_json::json!(document.revision);
                }
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
                    source_index: None,
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

    fn snapshot(&self, index: usize) -> Option<&ProjectTreeNode> {
        self.nodes
            .get(index)?
            .source_index
            .map(|index| &self.source[index])
    }

    fn active_depth(&self, id: &str, forbidden: Option<&str>) -> Result<usize, &'static str> {
        let mut cursor = Some(id);
        let mut seen = HashSet::new();
        while let Some(id) = cursor {
            if Some(id) == forbidden {
                return Err("An item cannot be moved into its own branch.");
            }
            if !seen.insert(id) || seen.len() > 512 {
                return Err("This branch is unavailable. Refresh the project.");
            }
            let node = self
                .source_rows
                .get(id)
                .map(|&index| &self.source[index])
                .ok_or("This branch is unavailable. Refresh the project.")?;
            if super::project_tree::trashed(node) {
                return Err("This branch is unavailable. Refresh the project.");
            }
            cursor = node.metadata["parentId"].as_str();
        }
        Ok(seen.len())
    }

    fn subtree_height(&self, id: &str) -> Option<usize> {
        let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
        for node in &self.source {
            if let Some(parent) = node.metadata["parentId"].as_str() {
                children.entry(parent).or_default().push(&node.logical_id);
            }
        }
        let mut pending = vec![(id, 0)];
        let mut seen = HashSet::new();
        let mut height = 0;
        while let Some((id, depth)) = pending.pop() {
            if !seen.insert(id) || depth >= 512 {
                return None;
            }
            height = height.max(depth);
            if let Some(children) = children.get(id) {
                pending.extend(children.iter().map(|id| (*id, depth + 1)));
            }
        }
        Some(height)
    }

    fn drag_payload(
        &self,
        index: usize,
        structure: &ImportedProjectStructure,
    ) -> Result<OutlineDrag, &'static str> {
        let original = self
            .snapshot(index)
            .ok_or("This branch is unavailable. Refresh the project.")?;
        self.active_depth(&original.logical_id, None)?;
        Ok(OutlineDrag {
            project: self.project_id.clone(),
            original: original.clone(),
            structure: structure.clone(),
            subtree_height: self
                .subtree_height(&original.logical_id)
                .ok_or("This branch is unavailable. Refresh the project.")?,
        })
    }

    fn drop_request(
        &self,
        drag: &OutlineDrag,
        target: Option<(usize, DropZone)>,
        structure: &ImportedProjectStructure,
    ) -> Result<TreeDrop, &'static str> {
        if drag.project != self.project_id || structure.project_id != self.project_id {
            return Err("Items can only be moved within this project.");
        }
        if drag.structure != *structure {
            return Err("The project structure changed. Start the drag again.");
        }
        if self
            .source_rows
            .get(&drag.original.logical_id)
            .map(|&index| &self.source[index])
            != Some(&drag.original)
        {
            return Err("The item changed. Start the drag again.");
        }
        self.active_depth(&drag.original.logical_id, None)?;
        let mut targets = Vec::new();
        let (parent, before) = if let Some((index, zone)) = target {
            let node = self
                .snapshot(index)
                .ok_or("This branch is unavailable. Refresh the project.")?;
            if node.logical_id == drag.original.logical_id {
                return Err("An item cannot be moved into its own branch.");
            }
            self.active_depth(&node.logical_id, None)?;
            targets.push(node.clone());
            match zone {
                DropZone::Inside => (Some(node.logical_id.clone()), None),
                DropZone::Above => (
                    node.metadata["parentId"].as_str().map(str::to_owned),
                    Some(node.logical_id.clone()),
                ),
                DropZone::Below => {
                    let siblings: Vec<_> = self
                        .order
                        .iter()
                        .filter_map(|&(sibling, _)| {
                            let sibling = self.snapshot(sibling)?;
                            (sibling.logical_id != drag.original.logical_id
                                && sibling.metadata["parentId"].as_str()
                                    == node.metadata["parentId"].as_str())
                            .then_some(sibling)
                        })
                        .collect();
                    let next = siblings
                        .iter()
                        .position(|sibling| sibling.logical_id == node.logical_id)
                        .and_then(|position| siblings.get(position + 1));
                    (
                        node.metadata["parentId"].as_str().map(str::to_owned),
                        next.map(|sibling| sibling.logical_id.clone()),
                    )
                }
            }
        } else {
            (None, None)
        };
        let parent_node = parent
            .as_deref()
            .and_then(|id| self.source_rows.get(id))
            .map(|&index| &self.source[index]);
        let depth = if let Some(parent) = &parent {
            self.active_depth(parent, Some(&drag.original.logical_id))?
        } else {
            0
        };
        if depth + drag.subtree_height + 1 > 512 {
            return Err("This move would make the project tree too deep.");
        }
        let kind = drag.original.metadata["kind"].as_str().unwrap_or_default();
        if !allowed_for_kind(
            structure,
            parent_node.and_then(|node| node.metadata["kind"].as_str()),
            parent.is_none(),
        )
        .iter()
        .any(|(allowed, _)| allowed == kind)
        {
            return Err("This location does not allow this item type.");
        }
        for id in [parent.as_deref(), before.as_deref()].into_iter().flatten() {
            if !targets.iter().any(|node| node.logical_id == id) {
                let node = self
                    .source_rows
                    .get(id)
                    .map(|&index| &self.source[index])
                    .ok_or("This branch is unavailable. Refresh the project.")?;
                targets.push(node.clone());
            }
        }
        Ok(TreeDrop {
            project: drag.project.clone(),
            structure: drag.structure.clone(),
            original: drag.original.clone(),
            targets,
            parent,
            before,
        })
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
        let mut rows = Vec::new();
        let mut root_rect = None;
        let carrying = egui::DragAndDrop::has_payload_of_type::<OutlineDrag>(ui.ctx());
        if !carrying {
            self.hover_expand = None;
        }
        let count = self.visible.len() + usize::from(carrying);
        if count == 0 && !self.query.is_empty() {
            ui.label(RichText::new(language.text("No matching documents")).color(t.muted));
        }
        let row_spacing = ui.spacing().item_spacing.y;
        ui.spacing_mut().item_spacing.y = 0.0;
        let output = egui::ScrollArea::vertical()
            .id_salt(("project-hierarchy", &self.project_id))
            .auto_shrink([false, false])
            .max_height(ui.available_height())
            .scroll_source(
                egui::scroll_area::ScrollSource::SCROLL_BAR
                    | egui::scroll_area::ScrollSource::MOUSE_WHEEL,
            )
            .show_rows(ui, ROW_HEIGHT, count, |ui, range| {
                ui.spacing_mut().item_spacing.y = 0.0;
                if carrying && let Some(pointer) = ui.input(|input| input.pointer.hover_pos()) {
                    let delta =
                        edge_scroll(ui.clip_rect(), pointer, ui.input(|input| input.stable_dt));
                    if delta != 0.0 && ui.ctx().layer_id_at(pointer) == Some(ui.layer_id()) {
                        ui.scroll_with_delta_animation(
                            egui::vec2(0.0, delta),
                            egui::style::ScrollAnimation::none(),
                        );
                        ui.ctx().request_repaint();
                    }
                }
                for ordinal in range {
                    if ordinal == self.visible.len() {
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), ROW_HEIGHT),
                            egui::Sense::hover(),
                        );
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            language.text("Move to project root"),
                            egui::FontId::proportional(12.0),
                            t.muted,
                        );
                        root_rect = Some(rect);
                        continue;
                    }
                    let (index, depth) = self.visible[ordinal];
                    let node = &self.nodes[index];
                    let can_add = selection.structure.is_some_and(|structure| {
                        !allowed_for_kind(structure, Some(&node.kind), false).is_empty()
                            && self.active_depth(&node.logical_id, None).is_ok()
                    });
                    let expanded = !(if self.query.is_empty() {
                        &self.collapsed
                    } else {
                        &self.search_collapsed
                    })
                    .contains(&index);
                    let outer = ui
                        .push_id((&self.project_id, &node.logical_id), |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.0;
                                ui.add_space(depth.min(4) as f32 * INDENT);
                                if node.children.is_empty() {
                                    ui.add_space(20.0);
                                } else {
                                    let label = language.format(
                                        if expanded {
                                            "Collapse {title}"
                                        } else {
                                            "Expand {title}"
                                        },
                                        &[("title", node.title.clone())],
                                    );
                                    let response = disclosure_button(ui, expanded, &label, t);
                                    #[cfg(feature = "editor-preview")]
                                    if super::capture_connected_writing::enabled(ui.ctx()) {
                                        super::capture_connected_writing::control(
                                            ui.ctx(),
                                            &format!("hierarchy.disclosure.{}", node.logical_id),
                                            response.rect,
                                        );
                                    }
                                    if !carrying && response.clicked() {
                                        toggle = Some(index);
                                    }
                                }
                                let active = selection.node == Some(node.logical_id.as_str())
                                    || (selection.node.is_none()
                                        && node
                                            .document_id
                                            .as_deref()
                                            .is_some_and(|id| Some(id) == selection.document));
                                let content = node.document_id.is_some()
                                    || selection.structure.is_some_and(|structure| {
                                        structure.is_content_bearing(&node.kind)
                                    });
                                let response = ui
                                    .allocate_ui_with_layout(
                                        egui::vec2(ui.available_width().max(30.0), ROW_HEIGHT),
                                        Layout::left_to_right(Align::Center),
                                        |ui| {
                                            tree_row(
                                                ui,
                                                if content {
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
                                if !carrying
                                    && response.drag_started_by(egui::PointerButton::Primary)
                                    && let Some(structure) = selection.structure
                                    && let Ok(payload) = self.drag_payload(index, structure)
                                {
                                    chosen = Some(TreeAction::BeginDrag(
                                        Box::new(payload),
                                        ui.ctx().clone(),
                                    ));
                                }
                                if !carrying
                                    && let Some(action) =
                                        item_context_menu(&response, node, language, can_add)
                                {
                                    chosen = Some(action);
                                }
                                #[cfg(feature = "editor-preview")]
                                if super::capture_connected_writing::enabled(ui.ctx()) {
                                    super::capture_connected_writing::control(
                                        ui.ctx(),
                                        &format!("hierarchy.node.{}", node.logical_id),
                                        response.rect,
                                    );
                                    if let Some(document) = &node.document_id {
                                        super::capture_connected_writing::control(
                                            ui.ctx(),
                                            &format!("hierarchy.document.{document}"),
                                            response.rect,
                                        );
                                    }
                                }
                                if !carrying {
                                    let right = response.has_focus()
                                        && ui.input(|input| {
                                            input.key_pressed(egui::Key::ArrowRight)
                                        });
                                    let left = response.has_focus()
                                        && ui
                                            .input(|input| input.key_pressed(egui::Key::ArrowLeft));
                                    if !node.children.is_empty()
                                        && ((right && !expanded) || (left && expanded))
                                    {
                                        toggle = Some(index);
                                    }
                                    if response.clicked() {
                                        chosen = Some(TreeAction::Browse {
                                            id: node.logical_id.clone(),
                                            mapped: node.mapped,
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
                                }
                                if node.mapped {
                                    response.on_hover_text(format!(
                                        "{}\n{}",
                                        language.text("Drag to move. Escape cancels."),
                                        language.text(
                                            "Right-click or press Shift+F10 for item actions."
                                        ),
                                    ));
                                }
                            })
                        })
                        .inner;
                    let mut target_rect = outer.response.rect;
                    target_rect.min.x += depth.min(4) as f32 * INDENT;
                    rows.push((index, target_rect));
                }
            });
        ui.spacing_mut().item_spacing.y = row_spacing;
        if let Some(index) = toggle {
            self.toggle(index);
            ui.ctx().request_repaint();
        }
        if let Some(drag) = egui::DragAndDrop::payload::<OutlineDrag>(ui.ctx()) {
            chosen = None;
            if let Some(pointer) = ui.input(|input| input.pointer.hover_pos()) {
                let viewport = output.inner_rect.intersect(ui.clip_rect());
                let hovered = viewport.contains(pointer)
                    && ui.ctx().layer_id_at(pointer) == Some(ui.layer_id())
                    && ui.is_enabled();
                let target = hovered
                    .then(|| rows.iter().find(|(_, rect)| rect.contains(pointer)))
                    .flatten();
                let root = hovered && root_rect.is_some_and(|rect| rect.contains(pointer));
                let mut label = language.format(
                    "Move {title}",
                    &[(
                        "title",
                        drag.original.metadata["title"]
                            .as_str()
                            .unwrap_or_default()
                            .into(),
                    )],
                );
                let mut valid = false;
                let mut expand = None;
                if target.is_some() || root {
                    let location = target.map(|(index, rect)| (*index, drop_zone(*rect, pointer)));
                    let request = selection
                        .structure
                        .ok_or("The project structure changed. Start the drag again.")
                        .and_then(|structure| self.drop_request(&drag, location, structure));
                    valid = request.is_ok();
                    let color = if valid { t.accent } else { t.warning };
                    if let Some((index, rect)) = target {
                        let zone = drop_zone(*rect, pointer);
                        paint_drop(ui, rect.intersect(viewport), zone, color, valid);
                        if valid {
                            label = language.format(
                                match zone {
                                    DropZone::Above => "Move before {title}",
                                    DropZone::Inside => "Move inside {title}",
                                    DropZone::Below => "Move after {title}",
                                },
                                &[("title", self.nodes[*index].title.clone())],
                            );
                            if zone == DropZone::Inside {
                                expand = Some(*index);
                            }
                        }
                    } else if let Some(rect) = root_rect {
                        paint_drop(ui, rect.intersect(viewport), DropZone::Inside, color, valid);
                        label = language.text("Move to project root").into();
                    }
                    match request {
                        Ok(request) => {
                            if ui.input(|input| input.pointer.primary_released()) {
                                egui::DragAndDrop::clear_payload(ui.ctx());
                                self.hover_expand = None;
                                chosen = Some(TreeAction::Drop(Box::new(request)));
                            }
                        }
                        Err(reason) => label = language.text(reason).into(),
                    }
                }
                if chosen.is_none() {
                    self.delayed_expand(expand, ui.input(|input| input.time));
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(40));
                }
                ui.ctx().set_cursor_icon(if valid {
                    egui::CursorIcon::Grabbing
                } else {
                    egui::CursorIcon::NotAllowed
                });
                paint_drag_label(ui.ctx(), pointer, &label, valid, t);
            } else {
                self.hover_expand = None;
            }
            if ui.input(|input| input.pointer.primary_released()) {
                egui::DragAndDrop::clear_payload(ui.ctx());
                self.hover_expand = None;
            }
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

    fn delayed_expand(&mut self, target: Option<usize>, now: f64) {
        let Some(index) = target else {
            self.hover_expand = None;
            return;
        };
        let collapsed = if self.query.is_empty() {
            &mut self.collapsed
        } else {
            &mut self.search_collapsed
        };
        if !collapsed.contains(&index) {
            self.hover_expand = None;
            return;
        }
        let id = &self.nodes[index].logical_id;
        if let Some((hovered, since)) = &self.hover_expand
            && hovered == id
        {
            if now - since >= 0.65 {
                collapsed.remove(&index);
                self.hover_expand = None;
                self.dirty = true;
            }
        } else {
            self.hover_expand = Some((id.clone(), now));
        }
    }
}

fn tree_row(ui: &mut egui::Ui, icon: Icon, title: &str, active: bool, t: Tokens) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT),
        egui::Sense::click_and_drag(),
    );
    if active || response.hovered() || response.dragged() {
        ui.painter()
            .rect_filled(rect, 4.0, if active { t.accent_soft } else { t.surface });
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.0, t.accent),
            egui::StrokeKind::Inside,
        );
    }
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(egui::vec2(4.0, 2.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );
    child.spacing_mut().item_spacing.x = 6.0;
    icons::icon(
        &mut child,
        icon,
        17.0,
        if active { t.accent } else { t.muted },
    );
    // The row owns pointer selection, dragging and context actions, including over its title.
    child.add(
        egui::Label::new(RichText::new(title).size(13.5).color(t.text))
            .selectable(false)
            .truncate(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            active,
            title,
        )
    });
    response.on_hover_text(title)
}

fn disclosure_button(ui: &mut egui::Ui, expanded: bool, label: &str, t: Tokens) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(20.0, ROW_HEIGHT), egui::Sense::click());
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            3.0,
            egui::Stroke::new(1.0, t.accent),
            egui::StrokeKind::Inside,
        );
    }
    icons::paint_disclosure(
        ui.painter(),
        egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(14.0)),
        expanded,
        t.muted,
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    response.on_hover_text(label)
}

fn paint_drop(ui: &egui::Ui, rect: egui::Rect, zone: DropZone, color: egui::Color32, valid: bool) {
    if zone == DropZone::Inside || !valid {
        ui.painter()
            .rect_filled(rect, 3.0, color.gamma_multiply(0.12));
        ui.painter().rect_stroke(
            rect,
            3.0,
            egui::Stroke::new(1.5, color),
            egui::StrokeKind::Inside,
        );
    } else {
        let y = if zone == DropZone::Above {
            rect.top() + 1.0
        } else {
            rect.bottom() - 1.0
        };
        let start = egui::pos2(rect.left() + 3.0, y);
        ui.painter().line_segment(
            [start, egui::pos2(rect.right(), y)],
            egui::Stroke::new(2.0, color),
        );
        ui.painter().circle_filled(start, 3.0, color);
    }
}

fn paint_drag_label(ctx: &egui::Context, pointer: egui::Pos2, label: &str, valid: bool, t: Tokens) {
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("tree-drag-feedback"),
    ));
    let galley = painter.layout(
        label.into(),
        egui::FontId::proportional(12.0),
        t.text,
        280.0,
    );
    let size = galley.size() + egui::vec2(16.0, 10.0);
    let bounds = ctx.content_rect();
    let position = egui::pos2(
        (pointer.x + 16.0)
            .min(bounds.right() - size.x)
            .max(bounds.left()),
        (pointer.y + 18.0)
            .min(bounds.bottom() - size.y)
            .max(bounds.top()),
    );
    let rect = egui::Rect::from_min_size(position, size);
    painter.rect_filled(rect, 5.0, t.panel);
    painter.rect_stroke(
        rect,
        5.0,
        egui::Stroke::new(1.0, if valid { t.accent } else { t.warning }),
        egui::StrokeKind::Inside,
    );
    painter.galley(position + egui::vec2(8.0, 5.0), galley, t.text);
}

fn item_context_menu(
    response: &egui::Response,
    node: &OutlineNode,
    language: Language,
    can_add: bool,
) -> Option<TreeAction> {
    if response.clicked() || response.secondary_clicked() {
        response.request_focus();
    }
    let keyboard = response.has_focus()
        && response
            .ctx
            .input_mut(|input| input.consume_key(egui::Modifiers::SHIFT, egui::Key::F10));
    let popup_id = response.id.with("tree-item-actions");
    let keyboard_id = popup_id.with("keyboard-anchor");
    if response.secondary_clicked() {
        response
            .ctx
            .data_mut(|data| data.remove::<bool>(keyboard_id));
    }
    if keyboard {
        response
            .ctx
            .data_mut(|data| data.insert_temp(keyboard_id, true));
        egui::Popup::open_id(&response.ctx, popup_id);
    }
    let mut popup = egui::Popup::context_menu(response).id(popup_id);
    // Keyboard opening has no fixed pointer position. Keep the row anchor after
    // the key frame, including the initial hidden sizing pass.
    if response.ctx.data(|data| data.get_temp::<bool>(keyboard_id)) == Some(true) {
        popup = popup.at_position(response.rect.left_bottom());
    }
    let action = popup
        .show(|ui| item_menu(ui, node, language, can_add))
        .and_then(|inner| inner.inner);
    if !egui::Popup::is_id_open(&response.ctx, popup_id) {
        response
            .ctx
            .data_mut(|data| data.remove::<bool>(keyboard_id));
    }
    action
}

fn item_menu(
    ui: &mut egui::Ui,
    node: &OutlineNode,
    l: Language,
    can_add: bool,
) -> Option<TreeAction> {
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
    if ui
        .add_enabled(can_add, egui::Button::new(l.text("Add inside…")))
        .clicked()
    {
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

    fn drag_structure() -> ImportedProjectStructure {
        ImportedProjectStructure {
            project_id: "project".into(),
            logical_project_id: "source".into(),
            revision: 1,
            schema: json!({"nodeTypes":[
                {"key":"folder","label":"Folder","contentBearing":false,"canBeRoot":true,"allowedChildren":["folder","document"]},
                {"key":"document","label":"Document","contentBearing":true,"canBeRoot":true,"allowedChildren":[]}
            ]}),
        }
    }

    fn drag_node(id: &str, parent: Option<&str>, position: f64, kind: &str) -> ProjectTreeNode {
        let mut value = node(
            id,
            parent,
            position,
            (kind == "document").then_some(id),
            false,
        );
        value.metadata["kind"] = json!(kind);
        value
    }

    #[test]
    fn native_drop_selects_before_after_or_inside_without_mutating_the_projection() -> TestResult {
        let source = vec![
            drag_node("part", None, 1.0, "folder"),
            drag_node("scene", Some("part"), 1.0, "document"),
            drag_node("next", None, 2.0, "folder"),
            drag_node("last", None, 3.0, "document"),
        ];
        let outline = ProjectOutline::prepare("project", &source)?;
        let structure = drag_structure();
        let drag = outline.drag_payload(1, &structure)?;
        let above = outline.drop_request(&drag, Some((2, DropZone::Above)), &structure)?;
        assert_eq!(
            (above.parent.as_deref(), above.before.as_deref()),
            (None, Some("next"))
        );
        let below = outline.drop_request(&drag, Some((2, DropZone::Below)), &structure)?;
        assert_eq!(
            (below.parent.as_deref(), below.before.as_deref()),
            (None, Some("last"))
        );
        let inside = outline.drop_request(&drag, Some((2, DropZone::Inside)), &structure)?;
        assert_eq!(
            (inside.parent.as_deref(), inside.before.as_deref()),
            (Some("next"), None)
        );
        let root = outline.drop_request(&drag, None, &structure)?;
        assert_eq!((root.parent, root.before), (None, None));
        assert_eq!(outline.source, source);
        Ok(())
    }

    #[test]
    fn drop_refuses_self_cycle_leaf_foreign_stale_and_legacy_hidden_sources() -> TestResult {
        let source = vec![
            drag_node("part", None, 1.0, "folder"),
            drag_node("nested", Some("part"), 1.0, "folder"),
            drag_node("scene", Some("nested"), 1.0, "document"),
        ];
        let mut outline = ProjectOutline::prepare("project", &source)?;
        let structure = drag_structure();
        let drag = outline.drag_payload(0, &structure)?;
        assert!(
            outline
                .drop_request(&drag, Some((0, DropZone::Above)), &structure)
                .is_err()
        );
        assert!(
            outline
                .drop_request(&drag, Some((1, DropZone::Inside)), &structure)
                .is_err()
        );
        let scene = outline.drag_payload(2, &structure)?;
        assert!(
            outline
                .drop_request(&scene, Some((2, DropZone::Inside)), &structure)
                .is_err()
        );
        assert!(
            outline
                .drop_request(&drag, Some((2, DropZone::Inside)), &structure)
                .is_err()
        );
        let mut foreign = scene.clone();
        foreign.project = "other".into();
        assert!(outline.drop_request(&foreign, None, &structure).is_err());
        let mut changed_schema = structure.clone();
        changed_schema.revision += 1;
        assert!(outline.drop_request(&scene, None, &changed_schema).is_err());
        outline.source[2].metadata["title"] = json!("Changed elsewhere");
        assert!(outline.drop_request(&scene, None, &structure).is_err());
        outline.source[2] = source[2].clone();
        outline.source[0].metadata["trashedAt"] = json!("2026-11-09");
        assert!(outline.drop_request(&scene, None, &structure).is_err());
        Ok(())
    }

    #[test]
    fn drag_geometry_scrolls_only_at_visible_edges_and_expands_after_a_continuous_hover()
    -> TestResult {
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(200.0, ROW_HEIGHT));
        assert_eq!(drop_zone(rect, egui::pos2(80.0, 2.0)), DropZone::Above);
        assert_eq!(drop_zone(rect, egui::pos2(80.0, 17.0)), DropZone::Inside);
        assert_eq!(drop_zone(rect, egui::pos2(80.0, 32.0)), DropZone::Below);
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(200.0, 300.0));
        assert!(edge_scroll(viewport, egui::pos2(40.0, 5.0), 0.016) > 0.0);
        assert!(edge_scroll(viewport, egui::pos2(40.0, 295.0), 0.016) < 0.0);
        assert_eq!(edge_scroll(viewport, egui::pos2(201.0, 295.0), 0.016), 0.0);
        let mut outline =
            ProjectOutline::prepare("project", &[drag_node("part", None, 1.0, "folder")])?;
        outline.collapsed.insert(0);
        outline.delayed_expand(Some(0), 1.0);
        outline.delayed_expand(Some(0), 1.6);
        assert!(outline.collapsed.contains(&0));
        outline.delayed_expand(None, 1.7);
        outline.delayed_expand(Some(0), 2.0);
        assert!(outline.collapsed.contains(&0));
        outline.delayed_expand(Some(0), 2.7);
        assert!(!outline.collapsed.contains(&0));
        Ok(())
    }

    #[test]
    fn escape_removes_native_drag_payload_before_a_same_frame_release() -> TestResult {
        let outline =
            ProjectOutline::prepare("project", &[drag_node("part", None, 1.0, "folder")])?;
        let ctx = egui::Context::default();
        egui::DragAndDrop::set_payload(&ctx, outline.drag_payload(0, &drag_structure())?);
        let input = egui::RawInput {
            events: vec![
                egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos: egui::pos2(20.0, 20.0),
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            assert!(!egui::DragAndDrop::has_payload_of_type::<OutlineDrag>(
                ui.ctx()
            ));
        });
        // This headless input test has no GPU to consume font texture uploads.
        output.textures_delta.clear();
        Ok(())
    }

    #[test]
    fn keyboard_item_menu_stays_painted_after_its_sizing_and_key_frames() -> TestResult {
        fn contains_text(shape: &egui::epaint::Shape, text: &str) -> bool {
            match shape {
                egui::epaint::Shape::Text(shape) => shape.galley.job.text == text,
                egui::epaint::Shape::Vec(shapes) => {
                    shapes.iter().any(|shape| contains_text(shape, text))
                }
                _ => false,
            }
        }
        let outline =
            ProjectOutline::prepare("project", &[drag_node("part", None, 1.0, "folder")])?;
        let ctx = egui::Context::default();
        for frame in 0..4 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(640.0, 480.0),
                )),
                events: if frame == 1 {
                    vec![egui::Event::Key {
                        key: egui::Key::F10,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::SHIFT,
                    }]
                } else {
                    vec![]
                },
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                let row = tree_row(
                    ui,
                    Icon::Folder,
                    "part",
                    true,
                    crate::theme::tokens(ui.ctx()),
                );
                if frame == 0 {
                    row.request_focus();
                }
                assert!(
                    item_context_menu(&row, &outline.nodes[0], Language::English, true).is_none()
                );
            });
            let painted = output
                .shapes
                .iter()
                .any(|shape| contains_text(&shape.shape, "Rename…"));
            output.textures_delta.clear();
            if frame >= 2 {
                assert!(
                    painted,
                    "The keyboard menu must remain painted after the opening frame"
                );
            }
        }
        Ok(())
    }
}
