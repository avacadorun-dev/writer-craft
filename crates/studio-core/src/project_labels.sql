-- Named project labels are independent of passage assignments and manuscript revisions.
CREATE TABLE project_label_catalog (
    project_id TEXT PRIMARY KEY REFERENCES projects(id),
    revision INTEGER NOT NULL CHECK(revision >= 0),
    raw_json TEXT NOT NULL
);
