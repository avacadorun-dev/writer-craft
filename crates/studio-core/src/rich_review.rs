//! Scoped, durable structured proposals; preparation never publishes canonical writing.
use craft_editor_model::{Change, Document as RichTree, Editor, TableGrid, Transaction};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    Document, ProposalStatus, Result, StudioError, Workspace, read_document, require_revision,
};

/// Content-light metadata, loaded independently of before/after manuscript trees.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RichProposalSummary {
    /// Opaque local proposal identity; always used with a fixed project scope.
    pub id: String,
    /// Monotonic page cursor, not a canonical revision or mutation capability.
    pub cursor: i64,
    /// Owning local document.
    pub document_id: String,
    /// Exact canonical revision used during preparation.
    pub base_revision: i64,
    /// Current explicit lifecycle.
    pub status: ProposalStatus,
    /// Author/provider explanation; not a command or instruction.
    pub reason: String,
    /// Canonical revision created by explicit Apply, if any.
    pub applied_revision: Option<i64>,
    /// Compensating revision created by explicit Undo Apply, if any.
    pub undone_revision: Option<i64>,
}

/// Selected review data. Deserializing these fields grants no Apply authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RichProposal {
    /// Content-light record metadata.
    pub summary: RichProposalSummary,
    /// Complete unchanged base, including unknown attributes and provenance.
    pub before: Value,
    /// Prepared structured result, never promoted simply by loading it.
    pub after: Value,
    /// Typed sequential transaction, replayed and validated again on Apply.
    pub command: Transaction,
}

fn invalid(message: &str) -> StudioError {
    StudioError::Validation(message.into())
}
fn model(error: craft_editor_model::Error) -> StudioError {
    StudioError::Validation(format!("Structured proposal command failed: {error}"))
}
fn scoped_document(connection: &Connection, project: &str, id: &str) -> Result<Document> {
    let document = read_document(connection, id)?;
    if document.project_id != project {
        return Err(invalid(
            "The proposal document does not belong to this project",
        ));
    }
    if document.read_only {
        return Err(StudioError::ReadOnly {
            document_id: document.id,
        });
    }
    if document.rich_json.is_none() {
        return Err(invalid(
            "Structured editing is not enabled for this document",
        ));
    }
    Ok(document)
}
fn no_working_draft(connection: &Connection, document: &Document) -> Result<()> {
    let pending: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM rich_working_drafts WHERE document_id=?1) OR EXISTS(SELECT 1 FROM working_drafts WHERE document_id=?1)",
        [&document.id], |row| row.get(0),
    )?;
    if pending {
        return Err(invalid(
            "Save or preserve the working draft before applying or undoing a proposal",
        ));
    }
    Ok(())
}
fn resources(connection: &Connection, project: &str, tree: &RichTree) -> Result<()> {
    let mut stack = vec![tree.root()];
    while let Some(node) = stack.pop() {
        if node.node_type == "table" && TableGrid::new(node).map_err(model)?.columns > 63 {
            return Err(invalid(
                "This proposal exceeds the native limit of 63 table columns",
            ));
        }
        if node.node_type == "imageBlock" {
            let asset = node
                .attrs
                .get("assetId")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("A proposal image requires a project-owned asset"))?;
            let available: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM project_assets a WHERE a.project_id=?1 AND a.id=?2 AND a.media_type IN ('image/png','image/jpeg','image/gif','image/webp')) OR EXISTS(SELECT 1 FROM project_resource_aliases x JOIN project_assets a ON a.id=x.native_id AND a.project_id=x.project_id WHERE x.project_id=?1 AND x.resource_kind='media' AND x.logical_id=?2 AND a.media_type IN ('image/png','image/jpeg','image/gif','image/webp'))",
                params![project, asset], |row| row.get(0),
            )?;
            if !available {
                return Err(invalid(
                    "A proposal image is missing or belongs to another project",
                ));
            }
        }
        stack.extend(node.children());
    }
    Ok(())
}
fn prepare(connection: &Connection, current: &Document, command: &Transaction) -> Result<Change> {
    let revision = i64::try_from(command.base_revision.0)
        .map_err(|_| invalid("Proposal revision is out of range"))?;
    require_revision(current, revision)?;
    if command.commands.is_empty()
        || command.commands.len() > 1_000
        || serde_json::to_vec(command)?.len() > super::MAX_BODY_BYTES
    {
        return Err(invalid(
            "A structured proposal requires 1–1000 commands within 16 MiB",
        ));
    }
    let tree = RichTree::from_json(
        current
            .rich_json
            .clone()
            .ok_or_else(|| invalid("Structured proposal base is unavailable"))?,
    )
    .map_err(model)?;
    resources(connection, &current.project_id, &tree)?;
    let mut editor = Editor::new(tree, command.base_revision);
    let change = editor.apply(command.clone()).map_err(model)?;
    super::rich::validate_tree(change.after.to_json().map_err(model)?)?;
    resources(connection, &current.project_id, &change.after)?;
    if change.before == change.after {
        return Err(invalid("A structured proposal must change the document"));
    }
    Ok(change)
}
fn row_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<(RichProposalSummary, String)> {
    let status: String = row.get(4)?;
    Ok((
        RichProposalSummary {
            id: row.get(0)?,
            cursor: row.get(1)?,
            document_id: row.get(2)?,
            base_revision: row.get(3)?,
            status: ProposalStatus::Staged,
            reason: row.get(5)?,
            applied_revision: row.get(6)?,
            undone_revision: row.get(7)?,
        },
        status,
    ))
}
fn read_proposal(connection: &Connection, project: &str, id: &str) -> Result<RichProposal> {
    let row = connection.query_row(
        "SELECT p.id,p.rowid,p.document_id,p.base_revision,p.status,p.reason,p.applied_revision,p.undone_revision,p.before_json,p.after_json,p.command_json FROM rich_proposals p JOIN documents d ON d.id=p.document_id WHERE p.id=?1 AND d.project_id=?2",
        params![id, project], |row| {
            let (summary,status)=row_summary(row)?;
            Ok((summary,status,row.get::<_,String>(8)?,row.get::<_,String>(9)?,row.get::<_,String>(10)?))
        },
    ).optional()?.ok_or_else(|| StudioError::NotFound { kind:"rich proposal", id:id.into() })?;
    let (mut summary, status, before, after, command) = row;
    if [&before, &after, &command]
        .iter()
        .any(|value| value.len() > super::MAX_BODY_BYTES)
    {
        return Err(StudioError::CorruptData(
            "Structured proposal exceeds its stored size bound".into(),
        ));
    }
    summary.status = ProposalStatus::from_str(&status)?;
    Ok(RichProposal {
        summary,
        before: serde_json::from_str(&before)?,
        after: serde_json::from_str(&after)?,
        command: serde_json::from_str(&command)?,
    })
}
fn require_status(proposal: &RichProposal, expected: ProposalStatus) -> Result<()> {
    if proposal.summary.status != expected {
        return Err(StudioError::InvalidProposalState {
            expected,
            actual: proposal.summary.status,
        });
    }
    Ok(())
}
fn verified_base(connection: &Connection, proposal: &RichProposal) -> Result<()> {
    let original: Option<String> = connection
        .query_row(
            "SELECT content_json FROM rich_document_revisions WHERE document_id=?1 AND revision=?2",
            params![proposal.summary.document_id, proposal.summary.base_revision],
            |row| row.get(0),
        )
        .optional()?;
    let matches = original
        .as_deref()
        .map(serde_json::from_str::<Value>)
        .transpose()?
        .as_ref()
        == Some(&proposal.before);
    if !matches {
        return Err(StudioError::CorruptData(
            "Structured proposal base differs from its immutable history".into(),
        ));
    }
    Ok(())
}

impl Workspace {
    /// Stage a validated detached transaction; canonical content/notes/history stay unchanged.
    pub fn stage_rich_proposal(
        &self,
        project: &str,
        document: &str,
        command: Transaction,
        reason: &str,
    ) -> Result<RichProposal> {
        if reason.trim().is_empty() || reason.chars().count() > 4_000 {
            return Err(invalid("Proposal reason must contain 1–4000 characters"));
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = scoped_document(&transaction, project, document)?;
        no_working_draft(&transaction, &current)?;
        let change = prepare(&transaction, &current, &command)?;
        let id = uuid::Uuid::new_v4().to_string();
        let before = change.before.to_json().map_err(model)?;
        let after = change.after.to_json().map_err(model)?;
        transaction.execute("INSERT INTO rich_proposals(id,document_id,base_revision,status,before_json,after_json,command_json,reason) VALUES(?1,?2,?3,'staged',?4,?5,?6,?7)",params![id,document,current.revision,serde_json::to_string(&before)?,serde_json::to_string(&after)?,serde_json::to_string(&command)?,reason.trim()])?;
        let result = read_proposal(&transaction, project, &id)?;
        transaction.commit()?;
        Ok(result)
    }
    /// Page newest-first metadata without loading canonical before/after trees.
    pub fn rich_proposals(
        &self,
        project: &str,
        document: &str,
        before_cursor: Option<i64>,
        limit: usize,
    ) -> Result<Vec<RichProposalSummary>> {
        if !(1..=128).contains(&limit) {
            return Err(invalid("Proposal page size must be 1–128"));
        }
        let connection = self.connection()?;
        scoped_document(&connection, project, document)?;
        let mut statement=connection.prepare("SELECT id,rowid,document_id,base_revision,status,reason,applied_revision,undone_revision FROM rich_proposals WHERE document_id=?1 AND (?2 IS NULL OR rowid<?2) ORDER BY rowid DESC LIMIT ?3")?;
        let rows =
            statement.query_map(params![document, before_cursor, limit as i64], row_summary)?;
        rows.map(|row| {
            let (mut summary, status) = row?;
            summary.status = ProposalStatus::from_str(&status)?;
            Ok(summary)
        })
        .collect()
    }
    /// Load only a selected proposal inside the caller's fixed project capability.
    pub fn rich_proposal(&self, project: &str, id: &str) -> Result<RichProposal> {
        read_proposal(&self.connection()?, project, id)
    }
    /// Replay and atomically publish a current proposal with exact note position maps.
    pub fn apply_rich_proposal(&self, project: &str, id: &str) -> Result<Document> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let proposal = read_proposal(&transaction, project, id)?;
        require_status(&proposal, ProposalStatus::Staged)?;
        let current = scoped_document(&transaction, project, &proposal.summary.document_id)?;
        no_working_draft(&transaction, &current)?;
        if current.revision != proposal.summary.base_revision
            || current.rich_json.as_ref() != Some(&proposal.before)
        {
            return Err(StudioError::StaleProposal {
                expected: proposal.summary.base_revision,
                actual: current.revision,
            });
        }
        verified_base(&transaction, &proposal)?;
        let change = prepare(&transaction, &current, &proposal.command)?;
        if change.after.to_json().map_err(model)? != proposal.after {
            return Err(StudioError::CorruptData(
                "Structured proposal does not match its replayed commands".into(),
            ));
        }
        let (_, body, encoded) = super::rich::validate_tree(proposal.after)?;
        let saved = super::write_revision(&transaction, &current, &body, "proposal")?;
        super::rich::write_rich_snapshot(&transaction, &current.id, saved.revision, &encoded)?;
        let result = read_document(&transaction, &current.id)?;
        super::annotations::reconcile_annotations(
            &transaction,
            &current,
            &result,
            Some(&change.mappings),
            None,
        )?;
        transaction.execute(
            "UPDATE rich_proposals SET status='applied',applied_revision=?2 WHERE id=?1",
            params![id, result.revision],
        )?;
        transaction.commit()?;
        Ok(result)
    }
    /// Reject changes lifecycle only; manuscript, drafts, notes and history are untouched.
    pub fn reject_rich_proposal(&self, project: &str, id: &str) -> Result<()> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let proposal = read_proposal(&transaction, project, id)?;
        require_status(&proposal, ProposalStatus::Staged)?;
        transaction.execute(
            "UPDATE rich_proposals SET status='rejected' WHERE id=?1",
            [id],
        )?;
        transaction.commit()?;
        Ok(())
    }
    /// Append an exact compensating revision while retaining current author note metadata.
    pub fn undo_rich_proposal(&self, project: &str, id: &str) -> Result<Document> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let proposal = read_proposal(&transaction, project, id)?;
        require_status(&proposal, ProposalStatus::Applied)?;
        let current = scoped_document(&transaction, project, &proposal.summary.document_id)?;
        no_working_draft(&transaction, &current)?;
        let expected = proposal.summary.applied_revision.ok_or_else(|| {
            StudioError::CorruptData("Applied rich proposal has no applied revision".into())
        })?;
        if current.revision != expected || current.rich_json.as_ref() != Some(&proposal.after) {
            return Err(StudioError::UndoConflict {
                expected,
                actual: current.revision,
            });
        }
        verified_base(&transaction, &proposal)?;
        let original = scoped_document(&transaction, project, &current.id)?;
        let mut base = original;
        base.revision = proposal.summary.base_revision;
        base.rich_json = Some(proposal.before.clone());
        let prepared = prepare(&transaction, &base, &proposal.command)?;
        if prepared.after.to_json().map_err(model)? != proposal.after {
            return Err(StudioError::CorruptData(
                "Stored rich proposal result fails command verification".into(),
            ));
        }
        let (tree, body, encoded) = super::rich::validate_tree(proposal.before)?;
        resources(
            &transaction,
            project,
            &RichTree::from_json(tree).map_err(model)?,
        )?;
        let saved = super::write_revision(&transaction, &current, &body, "undo")?;
        super::rich::write_rich_snapshot(&transaction, &current.id, saved.revision, &encoded)?;
        let result = read_document(&transaction, &current.id)?;
        super::annotations::reconcile_annotations(
            &transaction,
            &current,
            &result,
            None,
            Some(proposal.summary.base_revision),
        )?;
        transaction.execute(
            "UPDATE rich_proposals SET status='undone',undone_revision=?2 WHERE id=?1",
            params![id, result.revision],
        )?;
        transaction.commit()?;
        Ok(result)
    }
}
