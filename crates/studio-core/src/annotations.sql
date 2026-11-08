-- Author metadata stays outside canonical manuscript JSON and reader text.
CREATE TABLE document_annotations (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES documents(id),
    revision INTEGER NOT NULL CHECK(revision > 0),
    deleted INTEGER NOT NULL DEFAULT 0 CHECK(deleted IN (0,1)),
    annotation_json TEXT NOT NULL
);
CREATE INDEX document_annotations_document ON document_annotations(document_id,deleted);
CREATE TABLE document_annotation_history (
    id INTEGER PRIMARY KEY,
    annotation_id TEXT NOT NULL REFERENCES document_annotations(id),
    annotation_revision INTEGER NOT NULL CHECK(annotation_revision > 0),
    document_revision INTEGER NOT NULL CHECK(document_revision > 0),
    source TEXT NOT NULL CHECK(source IN ('create','update','reattach','delete','mapping','restore')),
    before_json TEXT,
    after_json TEXT NOT NULL,
    UNIQUE(annotation_id,annotation_revision)
);
CREATE TABLE document_annotation_snapshots (
    document_id TEXT NOT NULL,
    document_revision INTEGER NOT NULL,
    annotation_id TEXT NOT NULL REFERENCES document_annotations(id),
    attachment_json TEXT NOT NULL,
    PRIMARY KEY(document_id,document_revision,annotation_id),
    FOREIGN KEY(document_id,document_revision) REFERENCES document_revisions(document_id,revision)
);
