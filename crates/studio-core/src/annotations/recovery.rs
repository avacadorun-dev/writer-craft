//! Durable draft attachment checkpoints; no replay proof or receipt is serialized.
use super::*;
use crate::{RichDraft, digest};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
struct DraftTarget {
    block_id: String,
    start: ScalarOffset,
    end: ScalarOffset,
}
#[derive(Serialize, Deserialize)]
enum DraftAttachment {
    Attached(DraftTarget),
    Detached(AttachmentSnapshot),
}
#[derive(Serialize, Deserialize)]
struct DraftNote {
    id: String,
    revision: i64,
    guard_hash: String,
    after_hash: String,
    attachment: DraftAttachment,
}
#[derive(Serialize, Deserialize)]
struct DraftCheckpoint {
    version: u8,
    document_id: String,
    base_revision: i64,
    base_hash: String,
    content_hash: String,
    changed: bool,
    notes: Vec<DraftNote>,
}
fn attachment_hash(note: &PassageAnnotation) -> Result<String> {
    Ok(digest(&serde_json::to_string(&AttachmentSnapshot {
        anchor: note.anchor.clone(),
        status: note.anchor_status,
    })?))
}
impl Workspace {
    /// Persist rich writing and private mapped attachments from a sealed native journal.
    ///
    /// Exact replay, complete-result, revision and draft-version guards match canonical
    /// save. Neither candidate history receipts nor canonical notes are acknowledged.
    /// Attached ranges use compact positions/hashes; detached ranges retain their
    /// complete last-safe quote/context, which may no longer exist in either tree.
    /// No ReplayProof, HistoryReceipt, caller map or note body is persisted here.
    pub fn save_rich_editor_draft(
        &self,
        document_id: &str,
        expected_revision: i64,
        expected_draft_version: Option<&str>,
        proofs: &[ReplayProof],
        expected_content: Value,
    ) -> Result<RichDraft> {
        if proofs.len() > 1_000 {
            return Err(StudioError::Validation(
                "Native edit batch exceeds 1000 changes".into(),
            ));
        }
        let (content, _, encoded) = crate::rich::validate_tree(expected_content)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = read_document(&transaction, document_id)?;
        require_revision(&current, expected_revision)?;
        let tree = rich_tree(&current)?;
        let original = list_annotations(&transaction, document_id)?;
        let receipts = self
            .replay_receipts
            .lock()
            .map_err(|_| StudioError::LockUnavailable)?;
        let (notes, _) = preview_replay_annotations(
            &current,
            expected_revision,
            proofs,
            &content,
            original.clone(),
            &receipts,
        )?;
        let existing = transaction
            .query_row(
                "SELECT base_revision,version FROM rich_working_drafts WHERE document_id=?1",
                [document_id],
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
        let checkpoint = DraftCheckpoint {
            version: 1,
            document_id: document_id.into(),
            base_revision: expected_revision,
            base_hash: digest(&serde_json::to_string(
                &tree
                    .to_json()
                    .map_err(|error| StudioError::Validation(error.to_string()))?,
            )?),
            content_hash: digest(&encoded),
            changed: proofs.iter().any(|proof| proof.before() != proof.after()),
            notes: original
                .iter()
                .zip(&notes)
                .map(|(before, after)| {
                    Ok(DraftNote {
                        id: before.id.clone(),
                        revision: before.revision,
                        guard_hash: attachment_hash(before)?,
                        after_hash: attachment_hash(after)?,
                        attachment: if after.anchor_status == AnchorStatus::Attached {
                            DraftAttachment::Attached(DraftTarget {
                                block_id: after.anchor.block_id.clone(),
                                start: after.anchor.start,
                                end: after.anchor.end,
                            })
                        } else {
                            DraftAttachment::Detached(AttachmentSnapshot {
                                anchor: after.anchor.clone(),
                                status: after.anchor_status,
                            })
                        },
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        };
        let checkpoint = serde_json::to_string(&checkpoint)?;
        let version = uuid::Uuid::new_v4().to_string();
        transaction.execute("INSERT INTO rich_working_drafts(document_id,base_revision,version,content_json) VALUES(?1,?2,?3,?4) ON CONFLICT(document_id) DO UPDATE SET base_revision=excluded.base_revision,version=excluded.version,content_json=excluded.content_json",params![document_id,expected_revision,version,encoded])?;
        transaction.execute("INSERT INTO rich_draft_annotations(document_id,draft_version,checkpoint_json) VALUES(?1,?2,?3) ON CONFLICT(document_id) DO UPDATE SET draft_version=excluded.draft_version,checkpoint_json=excluded.checkpoint_json",params![document_id,version,checkpoint])?;
        transaction.commit()?;
        Ok(RichDraft {
            base_revision: expected_revision,
            version,
            content,
            has_annotation_checkpoint: true,
        })
    }

    /// Recover one exact durable draft and its validated mapped attachments atomically.
    ///
    /// Start a fresh native editor session after success; old in-process history is
    /// not recreated. A missing proof-derived checkpoint uses the conservative
    /// snapshot path. Stale base/version, later reattachment, changed note set or
    /// corrupt binding leaves the draft, canonical manuscript and notes untouched.
    /// Authored note/lifecycle-only changes remain intact when their complete
    /// intervening history proves attachments were not changed.
    pub fn recover_rich_editor_draft(
        &self,
        document_id: &str,
        expected_revision: i64,
        expected_draft_version: &str,
    ) -> Result<crate::Document> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = read_document(&transaction, document_id)?;
        require_revision(&current, expected_revision)?;
        let original_tree = rich_tree(&current)?;
        let(base,version,encoded)=transaction.query_row("SELECT base_revision,version,content_json FROM rich_working_drafts WHERE document_id=?1",[document_id],|row|Ok((row.get::<_,i64>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?))).optional()?.ok_or(StudioError::DraftChanged)?;
        if base != expected_revision {
            return Err(StudioError::DraftConflict {
                draft_revision: base,
                current_revision: expected_revision,
            });
        }
        if version != expected_draft_version {
            return Err(StudioError::DraftChanged);
        }
        let (content, body, encoded) = crate::rich::validate_tree(serde_json::from_str(&encoded)?)?;
        let checkpoint=transaction.query_row("SELECT draft_version,checkpoint_json FROM rich_draft_annotations WHERE document_id=?1",[document_id],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?))).optional()?;
        let original = list_annotations(&transaction, document_id)?;
        let recovered = if let Some((draft_version, checkpoint)) = checkpoint {
            let checkpoint: DraftCheckpoint = serde_json::from_str(&checkpoint)?;
            let original_json = serde_json::to_string(
                &original_tree
                    .to_json()
                    .map_err(|error| StudioError::Validation(error.to_string()))?,
            )?;
            if draft_version != version
                || checkpoint.version != 1
                || checkpoint.document_id != document_id
                || checkpoint.base_revision != base
                || checkpoint.base_hash != digest(&original_json)
                || checkpoint.content_hash != digest(&encoded)
            {
                return Err(recovery_conflict());
            }
            if checkpoint.notes.len() != original.len() {
                return Err(recovery_conflict());
            }
            let tree = RichTree::from_json(content.clone())
                .map_err(|error| StudioError::Validation(error.to_string()))?;
            let mut notes = original.clone();
            for note in &mut notes {
                let guard = checkpoint
                    .notes
                    .iter()
                    .find(|guard| guard.id == note.id)
                    .ok_or_else(recovery_conflict)?;
                if guard.guard_hash != attachment_hash(note)?
                    || !metadata_only_since(&transaction, note, guard.revision)?
                {
                    return Err(recovery_conflict());
                }
                match &guard.attachment {
                    DraftAttachment::Attached(target) => {
                        note.anchor = anchor_at(&tree, &target.block_id, target.start, target.end)
                            .map_err(|_| recovery_conflict())?;
                        note.anchor_status = AnchorStatus::Attached;
                    }
                    DraftAttachment::Detached(snapshot) => {
                        if snapshot.status != AnchorStatus::Detached {
                            return Err(recovery_conflict());
                        }
                        note.anchor = snapshot.anchor.clone();
                        note.anchor_status = snapshot.status;
                    }
                }
                if attachment_hash(note)? != guard.after_hash {
                    return Err(recovery_conflict());
                }
            }
            Some((notes, checkpoint.changed))
        } else {
            None
        };
        let changed = current.rich_json.as_ref() != Some(&content)
            || recovered.as_ref().is_some_and(|(_, changed)| *changed);
        let result = if changed {
            let saved = crate::write_revision(&transaction, &current, &body, "manual")?;
            crate::rich::write_rich_snapshot(&transaction, document_id, saved.revision, &encoded)?;
            if let Some((notes, _)) = recovered {
                for (before, mut after) in original.iter().zip(notes) {
                    save_attachment(&transaction, before, current.revision)?;
                    after.revision = next_revision(before.revision)?;
                    after.base_revision = saved.revision;
                    store_annotation(&transaction, before, &after, "restore")?;
                    save_attachment(&transaction, &after, saved.revision)?;
                }
            } else {
                let updated = read_document(&transaction, document_id)?;
                reconcile_annotations(&transaction, &current, &updated, None, None)?;
            }
            read_document(&transaction, document_id)?
        } else {
            current
        };
        transaction.execute(
            "DELETE FROM rich_working_drafts WHERE document_id=?1 AND version=?2",
            params![document_id, version],
        )?;
        transaction.commit()?;
        Ok(result)
    }
}
fn recovery_conflict() -> StudioError {
    StudioError::DraftAnnotationConflict
}
fn metadata_only_since(
    connection: &Connection,
    note: &PassageAnnotation,
    revision: i64,
) -> Result<bool> {
    if note.revision == revision {
        return Ok(true);
    }
    if note.revision < revision {
        return Ok(false);
    }
    let mut statement=connection.prepare("SELECT annotation_revision,source,before_json,after_json FROM document_annotation_history WHERE annotation_id=?1 AND annotation_revision>?2 ORDER BY annotation_revision")?;
    let rows = statement
        .query_map(params![note.id, revision], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut expected = revision;
    for (revision, source, before, after) in rows {
        expected = next_revision(expected)?;
        if revision != expected || source != "update" {
            return Ok(false);
        }
        let Some(before) = before else {
            return Ok(false);
        };
        let before: PassageAnnotation = serde_json::from_str(&before)?;
        let after: PassageAnnotation = serde_json::from_str(&after)?;
        if before.deleted || after.deleted || attachment_hash(&before)? != attachment_hash(&after)?
        {
            return Ok(false);
        }
    }
    Ok(expected == note.revision)
}
