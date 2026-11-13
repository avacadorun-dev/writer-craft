//! Schema-aware native tree actions. All mutations are shared guarded core commands.
use super::*;

#[cfg(all(test, feature = "editor-preview"))]
mod input_tests;
use studio_core::{ImportedProjectStructure, ProjectTreeNode};

#[derive(Debug)]
pub(super) struct TrashedDocument;
impl std::fmt::Display for TrashedDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("The requested document or its parent is in Trash")
    }
}

impl std::error::Error for TrashedDocument {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RestoreBlocked {
    ParentInTrash,
    UnavailableBranch,
}

impl RestoreBlocked {
    pub(super) fn msgid(self) -> &'static str {
        match self {
            Self::ParentInTrash => "Restore a parent before restoring an item inside it.",
            Self::UnavailableBranch => "This branch is unavailable. Refresh the project.",
        }
    }
}

impl std::fmt::Display for RestoreBlocked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.msgid())
    }
}

impl std::error::Error for RestoreBlocked {}

#[derive(Clone)]
pub(super) enum TreeAction {
    Browse { id: String, mapped: bool },
    Open(String),
    OpenDocument(String, bool),
    Reference(String),
    Create(Option<String>),
    Rename(String),
    Move(String),
    Trash(String),
    Restore(String),
    Drop(Box<TreeDrop>),
    BeginDrag(Box<super::project_navigation::OutlineDrag>, egui::Context),
}

#[derive(Clone)]
pub(super) struct TreeDrop {
    pub(super) project: String,
    pub(super) structure: ImportedProjectStructure,
    pub(super) original: ProjectTreeNode,
    pub(super) targets: Vec<ProjectTreeNode>,
    pub(super) parent: Option<String>,
    pub(super) before: Option<String>,
}

struct TreeSaveAck {
    document: String,
    before: i64,
    after: i64,
}

fn checked_drop_node<'a>(
    project: &str,
    expected: &ProjectTreeNode,
    nodes: &'a [ProjectTreeNode],
) -> studio_core::Result<&'a ProjectTreeNode> {
    nodes
        .iter()
        .find(|node| node.logical_id == expected.logical_id && *node == expected)
        .ok_or_else(|| studio_core::StudioError::ProjectNodeChanged {
            project_id: project.into(),
            node_id: expected.logical_id.clone(),
        })
}

fn node_after_own_save(
    project: &str,
    original: &ProjectTreeNode,
    nodes: &[ProjectTreeNode],
    acknowledgement: Option<&TreeSaveAck>,
) -> studio_core::Result<ProjectTreeNode> {
    let mut expected = original.clone();
    if let Some(saved) = acknowledgement
        && original.document_id.as_deref() == Some(saved.document.as_str())
        && original.metadata["currentRevision"].as_i64() == Some(saved.before)
        && saved.after > saved.before
    {
        // Only this acknowledged text revision may advance. Parent, title, order and every
        // extension field still match the drag snapshot; an external edit is never refreshed away.
        expected.metadata["currentRevision"] = serde_json::json!(saved.after);
    }
    checked_drop_node(project, &expected, nodes)?;
    Ok(expected)
}

#[derive(Clone, Copy, PartialEq)]
enum Operation {
    Create,
    Rename,
    Move,
    Trash,
    Restore,
}

struct TreeDialog {
    project: String,
    structure: ImportedProjectStructure,
    native_labels: bool,
    operation: Operation,
    original: Option<ProjectTreeNode>,
    parent: Option<String>,
    before: Option<String>,
    kind: String,
    title: String,
    focus: bool,
}

#[derive(Default)]
pub(super) struct TreeUi {
    pub(super) nodes: Vec<ProjectTreeNode>,
    pub(super) structure: Option<ImportedProjectStructure>,
    paths: std::collections::HashMap<String, String>,
    pub(super) selected: Option<String>,
    dialog: Option<TreeDialog>,
    pub(super) trash_open: bool,
}

impl TreeUi {
    pub(super) fn with_nodes(nodes: Vec<ProjectTreeNode>) -> Self {
        Self {
            paths: node_paths(&nodes),
            nodes,
            ..Self::default()
        }
    }
    pub(super) fn with_structure(
        nodes: Vec<ProjectTreeNode>,
        structure: ImportedProjectStructure,
    ) -> Self {
        Self {
            structure: Some(structure),
            ..Self::with_nodes(nodes)
        }
    }
    pub(super) fn modal_open(&self) -> bool {
        self.dialog.is_some() || self.trash_open
    }
}

pub(super) fn trashed(node: &ProjectTreeNode) -> bool {
    node.metadata["trashedAt"]
        .as_str()
        .is_some_and(|value| !value.is_empty())
}

fn restore_ancestors_available(
    node: &ProjectTreeNode,
    nodes: &[ProjectTreeNode],
) -> Result<(), RestoreBlocked> {
    let mut parent = node.metadata["parentId"].as_str();
    let mut seen = std::collections::HashSet::from([node.logical_id.as_str()]);
    while let Some(id) = parent {
        if !seen.insert(id) {
            return Err(RestoreBlocked::UnavailableBranch);
        }
        let ancestor = nodes
            .iter()
            .find(|candidate| candidate.logical_id == id)
            .ok_or(RestoreBlocked::UnavailableBranch)?;
        if trashed(ancestor) {
            return Err(RestoreBlocked::ParentInTrash);
        }
        parent = ancestor.metadata["parentId"].as_str();
    }
    Ok(())
}

pub(super) fn document_is_active(nodes: &[ProjectTreeNode], document: &str) -> bool {
    let mut current = nodes
        .iter()
        .find(|node| node.document_id.as_deref() == Some(document));
    let mut seen = std::collections::HashSet::new();
    while let Some(node) = current {
        if trashed(node) || !seen.insert(node.logical_id.as_str()) {
            return false;
        }
        current = node.metadata["parentId"]
            .as_str()
            .and_then(|id| nodes.iter().find(|parent| parent.logical_id == id));
    }
    true
}

fn node_title(node: &ProjectTreeNode) -> &str {
    node.metadata["title"].as_str().unwrap_or_default()
}

fn node_paths(nodes: &[ProjectTreeNode]) -> std::collections::HashMap<String, String> {
    let index: std::collections::HashMap<_, _> = nodes
        .iter()
        .map(|node| (node.logical_id.as_str(), node))
        .collect();
    nodes
        .iter()
        .map(|node| {
            let mut parts = Vec::new();
            let mut cursor = Some(node);
            let mut seen = std::collections::HashSet::new();
            while let Some(current) = cursor {
                if !seen.insert(current.logical_id.as_str()) {
                    break;
                }
                // Bound presentation cost for deep imported trees without changing their data.
                if parts.len() == 4 {
                    parts.push("…".into());
                    break;
                }
                let mut title: String = node_title(current).chars().take(100).collect();
                if node_title(current).chars().count() > 100 {
                    title.push('…');
                }
                parts.push(title);
                cursor = current.metadata["parentId"]
                    .as_str()
                    .and_then(|id| index.get(id).copied());
            }
            parts.reverse();
            (node.logical_id.clone(), parts.join(" / "))
        })
        .collect()
}

fn allowed_kinds(
    structure: &ImportedProjectStructure,
    nodes: &[ProjectTreeNode],
    parent: Option<&str>,
) -> Vec<(String, String)> {
    let parent_kind = parent
        .and_then(|id| nodes.iter().find(|node| node.logical_id == id))
        .and_then(|node| node.metadata["kind"].as_str());
    allowed_for_kind(structure, parent_kind, parent.is_none())
}

pub(super) fn allowed_for_kind(
    structure: &ImportedProjectStructure,
    parent_kind: Option<&str>,
    root: bool,
) -> Vec<(String, String)> {
    let Some(types) = structure.schema["nodeTypes"].as_array() else {
        return Vec::new();
    };
    let children = parent_kind
        .and_then(|kind| {
            types
                .iter()
                .find(|node_type| node_type["key"].as_str() == Some(kind))
        })
        .and_then(|node_type| node_type["allowedChildren"].as_array());
    types
        .iter()
        .filter(|node_type| {
            if root {
                node_type["canBeRoot"].as_bool() == Some(true)
            } else {
                children
                    .is_some_and(|children| children.iter().any(|child| child == &node_type["key"]))
            }
        })
        .filter_map(|node_type| {
            Some((
                node_type["key"].as_str()?.into(),
                node_type["label"].as_str()?.into(),
            ))
        })
        .collect()
}

fn type_label<'a>(native_labels: bool, key: &'a str, stored: &'a str, l: Language) -> &'a str {
    // Native defaults are app messages; imported/custom labels are the author's data.
    if native_labels && matches!(stored, "Folder" | "Document" | "Note") {
        match key {
            "folder" => l.text("Folder"),
            "document" => l.text("Document"),
            "note" => l.text("Note"),
            _ => stored,
        }
    } else {
        stored
    }
}

impl StudioApp {
    fn save_tree_context(&mut self, project: &str) -> AppResult<Option<TreeSaveAck>> {
        let saving = self
            .document
            .as_ref()
            .filter(|document| self.dirty.is_some() && document.project_id == project)
            .map(|document| (document.id.clone(), document.revision));
        self.save_explicit()?;
        if self.dirty.is_some() {
            return Err(studio_core::StudioError::Validation(
                "Finish saving the current text before moving this item".into(),
            )
            .into());
        }
        Ok(saving.and_then(|(id, revision)| {
            self.document.as_ref().and_then(|document| {
                (document.id == id && document.project_id == project).then_some(TreeSaveAck {
                    document: id,
                    before: revision,
                    after: document.revision,
                })
            })
        }))
    }

    fn begin_tree_drag(
        &mut self,
        mut drag: super::project_navigation::OutlineDrag,
        ctx: &egui::Context,
    ) -> AppResult<()> {
        if self.project.as_ref().map(|project| project.id.as_str()) != Some(drag.project.as_str()) {
            return Err("tree_document_scope_changed".into());
        }
        let structure = self.workspace.project_structure(&drag.project)?;
        if structure != drag.structure {
            return Err(studio_core::StudioError::ProjectStructureConflict {
                expected: drag.structure.revision,
                actual: structure.revision,
            }
            .into());
        }
        let before = self.workspace.initialize_project_tree(&drag.project)?;
        checked_drop_node(&drag.project, &drag.original, &before)?;
        // A clean drag source cannot race the editor's ordinary 650 ms autosave during hover.
        // This acknowledges authored text only; hierarchy mutation still waits for release.
        let saved = self.save_tree_context(&drag.project)?;
        let after = self.workspace.initialize_project_tree(&drag.project)?;
        drag.original = node_after_own_save(&drag.project, &drag.original, &after, saved.as_ref())?;
        self.refresh_project_tree()?;
        if ctx.input(|input| input.pointer.primary_down() && !input.key_pressed(egui::Key::Escape))
        {
            egui::DragAndDrop::set_payload(ctx, drag);
            ctx.request_repaint();
        }
        Ok(())
    }

    fn apply_tree_drop(&mut self, request: &TreeDrop) -> AppResult<()> {
        if self.project.as_ref().map(|project| project.id.as_str())
            != Some(request.project.as_str())
        {
            return Err("tree_document_scope_changed".into());
        }
        let structure = self.workspace.project_structure(&request.project)?;
        if structure != request.structure {
            return Err(studio_core::StudioError::ProjectStructureConflict {
                expected: request.structure.revision,
                actual: structure.revision,
            }
            .into());
        }
        let before = self.workspace.initialize_project_tree(&request.project)?;
        checked_drop_node(&request.project, &request.original, &before)?;
        for target in &request.targets {
            checked_drop_node(&request.project, target, &before)?;
        }
        let acknowledgement = self.save_tree_context(&request.project)?;
        let after = self.workspace.initialize_project_tree(&request.project)?;
        let original = node_after_own_save(
            &request.project,
            &request.original,
            &after,
            acknowledgement.as_ref(),
        )?;
        let targets = request
            .targets
            .iter()
            .map(|target| {
                node_after_own_save(&request.project, target, &after, acknowledgement.as_ref())
            })
            .collect::<studio_core::Result<Vec<_>>>()?;
        let structure = self.workspace.project_structure(&request.project)?;
        if structure != request.structure {
            return Err(studio_core::StudioError::ProjectStructureConflict {
                expected: request.structure.revision,
                actual: structure.revision,
            }
            .into());
        }
        let moved = self.workspace.move_project_tree_node_at_snapshot(
            &request.structure,
            &original,
            &targets,
            request.parent.as_deref(),
            request.before.as_deref(),
        )?;
        // A durable drop is consumed once even if refreshing its presentation fails.
        if let Err(error) = self.acknowledge_tree_change(Operation::Move, &moved) {
            self.error = Some(UiError {
                msgid: "The item was saved. Refresh the project to see it.",
                details: error.to_string(),
            });
        }
        Ok(())
    }

    pub(super) fn refresh_project_tree(&mut self) -> AppResult<()> {
        let Some(project) = self.project.as_ref() else {
            return Ok(());
        };
        let nodes = self.workspace.initialize_project_tree(&project.id)?;
        self.documents = self.workspace.list_documents(&project.id)?;
        self.project_outline.refresh(&project.id, &nodes)?;
        self.tree_ui.paths = node_paths(&nodes);
        self.tree_ui.nodes = nodes;
        self.tree_ui.structure = Some(self.workspace.project_structure(&project.id)?);
        Ok(())
    }

    pub(super) fn tree_action(&mut self, action: TreeAction) -> AppResult<()> {
        if let TreeAction::BeginDrag(drag, ctx) = action {
            return self.begin_tree_drag(*drag, &ctx);
        }
        if let TreeAction::Drop(request) = action {
            return self.apply_tree_drop(&request);
        }
        let (action, browsing) = match action {
            TreeAction::Browse { id, mapped } => (
                if mapped {
                    TreeAction::Open(id)
                } else {
                    TreeAction::OpenDocument(id, false)
                },
                true,
            ),
            action => (action, false),
        };
        self.save_explicit()?;
        self.refresh_project_tree()?;
        let project = self
            .project
            .as_ref()
            .ok_or("tree_project_missing")?
            .id
            .clone();
        let structure = self.workspace.project_structure(&project)?;
        let native_labels = self
            .workspace
            .imported_project_structure(&project)?
            .is_none();
        if let TreeAction::OpenDocument(id, reference) = &action {
            let document = self.workspace.document(id)?;
            if document.project_id != project {
                return Err("tree_document_scope_changed".into());
            }
            if *reference {
                self.begin_reference_excursion(document)?;
            } else {
                self.open_tree_document(document, browsing)?;
            }
            return Ok(());
        }
        if let TreeAction::Create(parent) = action {
            let kinds = allowed_kinds(&structure, &self.tree_ui.nodes, parent.as_deref());
            let kind = kinds
                .first()
                .map(|(key, _)| key.clone())
                .unwrap_or_default();
            self.tree_ui.dialog = Some(TreeDialog {
                project,
                structure,
                native_labels,
                operation: Operation::Create,
                original: None,
                parent,
                before: None,
                kind,
                title: String::new(),
                focus: true,
            });
            return Ok(());
        }
        let id = match &action {
            TreeAction::Open(id)
            | TreeAction::Reference(id)
            | TreeAction::Rename(id)
            | TreeAction::Move(id)
            | TreeAction::Trash(id)
            | TreeAction::Restore(id) => id,
            TreeAction::Create(_)
            | TreeAction::OpenDocument(_, _)
            | TreeAction::Browse { .. }
            | TreeAction::Drop(_)
            | TreeAction::BeginDrag(_, _) => {
                return Ok(());
            }
        };
        let node = self
            .tree_ui
            .nodes
            .iter()
            .find(|node| &node.logical_id == id)
            .cloned()
            .ok_or("tree_node_missing")?;
        if matches!(action, TreeAction::Restore(_))
            && let Err(error) = restore_ancestors_available(&node, &self.tree_ui.nodes)
        {
            self.tree_ui.trash_open = true;
            return Err(error.into());
        }
        self.tree_ui.selected = Some(node.logical_id.clone());
        if matches!(action, TreeAction::Open(_) | TreeAction::Reference(_)) {
            if trashed(&node) {
                return Err(TrashedDocument.into());
            }
            let document = if let Some(document) = &node.document_id {
                self.workspace.document(document)?
            } else if structure
                .is_content_bearing(node.metadata["kind"].as_str().unwrap_or_default())
            {
                let format = if cfg!(feature = "editor-preview") {
                    studio_core::InitialManuscript::Structured
                } else {
                    studio_core::InitialManuscript::Plain
                };
                let document = self
                    .workspace
                    .initialize_project_tree_node_with_format(&project, &node, format)?;
                self.refresh_project_tree()?;
                document
            } else {
                return Ok(());
            };
            if matches!(action, TreeAction::Reference(_)) {
                self.begin_reference_excursion(document)?;
            } else {
                self.open_tree_document(document, browsing)?;
            }
            return Ok(());
        }
        let operation = match action {
            TreeAction::Rename(_) => Operation::Rename,
            TreeAction::Move(_) => Operation::Move,
            TreeAction::Trash(_) => Operation::Trash,
            TreeAction::Restore(_) => Operation::Restore,
            _ => return Ok(()),
        };
        self.tree_ui.trash_open = false;
        self.tree_ui.dialog = Some(TreeDialog {
            project,
            structure,
            native_labels,
            operation,
            parent: node.metadata["parentId"].as_str().map(str::to_owned),
            before: None,
            kind: node.metadata["kind"].as_str().unwrap_or_default().into(),
            title: node_title(&node).into(),
            original: Some(node),
            focus: true,
        });
        Ok(())
    }

    fn open_tree_document(&mut self, document: Document, browsing: bool) -> AppResult<()> {
        if !document_is_active(
            &self.workspace.project_tree(&document.project_id)?,
            &document.id,
        ) {
            return Err(TrashedDocument.into());
        }
        let reuse = browsing
            && self.document.as_ref().is_some_and(|current| {
                current.id == document.id && current.revision == document.revision
            });
        #[cfg(feature = "editor-preview")]
        let reuse = reuse && self.canvas.is_none() && !self.is_recovery_read_only();
        // Selecting the current tree row must not replace its live editor, undo history or
        // measured scroll merely to give the navigator keyboard focus.
        if !reuse {
            self.open_document(document)?;
        }
        self.focus_manuscript = !browsing;
        Ok(())
    }

    fn apply_tree_dialog(&mut self, dialog: &TreeDialog) -> AppResult<ProjectTreeNode> {
        self.save_explicit()?;
        let format = if cfg!(feature = "editor-preview") {
            studio_core::InitialManuscript::Structured
        } else {
            studio_core::InitialManuscript::Plain
        };
        let result = match dialog.operation {
            Operation::Create => self.workspace.create_project_tree_node_with_format(
                &dialog.project,
                dialog.structure.revision,
                dialog.parent.as_deref(),
                &dialog.kind,
                &dialog.title,
                format,
            )?,
            Operation::Rename => self.workspace.rename_project_tree_node(
                &dialog.project,
                dialog.original.as_ref().ok_or("tree_original_missing")?,
                &dialog.title,
            )?,
            Operation::Move => self.workspace.move_project_tree_node(
                &dialog.project,
                dialog.original.as_ref().ok_or("tree_original_missing")?,
                dialog.parent.as_deref(),
                dialog.before.as_deref(),
            )?,
            Operation::Trash => self.workspace.trash_project_tree_node(
                &dialog.project,
                dialog.original.as_ref().ok_or("tree_original_missing")?,
            )?,
            Operation::Restore => {
                match self.workspace.restore_project_tree_node(
                    &dialog.project,
                    dialog.original.as_ref().ok_or("tree_original_missing")?,
                ) {
                    Ok(restored) => restored,
                    Err(error) => {
                        // A parent can change after confirmation opens. The transaction remains
                        // authoritative; refresh the list and explain this restore-specific refusal.
                        self.refresh_project_tree()?;
                        if matches!(&error, studio_core::StudioError::Validation(message)
                            if message == "Restore the trashed project node before writing or adding children")
                        {
                            return Err(RestoreBlocked::ParentInTrash.into());
                        }
                        return Err(error.into());
                    }
                }
            }
        };
        Ok(result)
    }

    fn acknowledge_tree_change(
        &mut self,
        operation: Operation,
        result: &ProjectTreeNode,
    ) -> AppResult<()> {
        self.refresh_project_tree()?;
        self.tree_ui.selected = Some(result.logical_id.clone());
        if let Some(document) = &self.document {
            if !document_is_active(&self.tree_ui.nodes, &document.id) {
                self.document = None;
                self.body.clear();
                self.daily_controls.document_changed();
                #[cfg(feature = "editor-preview")]
                {
                    self.rich = None;
                    self.notes.clear();
                }
            } else if operation == Operation::Rename
                && result.document_id.as_deref() == Some(document.id.as_str())
            {
                self.document = Some(self.workspace.document(&document.id)?);
            }
        }
        if operation == Operation::Create
            && let Some(id) = &result.document_id
        {
            self.open_document(self.workspace.document(id)?)?;
            self.focus_manuscript = true;
        }
        self.status = "All changes saved".into();
        Ok(())
    }

    pub(super) fn tree_dialogs(&mut self, ctx: &egui::Context, t: Tokens) {
        if self.error.is_some() {
            return;
        }
        let l = self.language;
        if let Some(mut dialog) = self.tree_ui.dialog.take() {
            let heading = match dialog.operation {
                Operation::Create => "New item",
                Operation::Rename => "Rename item",
                Operation::Move => "Move item",
                Operation::Trash => "Move to Trash",
                Operation::Restore => "Restore item",
            };
            let mut apply = false;
            let mut cancel = false;
            let response = modal(ctx, l, "tree-item-dialog", l.text(heading), 520.0, |ui| {
                if matches!(dialog.operation, Operation::Create | Operation::Move) {
                    ui.label(l.text("Location"));
                    let selected = dialog
                        .parent
                        .as_deref()
                        .and_then(|id| self.tree_ui.paths.get(id).cloned())
                        .unwrap_or_else(|| l.text("Project root").into());
                    egui::ComboBox::from_id_salt("tree-parent")
                        .selected_text(selected)
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            let previous = dialog.parent.clone();
                            ui.selectable_value(&mut dialog.parent, None, l.text("Project root"));
                            for node in self.tree_ui.nodes.iter().filter(|node| !trashed(node)) {
                                if dialog
                                    .original
                                    .as_ref()
                                    .is_some_and(|original| original.logical_id == node.logical_id)
                                {
                                    continue;
                                }
                                let allowed = allowed_for_kind(
                                    &dialog.structure,
                                    node.metadata["kind"].as_str(),
                                    false,
                                );
                                if allowed.is_empty()
                                    || (dialog.operation == Operation::Move
                                        && !allowed.iter().any(|(kind, _)| kind == &dialog.kind))
                                {
                                    continue;
                                }
                                ui.selectable_value(
                                    &mut dialog.parent,
                                    Some(node.logical_id.clone()),
                                    self.tree_ui
                                        .paths
                                        .get(&node.logical_id)
                                        .map(String::as_str)
                                        .unwrap_or_else(|| node_title(node)),
                                );
                            }
                            if previous != dialog.parent {
                                dialog.before = None;
                            }
                        });
                }
                if dialog.operation == Operation::Create {
                    let kinds = allowed_kinds(
                        &dialog.structure,
                        &self.tree_ui.nodes,
                        dialog.parent.as_deref(),
                    );
                    if !kinds.iter().any(|(kind, _)| kind == &dialog.kind) {
                        dialog.kind = kinds
                            .first()
                            .map(|(key, _)| key.clone())
                            .unwrap_or_default();
                    }
                    ui.label(l.text("Item type"));
                    let selected = kinds
                        .iter()
                        .find(|(key, _)| key == &dialog.kind)
                        .map(|(key, label)| type_label(dialog.native_labels, key, label, l))
                        .unwrap_or_else(|| l.text("No items are allowed here."));
                    egui::ComboBox::from_id_salt("tree-kind")
                        .selected_text(selected)
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            for (key, label) in &kinds {
                                ui.selectable_value(
                                    &mut dialog.kind,
                                    key.clone(),
                                    type_label(dialog.native_labels, key, label, l),
                                );
                            }
                        });
                }
                if matches!(dialog.operation, Operation::Create | Operation::Rename) {
                    ui.label(l.text("Title"));
                    let input = components::singleline(ui, &mut dialog.title, l.text("Title"));
                    if dialog.focus {
                        input.request_focus();
                        dialog.focus = false;
                    }
                    apply =
                        input.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                } else {
                    ui.label(RichText::new(&dialog.title).strong());
                }
                if dialog.operation == Operation::Move {
                    ui.label(l.text("Position"));
                    let selected = dialog
                        .before
                        .as_deref()
                        .and_then(|id| self.tree_ui.nodes.iter().find(|node| node.logical_id == id))
                        .map(|node| {
                            l.format("Before {title}", &[("title", node_title(node).into())])
                        })
                        .unwrap_or_else(|| l.text("At the end").into());
                    egui::ComboBox::from_id_salt("tree-before")
                        .selected_text(selected)
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut dialog.before, None, l.text("At the end"));
                            for node in self.tree_ui.nodes.iter().filter(|node| {
                                !trashed(node)
                                    && node.metadata["parentId"].as_str()
                                        == dialog.parent.as_deref()
                                    && !dialog.original.as_ref().is_some_and(|original| {
                                        original.logical_id == node.logical_id
                                    })
                            }) {
                                ui.selectable_value(
                                    &mut dialog.before,
                                    Some(node.logical_id.clone()),
                                    l.format(
                                        "Before {title}",
                                        &[("title", node_title(node).into())],
                                    ),
                                );
                            }
                        });
                }
                if dialog.operation == Operation::Trash {
                    ui.label(l.text("This item and its active children will move to Trash. Their text and history are preserved."));
                }
                if dialog.operation == Operation::Restore {
                    ui.label(l.text("Restore this branch to its saved location. Restore its parent first if needed."));
                }
                let valid = !dialog.title.trim().is_empty()
                    && (dialog.operation != Operation::Create || !dialog.kind.is_empty());
                apply &= valid;
                components::footer(ui, |ui| {
                    apply |= components::action(
                        ui,
                        l.text(match dialog.operation {
                            Operation::Create => "Create",
                            Operation::Rename => "Save",
                            Operation::Move => "Move",
                            Operation::Trash => "Move to Trash",
                            Operation::Restore => "Restore",
                        }),
                        valid,
                        Some(t),
                    )
                    .clicked();
                    cancel = components::action(ui, l.text("Cancel"), true, None).clicked();
                });
            });
            if apply && !cancel {
                match self.apply_tree_dialog(&dialog) {
                    Ok(receipt) => {
                        // The mutation is durable. A later refresh/navigation failure must never
                        // reopen Create and let a retry publish a duplicate item.
                        if let Err(error) = self.acknowledge_tree_change(dialog.operation, &receipt)
                        {
                            self.error = Some(UiError {
                                msgid: "The item was saved. Refresh the project to see it.",
                                details: error.to_string(),
                            });
                        }
                    }
                    Err(error) => {
                        if dialog.operation == Operation::Restore {
                            // After dismissing the error, return to the actionable parent/child
                            // list instead of the same confirmation that cannot succeed.
                            self.tree_ui.trash_open = true;
                        } else {
                            self.tree_ui.dialog = Some(dialog);
                        }
                        self.record_error(Err(error));
                    }
                }
            } else if !cancel && !response.should_close() {
                self.tree_ui.dialog = Some(dialog);
            }
        }
        if self.tree_ui.trash_open && self.error.is_none() {
            let mut chosen = None;
            let mut close = false;
            let response = modal(ctx, l, "tree-trash", l.text("Trash"), 600.0, |ui| {
                ui.label(l.text("Restore a parent before restoring an item inside it."));
                egui::ScrollArea::vertical()
                    .max_height((ctx.content_rect().height() - 300.0).max(100.0))
                    .show(ui, |ui| {
                        for node in self.tree_ui.nodes.iter().filter(|node| trashed(node)) {
                            let blocked =
                                restore_ancestors_available(node, &self.tree_ui.nodes).err();
                            ui.horizontal(|ui| {
                                ui.allocate_ui_with_layout(
                                    egui::vec2((ui.available_width() - 125.0).max(80.0), 40.0),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        ui.add(
                                            egui::Label::new(
                                                self.tree_ui
                                                    .paths
                                                    .get(&node.logical_id)
                                                    .map(String::as_str)
                                                    .unwrap_or_else(|| node_title(node)),
                                            )
                                            .truncate(),
                                        );
                                        if let Some(reason) = blocked {
                                            ui.label(
                                                RichText::new(l.text(reason.msgid()))
                                                    .small()
                                                    .color(t.muted),
                                            );
                                        }
                                    },
                                );
                                if ui
                                    .add_enabled(
                                        blocked.is_none(),
                                        egui::Button::new(l.text("Restore")),
                                    )
                                    .clicked()
                                {
                                    chosen = Some(node.logical_id.clone());
                                }
                            });
                        }
                        if !self.tree_ui.nodes.iter().any(trashed) {
                            ui.label(l.text("Trash is empty."));
                        }
                    });
                components::footer(ui, |ui| {
                    close = components::action(ui, l.text("Done"), true, Some(t)).clicked();
                });
            });
            if close || response.should_close() {
                self.tree_ui.trash_open = false;
            }
            if let Some(id) = chosen {
                let result = self.tree_action(TreeAction::Restore(id));
                self.record_error(result);
            }
        }
    }
}

#[cfg(test)]
mod drop_tests {
    use super::*;
    type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

    #[test]
    fn restore_availability_checks_all_ancestors_and_preserves_independent_trash() -> TestResult {
        let workspace = studio_core::Workspace::memory()?;
        let project = workspace.create_project("Restore order")?;
        let outer =
            workspace.create_project_tree_node(&project.id, 1, None, "folder", "Outtakes")?;
        let inner = workspace.create_project_tree_node(
            &project.id,
            1,
            Some(&outer.logical_id),
            "folder",
            "Earlier draft",
        )?;
        let child = workspace.create_project_tree_node(
            &project.id,
            1,
            Some(&inner.logical_id),
            "document",
            "Ferry scene",
        )?;
        let child = workspace.trash_project_tree_node(&project.id, &child)?;
        let outer = workspace.trash_project_tree_node(&project.id, &outer)?;
        let mut nodes = workspace.project_tree(&project.id)?;
        assert_eq!(
            restore_ancestors_available(&child, &nodes),
            Err(RestoreBlocked::ParentInTrash)
        );
        assert_eq!(restore_ancestors_available(&outer, &nodes), Ok(()));
        // Legacy imports can leave the immediate parent active under a trashed ancestor.
        nodes
            .iter_mut()
            .find(|node| node.logical_id == inner.logical_id)
            .ok_or("inner folder")?
            .metadata["trashedAt"] = serde_json::Value::Null;
        assert_eq!(
            restore_ancestors_available(&child, &nodes),
            Err(RestoreBlocked::ParentInTrash)
        );
        nodes.retain(|node| node.logical_id != outer.logical_id);
        assert_eq!(
            restore_ancestors_available(&child, &nodes),
            Err(RestoreBlocked::UnavailableBranch)
        );
        nodes
            .iter_mut()
            .find(|node| node.logical_id == inner.logical_id)
            .ok_or("inner folder")?
            .metadata["parentId"] = serde_json::json!(child.logical_id);
        assert_eq!(
            restore_ancestors_available(&child, &nodes),
            Err(RestoreBlocked::UnavailableBranch)
        );

        workspace.restore_project_tree_node(&project.id, &outer)?;
        let restored = workspace.project_tree(&project.id)?;
        let retained = restored
            .iter()
            .find(|node| node.logical_id == child.logical_id)
            .ok_or("independently trashed scene")?;
        assert_eq!(retained, &child);
        assert_eq!(restore_ancestors_available(retained, &restored), Ok(()));
        Ok(())
    }

    #[test]
    fn restore_preflight_and_later_parent_change_report_parent_first_without_mutation() -> TestResult
    {
        let directory = tempfile::tempdir()?;
        let context = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(context);
        let mut app = StudioApp::new(
            &creation,
            directory.path().join("restore.sqlite"),
            false,
            None,
        )?;
        let project = app.workspace.create_project("Restore order")?;
        let folder =
            app.workspace
                .create_project_tree_node(&project.id, 1, None, "folder", "Outtakes")?;
        let child = app.workspace.create_project_tree_node_with_format(
            &project.id,
            1,
            Some(&folder.logical_id),
            "document",
            "Ferry scene",
            studio_core::InitialManuscript::Plain,
        )?;
        let document = app
            .workspace
            .document(child.document_id.as_deref().ok_or("scene document")?)?;
        let document = app.workspace.save_document(
            &document.id,
            document.revision,
            "Asya waits for the last ferry.",
        )?;
        // Use the same current-head projection as the app. project_tree() preserves raw node
        // metadata and can still contain the revision from before this authored text save.
        let child = app
            .workspace
            .initialize_project_tree(&project.id)?
            .into_iter()
            .find(|node| node.logical_id == child.logical_id)
            .ok_or("current scene")?;
        assert_eq!(
            child.metadata["currentRevision"].as_i64(),
            Some(document.revision)
        );
        let child = app
            .workspace
            .trash_project_tree_node(&project.id, &child)
            .map_err(|error| format!("trash authored scene fixture: {error}"))?;
        app.open_project(project.clone())?;
        let history = app.workspace.revisions(&document.id)?;

        // The list cache still shows an active parent, but the command refreshes before opening.
        let folder = app
            .workspace
            .trash_project_tree_node(&project.id, &folder)?;
        let before = app.workspace.project_tree(&project.id)?;
        let error = app
            .tree_action(TreeAction::Restore(child.logical_id.clone()))
            .err()
            .ok_or("unavailable parent must refuse")?;
        assert_eq!(
            error.downcast_ref::<RestoreBlocked>(),
            Some(&RestoreBlocked::ParentInTrash)
        );
        assert!(app.tree_ui.dialog.is_none());
        assert!(app.tree_ui.trash_open);
        assert_eq!(app.workspace.project_tree(&project.id)?, before);

        let folder = app
            .workspace
            .restore_project_tree_node(&project.id, &folder)?;
        app.tree_action(TreeAction::Restore(child.logical_id.clone()))?;
        let dialog = app.tree_ui.dialog.take().ok_or("restore confirmation")?;
        // The same refusal must remain actionable if the parent changes after confirmation.
        app.workspace
            .trash_project_tree_node(&project.id, &folder)?;
        let before = app.workspace.project_tree(&project.id)?;
        let error = app
            .apply_tree_dialog(&dialog)
            .err()
            .ok_or("changed parent must refuse")?;
        assert_eq!(
            error.downcast_ref::<RestoreBlocked>(),
            Some(&RestoreBlocked::ParentInTrash)
        );
        assert_eq!(app.workspace.project_tree(&project.id)?, before);
        assert_eq!(app.workspace.document(&document.id)?, document);
        assert_eq!(app.workspace.revisions(&document.id)?, history);
        assert_eq!(
            restore_ancestors_available(&child, &app.tree_ui.nodes),
            Err(RestoreBlocked::ParentInTrash)
        );
        Ok(())
    }

    #[test]
    fn acknowledged_text_save_can_move_once_but_cannot_refresh_away_a_foreign_rename() -> TestResult
    {
        let workspace = studio_core::Workspace::memory()?;
        let project = workspace.create_project("Drop acknowledgement")?;
        let document = workspace.create_document(&project.id, "Scene", "Before the move.")?;
        let folder =
            workspace.create_project_tree_node(&project.id, 1, None, "folder", "Chapter")?;
        let structure = workspace.project_structure(&project.id)?;
        let before = workspace.initialize_project_tree(&project.id)?;
        let original = before
            .iter()
            .find(|node| node.document_id.as_deref() == Some(document.id.as_str()))
            .ok_or("missing scene")?;
        let saved = workspace.save_document(
            &document.id,
            document.revision,
            "Draft saved before dragging.",
        )?;
        let acknowledgement = TreeSaveAck {
            document: document.id.clone(),
            before: document.revision,
            after: saved.revision,
        };
        let after = workspace.initialize_project_tree(&project.id)?;
        assert!(checked_drop_node(&project.id, original, &after).is_err());
        let updated = node_after_own_save(&project.id, original, &after, Some(&acknowledgement))?;
        let moved = workspace.move_project_tree_node_at_snapshot(
            &structure,
            &updated,
            std::slice::from_ref(&folder),
            Some(&folder.logical_id),
            None,
        )?;
        assert_eq!(moved.metadata["parentId"], folder.logical_id);
        assert_eq!(
            workspace.document(&document.id)?.body,
            "Draft saved before dragging."
        );
        assert!(
            workspace
                .move_project_tree_node_at_snapshot(
                    &structure,
                    &updated,
                    std::slice::from_ref(&folder),
                    Some(&folder.logical_id),
                    None
                )
                .is_err()
        );

        let saved_again =
            workspace.save_document(&document.id, saved.revision, "Another acknowledged draft.")?;
        let after_save = workspace.initialize_project_tree(&project.id)?;
        let current = after_save
            .iter()
            .find(|node| node.logical_id == moved.logical_id)
            .ok_or("missing moved scene")?;
        workspace.rename_project_tree_node(&project.id, current, "A concurrent title")?;
        let acknowledgement = TreeSaveAck {
            document: document.id.clone(),
            before: saved.revision,
            after: saved_again.revision,
        };
        assert!(
            node_after_own_save(
                &project.id,
                &moved,
                &workspace.initialize_project_tree(&project.id)?,
                Some(&acknowledgement)
            )
            .is_err()
        );
        assert_eq!(
            workspace.document(&document.id)?.title,
            "A concurrent title"
        );
        Ok(())
    }
}
