//! Saved Studio structure policy and atomic native node creation; no UI/profile defaults.
use std::collections::HashSet;

use craft_editor_model::Document as RichTree;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    Document, ProjectTreeNode, Result, StudioError, Workspace, read_document, require_project,
    validate_title,
};

/// The saved imported schema, retaining unknown fields and its exact project namespace.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportedProjectStructure {
    /// Native project namespace; logical source IDs never replace this scope.
    pub project_id: String,
    /// Original project identity used by source-compatible node metadata.
    pub logical_project_id: String,
    /// Source structure revision; node creation does not change the schema itself.
    pub revision: u64,
    /// Complete original schema with custom type labels/rules and unknown fields.
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
        if let Some(parent) = parent {
            let parent_node = read_node(&transaction, project, parent)?;
            require_active(&parent_node)?;
            let parent_kind = parent_node.metadata["kind"]
                .as_str()
                .ok_or_else(|| StudioError::CorruptData("Project parent kind is missing".into()))?;
            let allowed = structure.node_type(parent_kind)?["allowedChildren"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|value| value.as_str() == Some(kind)));
            if !allowed {
                return Err(StudioError::Validation(
                    "The saved project structure does not allow this child kind".into(),
                ));
            }
            let mut cursor = Some(parent.to_owned());
            let mut seen = HashSet::new();
            while let Some(current) = cursor {
                if !seen.insert(current.clone()) || seen.len() >= 512 {
                    return Err(StudioError::Validation(
                        "The new node would exceed the supported project depth".into(),
                    ));
                }
                cursor = read_node(&transaction, project, &current)?.metadata["parentId"]
                    .as_str()
                    .map(str::to_owned);
            }
        } else if node_type["canBeRoot"].as_bool() != Some(true) {
            return Err(StudioError::Validation(
                "This saved project kind cannot be created at the root".into(),
            ));
        }
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
        let mut position = 0.0_f64;
        let mut statement = transaction.prepare(
            "SELECT raw_json FROM project_nodes WHERE project_id=?1 AND parent_logical_id IS ?2",
        )?;
        let mut siblings = statement.query(params![project, parent])?;
        while let Some(row) = siblings.next()? {
            let raw: Value = serde_json::from_str(&row.get::<_, String>(0)?)?;
            let current = raw["position"]
                .as_f64()
                .filter(|value| value.is_finite() && value.abs() <= 1e12)
                .ok_or_else(|| {
                    StudioError::CorruptData("Project sibling position is invalid".into())
                })?;
            position = position.max(current);
        }
        drop(siblings);
        drop(statement);
        position += 1000.0;
        if position > 1e12 {
            return Err(StudioError::Validation(
                "Sibling positions need to be reordered before adding another node".into(),
            ));
        }
        let logical_id = Uuid::new_v4().to_string();
        let native_id = Uuid::new_v4().to_string();
        let content_bearing = node_type["contentBearing"].as_bool() == Some(true);
        if content_bearing {
            insert_empty_document(&transaction, project, &native_id, title)?;
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
        require_active(&current)?;
        let structure = require_structure(&transaction, project)?;
        let kind = current.metadata["kind"]
            .as_str()
            .ok_or_else(|| StudioError::CorruptData("Project node kind is missing".into()))?;
        if !structure.is_content_bearing(kind) {
            return Err(StudioError::Validation(
                "The saved project section does not contain manuscript text".into(),
            ));
        }
        let title = current.metadata["title"]
            .as_str()
            .ok_or_else(|| StudioError::CorruptData("Project node title is missing".into()))?;
        let aliases: (String, String) = transaction.query_row("SELECT n.native_id,d.native_id FROM project_resource_aliases n JOIN project_resource_aliases d ON d.project_id=n.project_id AND d.logical_id=n.logical_id AND d.resource_kind='document' WHERE n.project_id=?1 AND n.logical_id=?2 AND n.resource_kind='node'", params![project,current.logical_id], |row| Ok((row.get(0)?,row.get(1)?)))?;
        if aliases.0 != aliases.1 {
            return Err(StudioError::CorruptData(
                "The reserved project node aliases disagree".into(),
            ));
        }
        insert_empty_document(&transaction, project, &aliases.0, title)?;
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
    read_structure(connection, project)?.ok_or_else(|| StudioError::NotFound {
        kind: "Imported project structure",
        id: project.into(),
    })
}
fn read_node(connection: &Connection, project: &str, logical: &str) -> Result<ProjectTreeNode> {
    let (native, ordinal, raw): (Option<String>,i64,String) = connection.query_row("SELECT native_document_id,ordinal,raw_json FROM project_nodes WHERE project_id=?1 AND logical_id=?2", params![project,logical], |row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional()?.ok_or_else(||StudioError::NotFound {kind:"Project node",id:logical.into()})?;
    let ordinal = u64::try_from(ordinal)
        .map_err(|_| StudioError::CorruptData("Project node order is negative".into()))?;
    Ok(ProjectTreeNode {
        logical_id: logical.into(),
        document_id: native,
        ordinal,
        metadata: serde_json::from_str(&raw)?,
    })
}
fn require_active(node: &ProjectTreeNode) -> Result<()> {
    if node.metadata["trashedAt"]
        .as_str()
        .is_some_and(|value| !value.is_empty())
    {
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
) -> Result<()> {
    let tree = RichTree::plain(Uuid::new_v4().to_string(), "")
        .and_then(|tree| tree.to_json())
        .map_err(|error| {
            StudioError::Validation(format!("Cannot initialize structured manuscript: {error}"))
        })?;
    let encoded = serde_json::to_string(&tree)?;
    connection.execute("INSERT INTO documents(id,project_id,title,body,revision,read_only,source_json) VALUES(?1,?2,?3,'',1,0,NULL)", params![id,project,title])?;
    connection.execute(
        "INSERT INTO document_revisions(document_id,revision,body,source) VALUES(?1,1,'','manual')",
        [id],
    )?;
    connection.execute(
        "INSERT INTO rich_documents(document_id,content_json) VALUES(?1,?2)",
        params![id, encoded],
    )?;
    connection.execute(
        "INSERT INTO rich_document_revisions(document_id,revision,content_json) VALUES(?1,1,?2)",
        params![id, encoded],
    )?;
    Ok(())
}
