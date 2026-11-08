//! Revision-guarded project Canvas persistence, independent of the generic renderer.
use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::Serialize;
use serde_json::{Value, json};

use super::validation::{number, string};
use super::{
    Result, StudioError, Workspace, canvas_validation, json::invalid, read_unsigned,
    require_project, sqlite_integer,
};

/// Lightweight project board metadata; content is loaded only when a board/revision is selected.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCanvasInfo {
    /// Project-scoped original or newly created board identity.
    pub id: String,
    /// User-authored display title.
    pub title: String,
    /// Source ordering coordinate.
    pub position: f64,
    /// Current saved head; restores append instead of rewinding it.
    pub current_revision: u64,
    /// Complete board metadata, including unrecognized source fields.
    pub metadata: Value,
}
/// A bounded history row without allocating every historical graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCanvasRevisionInfo {
    /// Project-scoped board identity.
    pub canvas_id: String,
    /// Monotonic saved revision.
    pub revision: u64,
    /// Saved head from which this revision was created.
    pub base_revision: u64,
    /// JSON Canvas extension schema.
    pub schema_version: u64,
    /// Persisted authoring source, never invented provider output.
    pub source: String,
    /// Actual source time, if present; historical missing times remain unknown.
    pub created_at: Option<String>,
}
/// One selected immutable saved graph.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCanvasRevision {
    /// Saved history metadata.
    pub info: ProjectCanvasRevisionInfo,
    /// Complete canonical content, retaining unknown node/edge/extensions.
    pub content: Value,
}
/// A native unfinished graph; reading it never changes saved content.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCanvasDraft {
    /// Saved revision on which this draft began.
    pub base_revision: u64,
    /// Compare-and-swap token, changed on each acknowledged draft write.
    pub version: String,
    /// Complete unfinished graph.
    pub content: Value,
}
/// One transaction-consistent selected board and saved graph.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCanvasSnapshot {
    /// Current board metadata.
    pub canvas: ProjectCanvasInfo,
    /// Current saved graph.
    pub revision: ProjectCanvasRevision,
    /// Separate unfinished graph, including stale recovery which must be reviewed explicitly.
    pub draft: Option<ProjectCanvasDraft>,
}

impl Workspace {
    /// List this project's board metadata without parsing its full manifest or historical graphs.
    pub fn project_canvases(&self, project: &str) -> Result<Vec<ProjectCanvasInfo>> {
        let connection = self.connection()?;
        require_project(&connection, project)?;
        let mut statement=connection.prepare("SELECT logical_id,current_revision,raw_json FROM project_canvases WHERE project_id=?1 ORDER BY rowid")?;
        let rows = statement
            .query_map([project], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    read_unsigned(row, 1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut boards = rows
            .into_iter()
            .map(|(id, revision, raw)| canvas_info(&id, revision, &raw))
            .collect::<Result<Vec<_>>>()?;
        boards.sort_by(|a, b| a.position.total_cmp(&b.position));
        Ok(boards)
    }

    /// Read only the selected board, saved graph and separate native recovery draft.
    pub fn project_canvas(&self, project: &str, canvas: &str) -> Result<ProjectCanvasSnapshot> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Deferred)?;
        require_project(&transaction, project)?;
        let board = read_canvas(&transaction, project, canvas)?;
        let revision = read_revision(&transaction, project, canvas, board.current_revision)?;
        let draft = read_draft(&transaction, project, canvas)?;
        transaction.commit()?;
        Ok(ProjectCanvasSnapshot {
            canvas: board,
            revision,
            draft,
        })
    }

    /// Read one exact selected history graph; another project's matching logical ID grants no access.
    pub fn project_canvas_revision(
        &self,
        project: &str,
        canvas: &str,
        revision: u64,
    ) -> Result<ProjectCanvasRevision> {
        let connection = self.connection()?;
        require_project(&connection, project)?;
        read_canvas(&connection, project, canvas)?;
        read_revision(&connection, project, canvas, revision)
    }

    /// Read up to 256 history metadata rows, newest first, before an exclusive revision cursor.
    pub fn project_canvas_history(
        &self,
        project: &str,
        canvas: &str,
        before: Option<u64>,
        limit: usize,
    ) -> Result<Vec<ProjectCanvasRevisionInfo>> {
        if !(1..=256).contains(&limit) {
            return invalid("Canvas history page size must be between 1 and 256");
        }
        let connection = self.connection()?;
        require_project(&connection, project)?;
        read_canvas(&connection, project, canvas)?;
        let cursor = before.map(sqlite_integer).transpose()?;
        let mut statement=connection.prepare("SELECT revision,base_revision,schema_version,json_extract(raw_json,'$.source'),json_extract(raw_json,'$.createdAt') FROM project_canvas_revisions WHERE project_id=?1 AND canvas_id=?2 AND (?3 IS NULL OR revision<?3) ORDER BY revision DESC LIMIT ?4")?;
        Ok(statement
            .query_map(
                params![project, canvas, cursor, sqlite_integer(limit)?],
                |row| {
                    Ok(ProjectCanvasRevisionInfo {
                        canvas_id: canvas.into(),
                        revision: read_unsigned(row, 0)?,
                        base_revision: read_unsigned(row, 1)?,
                        schema_version: read_unsigned(row, 2)?,
                        source: row.get(3)?,
                        created_at: row.get(4)?,
                    })
                },
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// Create a project-owned board with one validated initial saved revision.
    pub fn create_project_canvas(
        &self,
        project: &str,
        title: &str,
        content: Value,
    ) -> Result<ProjectCanvasSnapshot> {
        let title = canvas_title(title)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_project(&transaction, project)?;
        validate_scoped_content(&transaction, project, &content)?;
        let position:f64=transaction.query_row("SELECT COALESCE(MAX(CAST(json_extract(raw_json,'$.position') AS REAL)),-1)+1 FROM project_canvases WHERE project_id=?1",[project],|row|row.get(0))?;
        if !position.is_finite() || position.abs() > 1e12 {
            return invalid("Canvas order coordinate is exhausted");
        }
        let id = uuid::Uuid::new_v4().to_string();
        let now = timestamp(&transaction)?;
        let metadata = json!({"id":id,"title":title,"position":position,"currentRevision":1,"createdAt":now,"updatedAt":now});
        transaction.execute("INSERT INTO project_canvases(project_id,logical_id,current_revision,raw_json) VALUES(?1,?2,1,?3)",params![project,id,serde_json::to_string(&metadata)?])?;
        super::insert_alias(
            &transaction,
            project,
            "canvas",
            &id,
            &uuid::Uuid::new_v4().to_string(),
        )?;
        let raw = json!({"id":uuid::Uuid::new_v4().to_string(),"canvasId":id,"revision":1,"baseRevision":0,"schemaVersion":canvas_schema(&content)?,"content":content,"source":"manual","createdAt":now});
        insert_revision(
            &transaction,
            project,
            &id,
            &raw,
            serde_json::to_string(&raw)?,
        )?;
        let canvas = read_canvas(&transaction, project, &id)?;
        let revision = read_revision(&transaction, project, &id, 1)?;
        transaction.commit()?;
        Ok(ProjectCanvasSnapshot {
            canvas,
            revision,
            draft: None,
        })
    }

    /// Rename board metadata while guarding the saved head and retaining unknown fields.
    pub fn rename_project_canvas(
        &self,
        project: &str,
        canvas: &str,
        expected_revision: u64,
        title: &str,
    ) -> Result<ProjectCanvasInfo> {
        let title = canvas_title(title)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_project(&transaction, project)?;
        let mut board = read_canvas(&transaction, project, canvas)?;
        require_canvas_revision(&board, expected_revision)?;
        board.metadata["title"] = Value::String(title);
        board.metadata["updatedAt"] = Value::String(timestamp(&transaction)?);
        transaction.execute(
            "UPDATE project_canvases SET raw_json=?3 WHERE project_id=?1 AND logical_id=?2",
            params![project, canvas, serde_json::to_string(&board.metadata)?],
        )?;
        let renamed = read_canvas(&transaction, project, canvas)?;
        transaction.commit()?;
        Ok(renamed)
    }

    /// Append a validated graph only at the expected head. Same-content saves are no-ops.
    pub fn save_project_canvas(
        &self,
        project: &str,
        canvas: &str,
        expected_revision: u64,
        content: Value,
    ) -> Result<ProjectCanvasRevision> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_project(&transaction, project)?;
        let result = append_revision(
            &transaction,
            project,
            canvas,
            expected_revision,
            content,
            "manual",
        )?;
        transaction.execute("DELETE FROM project_canvas_drafts WHERE project_id=?1 AND canvas_id=?2 AND base_revision=?3 AND content_json=?4",params![project,canvas,sqlite_integer(expected_revision)?,serde_json::to_string(&result.content)?])?;
        transaction.commit()?;
        Ok(result)
    }

    /// Restore by appending a compensating revision; old history and saved head are never rewound.
    pub fn restore_project_canvas_revision(
        &self,
        project: &str,
        canvas: &str,
        expected_revision: u64,
        target_revision: u64,
    ) -> Result<ProjectCanvasRevision> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_project(&transaction, project)?;
        let target = read_revision(&transaction, project, canvas, target_revision)?;
        let result = append_revision(
            &transaction,
            project,
            canvas,
            expected_revision,
            target.content,
            "restore",
        )?;
        transaction.commit()?;
        Ok(result)
    }

    /// Save a native working draft with both saved-head and acknowledged-draft compare-and-swap.
    /// Imported external draft/checkpoint JSON is never inserted here by project migration.
    pub fn save_project_canvas_draft(
        &self,
        project: &str,
        canvas: &str,
        expected_revision: u64,
        expected_draft_version: Option<&str>,
        content: Value,
    ) -> Result<ProjectCanvasDraft> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_project(&transaction, project)?;
        let board = read_canvas(&transaction, project, canvas)?;
        require_canvas_revision(&board, expected_revision)?;
        validate_scoped_content(&transaction, project, &content)?;
        let existing = read_draft(&transaction, project, canvas)?;
        if let Some(draft) = &existing
            && draft.base_revision != expected_revision
        {
            return Err(StudioError::DraftConflict {
                draft_revision: sqlite_integer(draft.base_revision)?,
                current_revision: sqlite_integer(expected_revision)?,
            });
        }
        if existing.as_ref().map(|draft| draft.version.as_str()) != expected_draft_version {
            return Err(StudioError::DraftChanged);
        }
        let version = uuid::Uuid::new_v4().to_string();
        transaction.execute("INSERT INTO project_canvas_drafts(project_id,canvas_id,base_revision,version,content_json) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(project_id,canvas_id) DO UPDATE SET base_revision=excluded.base_revision,version=excluded.version,content_json=excluded.content_json",params![project,canvas,sqlite_integer(expected_revision)?,version,serde_json::to_string(&content)?])?;
        transaction.commit()?;
        Ok(ProjectCanvasDraft {
            base_revision: expected_revision,
            version,
            content,
        })
    }

    /// Discard only the exact acknowledged draft; stale controls cannot delete newer recovery.
    pub fn discard_project_canvas_draft(
        &self,
        project: &str,
        canvas: &str,
        expected_draft_version: &str,
    ) -> Result<()> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_project(&transaction, project)?;
        read_canvas(&transaction, project, canvas)?;
        let affected = transaction.execute(
            "DELETE FROM project_canvas_drafts WHERE project_id=?1 AND canvas_id=?2 AND version=?3",
            params![project, canvas, expected_draft_version],
        )?;
        if affected != 1 {
            return Err(StudioError::DraftChanged);
        }
        transaction.commit()?;
        Ok(())
    }

    /// Explicitly promote an acknowledged native draft at its original saved base.
    /// Stale drafts remain readable and cannot overwrite a newer saved graph.
    pub fn recover_project_canvas_draft(
        &self,
        project: &str,
        canvas: &str,
        expected_revision: u64,
        expected_draft_version: &str,
    ) -> Result<ProjectCanvasRevision> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_project(&transaction, project)?;
        let draft = read_draft(&transaction, project, canvas)?
            .filter(|draft| draft.version == expected_draft_version)
            .ok_or(StudioError::DraftChanged)?;
        if draft.base_revision != expected_revision {
            return Err(StudioError::DraftConflict {
                draft_revision: sqlite_integer(draft.base_revision)?,
                current_revision: sqlite_integer(expected_revision)?,
            });
        }
        let result = append_revision(
            &transaction,
            project,
            canvas,
            expected_revision,
            draft.content,
            "recover",
        )?;
        transaction.execute(
            "DELETE FROM project_canvas_drafts WHERE project_id=?1 AND canvas_id=?2 AND version=?3",
            params![project, canvas, expected_draft_version],
        )?;
        transaction.commit()?;
        Ok(result)
    }
}

fn canvas_title(title: &str) -> Result<String> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 160 {
        return invalid("Canvas title must contain between 1 and 160 characters");
    }
    Ok(title.to_owned())
}
fn timestamp(connection: &Connection) -> Result<String> {
    Ok(
        connection.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')", [], |row| {
            row.get(0)
        })?,
    )
}
fn canvas_schema(content: &Value) -> Result<u64> {
    let schema = match content["x-writers-studio"].get("schemaVersion") {
        None | Some(Value::Null) => 1,
        Some(value) => value.as_u64().ok_or_else(|| {
            StudioError::Validation("Canvas schema must be integer 1 or 2".into())
        })?,
    };
    if !(1..=2).contains(&schema) {
        return invalid("Canvas schema must be 1 or 2");
    }
    Ok(schema)
}
fn canvas_info(id: &str, revision: u64, raw: &str) -> Result<ProjectCanvasInfo> {
    let metadata: Value = serde_json::from_str(raw)?;
    if metadata["id"].as_str() != Some(id) || number(&metadata, "currentRevision")? != revision {
        return Err(StudioError::CorruptData(
            "Canvas metadata identity/head differs from its stored row".into(),
        ));
    }
    let title = string(&metadata, "title")?.to_owned();
    let position = metadata["position"]
        .as_f64()
        .filter(|n| n.is_finite())
        .ok_or_else(|| StudioError::CorruptData("Canvas position is invalid".into()))?;
    Ok(ProjectCanvasInfo {
        id: id.to_owned(),
        title,
        position,
        current_revision: revision,
        metadata,
    })
}
fn read_canvas(connection: &Connection, project: &str, canvas: &str) -> Result<ProjectCanvasInfo> {
    let row=connection.query_row("SELECT current_revision,raw_json FROM project_canvases WHERE project_id=?1 AND logical_id=?2",params![project,canvas],|row|Ok((read_unsigned(row,0)?,row.get::<_,String>(1)?))).optional();
    let (revision, raw) = row?.ok_or_else(|| StudioError::NotFound {
        kind: "Canvas",
        id: canvas.to_owned(),
    })?;
    canvas_info(canvas, revision, &raw)
}
fn require_canvas_revision(board: &ProjectCanvasInfo, expected: u64) -> Result<()> {
    if board.current_revision != expected {
        return Err(StudioError::RevisionConflict {
            expected: sqlite_integer(expected)?,
            actual: sqlite_integer(board.current_revision)?,
        });
    }
    sqlite_integer(expected)?;
    Ok(())
}
fn read_revision(
    connection: &Connection,
    project: &str,
    canvas: &str,
    revision: u64,
) -> Result<ProjectCanvasRevision> {
    let row=connection.query_row("SELECT raw_json,content_json FROM project_canvas_revisions WHERE project_id=?1 AND canvas_id=?2 AND revision=?3",params![project,canvas,sqlite_integer(revision)?],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?))).optional()?.ok_or_else(||StudioError::NotFound {kind:"Canvas revision",id:format!("{canvas}/{revision}")})?;
    let raw: Value = serde_json::from_str(&row.0)?;
    let content: Value = serde_json::from_str(&row.1)?;
    if raw["canvasId"].as_str() != Some(canvas)
        || number(&raw, "revision")? != revision
        || raw["content"] != content
    {
        return Err(StudioError::CorruptData(
            "Canvas revision content/identity differs from its stored row".into(),
        ));
    }
    Ok(ProjectCanvasRevision {
        info: ProjectCanvasRevisionInfo {
            canvas_id: canvas.into(),
            revision,
            base_revision: number(&raw, "baseRevision")?,
            schema_version: number(&raw, "schemaVersion")?,
            source: string(&raw, "source")?.to_owned(),
            created_at: raw["createdAt"].as_str().map(str::to_owned),
        },
        content,
    })
}
fn read_draft(
    connection: &Connection,
    project: &str,
    canvas: &str,
) -> Result<Option<ProjectCanvasDraft>> {
    let row=connection.query_row("SELECT base_revision,version,content_json FROM project_canvas_drafts WHERE project_id=?1 AND canvas_id=?2",params![project,canvas],|row|Ok((read_unsigned(row,0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?))).optional()?;
    row.map(|(base_revision, version, raw)| {
        Ok(ProjectCanvasDraft {
            base_revision,
            version,
            content: serde_json::from_str(&raw)?,
        })
    })
    .transpose()
}
fn append_revision(
    transaction: &Transaction<'_>,
    project: &str,
    canvas: &str,
    expected: u64,
    content: Value,
    source: &str,
) -> Result<ProjectCanvasRevision> {
    let mut board = read_canvas(transaction, project, canvas)?;
    require_canvas_revision(&board, expected)?;
    validate_scoped_content(transaction, project, &content)?;
    let current = read_revision(transaction, project, canvas, expected)?;
    if current.content == content {
        return Ok(current);
    }
    let next = expected
        .checked_add(1)
        .ok_or_else(|| StudioError::Validation("Canvas revision is exhausted".into()))?;
    sqlite_integer(next)?;
    let original:String=transaction.query_row("SELECT raw_json FROM project_canvas_revisions WHERE project_id=?1 AND canvas_id=?2 AND revision=?3",params![project,canvas,sqlite_integer(expected)?],|row|row.get(0))?;
    let mut raw: Value = serde_json::from_str(&original)?;
    let now = timestamp(transaction)?;
    raw["id"] = json!(uuid::Uuid::new_v4().to_string());
    raw["canvasId"] = json!(canvas);
    raw["revision"] = json!(next);
    raw["baseRevision"] = json!(expected);
    raw["schemaVersion"] = json!(canvas_schema(&content)?);
    raw["content"] = content;
    raw["source"] = json!(source);
    raw["createdAt"] = json!(now);
    let encoded = String::from_utf8(super::json::encode_preserving_manifest(&original, &raw)?)
        .map_err(|_| StudioError::CorruptData("Canvas revision is not UTF-8".into()))?;
    insert_revision(transaction, project, canvas, &raw, encoded)?;
    board.metadata["currentRevision"] = json!(next);
    board.metadata["updatedAt"] = json!(now);
    transaction.execute("UPDATE project_canvases SET current_revision=?3,raw_json=?4 WHERE project_id=?1 AND logical_id=?2",params![project,canvas,sqlite_integer(next)?,serde_json::to_string(&board.metadata)?])?;
    read_revision(transaction, project, canvas, next)
}
fn insert_revision(
    transaction: &Transaction<'_>,
    project: &str,
    canvas: &str,
    raw: &Value,
    encoded: String,
) -> Result<()> {
    transaction.execute("INSERT INTO project_canvas_revisions(project_id,canvas_id,revision,base_revision,schema_version,content_json,raw_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![project,canvas,sqlite_integer(number(raw,"revision")?)?,sqlite_integer(number(raw,"baseRevision")?)?,sqlite_integer(number(raw,"schemaVersion")?)?,serde_json::to_string(&raw["content"])?,encoded])?;
    Ok(())
}
fn resource_rows(connection: &Connection, project: &str, query: &str) -> Result<Vec<Value>> {
    let mut statement = connection.prepare(query)?;
    Ok(statement
        .query_map([project], |row| row.get::<_, String>(0))?
        .map(|row| row.map(|id| json!({"id":id})))
        .collect::<std::result::Result<Vec<_>, _>>()?)
}
fn validate_scoped_content(connection: &Connection, project: &str, content: &Value) -> Result<()> {
    canvas_schema(content)?;
    let documents = resource_rows(
        connection,
        project,
        "SELECT logical_id FROM project_nodes WHERE project_id=?1 UNION SELECT id FROM documents d WHERE d.project_id=?1 AND NOT EXISTS(SELECT 1 FROM project_nodes n WHERE n.project_id=?1 AND n.native_document_id=d.id)",
    )?;
    let media = resource_rows(
        connection,
        project,
        "SELECT logical_id FROM project_asset_metadata WHERE project_id=?1 AND resource_kind='media' UNION SELECT id FROM project_assets a WHERE a.project_id=?1 AND NOT EXISTS(SELECT 1 FROM project_asset_metadata m WHERE m.project_id=?1 AND m.native_asset_id=a.id)",
    )?;
    let canvases = resource_rows(
        connection,
        project,
        "SELECT logical_id FROM project_canvases WHERE project_id=?1",
    )?;
    let schema:Option<String>=connection.query_row("SELECT raw_json FROM project_source_records WHERE project_id=?1 AND domain='canvasSchema' AND ordinal=0",[project],|row|row.get(0)).optional()?;
    let schema = schema
        .map(|raw| serde_json::from_str::<Value>(&raw))
        .transpose()?;
    canvas_validation::validate_current_content(
        content,
        &resource_map(&documents),
        &resource_map(&media),
        &resource_map(&canvases),
        schema.as_ref(),
    )
}

fn resource_map(rows: &[Value]) -> HashMap<&str, &Value> {
    rows.iter()
        .filter_map(|value| value["id"].as_str().map(|id| (id, value)))
        .collect()
}
