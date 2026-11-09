//! Saved Studio structure policy and guarded, recoverable hierarchy authoring.
use std::collections::{HashMap, HashSet};

use craft_editor_model::Document as RichTree;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    Document, ProjectTreeNode, Result, StudioError, Workspace, read_document, require_project,
    validate_title,
};

/// The source-compatible structure, retaining imported fields or the native free defaults.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportedProjectStructure {
    /// Native project namespace; logical source IDs never replace this scope.
    pub project_id: String,
    /// Logical project identity used by source-compatible node metadata.
    pub logical_project_id: String,
    /// Source structure revision; node creation does not change the schema itself.
    pub revision: u64,
    /// Complete schema with custom type labels/rules and unknown imported fields.
    pub schema: Value,
}
impl ImportedProjectStructure {
    /// Whether this saved kind may contain manuscript text; missing kinds are not authorized.
    pub fn is_content_bearing(&self, kind: &str) -> bool {
        self.node_type(kind)
            .is_ok_and(|node_type| node_type["contentBearing"].as_bool() == Some(true))
    }
    fn node_type(&self, kind: &str) -> Result<&Value> {
        self.schema["nodeTypes"]
            .as_array()
            .and_then(|types| {
                types
                    .iter()
                    .find(|node_type| node_type["key"].as_str() == Some(kind))
            })
            .ok_or_else(|| {
                StudioError::Validation(
                    "The node kind is not allowed by the saved project structure".into(),
                )
            })
    }
}

impl Workspace {
    /// Read the saved imported schema or the original free structure for a native project.
    /// This does not select a genre, mutate imported metadata or create a new project type.
    pub fn project_structure(&self, project: &str) -> Result<ImportedProjectStructure> {
        let connection = self.connection()?;
        require_project(&connection, project)?;
        require_structure(&connection, project)
    }

    /// Bind older native flat documents into their source-compatible tree without changing text.
    /// Imported projects retain their saved schema and node bindings. Returned snapshots overlay
    /// current document titles/revisions so guarded operations cannot overwrite a newer title.
    pub fn initialize_project_tree(&self, project: &str) -> Result<Vec<ProjectTreeNode>> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_project(&transaction, project)?;
        require_structure(&transaction, project)?;
        if read_structure(&transaction, project)?.is_none() {
            let mut statement = transaction.prepare("SELECT d.id,d.title,d.revision FROM documents d WHERE d.project_id=?1 AND NOT EXISTS(SELECT 1 FROM project_nodes n WHERE n.project_id=d.project_id AND n.native_document_id=d.id) ORDER BY d.rowid")?;
            let documents = statement
                .query_map([project], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            drop(statement);
            let count: i64 = transaction.query_row(
                "SELECT count(*) FROM project_nodes WHERE project_id=?1",
                [project],
                |row| row.get(0),
            )?;
            if count < 0 || count as usize + documents.len() > 20_000 {
                return invalid("The project node capacity is reached");
            }
            let timestamp = timestamp(&transaction)?;
            let mut ordinal: i64 = transaction.query_row(
                "SELECT coalesce(max(ordinal),-1) FROM project_nodes WHERE project_id=?1",
                [project],
                |row| row.get(0),
            )?;
            let mut position = if documents.is_empty() {
                0.0
            } else {
                append_position(&transaction, project, None)?
            };
            for (native, title, revision) in documents {
                ordinal = ordinal.checked_add(1).ok_or_else(|| {
                    StudioError::CorruptData("Project node order overflow".into())
                })?;
                if position > 1e12 {
                    return invalid(
                        "Sibling positions need to be reordered before organizing these documents",
                    );
                }
                let metadata = json!({"id":native,"projectId":project,"parentId":null,"kind":"document","title":title,"status":"draft","position":position,"currentRevision":revision,"createdAt":timestamp,"updatedAt":timestamp});
                for resource in ["node", "document"] {
                    transaction.execute("INSERT INTO project_resource_aliases(project_id,resource_kind,logical_id,native_id) VALUES(?1,?2,?3,?3)", params![project,resource,native])?;
                }
                transaction.execute("INSERT INTO project_nodes(project_id,logical_id,native_document_id,parent_logical_id,ordinal,raw_json) VALUES(?1,?2,?2,NULL,?3,?4)",params![project,native,ordinal,serde_json::to_string(&metadata)?])?;
                position += 1000.0;
            }
        }
        let nodes = read_nodes(&transaction, project)?;
        transaction.commit()?;
        Ok(nodes)
    }

    /// Read the preserved imported structure without applying unvalidated creation defaults.
    pub fn imported_project_structure(
        &self,
        project: &str,
    ) -> Result<Option<ImportedProjectStructure>> {
        let connection = self.connection()?;
        require_project(&connection, project)?;
        read_structure(&connection, project)
    }

    /// Create one source-compatible node and, only for a content-bearing type, its rich head.
    /// All placement/revision checks and identities are committed in one transaction.
    pub fn create_project_tree_node(
        &self,
        project: &str,
        expected_structure_revision: u64,
        parent: Option<&str>,
        kind: &str,
        title: &str,
    ) -> Result<ProjectTreeNode> {
        self.create_project_tree_node_with_format(
            project,
            expected_structure_revision,
            parent,
            kind,
            title,
            crate::InitialManuscript::Structured,
        )
    }

    /// Create a schema-valid item in the editor representation supported by the caller.
    /// The plain desktop retains writable prose without flattening any existing rich document.
    pub fn create_project_tree_node_with_format(
        &self,
        project: &str,
        expected_structure_revision: u64,
        parent: Option<&str>,
        kind: &str,
        title: &str,
        format: crate::InitialManuscript,
    ) -> Result<ProjectTreeNode> {
        let title = validate_title(title, 200)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_project(&transaction, project)?;
        let structure = require_structure(&transaction, project)?;
        if structure.revision != expected_structure_revision {
            return Err(StudioError::ProjectStructureConflict {
                expected: expected_structure_revision,
                actual: structure.revision,
            });
        }
        let node_type = structure.node_type(kind)?;
        validate_placement(&transaction, project, &structure, kind, parent, None)?;
        let (count, maximum): (i64, Option<i64>) = transaction.query_row(
            "SELECT count(*),max(ordinal) FROM project_nodes WHERE project_id=?1",
            [project],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if !(0..20_000).contains(&count) {
            return Err(StudioError::Validation(
                "The project node capacity is reached".into(),
            ));
        }
        let ordinal = maximum
            .map_or(Some(0), |value| value.checked_add(1))
            .ok_or_else(|| StudioError::CorruptData("Project node order overflow".into()))?;
        let position = append_position(&transaction, project, parent)?;
        let logical_id = Uuid::new_v4().to_string();
        let native_id = Uuid::new_v4().to_string();
        let content_bearing = node_type["contentBearing"].as_bool() == Some(true);
        if content_bearing {
            insert_empty_document(&transaction, project, &native_id, title, format)?;
        }
        for resource in ["node", "document"] {
            transaction.execute("INSERT INTO project_resource_aliases(project_id,resource_kind,logical_id,native_id) VALUES(?1,?2,?3,?4)", params![project, resource, logical_id, native_id])?;
        }
        let timestamp: String =
            transaction.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')", [], |row| {
                row.get(0)
            })?;
        let metadata = json!({"id":logical_id,"projectId":structure.logical_project_id,"parentId":parent,"kind":kind,"title":title,"status":"draft","position":position,"currentRevision":if content_bearing {1}else{0},"createdAt":timestamp,"updatedAt":timestamp});
        transaction.execute("INSERT INTO project_nodes(project_id,logical_id,native_document_id,parent_logical_id,ordinal,raw_json) VALUES(?1,?2,?3,?4,?5,?6)", params![project,logical_id,content_bearing.then_some(&native_id),parent,ordinal,serde_json::to_string(&metadata)?])?;
        let node = read_node(&transaction, project, &logical_id)?;
        transaction.commit()?;
        Ok(node)
    }

    /// Initialize exactly the captured empty content-bearing node without changing its source ID.
    /// A stale snapshot cannot initialize a newer/trashed/contentless node or duplicate a head.
    pub fn initialize_project_tree_node(
        &self,
        project: &str,
        expected: &ProjectTreeNode,
    ) -> Result<Document> {
        self.initialize_project_tree_node_with_format(
            project,
            expected,
            crate::InitialManuscript::Structured,
        )
    }

    /// Write first content in the caller's supported representation, retaining the reserved ID.
    pub fn initialize_project_tree_node_with_format(
        &self,
        project: &str,
        expected: &ProjectTreeNode,
        format: crate::InitialManuscript,
    ) -> Result<Document> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_project(&transaction, project)?;
        let current = read_node(&transaction, project, &expected.logical_id)?;
        if current != *expected
            || current.document_id.is_some()
            || current.metadata["currentRevision"].as_u64() != Some(0)
        {
            return Err(StudioError::ProjectNodeChanged {
                project_id: project.into(),
                node_id: expected.logical_id.clone(),
            });
        }
        require_active_branch(&transaction, project, &current)?;
        let structure = require_structure(&transaction, project)?;
        let kind = current.metadata["kind"]
            .as_str()
            .ok_or_else(|| StudioError::CorruptData("Project node kind is missing".into()))?;
        if !structure.is_content_bearing(kind) {
            return Err(StudioError::Validation(
                "The saved project section does not contain manuscript text".into(),
            ));
        }
        validate_placement(
            &transaction,
            project,
            &structure,
            kind,
            current.metadata["parentId"].as_str(),
            Some(&current.logical_id),
        )?;
        let title = current.metadata["title"]
            .as_str()
            .ok_or_else(|| StudioError::CorruptData("Project node title is missing".into()))?;
        let aliases: (String, String) = transaction.query_row("SELECT n.native_id,d.native_id FROM project_resource_aliases n JOIN project_resource_aliases d ON d.project_id=n.project_id AND d.logical_id=n.logical_id AND d.resource_kind='document' WHERE n.project_id=?1 AND n.logical_id=?2 AND n.resource_kind='node'", params![project,current.logical_id], |row| Ok((row.get(0)?,row.get(1)?)))?;
        if aliases.0 != aliases.1 {
            return Err(StudioError::CorruptData(
                "The reserved project node aliases disagree".into(),
            ));
        }
        insert_empty_document(&transaction, project, &aliases.0, title, format)?;
        let mut metadata = current.metadata;
        metadata["currentRevision"] = json!(1);
        metadata["updatedAt"] = json!(transaction.query_row(
            "SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')",
            [],
            |row| row.get::<_, String>(0)
        )?);
        let changed = transaction.execute("UPDATE project_nodes SET native_document_id=?3,raw_json=?4 WHERE project_id=?1 AND logical_id=?2 AND native_document_id IS NULL", params![project,current.logical_id,aliases.0,serde_json::to_string(&metadata)?])?;
        if changed != 1 {
            return Err(StudioError::CorruptData(
                "The project node binding changed under its transaction".into(),
            ));
        }
        let document = read_document(&transaction, &aliases.0)?;
        transaction.commit()?;
        Ok(document)
    }

    /// Rename one captured live folder/document and synchronize its canonical document title.
    /// Text and immutable document revisions are not rewritten by a title change.
    pub fn rename_project_tree_node(
        &self,
        project: &str,
        expected: &ProjectTreeNode,
        title: &str,
    ) -> Result<ProjectTreeNode> {
        let title = validate_title(title, 200)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_project(&transaction, project)?;
        let mut current = guarded_node(&transaction, project, expected)?;
        require_active_branch(&transaction, project, &current)?;
        if current.metadata["title"].as_str() != Some(title) {
            current.metadata["title"] = json!(title);
            current.metadata["updatedAt"] = json!(timestamp(&transaction)?);
            if let Some(document) = &current.document_id {
                transaction.execute(
                    "UPDATE documents SET title=?3 WHERE id=?1 AND project_id=?2",
                    params![document, project, title],
                )?;
            }
            write_node(&transaction, project, &mut current)?;
        }
        transaction.commit()?;
        Ok(current)
    }

    /// Move a captured live subtree before an identified sibling, or append with `before=None`.
    /// The current schema, parent ancestry and complete subtree depth are checked atomically.
    pub fn move_project_tree_node(
        &self,
        project: &str,
        expected: &ProjectTreeNode,
        parent: Option<&str>,
        before: Option<&str>,
    ) -> Result<ProjectTreeNode> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_project(&transaction, project)?;
        let mut current = guarded_node(&transaction, project, expected)?;
        require_active_branch(&transaction, project, &current)?;
        let structure = require_structure(&transaction, project)?;
        let kind = current.metadata["kind"]
            .as_str()
            .ok_or_else(|| StudioError::CorruptData("Project node kind is missing".into()))?;
        let ancestors = validate_placement(
            &transaction,
            project,
            &structure,
            kind,
            parent,
            Some(&current.logical_id),
        )?;
        let nodes = read_nodes(&transaction, project)?;
        let descendants = subtree(&nodes, &current.logical_id)?;
        if descendants
            .iter()
            .any(|(_, depth)| ancestors + *depth + 1 > 512)
        {
            return invalid("The moved branch would exceed the supported project depth");
        }
        if before == Some(current.logical_id.as_str()) {
            if current.metadata["parentId"].as_str() == parent {
                transaction.commit()?;
                return Ok(current);
            }
            return invalid("A moved item cannot be its own destination sibling");
        }
        let mut siblings: Vec<_> = nodes
            .into_iter()
            .filter(|node| {
                node.logical_id != current.logical_id
                    && node.metadata["parentId"].as_str() == parent
                    && !is_trashed(node)
            })
            .collect();
        for node in &siblings {
            position(node)?;
        }
        siblings.sort_by(|a, b| {
            a.metadata["position"]
                .as_f64()
                .unwrap_or_default()
                .total_cmp(&b.metadata["position"].as_f64().unwrap_or_default())
                .then_with(|| {
                    a.metadata["title"]
                        .as_str()
                        .cmp(&b.metadata["title"].as_str())
                })
                .then(a.ordinal.cmp(&b.ordinal))
                .then(a.logical_id.cmp(&b.logical_id))
        });
        let index = match before {
            Some(id) => siblings
                .iter()
                .position(|node| node.logical_id == id)
                .ok_or_else(|| {
                    StudioError::Validation(
                        "The destination sibling changed; choose its current position again".into(),
                    )
                })?,
            None => siblings.len(),
        };
        current.metadata["parentId"] = json!(parent);
        current.metadata["updatedAt"] = json!(timestamp(&transaction)?);
        siblings.insert(index, current);
        for (index, node) in siblings.iter_mut().enumerate() {
            let new_position = (index + 1) as f64 * 1000.0;
            if node.metadata["position"].as_f64() != Some(new_position)
                || node.logical_id == expected.logical_id
            {
                node.metadata["position"] = json!(new_position);
                write_node(&transaction, project, node)?;
            }
        }
        let moved = read_node(&transaction, project, &expected.logical_id)?;
        transaction.commit()?;
        Ok(moved)
    }

    /// Move a captured live branch to recoverable Trash. Already-trashed descendants keep
    /// their independent deletion state and will not be revived by restoring this branch.
    pub fn trash_project_tree_node(
        &self,
        project: &str,
        expected: &ProjectTreeNode,
    ) -> Result<ProjectTreeNode> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_project(&transaction, project)?;
        let current = guarded_node(&transaction, project, expected)?;
        require_active_branch(&transaction, project, &current)?;
        let nodes = read_nodes(&transaction, project)?;
        let operation = Uuid::new_v4().to_string();
        let timestamp = timestamp(&transaction)?;
        let mut hidden = HashSet::new();
        for (index, _) in subtree(&nodes, &current.logical_id)? {
            let node = &nodes[index];
            if is_trashed(node)
                || node.metadata["parentId"]
                    .as_str()
                    .is_some_and(|parent| hidden.contains(parent))
            {
                hidden.insert(node.logical_id.as_str());
                continue;
            }
            let mut node = node.clone();
            let marker = trash_marker_mut(&mut node.metadata)?;
            marker.insert("operationId".into(), json!(operation));
            marker.insert("rootId".into(), json!(current.logical_id));
            node.metadata["trashedAt"] = json!(timestamp);
            node.metadata["updatedAt"] = json!(timestamp);
            write_node(&transaction, project, &mut node)?;
        }
        let trashed = read_node(&transaction, project, &current.logical_id)?;
        transaction.commit()?;
        Ok(trashed)
    }

    /// Restore a captured trashed item and only descendants deleted by that same operation.
    /// Its current saved parent must be active and still permit this kind; otherwise nothing moves.
    pub fn restore_project_tree_node(
        &self,
        project: &str,
        expected: &ProjectTreeNode,
    ) -> Result<ProjectTreeNode> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_project(&transaction, project)?;
        let current = guarded_node(&transaction, project, expected)?;
        if !is_trashed(&current) {
            return invalid("This item is no longer in Trash; refresh the project tree");
        }
        let structure = require_structure(&transaction, project)?;
        let kind = current.metadata["kind"]
            .as_str()
            .ok_or_else(|| StudioError::CorruptData("Project node kind is missing".into()))?;
        validate_placement(
            &transaction,
            project,
            &structure,
            kind,
            current.metadata["parentId"].as_str(),
            Some(&current.logical_id),
        )?;
        let operation = trash_operation(&current.metadata)?.map(str::to_owned);
        let mut nodes = read_nodes(&transaction, project)?;
        let descendants = subtree(&nodes, &current.logical_id)?;
        let timestamp = timestamp(&transaction)?;
        for (index, _) in descendants {
            let node = &mut nodes[index];
            if node.logical_id != current.logical_id
                && (operation.is_none()
                    || node.metadata[TRASH_MARKER]["schemaVersion"].as_u64() != Some(1)
                    || node.metadata[TRASH_MARKER]["operationId"].as_str() != operation.as_deref())
            {
                continue;
            }
            if !is_trashed(node) {
                continue;
            }
            let kind = node.metadata["kind"]
                .as_str()
                .ok_or_else(|| StudioError::CorruptData("Project node kind is missing".into()))?;
            validate_placement(
                &transaction,
                project,
                &structure,
                kind,
                node.metadata["parentId"].as_str(),
                Some(&node.logical_id),
            )?;
            node.metadata["trashedAt"] = Value::Null;
            node.metadata["updatedAt"] = json!(timestamp);
            if operation.is_some() {
                let marker = trash_marker_mut(&mut node.metadata)?;
                marker.remove("operationId");
                marker.remove("rootId");
            }
            write_node(&transaction, project, node)?;
        }
        let restored = read_node(&transaction, project, &current.logical_id)?;
        transaction.commit()?;
        Ok(restored)
    }
}

fn read_structure(
    connection: &Connection,
    project: &str,
) -> Result<Option<ImportedProjectStructure>> {
    let row: Option<(String, String)> = connection.query_row("SELECT p.logical_project_id,r.raw_json FROM project_portability p JOIN project_source_records r ON r.project_id=p.project_id AND r.domain='project' AND r.ordinal=0 WHERE p.project_id=?1", [project], |row|Ok((row.get(0)?,row.get(1)?))).optional()?;
    row.map(|(logical_project_id, raw)| {
        let metadata: Value = serde_json::from_str(&raw)?;
        let revision = metadata["structureRevision"]
            .as_u64()
            .filter(|value| *value > 0)
            .ok_or_else(|| {
                StudioError::CorruptData("Imported project structure revision is invalid".into())
            })?;
        let schema = metadata["structure"].clone();
        if !schema["nodeTypes"].is_array() {
            return Err(StudioError::CorruptData(
                "Imported project structure types are missing".into(),
            ));
        }
        Ok(ImportedProjectStructure {
            project_id: project.into(),
            logical_project_id,
            revision,
            schema,
        })
    })
    .transpose()
}
fn require_structure(connection: &Connection, project: &str) -> Result<ImportedProjectStructure> {
    if let Some(structure) = read_structure(connection, project)? {
        return Ok(structure);
    }
    let imported: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM project_portability WHERE project_id=?1)",
        [project],
        |row| row.get(0),
    )?;
    if imported {
        return Err(StudioError::CorruptData(
            "The imported project structure is missing; preserve the project before retrying"
                .into(),
        ));
    }
    Ok(ImportedProjectStructure {
        project_id: project.into(),
        logical_project_id: project.into(),
        revision: 1,
        schema: native_free_structure(),
    })
}

/// Original Studio free structure. Labels are metadata; the native UI localizes known built-ins.
pub(crate) fn native_free_structure() -> Value {
    json!({"schemaVersion":1,"template":"free","rootLabel":"Materials","nodeTypes":[
        {"key":"folder","label":"Folder","pluralLabel":"Folders","icon":"folder","contentBearing":false,"allowedChildren":["folder","document","note"],"canBeRoot":true},
        {"key":"document","label":"Document","pluralLabel":"Documents","icon":"file-text","contentBearing":true,"allowedChildren":["document","note"],"canBeRoot":true},
        {"key":"note","label":"Note","pluralLabel":"Notes","icon":"sticky-note","contentBearing":true,"allowedChildren":[],"canBeRoot":true}
    ]})
}

fn read_node(connection: &Connection, project: &str, logical: &str) -> Result<ProjectTreeNode> {
    let row = connection.query_row(
        "SELECT n.logical_id,n.native_document_id,n.ordinal,n.raw_json,d.title,d.revision FROM project_nodes n LEFT JOIN documents d ON d.id=n.native_document_id AND d.project_id=n.project_id WHERE n.project_id=?1 AND n.logical_id=?2",
        params![project,logical], node_row,
    ).optional()?.ok_or_else(||StudioError::NotFound {kind:"Project node",id:logical.into()})?;
    decode_node(row)
}

type NodeRow = (
    String,
    Option<String>,
    i64,
    String,
    Option<String>,
    Option<i64>,
);

fn node_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<NodeRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

fn decode_node(
    (logical, native, ordinal, raw, title, revision): NodeRow,
) -> Result<ProjectTreeNode> {
    let ordinal = u64::try_from(ordinal)
        .map_err(|_| StudioError::CorruptData("Project node order is negative".into()))?;
    let mut metadata: Value = serde_json::from_str(&raw)?;
    if !metadata.is_object() {
        return Err(StudioError::CorruptData(
            "Project node metadata is not an object".into(),
        ));
    }
    if let Some(title) = title {
        metadata["title"] = json!(title);
    }
    if let Some(revision) = revision {
        metadata["currentRevision"] = json!(revision);
    }
    Ok(ProjectTreeNode {
        logical_id: logical,
        document_id: native,
        ordinal,
        metadata,
    })
}

fn read_nodes(connection: &Connection, project: &str) -> Result<Vec<ProjectTreeNode>> {
    let mut statement = connection.prepare("SELECT n.logical_id,n.native_document_id,n.ordinal,n.raw_json,d.title,d.revision FROM project_nodes n LEFT JOIN documents d ON d.id=n.native_document_id AND d.project_id=n.project_id WHERE n.project_id=?1 ORDER BY n.ordinal")?;
    let rows = statement
        .query_map([project], node_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    rows.into_iter().map(decode_node).collect()
}

fn guarded_node(
    connection: &Connection,
    project: &str,
    expected: &ProjectTreeNode,
) -> Result<ProjectTreeNode> {
    let current = read_node(connection, project, &expected.logical_id)?;
    if current != *expected {
        return Err(StudioError::ProjectNodeChanged {
            project_id: project.into(),
            node_id: expected.logical_id.clone(),
        });
    }
    Ok(current)
}

fn write_node(connection: &Connection, project: &str, node: &mut ProjectTreeNode) -> Result<()> {
    const TREE_REVISION: &str = "x-writercraft-tree-revision";
    let revision = match node.metadata.get(TREE_REVISION) {
        None => 0,
        Some(value) => value.as_u64().ok_or_else(||StudioError::Validation("This item has an unfamiliar tree revision; preserve it before changing its structure".into()))?,
    };
    node.metadata[TREE_REVISION] =
        json!(
            revision
                .checked_add(1)
                .ok_or_else(|| StudioError::CorruptData(
                    "Project node metadata revision overflow".into()
                ))?
        );
    let changed = connection.execute("UPDATE project_nodes SET parent_logical_id=?3,raw_json=?4 WHERE project_id=?1 AND logical_id=?2",params![project,node.logical_id,node.metadata["parentId"].as_str(),serde_json::to_string(&node.metadata)?])?;
    if changed != 1 {
        return Err(StudioError::ProjectNodeChanged {
            project_id: project.into(),
            node_id: node.logical_id.clone(),
        });
    }
    Ok(())
}

fn timestamp(connection: &Connection) -> Result<String> {
    Ok(
        connection.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')", [], |row| {
            row.get(0)
        })?,
    )
}

fn invalid<T>(message: &str) -> Result<T> {
    Err(StudioError::Validation(message.into()))
}

fn position(node: &ProjectTreeNode) -> Result<f64> {
    node.metadata["position"]
        .as_f64()
        .filter(|position| position.is_finite() && position.abs() <= 1e12)
        .ok_or_else(|| StudioError::CorruptData("Project sibling position is invalid".into()))
}

fn append_position(connection: &Connection, project: &str, parent: Option<&str>) -> Result<f64> {
    let mut statement = connection.prepare(
        "SELECT raw_json FROM project_nodes WHERE project_id=?1 AND parent_logical_id IS ?2",
    )?;
    let mut rows = statement.query(params![project, parent])?;
    let mut maximum = 0.0_f64;
    while let Some(row) = rows.next()? {
        let raw: Value = serde_json::from_str(&row.get::<_, String>(0)?)?;
        let position = raw["position"]
            .as_f64()
            .filter(|position| position.is_finite() && position.abs() <= 1e12)
            .ok_or_else(|| {
                StudioError::CorruptData("Project sibling position is invalid".into())
            })?;
        maximum = maximum.max(position);
    }
    if maximum + 1000.0 > 1e12 {
        return invalid("Sibling positions need to be reordered before adding another node");
    }
    Ok(maximum + 1000.0)
}

/// Return ancestor count, enforcing both direct placement and the active, acyclic chain.
fn validate_placement(
    connection: &Connection,
    project: &str,
    structure: &ImportedProjectStructure,
    kind: &str,
    parent: Option<&str>,
    moving: Option<&str>,
) -> Result<usize> {
    let node_type = structure.node_type(kind)?;
    if let Some(parent) = parent {
        let parent = read_node(connection, project, parent)?;
        let parent_kind = parent.metadata["kind"]
            .as_str()
            .ok_or_else(|| StudioError::CorruptData("Project parent kind is missing".into()))?;
        if !structure.node_type(parent_kind)?["allowedChildren"]
            .as_array()
            .is_some_and(|kinds| kinds.iter().any(|value| value.as_str() == Some(kind)))
        {
            return invalid("The saved project structure does not allow this child kind");
        }
    } else if node_type["canBeRoot"].as_bool() != Some(true) {
        return invalid("This saved project kind cannot be created at the root");
    }
    active_ancestor_count(connection, project, parent, moving)
}

/// Source imports can mark only an ancestor as trashed, leaving its hidden descendants unmarked.
fn require_active_branch(
    connection: &Connection,
    project: &str,
    node: &ProjectTreeNode,
) -> Result<()> {
    require_active(node)?;
    active_ancestor_count(
        connection,
        project,
        node.metadata["parentId"].as_str(),
        Some(&node.logical_id),
    )?;
    Ok(())
}

fn active_ancestor_count(
    connection: &Connection,
    project: &str,
    parent: Option<&str>,
    moving: Option<&str>,
) -> Result<usize> {
    let mut cursor = parent.map(str::to_owned);
    let mut seen = HashSet::new();
    while let Some(current) = cursor {
        if Some(current.as_str()) == moving || !seen.insert(current.clone()) {
            return invalid("An item cannot be moved into its own branch");
        }
        if seen.len() >= 512 {
            return invalid("The project branch would exceed the supported project depth");
        }
        let node = read_node(connection, project, &current)?;
        require_active(&node)?;
        cursor = node.metadata["parentId"].as_str().map(str::to_owned);
    }
    Ok(seen.len())
}

fn subtree(nodes: &[ProjectTreeNode], root: &str) -> Result<Vec<(usize, usize)>> {
    let mut children: HashMap<&str, Vec<usize>> = HashMap::new();
    let mut root_index = None;
    for (index, node) in nodes.iter().enumerate() {
        if node.logical_id == root {
            root_index = Some(index);
        }
        if let Some(parent) = node.metadata["parentId"].as_str() {
            children.entry(parent).or_default().push(index);
        }
    }
    let root_index = root_index.ok_or_else(|| StudioError::NotFound {
        kind: "Project node",
        id: root.into(),
    })?;
    let mut output = Vec::new();
    let mut pending = vec![(root_index, 0)];
    let mut seen = HashSet::new();
    while let Some((index, depth)) = pending.pop() {
        if !seen.insert(index) || depth >= 512 {
            return invalid("The project branch is cyclic or exceeds its supported depth");
        }
        output.push((index, depth));
        if let Some(children) = children.get(nodes[index].logical_id.as_str()) {
            pending.extend(children.iter().map(|&index| (index, depth + 1)));
        }
    }
    Ok(output)
}

const TRASH_MARKER: &str = "x-writercraft-tree-trash";

fn trash_marker_mut(metadata: &mut Value) -> Result<&mut serde_json::Map<String, Value>> {
    if metadata.get(TRASH_MARKER).is_none() {
        metadata[TRASH_MARKER] = json!({"schemaVersion":1});
    }
    if metadata[TRASH_MARKER]["schemaVersion"].as_u64() != Some(1) {
        return invalid(
            "This item has an unfamiliar trash extension; preserve it before changing its lifecycle",
        );
    }
    metadata[TRASH_MARKER]
        .as_object_mut()
        .ok_or_else(|| StudioError::Validation("The trash extension is not an object".into()))
}

fn trash_operation(metadata: &Value) -> Result<Option<&str>> {
    let Some(marker) = metadata.get(TRASH_MARKER) else {
        return Ok(None);
    };
    if marker["schemaVersion"].as_u64() != Some(1) {
        return invalid(
            "This item has an unfamiliar trash extension; preserve it before restoring",
        );
    }
    match marker.get("operationId") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if !value.is_empty() => Ok(Some(value)),
        _ => invalid(
            "The trash operation identity is invalid; preserve the project before restoring",
        ),
    }
}

fn is_trashed(node: &ProjectTreeNode) -> bool {
    node.metadata["trashedAt"]
        .as_str()
        .is_some_and(|value| !value.is_empty())
}

fn require_active(node: &ProjectTreeNode) -> Result<()> {
    if is_trashed(node) {
        return Err(StudioError::Validation(
            "Restore the trashed project node before writing or adding children".into(),
        ));
    }
    Ok(())
}
fn insert_empty_document(
    connection: &Connection,
    project: &str,
    id: &str,
    title: &str,
    format: crate::InitialManuscript,
) -> Result<()> {
    if matches!(format, crate::InitialManuscript::Plain) {
        crate::rich::insert_initial_manuscript(connection, project, id, title, "", None)?;
        return Ok(());
    }
    let tree = RichTree::plain(Uuid::new_v4().to_string(), "")
        .and_then(|tree| tree.to_json())
        .map_err(|error| {
            StudioError::Validation(format!("Cannot initialize structured manuscript: {error}"))
        })?;
    let encoded = serde_json::to_string(&tree)?;
    crate::rich::insert_initial_manuscript(connection, project, id, title, "", Some(&encoded))?;
    Ok(())
}
