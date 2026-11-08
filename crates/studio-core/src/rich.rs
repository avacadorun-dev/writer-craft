//! Structured snapshots and drafts share the application revision authority.
//!
//! Native annotation decorations and full editor parity require separate UI evidence.
//! Original opaque imports remain read-only; legacy text commands cannot flatten
//! a document after its structured representation has been enabled.

use craft_editor_model::{Document as RichTree, Editor, Revision as ModelRevision, Transaction};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    Document, Result, StudioError, Workspace, append_revision, read_document, require_project,
    require_revision, validate_body, validate_title, write_revision,
};

/// A complete recoverable structured draft pinned to its canonical base.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RichDraft {
    /// Canonical storage revision at the start of this working draft.
    pub base_revision: i64,
    /// Content-free optimistic draft identity, renewed after every acknowledged write.
    pub version: String,
    /// Complete structured content; never a flattened display projection.
    pub content: Value,
}

/// One complete historical structured document snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RichRevision {
    /// Canonical document revision, shared with its text projection history.
    pub revision: i64,
    /// Complete canonical tree at this revision.
    pub content: Value,
    /// Existing application revision origin.
    pub source: String,
}

impl Workspace {
    /// Create a structured document and both histories in one transaction.
    pub fn create_rich_document(
        &self,
        project_id: &str,
        title: &str,
        content: Value,
    ) -> Result<Document> {
        let title = validate_title(title, 200)?;
        let (_, body, encoded) = validate_tree(content)?;
        let id = uuid::Uuid::new_v4().to_string();
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_project(&transaction, project_id)?;
        transaction.execute(
            "INSERT INTO documents(id,project_id,title,body,revision,read_only,source_json) VALUES(?1,?2,?3,?4,1,0,NULL)",
            params![id, project_id, title, body],
        )?;
        append_revision(&transaction, &id, 1, &body, "manual")?;
        write_rich_snapshot(&transaction, &id, 1, &encoded)?;
        let result = read_document(&transaction, &id)?;
        transaction.commit()?;
        Ok(result)
    }

    /// Explicitly convert an editable native prose document, retaining old history.
    /// Existing opaque imports and unresolved prose drafts are never converted.
    pub fn enable_rich_document(&self, id: &str, expected_revision: i64) -> Result<Document> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = read_document(&transaction, id)?;
        require_structured_source(&current)?;
        require_revision(&current, expected_revision)?;
        if current.rich_json.is_some() {
            return Ok(current);
        }
        if super::read_draft(&transaction, id)?.is_some() {
            return Err(StudioError::Validation(
                "Recover or save the prose draft before enabling structured editing".into(),
            ));
        }
        let block_id = uuid::Uuid::new_v4().to_string();
        let tree = RichTree::plain(block_id, &current.body).map_err(model_error)?;
        let (_, body, encoded) = validate_tree(tree.to_json().map_err(model_error)?)?;
        let updated = write_revision(&transaction, &current, &body, "manual")?;
        write_rich_snapshot(&transaction, id, updated.revision, &encoded)?;
        let result = read_document(&transaction, id)?;
        transaction.commit()?;
        Ok(result)
    }

    /// Save a complete validated manual rich snapshot with optimistic concurrency.
    /// Formatting-only changes create a revision even when plain text is unchanged.
    pub fn save_rich_document(
        &self,
        id: &str,
        expected_revision: i64,
        content: Value,
    ) -> Result<Document> {
        self.save_rich_snapshot(id, expected_revision, content, "manual", None)
    }

    fn save_rich_snapshot(
        &self,
        id: &str,
        expected_revision: i64,
        content: Value,
        source: &str,
        historical_revision: Option<i64>,
    ) -> Result<Document> {
        let (content, body, encoded) = validate_tree(content)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = read_document(&transaction, id)?;
        require_structured_source(&current)?;
        require_revision(&current, expected_revision)?;
        let previous = current.rich_json.as_ref().ok_or_else(|| {
            StudioError::Validation("Structured editing is not enabled for this document".into())
        })?;
        let result = if previous == &content && historical_revision.is_none() {
            current
        } else {
            let updated = write_revision(&transaction, &current, &body, source)?;
            write_rich_snapshot(&transaction, id, updated.revision, &encoded)?;
            let result = read_document(&transaction, id)?;
            super::annotations::reconcile_annotations(
                &transaction,
                &current,
                &result,
                None,
                historical_revision,
            )?;
            result
        };
        transaction.execute("DELETE FROM rich_working_drafts WHERE document_id=?1 AND base_revision=?2 AND content_json=?3", params![id, expected_revision, encoded])?;
        transaction.commit()?;
        Ok(result)
    }

    /// Apply a headless structured transaction through the same canonical model.
    /// Revision/tree failures leave both structured and projection histories intact.
    pub fn apply_rich_transaction(&self, id: &str, command: Transaction) -> Result<Document> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = read_document(&transaction, id)?;
        require_structured_source(&current)?;
        let expected_revision = i64::try_from(command.base_revision.0)
            .map_err(|_| StudioError::Validation("Document revision is out of range".into()))?;
        require_revision(&current, expected_revision)?;
        let original = current.rich_json.as_ref().ok_or_else(|| {
            StudioError::Validation("Structured editing is not enabled for this document".into())
        })?;
        let tree = RichTree::from_json(original.clone()).map_err(model_error)?;
        let mut editor = Editor::new(tree, ModelRevision(command.base_revision.0));
        let change = editor.apply(command).map_err(model_error)?;
        let (content, body, encoded) = validate_tree(change.after.to_json().map_err(model_error)?)?;
        let result = if &content == original {
            current
        } else {
            let updated = write_revision(&transaction, &current, &body, "manual")?;
            write_rich_snapshot(&transaction, id, updated.revision, &encoded)?;
            let result = read_document(&transaction, id)?;
            super::annotations::reconcile_annotations(
                &transaction,
                &current,
                &result,
                Some(&change.mappings),
                None,
            )?;
            result
        };
        transaction.execute("DELETE FROM rich_working_drafts WHERE document_id=?1 AND base_revision=?2 AND content_json=?3", params![id, expected_revision, encoded])?;
        transaction.commit()?;
        Ok(result)
    }

    /// Save a complete rich working draft without changing canonical history.
    pub fn save_rich_draft(
        &self,
        id: &str,
        expected_revision: i64,
        expected_draft_version: Option<&str>,
        content: Value,
    ) -> Result<RichDraft> {
        let (content, _, encoded) = validate_tree(content)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = read_document(&transaction, id)?;
        require_structured_source(&current)?;
        require_revision(&current, expected_revision)?;
        if current.rich_json.is_none() {
            return Err(StudioError::Validation(
                "Structured editing is not enabled for this document".into(),
            ));
        }
        let existing = transaction
            .query_row(
                "SELECT base_revision,version FROM rich_working_drafts WHERE document_id=?1",
                [id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        if let Some((base, _)) = &existing
            && *base != expected_revision
        {
            return Err(StudioError::DraftConflict {
                draft_revision: *base,
                current_revision: expected_revision,
            });
        }
        if existing.as_ref().map(|(_, version)| version.as_str()) != expected_draft_version {
            return Err(StudioError::DraftChanged);
        }
        let version = uuid::Uuid::new_v4().to_string();
        transaction.execute("INSERT INTO rich_working_drafts(document_id,base_revision,version,content_json) VALUES(?1,?2,?3,?4) ON CONFLICT(document_id) DO UPDATE SET base_revision=excluded.base_revision,version=excluded.version,content_json=excluded.content_json", params![id, expected_revision, version, encoded])?;
        transaction.commit()?;
        Ok(RichDraft {
            base_revision: expected_revision,
            version,
            content,
        })
    }

    /// Read a pinned rich draft without promoting it into canonical content.
    pub fn rich_draft(&self, id: &str) -> Result<Option<RichDraft>> {
        let connection = self.connection()?;
        read_document(&connection, id)?;
        let row = connection.query_row("SELECT base_revision,version,content_json FROM rich_working_drafts WHERE document_id=?1", [id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))).optional()?;
        row.map(|(base_revision, version, encoded)| {
            Ok(RichDraft {
                base_revision,
                version,
                content: serde_json::from_str(&encoded)?,
            })
        })
        .transpose()
    }

    /// List complete structured revisions; older prose-only snapshots stay separate.
    pub fn rich_revisions(&self, id: &str) -> Result<Vec<RichRevision>> {
        let connection = self.connection()?;
        read_document(&connection, id)?;
        let mut statement = connection.prepare("SELECT r.revision,r.content_json,d.source FROM rich_document_revisions r JOIN document_revisions d ON r.document_id=d.document_id AND r.revision=d.revision WHERE r.document_id=?1 ORDER BY r.revision DESC")?;
        let rows = statement
            .query_map([id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|(revision, encoded, source)| {
                Ok(RichRevision {
                    revision,
                    content: serde_json::from_str(&encoded)?,
                    source,
                })
            })
            .collect()
    }

    /// Restore a historical rich snapshot through a guarded compensating revision.
    pub fn restore_rich_revision(
        &self,
        id: &str,
        expected_revision: i64,
        historical_revision: i64,
    ) -> Result<Document> {
        let content = {
            let connection = self.connection()?;
            let encoded = connection.query_row("SELECT content_json FROM rich_document_revisions WHERE document_id=?1 AND revision=?2", params![id, historical_revision], |row| row.get::<_, String>(0)).optional()?.ok_or_else(|| StudioError::NotFound { kind: "Structured revision", id: historical_revision.to_string() })?;
            serde_json::from_str(&encoded)?
        };
        self.save_rich_snapshot(
            id,
            expected_revision,
            content,
            "undo",
            Some(historical_revision),
        )
    }
}

fn model_error(error: craft_editor_model::Error) -> StudioError {
    StudioError::Validation(format!("Structured document command failed: {error}"))
}

fn require_structured_source(document: &Document) -> Result<()> {
    if document.read_only {
        return Err(StudioError::ReadOnly {
            document_id: document.id.clone(),
        });
    }
    Ok(())
}

pub(super) fn validate_tree(content: Value) -> Result<(Value, String, String)> {
    let encoded_size = serde_json::to_vec(&content)?.len();
    if encoded_size > super::MAX_BODY_BYTES {
        return Err(StudioError::Validation(
            "Structured document exceeds the 16 MiB limit".into(),
        ));
    }
    let tree = RichTree::from_json(content).map_err(model_error)?;
    let body = tree.plain_text();
    validate_body(&body)?;
    let content = tree.to_json().map_err(model_error)?;
    let encoded = serde_json::to_string(&content)?;
    Ok((content, body, encoded))
}

pub(super) fn write_rich_snapshot(
    connection: &rusqlite::Connection,
    id: &str,
    revision: i64,
    encoded: &str,
) -> Result<()> {
    connection.execute("INSERT INTO rich_documents(document_id,content_json) VALUES(?1,?2) ON CONFLICT(document_id) DO UPDATE SET content_json=excluded.content_json", params![id, encoded])?;
    connection.execute(
        "INSERT INTO rich_document_revisions(document_id,revision,content_json) VALUES(?1,?2,?3)",
        params![id, revision, encoded],
    )?;
    Ok(())
}
