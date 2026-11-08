//! Current reading projections share the canonical export transaction, not a whole history Value.
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde_json::Value;

use super::{
    PreparedPortableProject, ProjectTransferCancellation, Result, StudioError, json::invalid,
    read_unsigned, sqlite_integer,
};

/// Lightweight node metadata for deterministic portable reading paths.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectReadingNode {
    /// Stable project-scoped logical identity.
    pub logical_id: String,
    /// Captured parent identity; absent for roots.
    pub parent_id: Option<String>,
    /// Authored title, never translated or inferred from a filename.
    pub title: String,
    /// Source ordering coordinate, with ordinal as a stable tie-breaker.
    pub position: f64,
    /// Stable captured array/creation order.
    pub ordinal: u64,
    /// Saved current revision; zero identifies a contentless container.
    pub current_revision: u64,
    /// The reading text is a named preview of a separately retained opaque original.
    pub read_only_preview: bool,
}
/// One bounded current saved writing projection. Histories/recovery remain canonical objects.
pub struct ProjectReadingDocument {
    /// Literal saved prose projection; never an unsaved draft.
    pub body: String,
    /// Current saved canonical rich content, when the source has it.
    pub rich_content: Option<Value>,
    /// Exact authored source frontmatter, kept separate from generated Markdown.
    pub source_frontmatter: String,
}
/// Lightweight current Canvas metadata, without historical graphs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectReadingCanvas {
    /// Stable owning-project board identity.
    pub logical_id: String,
    /// Authored board title.
    pub title: String,
    /// Captured ordering coordinate.
    pub position: f64,
    /// Saved current revision represented by the reading graph.
    pub current_revision: u64,
}

impl PreparedPortableProject {
    /// Visit one lightweight captured node at a time; no prose/history/metadata payload array.
    pub fn visit_reading_nodes(
        &self,
        visitor: &mut dyn FnMut(ProjectReadingNode) -> Result<()>,
    ) -> Result<()> {
        let mut statement=self.index.prepare("SELECT logical_id,parent_id,title,position,ordinal,current_revision,read_only FROM reading_nodes ORDER BY ordinal")?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            visitor(ProjectReadingNode {
                logical_id: row.get(0)?,
                parent_id: row.get(1)?,
                title: row.get(2)?,
                position: row.get(3)?,
                ordinal: read_unsigned(row, 4)?,
                current_revision: read_unsigned(row, 5)?,
                read_only_preview: row.get(6)?,
            })?;
        }
        Ok(())
    }
    /// Read one current saved writing object in this prepared project, never an ambient document.
    pub fn reading_document(&self, logical_id: &str) -> Result<ProjectReadingDocument> {
        let row:Option<(String,Option<String>,String)>=self.index.query_row("SELECT body,rich_json,frontmatter FROM reading_nodes WHERE logical_id=?1 AND current_revision>0",[logical_id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional()?;
        let (body, rich, source_frontmatter) = row.ok_or_else(|| StudioError::NotFound {
            kind: "captured reading document",
            id: logical_id.into(),
        })?;
        // Imported source objects retain their separately bounded 64 MiB source phase; native
        // authoring remains at its existing narrower rich/body bounds. No source is truncated.
        if body.len() > 16 * 1024 * 1024
            || rich
                .as_ref()
                .is_some_and(|raw| raw.len() > super::MAX_PROJECT_MANIFEST_BYTES)
            || source_frontmatter.len() > 10 * 1024 * 1024
        {
            return invalid("Captured reading content exceeds its original source bounds");
        }
        Ok(ProjectReadingDocument {
            body,
            rich_content: rich.map(|raw| serde_json::from_str(&raw)).transpose()?,
            source_frontmatter,
        })
    }
    /// Visit lightweight board metadata only; current graph bytes load on explicit selection.
    pub fn visit_reading_canvases(
        &self,
        visitor: &mut dyn FnMut(ProjectReadingCanvas) -> Result<()>,
    ) -> Result<()> {
        let mut statement=self.index.prepare("SELECT logical_id,title,position,current_revision FROM reading_canvases ORDER BY position,logical_id")?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            visitor(ProjectReadingCanvas {
                logical_id: row.get(0)?,
                title: row.get(1)?,
                position: row.get(2)?,
                current_revision: read_unsigned(row, 3)?,
            })?;
        }
        Ok(())
    }
    /// Read one captured saved graph; unfinished graphs stay separately quarantined evidence.
    pub fn reading_canvas(&self, logical_id: &str) -> Result<Value> {
        let raw: Option<String> = self
            .index
            .query_row(
                "SELECT content_json FROM reading_canvases WHERE logical_id=?1",
                [logical_id],
                |row| row.get(0),
            )
            .optional()?;
        let raw = raw.ok_or_else(|| StudioError::NotFound {
            kind: "captured reading Canvas",
            id: logical_id.into(),
        })?;
        if raw.len() > 12 * 1024 * 1024 {
            return invalid("Captured reading Canvas exceeds 12 MiB");
        }
        Ok(serde_json::from_str(&raw)?)
    }
}

pub(super) fn capture_reading(
    source: &Connection,
    target: &Connection,
    project: &str,
    cancel: &ProjectTransferCancellation,
) -> Result<()> {
    target.execute_batch("CREATE TABLE reading_nodes(logical_id TEXT PRIMARY KEY,parent_id TEXT,title TEXT NOT NULL,position REAL NOT NULL,ordinal INTEGER NOT NULL,current_revision INTEGER NOT NULL,read_only INTEGER NOT NULL,body TEXT NOT NULL,rich_json TEXT,frontmatter TEXT NOT NULL); CREATE TABLE reading_canvases(logical_id TEXT PRIMARY KEY,title TEXT NOT NULL,position REAL NOT NULL,current_revision INTEGER NOT NULL,content_json TEXT NOT NULL);")?;
    let mut statement=source.prepare("SELECT n.logical_id,n.parent_logical_id,coalesce(d.title,json_extract(n.raw_json,'$.title')),json_extract(n.raw_json,'$.position'),n.ordinal,coalesce(d.revision,0),coalesce(d.read_only,0),coalesce(d.body,''),r.content_json,coalesce(json_extract(n.raw_json,'$.sourceFrontmatter'),'') FROM project_nodes n LEFT JOIN documents d ON d.id=n.native_document_id AND d.project_id=n.project_id LEFT JOIN rich_document_revisions r ON r.document_id=d.id AND r.revision=d.revision WHERE n.project_id=?1 ORDER BY n.ordinal")?;
    let mut rows = statement.query([project])?;
    let mut next_ordinal = 0_u64;
    while let Some(row) = rows.next()? {
        cancel.check()?;
        let position: f64 = row.get(3)?;
        if !position.is_finite() {
            return invalid("Captured reading order is not finite");
        }
        let ordinal = read_unsigned(row, 4)?;
        next_ordinal =
            next_ordinal.max(ordinal.checked_add(1).ok_or_else(|| {
                StudioError::Validation("Captured reading ordinal overflow".into())
            })?);
        target.execute("INSERT INTO reading_nodes(logical_id,parent_id,title,position,ordinal,current_revision,read_only,body,rich_json,frontmatter) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![row.get::<_,String>(0)?,row.get::<_,Option<String>>(1)?,row.get::<_,String>(2)?,position,sqlite_integer(ordinal)?,sqlite_integer(read_unsigned(row,5)?)?,row.get::<_,bool>(6)?,row.get::<_,String>(7)?,row.get::<_,Option<String>>(8)?,row.get::<_,String>(9)?])?;
    }
    let mut statement=source.prepare("SELECT d.id,d.title,d.revision,d.read_only,d.body,r.content_json FROM documents d LEFT JOIN rich_document_revisions r ON r.document_id=d.id AND r.revision=d.revision WHERE d.project_id=?1 AND NOT EXISTS(SELECT 1 FROM project_nodes n WHERE n.project_id=d.project_id AND n.native_document_id=d.id) ORDER BY d.rowid")?;
    let mut rows = statement.query([project])?;
    while let Some(row) = rows.next()? {
        cancel.check()?;
        let ordinal = sqlite_integer(next_ordinal)?;
        target.execute("INSERT INTO reading_nodes(logical_id,parent_id,title,position,ordinal,current_revision,read_only,body,rich_json,frontmatter) VALUES(?1,NULL,?2,?3,?4,?5,?6,?7,?8,'')",params![row.get::<_,String>(0)?,row.get::<_,String>(1)?,next_ordinal as f64,ordinal,sqlite_integer(read_unsigned(row,2)?)?,row.get::<_,bool>(3)?,row.get::<_,String>(4)?,row.get::<_,Option<String>>(5)?])?;
        next_ordinal = next_ordinal
            .checked_add(1)
            .ok_or_else(|| StudioError::Validation("Captured reading ordinal overflow".into()))?;
    }
    let mut statement=source.prepare("SELECT c.logical_id,json_extract(c.raw_json,'$.title'),json_extract(c.raw_json,'$.position'),c.current_revision,r.content_json FROM project_canvases c JOIN project_canvas_revisions r ON r.project_id=c.project_id AND r.canvas_id=c.logical_id AND r.revision=c.current_revision WHERE c.project_id=?1 ORDER BY c.rowid")?;
    let mut rows = statement.query([project])?;
    while let Some(row) = rows.next()? {
        cancel.check()?;
        let position: f64 = row.get(2)?;
        if !position.is_finite() {
            return invalid("Captured reading Canvas order is not finite");
        }
        target.execute("INSERT INTO reading_canvases(logical_id,title,position,current_revision,content_json) VALUES(?1,?2,?3,?4,?5)",params![row.get::<_,String>(0)?,row.get::<_,String>(1)?,position,sqlite_integer(read_unsigned(row,3)?)?,row.get::<_,String>(4)?])?;
    }
    Ok(())
}
