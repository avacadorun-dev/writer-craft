-- Generic durable project media. Image authoring policy belongs to its adapter;
-- future archive import must retain other media without an image-only size cap.
CREATE TABLE project_assets (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    media_type TEXT NOT NULL,
    original_name TEXT NOT NULL,
    sha256 TEXT NOT NULL CHECK(length(sha256)=64),
    byte_size INTEGER NOT NULL CHECK(byte_size>=0),
    width INTEGER CHECK(width IS NULL OR width>0),
    height INTEGER CHECK(height IS NULL OR height>0),
    source_json TEXT,
    created_at INTEGER NOT NULL DEFAULT(unixepoch()),
    data BLOB NOT NULL,
    CHECK(length(data)=byte_size)
);
CREATE INDEX project_assets_project ON project_assets(project_id);
