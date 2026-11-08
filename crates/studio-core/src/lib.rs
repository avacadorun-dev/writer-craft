//! Local project commands shared by the native interface and headless tools.
//!
//! Every canonical save creates a revision. Proposed replacements remain separate
//! until explicit apply, and undo refuses to overwrite intervening changes.

mod annotations;
mod assets;
mod daily_controls;
mod project_portability;
mod rich;
mod workspace_backup;
mod workspace_paths;
pub use annotations::{
    AnchorStatus, AnnotationAnchor, AnnotationEvent, AnnotationPatch, AnnotationPreviewSnapshot,
    AnnotationStatus, LegacyAnchorProjection, LegacyAnchorResolution, PassageAnnotation,
    RichRecoverySnapshot,
};
pub use assets::{
    ImageAsset, ImageAssetFailure, ImageAssetFailureKind, ImageAssetSnapshot, ImageTransfer,
    ImportedImage, MAX_IMAGE_BYTES, MAX_IMAGE_DIMENSION, ProjectImages,
    is_valid_image_reference_id, validate_image_transfers,
};
pub use daily_controls::{
    DocumentStatistics, FindMatch, document_statistics, find_first_document_match,
    find_first_text_match, rich_document_statistics,
};
pub use project_portability::{
    ImportAssetSource, ImportedProject, ImportedProjectAnnotation, MAX_PROJECT_MANIFEST_BYTES,
    MAX_PROJECT_TRANSFER_BYTES, ProjectAssetSource, ProjectAttachment, ProjectCanvasDraft,
    ProjectCanvasInfo, ProjectCanvasRevision, ProjectCanvasRevisionInfo, ProjectCanvasSnapshot,
    ProjectExportSnapshot, ProjectImportWorker, ProjectTransferCancellation, ProjectTreeNode,
    ValidatedProjectImport,
};
pub use rich::{RichDraft, RichRevision};
pub use workspace_backup::{WorkspaceBackup, backup_workspace};
pub use workspace_paths::is_workspace_data_path;

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

const SCHEMA_VERSION: i64 = 7;
const APPLICATION_ID: i64 = 0x5753544e;
const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

/// The result of a local workspace command.
pub type Result<T> = std::result::Result<T, StudioError>;

/// A recoverable validation, conflict, or persistence failure.
#[derive(Debug, Error)]
pub enum StudioError {
    /// Input violates a documented command constraint.
    #[error("{0}")]
    Validation(String),
    /// The requested local resource does not exist.
    #[error("{kind} {id} was not found; refresh the project list")]
    NotFound { kind: &'static str, id: String },
    /// The editor is based on an older revision.
    #[error("Document changed from revision {expected} to {actual}; reload before saving")]
    RevisionConflict { expected: i64, actual: i64 },
    /// Older recovery text must be preserved before starting a new draft base.
    #[error(
        "An unfinished draft from revision {draft_revision} must be recovered before editing revision {current_revision}"
    )]
    DraftConflict {
        draft_revision: i64,
        current_revision: i64,
    },
    /// Another draft writer acknowledged newer unsaved work.
    #[error(
        "The working draft changed in another edit session; preserve this draft before retrying"
    )]
    DraftChanged,
    /// A durable draft's verified attachment context no longer matches current notes.
    #[error("draft_annotation_conflict")]
    DraftAnnotationConflict,
    /// Imported structured content cannot safely be edited as plain text.
    #[error("Imported document {document_id} is read-only to preserve its original structure")]
    ReadOnly { document_id: String },
    /// The literal replacement target was absent.
    #[error("The exact text was not found; refresh the document and choose an existing phrase")]
    NoMatch,
    /// A selected range is outside the document's Unicode scalar bounds.
    #[error(
        "Selected range {start}..{end} is invalid for {length} characters; select the text again"
    )]
    InvalidTextRange {
        start: usize,
        end: usize,
        length: usize,
    },
    /// Selected range no longer contains its captured literal text.
    #[error("Selected text changed; refresh the selection before preparing a replacement")]
    ExpectedTextMismatch,
    /// Annotation metadata changed after the author opened it.
    #[error("Annotation changed from revision {expected} to {actual}; reload before saving")]
    AnnotationConflict { expected: i64, actual: i64 },
    /// A replacement target was not uniquely addressable.
    #[error("The exact text appears more than once; choose a longer unique phrase")]
    AmbiguousMatch,
    /// A proposal has already left the state required by the command.
    #[error("Proposal is {actual:?}; this action requires {expected:?}")]
    InvalidProposalState {
        expected: ProposalStatus,
        actual: ProposalStatus,
    },
    /// A proposal's canonical base changed before apply.
    #[error(
        "The proposal is based on revision {expected}, but the document is now {actual}; review a new proposal"
    )]
    StaleProposal { expected: i64, actual: i64 },
    /// A later edit prevents a lossless compensating undo.
    #[error("Undo would overwrite later work; expected revision {expected}, found {actual}")]
    UndoConflict { expected: i64, actual: i64 },
    /// A newer application created the database schema.
    #[error(
        "Workspace schema {found} is newer than supported schema {supported}; update the application"
    )]
    UnsupportedSchema { found: i64, supported: i64 },
    /// Opening another application's database must not modify it.
    #[error("This file is not a WriterCraft workspace; choose a workspace database")]
    NotWorkspaceDatabase,
    /// Stored state is internally inconsistent.
    #[error("Workspace data is inconsistent: {0}; restore a backup before editing")]
    CorruptData(String),
    /// A thread previously panicked while holding the command lock.
    #[error("The workspace command lock is unavailable; close and reopen the workspace")]
    LockUnavailable,
    /// SQLite could not complete the operation.
    #[error("Local database operation failed: {0}")]
    Database(#[from] rusqlite::Error),
    /// The workspace directory could not be accessed.
    #[error("Cannot access the workspace location: {0}")]
    Io(#[from] std::io::Error),
    /// Preserved imported data could not be decoded.
    #[error("Cannot decode preserved imported content: {0}")]
    Json(#[from] serde_json::Error),
}

/// A project in the local library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    /// Stable local project identifier.
    pub id: String,
    /// Author-provided project title.
    pub title: String,
}

/// Current canonical document and its optimistic revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// Stable local document identifier.
    pub id: String,
    /// Owning project identifier.
    pub project_id: String,
    /// Author-provided title.
    pub title: String,
    /// Canonical text, or a preview for a read-only imported document.
    pub body: String,
    /// Monotonically increasing text revision, beginning at one.
    pub revision: i64,
    /// True when plain text editing would destroy preserved imported data.
    pub read_only: bool,
    /// Opaque original data retained without flattening its structure.
    pub source_json: Option<Value>,
    /// Editable canonical structured content, separate from opaque imported source.
    /// The body remains a derived projection; plain commands must not rewrite it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rich_json: Option<Value>,
}

/// One immutable text revision, newest first when listed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Revision {
    /// Monotonically increasing document revision.
    pub revision: i64,
    /// Complete text at this revision.
    pub body: String,
    /// Command origin: manual, import, proposal, or undo.
    pub source: String,
}

/// A durable working draft pinned to the canonical revision it was edited from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Draft {
    /// Canonical revision used when the draft was captured.
    pub base_revision: i64,
    /// Unsaved working text, preserved separately from canonical history.
    pub body: String,
}

/// Explicit proposal lifecycle; terminal states cannot be applied again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    /// Canonical content has not changed.
    Staged,
    /// Explicit apply committed a canonical revision.
    Applied,
    /// Explicit reject discarded the proposal without changing the document.
    Rejected,
    /// A compensating revision restored the proposal's previous text.
    Undone,
}

impl ProposalStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Staged => "staged",
            Self::Applied => "applied",
            Self::Rejected => "rejected",
            Self::Undone => "undone",
        }
    }

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "staged" => Ok(Self::Staged),
            "applied" => Ok(Self::Applied),
            "rejected" => Ok(Self::Rejected),
            "undone" => Ok(Self::Undone),
            _ => Err(StudioError::CorruptData("unknown proposal state".into())),
        }
    }
}

/// A half-open range measured in Unicode scalar values, never UTF-8 bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextAnchor {
    /// First Unicode scalar in the exact target.
    pub start: usize,
    /// Unicode scalar immediately after the exact target.
    pub end: usize,
    /// Literal text verified against the base revision.
    pub expected_text: String,
}

/// A stored before/after proposal, separate from canonical content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    /// Stable proposal identifier.
    pub id: String,
    /// Canonical document targeted by the proposal.
    pub document_id: String,
    /// Exact document revision used to prepare the proposal.
    pub base_revision: i64,
    /// Complete canonical text before the proposed edit.
    pub before: String,
    /// Complete proposed text shown for review.
    pub after: String,
    /// Human-readable explanation of the replacement.
    pub reason: String,
    /// Current lifecycle state.
    pub status: ProposalStatus,
    /// Canonical revision produced by explicit apply, when applied.
    pub applied_revision: Option<i64>,
    /// Verified literal target in the base text.
    pub anchor: TextAnchor,
}

/// SQLite-backed command service independent of any GUI toolkit.
///
/// A workspace owns one connection protected by a command mutex. Opening the same
/// file through another instance remains safe through SQLite write transactions
/// and optimistic revision checks. Run synchronous commands off the render thread
/// when accessing large projects.
pub struct Workspace {
    connection: Arc<Mutex<Connection>>,
    replay_receipts: Mutex<annotations::ReplayCache>,
}

impl Workspace {
    /// Opens a SQLite database file, creating its parent directory if necessary.
    /// Existing non-workspace databases and newer schemas are rejected unchanged.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        workspace_paths::validate_database_path(path)?;
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        Self::from_connection(Connection::open(path)?)
    }

    /// Creates an isolated in-memory workspace, useful for tests and previews.
    pub fn memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(mut connection: Connection) -> Result<Self> {
        connection.busy_timeout(Duration::from_secs(5))?;
        // Schema identity and version are read under the same write lock as migration.
        // Another opener may already have completed migration while we waited.
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let application_id: i64 =
            transaction.pragma_query_value(None, "application_id", |row| row.get(0))?;
        if application_id != 0 && application_id != APPLICATION_ID {
            return Err(StudioError::NotWorkspaceDatabase);
        }
        let version: i64 =
            transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > SCHEMA_VERSION {
            return Err(StudioError::UnsupportedSchema {
                found: version,
                supported: SCHEMA_VERSION,
            });
        }
        if version == 0 {
            let user_tables: i64 = transaction.query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'", [], |row| row.get(0),
            )?;
            if user_tables != 0 {
                return Err(StudioError::NotWorkspaceDatabase);
            }
            transaction.execute_batch(include_str!("schema.sql"))?;
            transaction.execute_batch(include_str!("drafts.sql"))?;
            transaction.execute_batch(include_str!("rich.sql"))?;
            transaction.execute_batch(include_str!("annotations.sql"))?;
            transaction.execute_batch(include_str!("assets.sql"))?;
            transaction.execute_batch(include_str!("rich_annotation_drafts.sql"))?;
            transaction.execute_batch(include_str!("project_portability.sql"))?;
            transaction.pragma_update(None, "application_id", APPLICATION_ID)?;
            transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        } else if application_id != APPLICATION_ID {
            return Err(StudioError::NotWorkspaceDatabase);
        } else if version < SCHEMA_VERSION {
            if version < 2 {
                transaction.execute_batch(include_str!("drafts.sql"))?;
            }
            if version < 3 {
                transaction.execute_batch(include_str!("rich.sql"))?;
            }
            if version < 4 {
                transaction.execute_batch(include_str!("annotations.sql"))?;
            }
            if version < 5 {
                transaction.execute_batch(include_str!("assets.sql"))?;
            }
            if version < 6 {
                transaction.execute_batch(include_str!("rich_annotation_drafts.sql"))?;
            }
            if version < 7 {
                transaction.execute_batch(include_str!("project_portability.sql"))?;
            }
            transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }
        transaction.commit()?;
        connection.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;",
        )?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            replay_receipts: Mutex::new(annotations::ReplayCache::default()),
        })
    }

    fn connection(&self) -> Result<MutexGuard<'_, Connection>> {
        self.connection
            .lock()
            .map_err(|_| StudioError::LockUnavailable)
    }

    /// Lists the local projects in creation order.
    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare("SELECT id,title FROM projects ORDER BY rowid")?;
        let projects = statement
            .query_map([], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    title: row.get(1)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(projects)
    }

    /// Creates a project after validating its nonempty title.
    pub fn create_project(&self, title: &str) -> Result<Project> {
        let title = validate_title(title, 120)?;
        let project = Project {
            id: Uuid::new_v4().to_string(),
            title: title.into(),
        };
        self.connection()?.execute(
            "INSERT INTO projects(id,title) VALUES(?1,?2)",
            params![project.id, project.title],
        )?;
        Ok(project)
    }

    /// Lists current documents in a project, preserving creation order.
    pub fn list_documents(&self, project_id: &str) -> Result<Vec<Document>> {
        let connection = self.connection()?;
        require_project(&connection, project_id)?;
        let mut statement =
            connection.prepare("SELECT id FROM documents WHERE project_id=?1 ORDER BY rowid")?;
        let ids = statement
            .query_map([project_id], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ids.iter()
            .map(|id| read_document(&connection, id))
            .collect()
    }

    /// Creates an editable plain text document and its first revision atomically.
    pub fn create_document(&self, project_id: &str, title: &str, body: &str) -> Result<Document> {
        self.insert_document(project_id, title, body, None)
    }

    /// Preserves opaque imported data with a read-only text preview.
    /// No imported rich JSON can be overwritten by plain text save or proposal.
    pub fn import_document(
        &self,
        project_id: &str,
        title: &str,
        body: &str,
        source_json: Value,
    ) -> Result<Document> {
        self.insert_document(project_id, title, body, Some(source_json))
    }

    fn insert_document(
        &self,
        project_id: &str,
        title: &str,
        body: &str,
        source_json: Option<Value>,
    ) -> Result<Document> {
        let title = validate_title(title, 200)?;
        validate_body(body)?;
        let read_only = source_json.is_some();
        let serialized = source_json
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let id = Uuid::new_v4().to_string();
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_project(&transaction, project_id)?;
        transaction.execute(
            "INSERT INTO documents(id,project_id,title,body,revision,read_only,source_json) VALUES(?1,?2,?3,?4,1,?5,?6)",
            params![id, project_id, title, body, read_only, serialized],
        )?;
        append_revision(
            &transaction,
            &id,
            1,
            body,
            if read_only { "import" } else { "manual" },
        )?;
        let document = read_document(&transaction, &id)?;
        transaction.commit()?;
        Ok(document)
    }

    /// Reads a current document, returning NotFound for an unknown identifier.
    pub fn document(&self, id: &str) -> Result<Document> {
        let connection = self.connection()?;
        read_document(&connection, id)
    }

    /// Durably saves working text without changing canonical content or history.
    /// A stale base revision leaves any newer stored draft unchanged.
    pub fn save_draft(&self, id: &str, expected_revision: i64, body: &str) -> Result<()> {
        validate_body(body)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let document = read_document(&transaction, id)?;
        require_editable(&document)?;
        require_revision(&document, expected_revision)?;
        if let Some(existing) = read_draft(&transaction, id)?
            && existing.base_revision != expected_revision
            && existing.body != body
        {
            return Err(StudioError::DraftConflict {
                draft_revision: existing.base_revision,
                current_revision: expected_revision,
            });
        }
        transaction.execute(
            "INSERT INTO working_drafts(document_id,base_revision,body) VALUES(?1,?2,?3) ON CONFLICT(document_id) DO UPDATE SET base_revision=excluded.base_revision,body=excluded.body",
            params![id, expected_revision, body],
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Reads crash-recovery working text without promoting it to canonical content.
    /// Callers compare its base revision to the current document before recovery.
    pub fn draft(&self, id: &str) -> Result<Option<Draft>> {
        let connection = self.connection()?;
        read_document(&connection, id)?;
        read_draft(&connection, id)
    }

    /// Preserves a working draft in a new editable document before consuming it.
    /// The original document and all its revisions remain unchanged. A missing
    /// draft returns NotFound and creates no document.
    pub fn recover_draft_as_document(&self, id: &str) -> Result<Document> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let original = read_document(&transaction, id)?;
        let draft = read_draft(&transaction, id)?.ok_or_else(|| StudioError::NotFound {
            kind: "Working draft",
            id: id.into(),
        })?;
        validate_body(&draft.body)?;
        let suffix = " · восстановленный черновик";
        let title_prefix: String = original
            .title
            .chars()
            .take(200 - suffix.chars().count())
            .collect();
        let title = format!("{title_prefix}{suffix}");
        let recovered_id = Uuid::new_v4().to_string();
        transaction.execute(
            "INSERT INTO documents(id,project_id,title,body,revision,read_only,source_json) VALUES(?1,?2,?3,?4,1,0,NULL)",
            params![recovered_id, original.project_id, title, draft.body],
        )?;
        append_revision(&transaction, &recovered_id, 1, &draft.body, "manual")?;
        transaction.execute("DELETE FROM working_drafts WHERE document_id=?1", [id])?;
        let recovered = read_document(&transaction, &recovered_id)?;
        transaction.commit()?;
        Ok(recovered)
    }

    /// Saves text only when the expected revision is current.
    /// An unchanged body is a no-op and does not generate a spurious revision.
    pub fn save_document(&self, id: &str, expected_revision: i64, body: &str) -> Result<Document> {
        validate_body(body)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let document = read_document(&transaction, id)?;
        require_editable(&document)?;
        require_revision(&document, expected_revision)?;
        let result = if document.body == body {
            document
        } else {
            write_revision(&transaction, &document, body, "manual")?
        };
        clear_matching_draft(&transaction, id, expected_revision, body)?;
        transaction.commit()?;
        Ok(result)
    }

    /// Lists immutable text history, newest revision first.
    pub fn revisions(&self, document_id: &str) -> Result<Vec<Revision>> {
        let connection = self.connection()?;
        read_document(&connection, document_id)?;
        let mut statement = connection.prepare("SELECT revision,body,source FROM document_revisions WHERE document_id=?1 ORDER BY revision DESC")?;
        let revisions = statement
            .query_map([document_id], |row| {
                Ok(Revision {
                    revision: row.get(0)?,
                    body: row.get(1)?,
                    source: row.get(2)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(revisions)
    }

    /// Stages one uniquely located exact replacement without changing the document.
    /// Use stage_replace_range when a captured selection disambiguates repeated text.
    pub fn stage_replace(
        &self,
        document_id: &str,
        expected_revision: i64,
        expected_text: &str,
        replacement: &str,
        reason: &str,
    ) -> Result<Proposal> {
        validate_replacement(expected_text, replacement, reason)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let document = read_document(&transaction, document_id)?;
        require_editable(&document)?;
        require_revision(&document, expected_revision)?;
        let mut matches = document
            .body
            .char_indices()
            .filter(|(position, _)| document.body[*position..].starts_with(expected_text))
            .map(|(position, _)| position);
        let start_byte = matches.next().ok_or(StudioError::NoMatch)?;
        if matches.next().is_some() {
            return Err(StudioError::AmbiguousMatch);
        }
        let start = document.body[..start_byte].chars().count();
        let anchor = TextAnchor {
            start,
            end: start + expected_text.chars().count(),
            expected_text: expected_text.into(),
        };
        let proposal = store_replacement(&transaction, document, &anchor, replacement, reason)?;
        transaction.commit()?;
        Ok(proposal)
    }

    /// Stages exactly the captured Unicode scalar range, including repeated text.
    /// The literal target, current revision and editable state are checked inside
    /// the same transaction that persists the proposal and its snapshot hashes.
    pub fn stage_replace_range(
        &self,
        document_id: &str,
        expected_revision: i64,
        anchor: &TextAnchor,
        replacement: &str,
        reason: &str,
    ) -> Result<Proposal> {
        validate_replacement(&anchor.expected_text, replacement, reason)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let document = read_document(&transaction, document_id)?;
        require_editable(&document)?;
        require_revision(&document, expected_revision)?;
        let proposal = store_replacement(&transaction, document, anchor, replacement, reason)?;
        transaction.commit()?;
        Ok(proposal)
    }

    /// Lists stored proposals for a document, newest first.
    pub fn proposals(&self, document_id: &str) -> Result<Vec<Proposal>> {
        let connection = self.connection()?;
        read_document(&connection, document_id)?;
        let mut statement = connection
            .prepare("SELECT id FROM proposals WHERE document_id=?1 ORDER BY rowid DESC")?;
        let ids = statement
            .query_map([document_id], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ids.iter()
            .map(|id| read_proposal(&connection, id))
            .collect()
    }

    /// Atomically applies a staged proposal when its exact base still matches.
    /// A stale proposal returns an error and leaves canonical content unchanged.
    pub fn apply_proposal(&self, id: &str) -> Result<Document> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let proposal = read_proposal(&transaction, id)?;
        require_status(&proposal, ProposalStatus::Staged)?;
        let document = read_document(&transaction, &proposal.document_id)?;
        require_editable(&document)?;
        if document.revision != proposal.base_revision || document.body != proposal.before {
            return Err(StudioError::StaleProposal {
                expected: proposal.base_revision,
                actual: document.revision,
            });
        }
        let result = write_revision(&transaction, &document, &proposal.after, "proposal")?;
        clear_matching_draft(
            &transaction,
            &document.id,
            document.revision,
            &document.body,
        )?;
        clear_matching_draft(
            &transaction,
            &document.id,
            document.revision,
            &proposal.after,
        )?;
        transaction.execute(
            "UPDATE proposals SET status='applied',applied_revision=?2 WHERE id=?1",
            params![id, result.revision],
        )?;
        transaction.commit()?;
        Ok(result)
    }

    /// Rejects a staged proposal without changing canonical content or history.
    pub fn reject_proposal(&self, id: &str) -> Result<()> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let proposal = read_proposal(&transaction, id)?;
        require_status(&proposal, ProposalStatus::Staged)?;
        transaction.execute("UPDATE proposals SET status='rejected' WHERE id=?1", [id])?;
        transaction.commit()?;
        Ok(())
    }

    /// Appends a compensating revision only if no intervening edit occurred.
    pub fn undo_proposal(&self, id: &str) -> Result<Document> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let proposal = read_proposal(&transaction, id)?;
        require_status(&proposal, ProposalStatus::Applied)?;
        let expected = proposal
            .applied_revision
            .ok_or_else(|| StudioError::CorruptData("applied proposal has no revision".into()))?;
        let document = read_document(&transaction, &proposal.document_id)?;
        require_editable(&document)?;
        if document.revision != expected || document.body != proposal.after {
            return Err(StudioError::UndoConflict {
                expected,
                actual: document.revision,
            });
        }
        let result = write_revision(&transaction, &document, &proposal.before, "undo")?;
        transaction.execute(
            "UPDATE proposals SET status='undone',undone_revision=?2 WHERE id=?1",
            params![id, result.revision],
        )?;
        transaction.commit()?;
        Ok(result)
    }
}

fn validate_replacement(expected_text: &str, replacement: &str, reason: &str) -> Result<()> {
    if expected_text.is_empty() {
        return Err(StudioError::Validation(
            "Choose a nonempty exact phrase to replace".into(),
        ));
    }
    if expected_text == replacement {
        return Err(StudioError::Validation(
            "Replacement must change the selected text".into(),
        ));
    }
    validate_body(replacement)?;
    if reason.trim().is_empty() || reason.chars().count() > 4_000 {
        return Err(StudioError::Validation(
            "Proposal reason must contain 1–4000 characters".into(),
        ));
    }
    Ok(())
}

fn store_replacement(
    connection: &Connection,
    document: Document,
    anchor: &TextAnchor,
    replacement: &str,
    reason: &str,
) -> Result<Proposal> {
    let length = document.body.chars().count();
    let invalid_range = || StudioError::InvalidTextRange {
        start: anchor.start,
        end: anchor.end,
        length,
    };
    if anchor.start >= anchor.end || anchor.end > length {
        return Err(invalid_range());
    }
    let scalar_boundary = |position| {
        document
            .body
            .char_indices()
            .map(|(byte, _)| byte)
            .chain(std::iter::once(document.body.len()))
            .nth(position)
    };
    let start_byte = scalar_boundary(anchor.start).ok_or_else(invalid_range)?;
    let end_byte = scalar_boundary(anchor.end).ok_or_else(invalid_range)?;
    if document.body[start_byte..end_byte] != anchor.expected_text {
        return Err(StudioError::ExpectedTextMismatch);
    }
    let mut after = String::with_capacity(document.body.len().saturating_add(replacement.len()));
    after.push_str(&document.body[..start_byte]);
    after.push_str(replacement);
    after.push_str(&document.body[end_byte..]);
    validate_body(&after)?;
    let proposal = Proposal {
        id: Uuid::new_v4().to_string(),
        document_id: document.id,
        base_revision: document.revision,
        before: document.body,
        after,
        reason: reason.trim().into(),
        status: ProposalStatus::Staged,
        applied_revision: None,
        anchor: anchor.clone(),
    };
    connection.execute(
        "INSERT INTO proposals(id,document_id,base_revision,before_body,after_body,reason,status,anchor_json,before_hash,after_hash) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![proposal.id, proposal.document_id, proposal.base_revision, proposal.before, proposal.after,
            proposal.reason, proposal.status.as_str(), serde_json::to_string(&proposal.anchor)?,
            digest(&proposal.before), digest(&proposal.after)],
    )?;
    Ok(proposal)
}

fn validate_title(title: &str, limit: usize) -> Result<&str> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > limit {
        return Err(StudioError::Validation(format!(
            "Title must contain 1–{limit} characters"
        )));
    }
    Ok(title)
}

fn validate_body(body: &str) -> Result<()> {
    if body.len() > MAX_BODY_BYTES {
        return Err(StudioError::Validation(
            "Document text exceeds the 16 MiB limit".into(),
        ));
    }
    Ok(())
}

fn require_project(connection: &Connection, id: &str) -> Result<()> {
    if !connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM projects WHERE id=?1)",
        [id],
        |row| row.get::<_, bool>(0),
    )? {
        return Err(StudioError::NotFound {
            kind: "Project",
            id: id.into(),
        });
    }
    Ok(())
}

fn require_editable(document: &Document) -> Result<()> {
    if document.rich_json.is_some() {
        return Err(StudioError::Validation(
            "This document requires structured editing; a plain-text write would discard formatting".into(),
        ));
    }
    if document.read_only {
        return Err(StudioError::ReadOnly {
            document_id: document.id.clone(),
        });
    }
    Ok(())
}

fn require_revision(document: &Document, expected: i64) -> Result<()> {
    if expected != document.revision {
        return Err(StudioError::RevisionConflict {
            expected,
            actual: document.revision,
        });
    }
    Ok(())
}

fn require_status(proposal: &Proposal, expected: ProposalStatus) -> Result<()> {
    if proposal.status != expected {
        return Err(StudioError::InvalidProposalState {
            expected,
            actual: proposal.status,
        });
    }
    Ok(())
}

fn read_draft(connection: &Connection, id: &str) -> Result<Option<Draft>> {
    Ok(connection
        .query_row(
            "SELECT base_revision,body FROM working_drafts WHERE document_id=?1",
            [id],
            |row| {
                Ok(Draft {
                    base_revision: row.get(0)?,
                    body: row.get(1)?,
                })
            },
        )
        .optional()?)
}

fn read_document(connection: &Connection, id: &str) -> Result<Document> {
    let row = connection.query_row(
        "SELECT d.id,d.project_id,d.title,d.body,d.revision,d.read_only,d.source_json,r.content_json FROM documents d LEFT JOIN rich_documents r ON d.id=r.document_id WHERE d.id=?1", [id],
        |row| Ok((Document {
            id: row.get(0)?, project_id: row.get(1)?, title: row.get(2)?, body: row.get(3)?,
            revision: row.get(4)?, read_only: row.get(5)?, source_json: None, rich_json: None,
        }, row.get::<_, Option<String>>(6)?, row.get::<_, Option<String>>(7)?)),
    ).optional()?.ok_or_else(|| StudioError::NotFound { kind: "Document", id: id.into() })?;
    let (mut document, source_json, rich_json) = row;
    document.source_json = source_json
        .as_deref()
        .map(serde_json::from_str)
        .transpose()?;
    document.rich_json = rich_json.as_deref().map(serde_json::from_str).transpose()?;
    Ok(document)
}

fn read_proposal(connection: &Connection, id: &str) -> Result<Proposal> {
    let row = connection.query_row(
        "SELECT document_id,base_revision,before_body,after_body,reason,status,applied_revision,anchor_json,before_hash,after_hash FROM proposals WHERE id=?1", [id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?,
            row.get::<_, String>(3)?, row.get::<_, String>(4)?, row.get::<_, String>(5)?,
            row.get::<_, Option<i64>>(6)?, row.get::<_, String>(7)?, row.get::<_, String>(8)?, row.get::<_, String>(9)?)),
    ).optional()?.ok_or_else(|| StudioError::NotFound { kind: "Proposal", id: id.into() })?;
    let (
        document_id,
        base_revision,
        before,
        after,
        reason,
        status,
        applied_revision,
        anchor_json,
        before_hash,
        after_hash,
    ) = row;
    if digest(&before) != before_hash || digest(&after) != after_hash {
        return Err(StudioError::CorruptData(
            "proposal snapshot checksum mismatch".into(),
        ));
    }
    Ok(Proposal {
        id: id.into(),
        document_id,
        base_revision,
        before,
        after,
        reason,
        status: ProposalStatus::from_str(&status)?,
        applied_revision,
        anchor: serde_json::from_str(&anchor_json)?,
    })
}

fn append_revision(
    connection: &Connection,
    id: &str,
    revision: i64,
    body: &str,
    source: &str,
) -> Result<()> {
    connection.execute(
        "INSERT INTO document_revisions(document_id,revision,body,source) VALUES(?1,?2,?3,?4)",
        params![id, revision, body, source],
    )?;
    Ok(())
}

fn write_revision(
    connection: &Connection,
    document: &Document,
    body: &str,
    source: &str,
) -> Result<Document> {
    let next = document
        .revision
        .checked_add(1)
        .ok_or_else(|| StudioError::CorruptData("document revision overflow".into()))?;
    connection.execute(
        "UPDATE documents SET body=?2,revision=?3 WHERE id=?1 AND revision=?4",
        params![document.id, body, next, document.revision],
    )?;
    append_revision(connection, &document.id, next, body, source)?;
    read_document(connection, &document.id)
}

fn clear_matching_draft(
    connection: &Connection,
    id: &str,
    base_revision: i64,
    body: &str,
) -> Result<()> {
    connection.execute(
        "DELETE FROM working_drafts WHERE document_id=?1 AND base_revision=?2 AND body=?3",
        params![id, base_revision, body],
    )?;
    Ok(())
}

fn digest(body: &str) -> String {
    format!("{:x}", Sha256::digest(body.as_bytes()))
}
