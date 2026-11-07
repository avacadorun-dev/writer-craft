CREATE TABLE working_drafts (
    document_id TEXT PRIMARY KEY REFERENCES documents(id),
    base_revision INTEGER NOT NULL CHECK(base_revision > 0),
    body TEXT NOT NULL
);
