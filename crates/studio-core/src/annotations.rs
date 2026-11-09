//! Persistent author notes with revision-pinned, exact scalar attachments.
//!
//! The source permits one block per note; assistant multi-block selections are a
//! different contract. No command locates a note by searching for matching text.

mod legacy;
mod recovery;
pub use legacy::{LegacyAnchorProjection, LegacyAnchorResolution};
pub use recovery::RichRecoverySnapshot;

use std::collections::HashMap;

use craft_editor_model::{
    BlockTextProjection, Document as RichTree, HistoryReceipt, Node, PositionMap, ReplayAction,
    ReplayProof, ScalarOffset, TextPoint, scalar_to_byte,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};

use super::{Document, Result, StudioError, Workspace, read_document, require_revision};

/// Portable exact attachment within one stable canonical block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotationAnchor {
    /// Stable block identity, independent of its current tree path.
    #[serde(alias = "blockId")]
    pub block_id: String,
    /// Inclusive Unicode scalar boundary.
    pub start: ScalarOffset,
    /// Exclusive Unicode scalar boundary.
    pub end: ScalarOffset,
    /// Literal selected occurrence; never a substring-search instruction.
    #[serde(alias = "exactText")]
    pub exact_text: String,
    /// Up to 32 preceding scalars, retained even after detachment.
    pub prefix: String,
    /// Up to 32 following scalars, retained even after detachment.
    pub suffix: String,
}

impl AnnotationAnchor {
    /// Capture an exact native selection without converting parent offsets into leaf carets.
    /// A quoted passage may span paragraphs within one identified blockquote. Other
    /// selections stay within their identified text leaf, including table/list prose.
    /// This constructs metadata only; creation still checks the current durable revision.
    pub fn from_selection(tree: &RichTree, first: &TextPoint, last: &TextPoint) -> Result<Self> {
        fn quote_for<'a>(
            tree: &'a RichTree,
            node: &'a Node,
            a: &TextPoint,
            b: &TextPoint,
        ) -> Option<&'a str> {
            if node.node_type == "blockquote" {
                let id = node.block_id()?;
                let projection = BlockTextProjection::new(tree, id).ok()?;
                if projection.contains_leaf(&a.block_id) && projection.contains_leaf(&b.block_id) {
                    return Some(id);
                }
            }
            node.children()
                .iter()
                .find_map(|child| quote_for(tree, child, a, b))
        }
        let id = if let Some(id) = quote_for(tree, tree.root(), first, last) {
            id
        } else if first.block_id == last.block_id {
            first.block_id.as_str()
        } else {
            return Err(StudioError::Validation(
                "Select a passage within one text block or blockquote".into(),
            ));
        };
        let projection = BlockTextProjection::new(tree, id).map_err(|error| {
            StudioError::Validation(format!("Invalid annotation selection: {error}"))
        })?;
        let a = projection
            .offset(first)
            .map_err(|_| StudioError::ExpectedTextMismatch)?;
        let b = projection
            .offset(last)
            .map_err(|_| StudioError::ExpectedTextMismatch)?;
        anchor_at(tree, id, a.min(b), a.max(b))
    }

    /// Resolve this exact attached range into native leaf carets, never parent IDs.
    /// The full quote must match before a mark may be painted or activated.
    pub fn selection(&self, tree: &RichTree) -> Result<(TextPoint, TextPoint)> {
        verified_anchor(tree, self)?;
        let projection = BlockTextProjection::new(tree, &self.block_id).map_err(|error| {
            StudioError::Validation(format!("Invalid annotation projection: {error}"))
        })?;
        Ok((
            projection
                .point(self.start, true)
                .map_err(|_| StudioError::ExpectedTextMismatch)?,
            projection
                .point(self.end, false)
                .map_err(|_| StudioError::ExpectedTextMismatch)?,
        ))
    }
}

/// Note lifecycle, independent of whether its anchor remains valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationStatus {
    /// Current author work.
    Open,
    /// Completed work; may be reopened explicitly.
    Resolved,
    /// Source-compatible author attention state.
    NeedsAttention,
}

/// Current validity of the exact passage attachment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnchorStatus {
    /// Valid at the recorded canonical document revision.
    Attached,
    /// Quote/context are retained; explicit current selection is required.
    Detached,
}

/// Durable passage note, separate from the manuscript and reader exports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PassageAnnotation {
    /// Stable note identity.
    pub id: String,
    /// Owning canonical document.
    pub document_id: String,
    /// Optimistic metadata revision; advances for attachment changes as well.
    pub revision: i64,
    /// Canonical document revision last checked or mapped.
    pub base_revision: i64,
    /// Last exact attachment, retained when detached.
    pub anchor: AnnotationAnchor,
    /// Author-written note, never manuscript content.
    pub note: String,
    /// Author-defined, case-sensitive labels. Older records have no labels.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Independent author lifecycle.
    pub status: AnnotationStatus,
    /// Independent attachment validity.
    pub anchor_status: AnchorStatus,
    /// Explicit deletion is a tombstone so audit history remains readable.
    pub deleted: bool,
}

/// Revision-guarded metadata changes. Reattachment has a separate exact API.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotationPatch {
    /// Replace note text when provided.
    pub note: Option<String>,
    /// Resolve, reopen or mark attention when provided.
    pub status: Option<AnnotationStatus>,
}

/// Immutable note history, including compensating attachment restores.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotationEvent {
    /// Metadata revision recorded by this event.
    pub revision: i64,
    /// Canonical document revision at the event.
    pub document_revision: i64,
    /// Machine-readable operation provenance.
    pub source: String,
    /// Complete prior state; absent only on creation.
    pub before: Option<PassageAnnotation>,
    /// Complete resulting state, including deletion tombstones.
    pub after: PassageAnnotation,
}

/// A project-scoped markup result, without loading its manuscript body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectAnnotationSummary {
    /// Current manuscript title; identities, not this label, drive navigation.
    pub document_title: String,
    /// Current document revision captured with the metadata.
    pub document_revision: i64,
    /// Current author metadata, including tombstones when explicitly requested.
    pub annotation: PassageAnnotation,
}

/// Exact project-wide tag edit captured for explicit review before committing.
/// Private targets prevent a caller from forging or omitting an affected occurrence.
#[derive(Debug, Clone)]
pub struct ProjectTagSnapshot {
    project_id: String,
    tag: String,
    targets: Vec<ProjectAnnotationSummary>,
    catalog: crate::ProjectLabelCatalog,
}
impl ProjectTagSnapshot {
    /// Exact case-sensitive label being changed.
    pub fn tag(&self) -> &str {
        &self.tag
    }
    /// Number of active passage records that the operation will change.
    pub fn count(&self) -> usize {
        self.targets
            .iter()
            .filter(|target| !target.annotation.deleted)
            .count()
    }
    /// Deleted current records also renamed, while their previous history remains immutable.
    pub fn deleted_count(&self) -> usize {
        self.targets
            .iter()
            .filter(|target| target.annotation.deleted)
            .count()
    }
    /// Records that would become empty and therefore move to deleted markup.
    pub fn tag_only_count(&self) -> usize {
        self.targets
            .iter()
            .filter(|target| {
                !target.annotation.deleted
                    && target.annotation.tags.len() == 1
                    && target.annotation.note.is_empty()
            })
            .count()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AttachmentSnapshot {
    anchor: AnnotationAnchor,
    status: AnchorStatus,
}

#[derive(Clone)]
pub(super) struct CachedReceipt {
    document_id: String,
    attachments: HashMap<String, AttachmentSnapshot>,
    acknowledged_revision: Option<i64>,
}
pub(super) type ReplayCache = HashMap<HistoryReceipt, CachedReceipt>;

/// Immutable, core-owned attachment context captured from one consistent workspace state.
///
/// Private fields prevent callers from supplying arbitrary note snapshots or receipt maps.
/// This value is not a serialized authority token; refreshed metadata still uses live guards.
#[derive(Clone)]
pub struct AnnotationPreviewSnapshot {
    document: Document,
    notes: Vec<PassageAnnotation>,
    receipts: ReplayCache,
}
impl AnnotationPreviewSnapshot {
    /// Canonical document whose note metadata was captured.
    pub fn document_id(&self) -> &str {
        &self.document.id
    }
    /// Durable document revision used for the immutable base.
    pub fn base_revision(&self) -> i64 {
        self.document.revision
    }
    /// Stored metadata before pending input, including resolved and detached notes.
    pub fn stored_notes(&self) -> &[PassageAnnotation] {
        &self.notes
    }
    /// Pure in-memory replay of exact attachments for the supplied current rich state.
    ///
    /// No SQLite, filesystem, network or shared receipt-cache access occurs here.
    /// Metadata revision fields remain stored optimistic guards; flush native writing
    /// and refresh the context before committing note lifecycle/text changes.
    pub fn preview(
        &self,
        proofs: &[ReplayProof],
        expected_content: serde_json::Value,
    ) -> Result<Vec<PassageAnnotation>> {
        if proofs.len() > 1_000 {
            return Err(StudioError::Validation(
                "Native edit batch exceeds 1000 changes".into(),
            ));
        }
        let (expected_content, _, _) = super::rich::validate_tree(expected_content)?;
        let (notes, _) = preview_replay_annotations(
            &self.document,
            self.document.revision,
            proofs,
            &expected_content,
            self.notes.clone(),
            &self.receipts,
        )?;
        Ok(notes)
    }
}

impl Workspace {
    /// Persist one debounced native edit batch using sealed engine provenance.
    /// Forward commands are replayed against the guarded canonical base; public
    /// Change maps/snapshots are never accepted as authority. Undo/Redo restore
    /// attachment state only from the exact retained history receipt.
    ///
    /// Keep pending proofs until success. A recovered draft without its original
    /// proof journal must use explicit snapshot recovery and a new editor session.
    pub fn save_rich_editor_changes(
        &self,
        document_id: &str,
        expected_revision: i64,
        proofs: &[ReplayProof],
        expected_content: serde_json::Value,
    ) -> Result<Document> {
        if proofs.len() > 1_000 {
            return Err(StudioError::Validation(
                "Native edit batch exceeds 1000 changes".into(),
            ));
        }
        let (expected_content, body, encoded) = super::rich::validate_tree(expected_content)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = read_document(&transaction, document_id)?;
        require_revision(&current, expected_revision)?;
        let original_notes = list_annotations(&transaction, document_id)?;
        let mut receipts = self
            .replay_receipts
            .lock()
            .map_err(|_| StudioError::LockUnavailable)?;
        let (mut notes, mut candidate_receipts) = preview_replay_annotations(
            &current,
            expected_revision,
            proofs,
            &expected_content,
            original_notes.clone(),
            &receipts,
        )?;
        let changed = proofs.iter().any(|proof| proof.before() != proof.after());
        let result = if changed {
            let saved = super::write_revision(&transaction, &current, &body, "manual")?;
            super::rich::write_rich_snapshot(&transaction, document_id, saved.revision, &encoded)?;
            for (before, after) in original_notes.iter().zip(&mut notes) {
                save_attachment(&transaction, before, current.revision)?;
                after.revision = next_revision(before.revision)?;
                after.base_revision = saved.revision;
                let source = if proofs
                    .last()
                    .is_some_and(|proof| proof.action() != ReplayAction::Apply)
                {
                    "restore"
                } else {
                    "mapping"
                };
                store_annotation(&transaction, before, after, source)?;
                save_attachment(&transaction, after, saved.revision)?;
            }
            read_document(&transaction, document_id)?
        } else {
            current
        };
        transaction.execute("DELETE FROM rich_working_drafts WHERE document_id=?1 AND base_revision=?2 AND content_json=?3", params![document_id, expected_revision, encoded])?;
        transaction.commit()?;
        if let Some(last) = proofs.last()
            && let Some(entry) = candidate_receipts.get_mut(last.after())
        {
            entry.acknowledged_revision = Some(result.revision);
        }
        *receipts = candidate_receipts;
        Ok(result)
    }

    /// Capture a consistent, immutable note/receipt context before entering native paint.
    ///
    /// Refresh after canonical save, note metadata changes, document switch or an
    /// external-update notification. Future previews are pure in-memory checks.
    pub fn annotation_preview_snapshot(
        &self,
        document_id: &str,
        expected_revision: i64,
    ) -> Result<AnnotationPreviewSnapshot> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Deferred)?;
        let document = read_document(&transaction, document_id)?;
        require_revision(&document, expected_revision)?;
        rich_tree(&document)?;
        let notes = list_annotations(&transaction, document_id)?;
        let mut receipts = self
            .replay_receipts
            .lock()
            .map_err(|_| StudioError::LockUnavailable)?
            .clone();
        receipts.retain(|receipt, _| receipt.reference_count() > 2);
        transaction.commit()?;
        Ok(AnnotationPreviewSnapshot {
            document,
            notes,
            receipts,
        })
    }

    /// Convenience preview outside paint; native render sources should retain a snapshot.
    ///
    /// The snapshot and save paths share exact sealed replay, stale-history, document
    /// ownership and Undo/Redo checks. Nothing persists candidate receipts or notes.
    pub fn preview_rich_editor_annotations(
        &self,
        document_id: &str,
        expected_revision: i64,
        proofs: &[ReplayProof],
        expected_content: serde_json::Value,
    ) -> Result<Vec<PassageAnnotation>> {
        self.annotation_preview_snapshot(document_id, expected_revision)?
            .preview(proofs, expected_content)
    }

    /// Capture an exact nonempty selection at a current revision. Callers pass
    /// the selected literal too, preventing stale or mixed offset projections.
    pub fn capture_annotation_anchor(
        &self,
        document_id: &str,
        expected_revision: i64,
        block_id: &str,
        start: ScalarOffset,
        end: ScalarOffset,
        expected_text: &str,
    ) -> Result<AnnotationAnchor> {
        let connection = self.connection()?;
        let document = read_document(&connection, document_id)?;
        require_revision(&document, expected_revision)?;
        let anchor = anchor_at(&rich_tree(&document)?, block_id, start, end)?;
        if anchor.exact_text != expected_text {
            return Err(StudioError::ExpectedTextMismatch);
        }
        Ok(anchor)
    }

    /// Create an author note only after checking its exact current selection.
    pub fn create_annotation(
        &self,
        document_id: &str,
        expected_revision: i64,
        anchor: AnnotationAnchor,
        note: &str,
    ) -> Result<PassageAnnotation> {
        self.create_tagged_annotation(document_id, expected_revision, anchor, note, &[])
    }

    /// Create exact passage markup with a note, author labels, or both.
    pub fn create_tagged_annotation(
        &self,
        document_id: &str,
        expected_revision: i64,
        anchor: AnnotationAnchor,
        note: &str,
        tags: &[String],
    ) -> Result<PassageAnnotation> {
        self.create_markup(document_id, expected_revision, anchor, note, tags, None)
    }

    /// Create markup from an exact reviewed label picker; renamed/retired labels never reappear.
    pub fn create_tagged_annotation_with_labels(
        &self,
        document_id: &str,
        expected_revision: i64,
        anchor: AnnotationAnchor,
        note: &str,
        tags: &[String],
        labels: &crate::ProjectLabelCatalog,
    ) -> Result<PassageAnnotation> {
        self.create_markup(
            document_id,
            expected_revision,
            anchor,
            note,
            tags,
            Some(labels),
        )
    }

    fn create_markup(
        &self,
        document_id: &str,
        expected_revision: i64,
        anchor: AnnotationAnchor,
        note: &str,
        tags: &[String],
        labels: Option<&crate::ProjectLabelCatalog>,
    ) -> Result<PassageAnnotation> {
        validate_tags(tags)?;
        let note = normalize_markup_note(note, tags)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let document = read_document(&transaction, document_id)?;
        require_revision(&document, expected_revision)?;
        if let Some(labels) = labels {
            crate::project_labels::guard_assignments(
                &transaction,
                &document.project_id,
                labels,
                tags,
                false,
            )?;
        }
        let anchor = verified_anchor(&rich_tree(&document)?, &anchor)?;
        let result = PassageAnnotation {
            id: uuid::Uuid::new_v4().to_string(),
            document_id: document_id.into(),
            revision: 1,
            base_revision: expected_revision,
            anchor,
            note,
            tags: tags.to_vec(),
            status: AnnotationStatus::Open,
            anchor_status: AnchorStatus::Attached,
            deleted: false,
        };
        crate::project_labels::ensure_assignments(&transaction, &document.project_id, tags, false)?;
        transaction.execute("INSERT INTO document_annotations(id,document_id,revision,annotation_json) VALUES(?1,?2,1,?3)", params![result.id, document_id, serde_json::to_string(&result)?])?;
        append_event(&transaction, None, &result, "create")?;
        save_attachment(&transaction, &result, expected_revision)?;
        transaction.commit()?;
        Ok(result)
    }

    /// Read a current, nondeleted note by stable identity.
    pub fn annotation(&self, id: &str) -> Result<PassageAnnotation> {
        let connection = self.connection()?;
        read_annotation(&connection, id, false)
    }

    /// Read all active metadata, including resolved and detached notes.
    pub fn annotations(&self, document_id: &str) -> Result<Vec<PassageAnnotation>> {
        let connection = self.connection()?;
        read_document(&connection, document_id)?;
        list_annotations(&connection, document_id)
    }

    /// Read markup for exactly one project, optionally including deleted records.
    /// Results contain no full manuscript bodies and retain stable document/note identities.
    pub fn project_annotations(
        &self,
        project_id: &str,
        include_deleted: bool,
    ) -> Result<Vec<ProjectAnnotationSummary>> {
        let connection = self.connection()?;
        super::require_project(&connection, project_id)?;
        project_annotations(&connection, project_id, include_deleted)
    }

    /// Update authored text/lifecycle with both document and note guards.
    pub fn update_annotation(
        &self,
        id: &str,
        expected_document_revision: i64,
        expected_annotation_revision: i64,
        patch: AnnotationPatch,
    ) -> Result<PassageAnnotation> {
        self.update_annotation_markup(
            id,
            expected_document_revision,
            expected_annotation_revision,
            patch,
            None,
        )
    }

    /// Edit note text and/or labels atomically under the same exact metadata guards.
    pub fn update_annotation_markup(
        &self,
        id: &str,
        expected_document_revision: i64,
        expected_annotation_revision: i64,
        patch: AnnotationPatch,
        tags: Option<Vec<String>>,
    ) -> Result<PassageAnnotation> {
        self.update_markup(
            id,
            expected_document_revision,
            expected_annotation_revision,
            patch,
            tags,
            None,
        )
    }

    /// Save an authored picker draft only while its exact project label catalog remains current.
    pub fn update_annotation_markup_with_labels(
        &self,
        id: &str,
        expected_document_revision: i64,
        expected_annotation_revision: i64,
        patch: AnnotationPatch,
        tags: Option<Vec<String>>,
        labels: &crate::ProjectLabelCatalog,
    ) -> Result<PassageAnnotation> {
        self.update_markup(
            id,
            expected_document_revision,
            expected_annotation_revision,
            patch,
            tags,
            Some(labels),
        )
    }

    fn update_markup(
        &self,
        id: &str,
        expected_document_revision: i64,
        expected_annotation_revision: i64,
        patch: AnnotationPatch,
        tags: Option<Vec<String>>,
        labels: Option<&crate::ProjectLabelCatalog>,
    ) -> Result<PassageAnnotation> {
        if patch.note.is_none() && patch.status.is_none() && tags.is_none() {
            return Err(StudioError::Validation("Annotation update is empty".into()));
        }
        self.change_annotation(
            id,
            expected_document_revision,
            expected_annotation_revision,
            "update",
            labels,
            |_, annotation| {
                if let Some(tags) = tags {
                    validate_tags(&tags)?;
                    annotation.tags = tags;
                }
                if let Some(note) = patch.note {
                    annotation.note = normalize_markup_note(&note, &annotation.tags)?;
                }
                normalize_markup_note(&annotation.note, &annotation.tags)?;
                if let Some(status) = patch.status {
                    annotation.status = status;
                }
                Ok(())
            },
        )
    }

    /// Reattach only through an explicit, exact selection at the current revision.
    /// Reattachment does not implicitly reopen a resolved note.
    pub fn reattach_annotation(
        &self,
        id: &str,
        expected_document_revision: i64,
        expected_annotation_revision: i64,
        anchor: AnnotationAnchor,
    ) -> Result<PassageAnnotation> {
        self.change_annotation(
            id,
            expected_document_revision,
            expected_annotation_revision,
            "reattach",
            None,
            |document, annotation| {
                annotation.anchor = verified_anchor(&rich_tree(document)?, &anchor)?;
                annotation.anchor_status = AnchorStatus::Attached;
                Ok(())
            },
        )
    }

    /// Delete author metadata without deleting manuscript content or audit events.
    pub fn delete_annotation(
        &self,
        id: &str,
        expected_document_revision: i64,
        expected_annotation_revision: i64,
    ) -> Result<()> {
        self.change_annotation(
            id,
            expected_document_revision,
            expected_annotation_revision,
            "delete",
            None,
            |_, annotation| {
                annotation.deleted = true;
                Ok(())
            },
        )?;
        Ok(())
    }

    /// Remove one mark while retaining optional authored draft metadata in its tombstone.
    /// The draft and deletion commit together; unchanged assignments do not require a catalog.
    pub fn remove_annotation_with_draft(
        &self,
        id: &str,
        expected_document_revision: i64,
        expected_annotation_revision: i64,
        body: Option<&str>,
        tags: Option<&[String]>,
        labels: Option<&crate::ProjectLabelCatalog>,
    ) -> Result<PassageAnnotation> {
        self.remove_markup(
            id,
            expected_document_revision,
            expected_annotation_revision,
            None,
            body,
            tags,
            labels,
        )
    }

    /// Remove only one local assignment, saving its pending note draft in the same transaction.
    /// An emptied tag-only mark is tombstoned with its previous assignments for exact recovery.
    #[expect(
        clippy::too_many_arguments,
        reason = "Revision and catalog guards stay explicit at the shared command boundary"
    )]
    pub fn remove_annotation_label_with_draft(
        &self,
        id: &str,
        expected_document_revision: i64,
        expected_annotation_revision: i64,
        label: &str,
        body: Option<&str>,
        tags: Option<&[String]>,
        labels: &crate::ProjectLabelCatalog,
    ) -> Result<PassageAnnotation> {
        self.remove_markup(
            id,
            expected_document_revision,
            expected_annotation_revision,
            Some(label),
            body,
            tags,
            Some(labels),
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The shared transaction accepts both guarded local removal forms"
    )]
    fn remove_markup(
        &self,
        id: &str,
        expected_document_revision: i64,
        expected_annotation_revision: i64,
        label: Option<&str>,
        body: Option<&str>,
        tags: Option<&[String]>,
        labels: Option<&crate::ProjectLabelCatalog>,
    ) -> Result<PassageAnnotation> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let before = read_annotation(&transaction, id, false)?;
        let document = read_document(&transaction, &before.document_id)?;
        require_revision(&document, expected_document_revision)?;
        rich_tree(&document)?;
        if before.revision != expected_annotation_revision {
            return Err(StudioError::AnnotationConflict {
                expected: expected_annotation_revision,
                actual: before.revision,
            });
        }
        let mut after = before.clone();
        if let Some(tags) = tags {
            validate_tags(tags)?;
            after.tags = tags.to_vec();
        }
        if let Some(body) = body {
            if body.trim().chars().count() > crate::MAX_PROJECT_ANNOTATION_NOTE_CHARACTERS {
                return Err(StudioError::Validation(
                    "The pending note is too long; preserve its text before removing the mark"
                        .into(),
                ));
            }
            after.note = body.trim().into();
        }
        let changed_assignments = after.tags != before.tags || label.is_some();
        if changed_assignments {
            let labels = labels.ok_or_else(|| {
                StudioError::Validation(
                    "Review the project label picker before removing changed assignments".into(),
                )
            })?;
            crate::project_labels::guard_assignments(
                &transaction,
                &document.project_id,
                labels,
                &after.tags,
                false,
            )?;
        }
        if let Some(label) = label {
            if !after.tags.iter().any(|name| name == label) {
                return Err(StudioError::Validation(
                    "This label is no longer assigned to the mark; refresh its note".into(),
                ));
            }
            let prior_tags = after.tags.clone();
            after.tags.retain(|name| name != label);
            if after.tags.is_empty() && after.note.trim().is_empty() {
                after.deleted = true;
                after.tags = prior_tags;
            }
        } else {
            after.deleted = true;
        }
        if !after.deleted {
            normalize_markup_note(&after.note, &after.tags)?;
        }
        if changed_assignments {
            // Materialize compatible legacy definitions before their final active usage
            // disappears; this never retires a project definition or rewrites other notes.
            crate::project_labels::ensure_assignments(
                &transaction,
                &document.project_id,
                &after.tags,
                false,
            )?;
        }
        if before != after {
            after.revision = next_revision(before.revision)?;
            after.base_revision = document.revision;
            store_annotation(
                &transaction,
                &before,
                &after,
                if after.deleted { "delete" } else { "update" },
            )?;
            save_attachment(&transaction, &after, document.revision)?;
        }
        transaction.commit()?;
        Ok(after)
    }

    /// Restore a deleted mark by identity. Changed manuscript content keeps its old quote
    /// detached until the author explicitly selects a current range for reattachment.
    pub fn restore_annotation(
        &self,
        id: &str,
        expected_document_revision: i64,
        expected_annotation_revision: i64,
    ) -> Result<PassageAnnotation> {
        self.restore_markup(
            id,
            expected_document_revision,
            expected_annotation_revision,
            None,
        )
    }

    /// Restore after disclosing any retained labels/colors under the same catalog snapshot.
    pub fn restore_annotation_with_labels(
        &self,
        id: &str,
        expected_document_revision: i64,
        expected_annotation_revision: i64,
        labels: &crate::ProjectLabelCatalog,
    ) -> Result<PassageAnnotation> {
        self.restore_markup(
            id,
            expected_document_revision,
            expected_annotation_revision,
            Some(labels),
        )
    }

    fn restore_markup(
        &self,
        id: &str,
        expected_document_revision: i64,
        expected_annotation_revision: i64,
        labels: Option<&crate::ProjectLabelCatalog>,
    ) -> Result<PassageAnnotation> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let before = read_annotation(&transaction, id, true)?;
        if !before.deleted {
            return Err(StudioError::Validation("This markup is not deleted".into()));
        }
        let document = read_document(&transaction, &before.document_id)?;
        require_revision(&document, expected_document_revision)?;
        if let Some(labels) = labels {
            crate::project_labels::guard_assignments(
                &transaction,
                &document.project_id,
                labels,
                &before.tags,
                true,
            )?;
        }
        let tree = rich_tree(&document)?;
        if before.revision != expected_annotation_revision {
            return Err(StudioError::AnnotationConflict {
                expected: expected_annotation_revision,
                actual: before.revision,
            });
        }
        let mut after = before.clone();
        after.deleted = false;
        if before.base_revision != document.revision
            || verified_anchor(&tree, &before.anchor).is_err()
        {
            after.anchor_status = AnchorStatus::Detached;
        }
        after.revision = next_revision(before.revision)?;
        after.base_revision = document.revision;
        crate::project_labels::ensure_assignments(
            &transaction,
            &document.project_id,
            &after.tags,
            true,
        )?;
        store_annotation(&transaction, &before, &after, "restore")?;
        save_attachment(&transaction, &after, document.revision)?;
        transaction.commit()?;
        Ok(after)
    }

    /// Capture every current occurrence of one exact label for a whole-project edit.
    pub fn project_tag_snapshot(&self, project_id: &str, tag: &str) -> Result<ProjectTagSnapshot> {
        validate_tags(&[tag.to_owned()])?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Deferred)?;
        super::require_project(&transaction, project_id)?;
        let catalog = crate::project_labels::read_catalog(&transaction, project_id)?;
        let targets = tag_targets(&transaction, project_id, tag)?;
        if !catalog.labels().iter().any(|label| label.name == tag) {
            return Err(StudioError::Validation(
                "This project label is not active; restore it before changing it".into(),
            ));
        }
        transaction.commit()?;
        Ok(ProjectTagSnapshot {
            project_id: project_id.into(),
            tag: tag.into(),
            targets,
            catalog,
        })
    }

    /// Rename one label across its exact reviewed set, without implicitly merging labels.
    pub fn rename_project_tag(
        &self,
        snapshot: &ProjectTagSnapshot,
        replacement: &str,
    ) -> Result<usize> {
        validate_tags(&[replacement.to_owned()])?;
        self.change_project_tag(snapshot, Some(replacement))
    }

    /// Remove one label across its exact reviewed set. Marks with no remaining note or label
    /// become recoverable tombstones; the caller must disclose that count before confirmation.
    pub fn remove_project_tag(&self, snapshot: &ProjectTagSnapshot) -> Result<usize> {
        self.change_project_tag(snapshot, None)
    }

    fn change_project_tag(
        &self,
        snapshot: &ProjectTagSnapshot,
        replacement: Option<&str>,
    ) -> Result<usize> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        super::require_project(&transaction, &snapshot.project_id)?;
        crate::project_labels::guard_catalog(&transaction, &snapshot.catalog)?;
        if tag_targets(&transaction, &snapshot.project_id, &snapshot.tag)? != snapshot.targets {
            return Err(StudioError::Validation(
                "The tagged passages changed; review the project tag again".into(),
            ));
        }
        let catalog = crate::project_labels::changed_definition(
            &snapshot.catalog,
            &snapshot.tag,
            replacement,
        )?;
        if replacement == Some(snapshot.tag.as_str()) {
            return Ok(0);
        }
        for target in &snapshot.targets {
            let before = &target.annotation;
            if replacement.is_none() && before.deleted {
                continue;
            }
            let document = read_document(&transaction, &before.document_id)?;
            require_revision(&document, target.document_revision)?;
            rich_tree(&document)?;
            let mut after = before.clone();
            if let Some(replacement) = replacement {
                for tag in &mut after.tags {
                    if tag == &snapshot.tag {
                        *tag = replacement.into();
                    }
                }
            } else {
                after.tags.retain(|tag| tag != &snapshot.tag);
                after.deleted = after.tags.is_empty() && after.note.is_empty();
                if after.deleted {
                    // A recoverable deletion retains the last authored label with its quote;
                    // it no longer participates in active tag queries until explicit restore.
                    after.tags.clone_from(&before.tags);
                }
            }
            after.revision = next_revision(before.revision)?;
            if !before.deleted {
                after.base_revision = document.revision;
            }
            store_annotation(
                &transaction,
                before,
                &after,
                if replacement.is_none() && after.deleted {
                    "delete"
                } else {
                    "update"
                },
            )?;
            if !before.deleted {
                save_attachment(&transaction, &after, document.revision)?;
            }
        }
        crate::project_labels::commit_catalog(&transaction, &snapshot.catalog, catalog)?;
        transaction.commit()?;
        Ok(snapshot.count())
    }

    /// Read append-only history, including the history of explicitly deleted notes.
    pub fn annotation_history(&self, id: &str) -> Result<Vec<AnnotationEvent>> {
        let connection = self.connection()?;
        read_annotation(&connection, id, true)?;
        let mut statement = connection.prepare("SELECT annotation_revision,document_revision,source,before_json,after_json FROM document_annotation_history WHERE annotation_id=?1 ORDER BY annotation_revision DESC")?;
        let rows = statement
            .query_map([id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|(revision, document_revision, source, before, after)| {
                Ok(AnnotationEvent {
                    revision,
                    document_revision,
                    source,
                    before: before.as_deref().map(serde_json::from_str).transpose()?,
                    after: serde_json::from_str(&after)?,
                })
            })
            .collect()
    }

    fn change_annotation(
        &self,
        id: &str,
        expected_document_revision: i64,
        expected_annotation_revision: i64,
        source: &str,
        labels: Option<&crate::ProjectLabelCatalog>,
        change: impl FnOnce(&Document, &mut PassageAnnotation) -> Result<()>,
    ) -> Result<PassageAnnotation> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let before = read_annotation(&transaction, id, false)?;
        let document = read_document(&transaction, &before.document_id)?;
        require_revision(&document, expected_document_revision)?;
        if document.read_only {
            return Err(StudioError::ReadOnly {
                document_id: document.id,
            });
        }
        if before.revision != expected_annotation_revision {
            return Err(StudioError::AnnotationConflict {
                expected: expected_annotation_revision,
                actual: before.revision,
            });
        }
        let mut after = before.clone();
        change(&document, &mut after)?;
        if let Some(labels) = labels {
            crate::project_labels::guard_assignments(
                &transaction,
                &document.project_id,
                labels,
                &after.tags,
                false,
            )?;
        }
        if before != after {
            if !after.deleted {
                crate::project_labels::ensure_assignments(
                    &transaction,
                    &document.project_id,
                    &after.tags,
                    false,
                )?;
            }
            after.revision = next_revision(before.revision)?;
            after.base_revision = document.revision;
            store_annotation(&transaction, &before, &after, source)?;
            save_attachment(&transaction, &after, document.revision)?;
        }
        transaction.commit()?;
        Ok(after)
    }
}

fn preview_replay_annotations(
    current: &Document,
    expected_revision: i64,
    proofs: &[ReplayProof],
    expected_content: &serde_json::Value,
    mut notes: Vec<PassageAnnotation>,
    receipts: &ReplayCache,
) -> Result<(Vec<PassageAnnotation>, ReplayCache)> {
    let document_id = current.id.as_str();
    let mut tree = rich_tree(current)?;
    let mut candidate_receipts = receipts.clone();
    // The original and candidate maps each hold one reference. The engine's
    // live history/pending proofs retain any still reachable target receipt.
    candidate_receipts.retain(|receipt, _| receipt.reference_count() > 2);
    if let Some(first) = proofs.first()
        && candidate_receipts
            .get(first.before())
            .is_some_and(|entry| entry.acknowledged_revision != Some(expected_revision))
    {
        return Err(StudioError::Validation(
            "Native history base changed outside this session; preserve the draft before reloading"
                .into(),
        ));
    }
    if let Some(first) = proofs.first()
        && first.action() != ReplayAction::Apply
        && let Some(cached) = candidate_receipts.get(first.before())
        && attachment_states(&notes).iter().any(|(id, snapshot)| {
            cached
                .attachments
                .get(id)
                .is_some_and(|previous| previous != snapshot)
        })
    {
        return Err(StudioError::Validation("Annotation attachments changed after this history checkpoint; preserve the current note before undoing".into()));
    }
    let mut prior_receipt = None;
    for proof in proofs {
        if prior_receipt.is_some_and(|previous| previous != proof.before()) {
            return Err(StudioError::Validation(
                "Native edit journal is not contiguous".into(),
            ));
        }
        if candidate_receipts
            .get(proof.before())
            .is_some_and(|entry| entry.document_id != document_id)
        {
            return Err(StudioError::Validation(
                "Native history belongs to another document".into(),
            ));
        }
        let (updated, maps) = proof.verify(&tree).map_err(|error| {
            StudioError::Validation(format!("Native edit replay failed: {error}"))
        })?;
        candidate_receipts.insert(
            proof.before().clone(),
            CachedReceipt {
                document_id: document_id.into(),
                attachments: attachment_states(&notes),
                acknowledged_revision: None,
            },
        );
        let restored = if proof.action() == ReplayAction::Apply {
            None
        } else {
            Some(candidate_receipts.get(proof.after()).filter(|entry| entry.document_id == document_id).ok_or_else(|| StudioError::Validation("Native Undo/Redo attachment receipt is unavailable; retain this draft and reload history".into()))?.attachments.clone())
        };
        for annotation in &mut notes {
            if let Some(snapshot) = restored
                .as_ref()
                .and_then(|states| states.get(&annotation.id))
            {
                annotation.anchor = snapshot.anchor.clone();
                annotation.anchor_status = snapshot.status;
            } else if annotation.anchor_status == AnchorStatus::Attached {
                let before_anchor = annotation.anchor.clone();
                let mapped = verified_anchor(&tree, &annotation.anchor).is_ok()
                    && map_anchor(
                        &mut annotation.anchor,
                        if restored.is_some() {
                            None
                        } else {
                            Some(&maps)
                        },
                        &tree,
                        &updated,
                    );
                if !mapped {
                    annotation.anchor = before_anchor;
                    annotation.anchor_status = AnchorStatus::Detached;
                }
            }
            if annotation.anchor_status == AnchorStatus::Attached {
                match verified_anchor(&updated, &annotation.anchor) {
                    Ok(anchor) => annotation.anchor = anchor,
                    Err(_) => annotation.anchor_status = AnchorStatus::Detached,
                }
            }
        }
        candidate_receipts.insert(
            proof.after().clone(),
            CachedReceipt {
                document_id: document_id.into(),
                attachments: attachment_states(&notes),
                acknowledged_revision: None,
            },
        );
        tree = updated;
        prior_receipt = Some(proof.after());
    }
    if tree.to_json().map_err(|error| {
        StudioError::Validation(format!("Native result cannot be encoded: {error}"))
    })? != *expected_content
    {
        return Err(StudioError::Validation("Native edit journal does not produce the acknowledged content; preserve the continued draft".into()));
    }
    Ok((notes, candidate_receipts))
}

fn attachment_states(notes: &[PassageAnnotation]) -> HashMap<String, AttachmentSnapshot> {
    notes
        .iter()
        .map(|annotation| {
            (
                annotation.id.clone(),
                AttachmentSnapshot {
                    anchor: annotation.anchor.clone(),
                    status: annotation.anchor_status,
                },
            )
        })
        .collect()
}

fn normalize_markup_note(note: &str, tags: &[String]) -> Result<String> {
    let note = note.trim();
    if (note.is_empty() && tags.is_empty())
        || note.chars().count() > crate::MAX_PROJECT_ANNOTATION_NOTE_CHARACTERS
    {
        return Err(StudioError::Validation(format!(
            "Add a note or at least one tag; notes can contain at most {} characters",
            crate::MAX_PROJECT_ANNOTATION_NOTE_CHARACTERS
        )));
    }
    Ok(note.into())
}

pub(crate) fn validate_annotation_tags(tags: &[String]) -> Result<()> {
    if tags.len() > 32 {
        return Err(StudioError::Validation(
            "Use at most 32 tags on one passage".into(),
        ));
    }
    let mut seen = std::collections::HashSet::new();
    for tag in tags {
        if tag.trim().is_empty() || tag.chars().count() > 80 || tag.chars().any(char::is_control) {
            return Err(StudioError::Validation(
                "Each tag needs 1–80 characters without line breaks".into(),
            ));
        }
        if !seen.insert(tag) {
            return Err(StudioError::Validation(
                "Each tag can occur only once on a passage".into(),
            ));
        }
    }
    Ok(())
}

fn validate_tags(tags: &[String]) -> Result<()> {
    validate_annotation_tags(tags)
}

fn project_annotations(
    connection: &Connection,
    project_id: &str,
    include_deleted: bool,
) -> Result<Vec<ProjectAnnotationSummary>> {
    let mut statement = connection.prepare("SELECT d.title,d.revision,a.annotation_json FROM document_annotations a JOIN documents d ON d.id=a.document_id WHERE d.project_id=?1 AND (a.deleted=0 OR ?2) ORDER BY d.title,d.id,a.id")?;
    let rows = statement
        .query_map(params![project_id, include_deleted], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(document_title, document_revision, raw)| {
            Ok(ProjectAnnotationSummary {
                document_title,
                document_revision,
                annotation: serde_json::from_str(&raw)?,
            })
        })
        .collect()
}

fn tag_targets(
    connection: &Connection,
    project_id: &str,
    tag: &str,
) -> Result<Vec<ProjectAnnotationSummary>> {
    Ok(project_annotations(connection, project_id, true)?
        .into_iter()
        .filter(|item| item.annotation.tags.iter().any(|label| label == tag))
        .collect())
}

fn rich_tree(document: &Document) -> Result<RichTree> {
    if document.read_only {
        return Err(StudioError::ReadOnly {
            document_id: document.id.clone(),
        });
    }
    let content = document.rich_json.clone().ok_or_else(|| {
        StudioError::Validation(
            "Structured editing must be enabled before attaching a passage note".into(),
        )
    })?;
    RichTree::from_json(content).map_err(|error| {
        StudioError::Validation(format!("Invalid structured annotation document: {error}"))
    })
}

fn anchor_at(
    tree: &RichTree,
    block_id: &str,
    start: ScalarOffset,
    end: ScalarOffset,
) -> Result<AnnotationAnchor> {
    let block = tree
        .block(block_id)
        .map_err(|_| StudioError::Validation("Annotation block was not found".into()))?;
    // Native parent anchors use explicit paragraph separators and leaf coordinates.
    // Legacy export projections are converted by an explicit compatibility adapter.
    if !matches!(
        block.node_type.as_str(),
        "paragraph" | "heading" | "codeBlock" | "blockquote"
    ) {
        return Err(StudioError::Validation(
            "Select text within one supported text block".into(),
        ));
    }
    let projection = BlockTextProjection::new(tree, block_id).map_err(|error| {
        StudioError::Validation(format!("Invalid annotation projection: {error}"))
    })?;
    let text = projection.text();
    let length = text.chars().count();
    if start >= end || end.0 > length {
        return Err(StudioError::InvalidTextRange {
            start: start.0,
            end: end.0,
            length,
        });
    }
    let byte = |offset| {
        scalar_to_byte(text, offset)
            .map(|offset| offset.0)
            .map_err(|_| StudioError::InvalidTextRange {
                start: start.0,
                end: end.0,
                length,
            })
    };
    projection
        .point(start, true)
        .map_err(|_| StudioError::ExpectedTextMismatch)?;
    projection
        .point(end, false)
        .map_err(|_| StudioError::ExpectedTextMismatch)?;
    let start_byte = byte(start)?;
    let end_byte = byte(end)?;
    Ok(AnnotationAnchor {
        block_id: block_id.into(),
        start,
        end,
        exact_text: text[start_byte..end_byte].into(),
        prefix: text[byte(ScalarOffset(start.0.saturating_sub(32)))?..start_byte].into(),
        suffix: text[end_byte..byte(ScalarOffset(end.0.saturating_add(32).min(length)))?].into(),
    })
}

fn verified_anchor(tree: &RichTree, anchor: &AnnotationAnchor) -> Result<AnnotationAnchor> {
    let current = anchor_at(tree, &anchor.block_id, anchor.start, anchor.end)?;
    if current.exact_text != anchor.exact_text {
        return Err(StudioError::ExpectedTextMismatch);
    }
    Ok(current)
}

fn read_annotation(
    connection: &Connection,
    id: &str,
    include_deleted: bool,
) -> Result<PassageAnnotation> {
    let encoded = connection
        .query_row(
            "SELECT annotation_json FROM document_annotations WHERE id=?1 AND (deleted=0 OR ?2)",
            params![id, include_deleted],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or_else(|| StudioError::NotFound {
            kind: "Annotation",
            id: id.into(),
        })?;
    Ok(serde_json::from_str(&encoded)?)
}

fn list_annotations(connection: &Connection, document_id: &str) -> Result<Vec<PassageAnnotation>> {
    let mut statement = connection.prepare("SELECT annotation_json FROM document_annotations WHERE document_id=?1 AND deleted=0 ORDER BY id")?;
    let rows = statement
        .query_map([document_id], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|encoded| Ok(serde_json::from_str(&encoded)?))
        .collect()
}

fn next_revision(revision: i64) -> Result<i64> {
    revision
        .checked_add(1)
        .ok_or_else(|| StudioError::CorruptData("annotation revision overflow".into()))
}

fn append_event(
    connection: &Connection,
    before: Option<&PassageAnnotation>,
    after: &PassageAnnotation,
    source: &str,
) -> Result<()> {
    connection.execute("INSERT INTO document_annotation_history(annotation_id,annotation_revision,document_revision,source,before_json,after_json) VALUES(?1,?2,?3,?4,?5,?6)", params![after.id, after.revision, after.base_revision, source, before.map(serde_json::to_string).transpose()?, serde_json::to_string(after)?])?;
    Ok(())
}

fn store_annotation(
    connection: &Connection,
    before: &PassageAnnotation,
    after: &PassageAnnotation,
    source: &str,
) -> Result<()> {
    let before_raw: String = connection.query_row(
        "SELECT annotation_json FROM document_annotations WHERE id=?1",
        [&before.id],
        |row| row.get(0),
    )?;
    let mut value: serde_json::Value = serde_json::from_str(&before_raw)?;
    let known = serde_json::to_value(after)?;
    let fields = known
        .as_object()
        .ok_or_else(|| StudioError::CorruptData("Annotation state is not an object".into()))?;
    for (key, replacement) in fields {
        if key == "anchor" && value[key].is_object() {
            let anchor = value[key].as_object_mut().ok_or_else(|| {
                StudioError::CorruptData("Annotation anchor is not an object".into())
            })?;
            // Deserialize accepts these legacy aliases; retaining both spellings would make
            // the next read a duplicate-field error. Only known aliases are canonicalized.
            anchor.remove("blockId");
            anchor.remove("exactText");
            let replacement = replacement.as_object().ok_or_else(|| {
                StudioError::CorruptData("Updated annotation anchor is not an object".into())
            })?;
            anchor.extend(
                replacement
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone())),
            );
        } else {
            value[key] = replacement.clone();
        }
    }
    let after_raw = serde_json::to_string(&value)?;
    let count = connection.execute("UPDATE document_annotations SET revision=?2,deleted=?3,annotation_json=?4 WHERE id=?1 AND revision=?5", params![after.id, after.revision, after.deleted, after_raw, before.revision])?;
    if count != 1 {
        return Err(StudioError::CorruptData(
            "annotation changed under transaction lock".into(),
        ));
    }
    connection.execute("INSERT INTO document_annotation_history(annotation_id,annotation_revision,document_revision,source,before_json,after_json) VALUES(?1,?2,?3,?4,?5,?6)", params![after.id,after.revision,after.base_revision,source,before_raw,after_raw])?;
    Ok(())
}

fn save_attachment(
    connection: &Connection,
    annotation: &PassageAnnotation,
    document_revision: i64,
) -> Result<()> {
    let snapshot = AttachmentSnapshot {
        anchor: annotation.anchor.clone(),
        status: annotation.anchor_status,
    };
    connection.execute("INSERT INTO document_annotation_snapshots(document_id,document_revision,annotation_id,attachment_json) VALUES(?1,?2,?3,?4) ON CONFLICT(document_id,document_revision,annotation_id) DO UPDATE SET attachment_json=excluded.attachment_json", params![annotation.document_id, document_revision, annotation.id, serde_json::to_string(&snapshot)?])?;
    Ok(())
}

/// Called under the same SQL transaction as the rich document/history save.
pub(super) fn reconcile_annotations(
    connection: &Connection,
    before: &Document,
    after: &Document,
    mappings: Option<&[PositionMap]>,
    historical_revision: Option<i64>,
) -> Result<()> {
    let annotations = list_annotations(connection, &before.id)?;
    if annotations.is_empty() {
        return Ok(());
    }
    let original = rich_tree(before)?;
    let updated = rich_tree(after)?;
    for annotation in annotations {
        save_attachment(connection, &annotation, before.revision)?;
        let mut changed = annotation.clone();
        changed.base_revision = after.revision;
        let restored = historical_revision.map(|revision| connection.query_row("SELECT attachment_json FROM document_annotation_snapshots WHERE document_id=?1 AND document_revision=?2 AND annotation_id=?3", params![before.id, revision, annotation.id], |row| row.get::<_, String>(0)).optional()).transpose()?.flatten();
        if let Some(encoded) = restored {
            let snapshot: AttachmentSnapshot = serde_json::from_str(&encoded)?;
            changed.anchor = snapshot.anchor;
            changed.anchor_status = snapshot.status;
        } else if changed.anchor_status == AnchorStatus::Attached {
            let attached = annotation.base_revision == before.revision
                && verified_anchor(&original, &changed.anchor).is_ok()
                && map_anchor(&mut changed.anchor, mappings, &original, &updated);
            if !attached {
                // Preserve the complete prior quote/context on uncertainty.
                changed.anchor = annotation.anchor.clone();
                changed.anchor_status = AnchorStatus::Detached;
            }
        }
        if changed.anchor_status == AnchorStatus::Attached {
            match verified_anchor(&updated, &changed.anchor) {
                Ok(anchor) => changed.anchor = anchor,
                Err(_) => changed.anchor_status = AnchorStatus::Detached,
            }
        }
        changed.revision = next_revision(annotation.revision)?;
        store_annotation(
            connection,
            &annotation,
            &changed,
            if historical_revision.is_some() {
                "restore"
            } else {
                "mapping"
            },
        )?;
        save_attachment(connection, &changed, after.revision)?;
    }
    Ok(())
}

fn map_anchor(
    anchor: &mut AnnotationAnchor,
    mappings: Option<&[PositionMap]>,
    before: &RichTree,
    after: &RichTree,
) -> bool {
    let Some(mappings) = mappings else {
        // Snapshot recovery has no proven edit history: unchanged projection only.
        return BlockTextProjection::new(before, &anchor.block_id)
            .ok()
            .map(|p| p.text().to_owned())
            == BlockTextProjection::new(after, &anchor.block_id)
                .ok()
                .map(|p| p.text().to_owned())
            && verified_anchor(after, anchor).is_ok();
    };
    let Ok((mut first, mut last)) = anchor.selection(before) else {
        return false;
    };
    let parent_quote = before
        .block(&anchor.block_id)
        .is_ok_and(|node| node.node_type == "blockquote");
    for mapping in mappings {
        // Match native ProseMirror endpoint deletion semantics. Exact boundary
        // deletion and interior edits remain deterministically attached; deletion
        // strictly across either endpoint does not acquire a different passage.
        if let PositionMap::Text {
            block_id,
            start,
            old_len,
            ..
        } = mapping
        {
            let Some(end) = start.checked_add(*old_len) else {
                return false;
            };
            if [&first, &last].iter().any(|point| {
                &point.block_id == block_id && *start < point.offset.0 && point.offset.0 < end
            }) {
                return false;
            }
        }
        let Some((id, offset)) = mapping.map_existing_caret(&first.block_id, first.offset, true)
        else {
            return false;
        };
        first = TextPoint {
            block_id: id,
            offset,
        };
        let Some((id, offset)) = mapping.map_existing_caret(&last.block_id, last.offset, false)
        else {
            return false;
        };
        last = TextPoint {
            block_id: id,
            offset,
        };
    }
    let id = if parent_quote {
        anchor.block_id.as_str()
    } else if first.block_id == last.block_id {
        first.block_id.as_str()
    } else {
        return false;
    };
    let Ok(projection) = BlockTextProjection::new(after, id) else {
        return false;
    };
    let (Ok(start), Ok(end)) = (projection.offset(&first), projection.offset(&last)) else {
        return false;
    };
    let Ok(current) = anchor_at(after, id, start, end) else {
        return false;
    };
    // Regeneration is justified only by these sealed/replayed exact maps, never
    // by a repeated-quote search or a caller-authored snapshot position map.
    *anchor = current;
    true
}
