-- Schema 8: imported unknown/recovery/staged payloads are data, never replay or Apply authority.
-- Artifact originals remain in project_assets; this store holds bounded opaque/evidence objects.
CREATE TABLE project_evidence_objects (
    project_id TEXT NOT NULL REFERENCES projects(id),
    sha256 TEXT NOT NULL CHECK(length(sha256)=64 AND sha256 NOT GLOB '*[^0-9a-f]*'),
    byte_size INTEGER NOT NULL CHECK(byte_size>=0 AND byte_size<=67108864),
    media_type TEXT NOT NULL CHECK(length(CAST(media_type AS BLOB)) BETWEEN 1 AND 255),
    data BLOB NOT NULL CHECK(typeof(data)='blob' AND length(data)=byte_size),
    PRIMARY KEY(project_id,sha256)
);
CREATE TABLE project_evidence_references (
    project_id TEXT NOT NULL REFERENCES projects(id),
    domain TEXT NOT NULL CHECK(length(CAST(domain AS BLOB)) BETWEEN 1 AND 128),
    logical_id TEXT NOT NULL CHECK(length(CAST(logical_id AS BLOB)) BETWEEN 1 AND 1024),
    logical_document_id TEXT,
    logical_canvas_id TEXT,
    object_sha256 TEXT NOT NULL,
    metadata_json TEXT NOT NULL CHECK(
        length(CAST(metadata_json AS BLOB))<=262144
        AND json_valid(metadata_json)
        AND json_type(metadata_json)='object'
    ),
    PRIMARY KEY(project_id,domain,logical_id),
    FOREIGN KEY(project_id,object_sha256)
        REFERENCES project_evidence_objects(project_id,sha256),
    FOREIGN KEY(project_id,logical_document_id)
        REFERENCES project_nodes(project_id,logical_id) DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(project_id,logical_canvas_id)
        REFERENCES project_canvases(project_id,logical_id) DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX project_evidence_references_document
    ON project_evidence_references(project_id,logical_document_id,domain);
CREATE INDEX project_evidence_references_canvas
    ON project_evidence_references(project_id,logical_canvas_id,domain);
