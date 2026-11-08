-- Only locally prepared typed commands enter this active review table.
-- Imported proposal/replay JSON belongs in project preservation evidence.
CREATE TABLE rich_proposals (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES rich_documents(document_id),
    base_revision INTEGER NOT NULL CHECK(base_revision>0),
    status TEXT NOT NULL CHECK(status IN ('staged','applied','rejected','undone')),
    before_json TEXT NOT NULL,
    after_json TEXT NOT NULL,
    command_json TEXT NOT NULL,
    reason TEXT NOT NULL,
    applied_revision INTEGER,
    undone_revision INTEGER,
    FOREIGN KEY(document_id,base_revision) REFERENCES rich_document_revisions(document_id,revision),
    FOREIGN KEY(document_id,applied_revision) REFERENCES rich_document_revisions(document_id,revision),
    FOREIGN KEY(document_id,undone_revision) REFERENCES rich_document_revisions(document_id,revision)
);
CREATE INDEX rich_proposals_document ON rich_proposals(document_id);
