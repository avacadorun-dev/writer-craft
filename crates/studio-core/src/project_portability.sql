-- Project-scoped source identities never replace native document/asset handles.
CREATE TABLE project_portability (
    project_id TEXT PRIMARY KEY REFERENCES projects(id),
    logical_project_id TEXT NOT NULL,
    format_version INTEGER NOT NULL CHECK(format_version BETWEEN 1 AND 8),
    raw_manifest TEXT NOT NULL,
    manifest_sha256 TEXT NOT NULL CHECK(length(manifest_sha256)=64)
);
CREATE TABLE project_resource_aliases (
    project_id TEXT NOT NULL REFERENCES projects(id),
    resource_kind TEXT NOT NULL,
    logical_id TEXT NOT NULL,
    native_id TEXT NOT NULL,
    PRIMARY KEY(project_id,resource_kind,logical_id),
    UNIQUE(project_id,resource_kind,native_id)
);
CREATE TABLE project_nodes (
    project_id TEXT NOT NULL REFERENCES projects(id),
    logical_id TEXT NOT NULL,
    native_document_id TEXT REFERENCES documents(id),
    parent_logical_id TEXT,
    ordinal INTEGER NOT NULL CHECK(ordinal>=0),
    raw_json TEXT NOT NULL,
    PRIMARY KEY(project_id,logical_id),
    UNIQUE(native_document_id),
    FOREIGN KEY(project_id,parent_logical_id) REFERENCES project_nodes(project_id,logical_id) DEFERRABLE INITIALLY DEFERRED
);
CREATE TABLE project_source_records (
    project_id TEXT NOT NULL REFERENCES projects(id),
    domain TEXT NOT NULL,
    ordinal INTEGER NOT NULL CHECK(ordinal>=0),
    raw_json TEXT NOT NULL,
    PRIMARY KEY(project_id,domain,ordinal)
);
CREATE TABLE project_asset_metadata (
    project_id TEXT NOT NULL REFERENCES projects(id),
    resource_kind TEXT NOT NULL CHECK(resource_kind IN ('media','audio')),
    logical_id TEXT NOT NULL,
    native_asset_id TEXT NOT NULL REFERENCES project_assets(id),
    archive_path TEXT NOT NULL,
    raw_json TEXT NOT NULL,
    PRIMARY KEY(project_id,resource_kind,logical_id),
    UNIQUE(project_id,archive_path)
);
CREATE TABLE project_canvases (
    project_id TEXT NOT NULL REFERENCES projects(id),
    logical_id TEXT NOT NULL,
    current_revision INTEGER NOT NULL CHECK(current_revision>0),
    raw_json TEXT NOT NULL,
    PRIMARY KEY(project_id,logical_id)
);
CREATE TABLE project_canvas_revisions (
    project_id TEXT NOT NULL,
    canvas_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision>0),
    base_revision INTEGER NOT NULL CHECK(base_revision>=0 AND base_revision<revision),
    schema_version INTEGER NOT NULL CHECK(schema_version IN (1,2)),
    content_json TEXT NOT NULL,
    raw_json TEXT NOT NULL,
    PRIMARY KEY(project_id,canvas_id,revision),
    FOREIGN KEY(project_id,canvas_id) REFERENCES project_canvases(project_id,logical_id)
);
CREATE TABLE project_canvas_drafts (
    project_id TEXT NOT NULL,
    canvas_id TEXT NOT NULL,
    base_revision INTEGER NOT NULL,
    version TEXT NOT NULL,
    content_json TEXT NOT NULL,
    PRIMARY KEY(project_id,canvas_id),
    FOREIGN KEY(project_id,canvas_id,base_revision) REFERENCES project_canvas_revisions(project_id,canvas_id,revision)
);
-- External draft/checkpoint payloads are source evidence only. They never enter
-- rich_draft_annotations, whose contents require verified native replay authority.
CREATE TABLE project_quarantined_recovery (
    project_id TEXT NOT NULL REFERENCES projects(id),
    source_key TEXT NOT NULL,
    raw_json TEXT NOT NULL,
    PRIMARY KEY(project_id,source_key)
);
CREATE TABLE project_annotation_projections (
    project_id TEXT NOT NULL REFERENCES projects(id),
    logical_id TEXT NOT NULL,
    document_id TEXT NOT NULL REFERENCES documents(id),
    native_annotation_id TEXT REFERENCES document_annotations(id),
    native_anchor_json TEXT,
    native_projection_json TEXT,
    anchor_status TEXT NOT NULL CHECK(anchor_status IN ('attached','detached')),
    raw_json TEXT NOT NULL,
    PRIMARY KEY(project_id,logical_id)
);
