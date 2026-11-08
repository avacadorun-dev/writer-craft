CREATE TABLE rich_documents (
    document_id TEXT PRIMARY KEY REFERENCES documents(id),
    content_json TEXT NOT NULL
);
CREATE TABLE rich_document_revisions (
    document_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    content_json TEXT NOT NULL,
    PRIMARY KEY(document_id,revision),
    FOREIGN KEY(document_id,revision) REFERENCES document_revisions(document_id,revision)
);
CREATE TABLE rich_working_drafts (
    document_id TEXT PRIMARY KEY REFERENCES rich_documents(document_id),
    base_revision INTEGER NOT NULL CHECK(base_revision > 0),
    version TEXT NOT NULL,
    content_json TEXT NOT NULL
);
