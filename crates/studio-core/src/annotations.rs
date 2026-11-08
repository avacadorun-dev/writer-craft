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
        let note = normalize_note(note)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let document = read_document(&transaction, document_id)?;
        require_revision(&document, expected_revision)?;
        let anchor = verified_anchor(&rich_tree(&document)?, &anchor)?;
        let result = PassageAnnotation {
            id: uuid::Uuid::new_v4().to_string(),
            document_id: document_id.into(),
            revision: 1,
            base_revision: expected_revision,
            anchor,
            note,
            status: AnnotationStatus::Open,
            anchor_status: AnchorStatus::Attached,
            deleted: false,
        };
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

    /// Update authored text/lifecycle with both document and note guards.
    pub fn update_annotation(
        &self,
        id: &str,
        expected_document_revision: i64,
        expected_annotation_revision: i64,
        patch: AnnotationPatch,
    ) -> Result<PassageAnnotation> {
        if patch.note.is_none() && patch.status.is_none() {
            return Err(StudioError::Validation("Annotation update is empty".into()));
        }
        self.change_annotation(
            id,
            expected_document_revision,
            expected_annotation_revision,
            "update",
            |_, annotation| {
                if let Some(note) = patch.note {
                    annotation.note = normalize_note(&note)?;
                }
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
            |_, annotation| {
                annotation.deleted = true;
                Ok(())
            },
        )?;
        Ok(())
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
        if before != after {
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

fn normalize_note(note: &str) -> Result<String> {
    let note = note.trim();
    if note.is_empty() || note.chars().count() > 10_000 {
        return Err(StudioError::Validation(
            "Annotation note must contain 1–10000 characters".into(),
        ));
    }
    Ok(note.into())
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
    let count = connection.execute("UPDATE document_annotations SET revision=?2,deleted=?3,annotation_json=?4 WHERE id=?1 AND revision=?5", params![after.id, after.revision, after.deleted, serde_json::to_string(after)?, before.revision])?;
    if count != 1 {
        return Err(StudioError::CorruptData(
            "annotation changed under transaction lock".into(),
        ));
    }
    append_event(connection, Some(before), after, source)
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
