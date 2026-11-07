CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL
);
CREATE TABLE documents (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    read_only INTEGER NOT NULL DEFAULT 0 CHECK(read_only IN (0,1)),
    source_json TEXT,
    CHECK((read_only = 0 AND source_json IS NULL) OR (read_only = 1 AND source_json IS NOT NULL))
);
CREATE INDEX documents_project ON documents(project_id);
CREATE TABLE document_revisions (
    document_id TEXT NOT NULL REFERENCES documents(id),
    revision INTEGER NOT NULL CHECK(revision > 0),
    body TEXT NOT NULL,
    source TEXT NOT NULL CHECK(source IN ('manual','import','proposal','undo')),
    PRIMARY KEY(document_id,revision)
);
CREATE TABLE proposals (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES documents(id),
    base_revision INTEGER NOT NULL CHECK(base_revision > 0),
    before_body TEXT NOT NULL,
    after_body TEXT NOT NULL,
    reason TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('staged','applied','rejected','undone')),
    applied_revision INTEGER,
    undone_revision INTEGER,
    anchor_json TEXT NOT NULL,
    before_hash TEXT NOT NULL,
    after_hash TEXT NOT NULL
);
CREATE INDEX proposals_document ON proposals(document_id);
