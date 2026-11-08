-- Private attachment checkpoints are derived only from verified native replay.
-- Replacing an unproven draft clears its checkpoint; deleting a draft cascades it.
CREATE TABLE rich_draft_annotations (
    document_id TEXT PRIMARY KEY REFERENCES rich_working_drafts(document_id) ON DELETE CASCADE,
    draft_version TEXT NOT NULL,
    checkpoint_json TEXT NOT NULL
);
