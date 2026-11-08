//! Source-compatible selection data; validation never grants provider or mutation authority.
use super::{Document, Result, StudioError, Workspace, read_document, require_revision};
use craft_editor_model::{
    BlockTextProjection, Document as RichTree, InlineAtomText, Node, ScalarOffset, TextPoint,
    TextProjectionOptions,
};
use serde::{Deserialize, Serialize};

/// One exact range in the original assistant's top-level block text projection.
/// These scalar offsets intentionally differ from native carets when inline atoms are omitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantSelectionRange {
    /// Stable supported top-level block identity.
    pub block_id: String,
    /// Inclusive Unicode scalar boundary in the explicit source projection.
    pub start: ScalarOffset,
    /// Exclusive Unicode scalar boundary in the explicit source projection.
    pub end: ScalarOffset,
    /// Complete literal selected occurrence, including hard breaks.
    pub expected_text: String,
}
/// Ordinary portable attachment data; deserializing it grants no read/send/Apply authority.
/// Every reveal or later command must validate its fixed project, revision and exact ranges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantSelectionAttachment {
    /// Owning native project; this hint must match the caller's fixed project capability.
    pub project_id: String,
    /// Owning native document, independent of imported source logical identities.
    #[serde(rename = "nodeId")]
    pub document_id: String,
    /// Acknowledged canonical document revision at capture, after flushing the editor draft.
    pub base_revision: i64,
    /// Exact supported ranges in document order; no implicit additional context.
    pub ranges: Vec<AssistantSelectionRange>,
    /// Source-compatible visible quote; not an instruction to search for another occurrence.
    pub quote: String,
}

fn unavailable() -> StudioError {
    StudioError::Validation(
        "Select text inside one or more supported paragraphs, headings, blockquotes or code blocks"
            .into(),
    )
}
fn tree(document: &Document) -> Result<RichTree> {
    RichTree::from_json(document.rich_json.clone().ok_or_else(unavailable)?).map_err(|error| {
        StudioError::Validation(format!("Invalid rich selection document: {error}"))
    })
}
fn supported(node: &Node) -> bool {
    matches!(
        node.node_type.as_str(),
        "paragraph" | "heading" | "blockquote" | "codeBlock"
    )
}
fn projection(document: &RichTree, id: &str) -> Result<BlockTextProjection> {
    BlockTextProjection::with_options(
        document,
        id,
        TextProjectionOptions {
            inline_math: InlineAtomText::Omit,
            block_atom: "",
            ..TextProjectionOptions::default()
        },
    )
    .map_err(|_| unavailable())
}
fn location(document: &RichTree, point: &TextPoint) -> Result<Vec<usize>> {
    // First reject container IDs and out-of-bounds native positions, rather than treating them
    // as parent offsets or inventing a stable ID from quoted text.
    BlockTextProjection::new(document, &point.block_id)
        .and_then(|projection| projection.offset(point))
        .map_err(|_| unavailable())?;
    fn visit(node: &Node, id: &str, path: &mut Vec<usize>) -> bool {
        if node.block_id() == Some(id)
            && matches!(
                node.node_type.as_str(),
                "paragraph" | "heading" | "codeBlock"
            )
        {
            return true;
        }
        for (index, child) in node.children().iter().enumerate() {
            path.push(index);
            if visit(child, id, path) {
                return true;
            }
            path.pop();
        }
        false
    }
    let mut path = vec![];
    if !visit(document.root(), &point.block_id, &mut path) || path.is_empty() {
        return Err(unavailable());
    }
    Ok(path)
}
fn quote(ranges: &[AssistantSelectionRange]) -> String {
    ranges
        .iter()
        .map(|range| range.expected_text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n")
        .trim_matches(super::daily_controls::is_ecmascript_whitespace)
        .to_owned()
}
fn exact_text(
    projection: &BlockTextProjection,
    start: ScalarOffset,
    end: ScalarOffset,
) -> Result<String> {
    let length = projection.text().chars().count();
    if start.0 >= end.0 || end.0 > length {
        return Err(unavailable());
    }
    Ok(projection
        .text()
        .chars()
        .skip(start.0)
        .take(end.0 - start.0)
        .collect())
}
fn capture_ranges(
    document: &RichTree,
    anchor: &TextPoint,
    head: &TextPoint,
) -> Result<Vec<AssistantSelectionRange>> {
    let a = location(document, anchor)?;
    let b = location(document, head)?;
    let ((first_path, first), (last_path, last)) = if (&a, anchor.offset.0) <= (&b, head.offset.0) {
        ((a, anchor), (b, head))
    } else {
        ((b, head), (a, anchor))
    };
    let mut ranges = vec![];
    for (index, node) in document.root().children().iter().enumerate() {
        if index < first_path[0] || index > last_path[0] || !supported(node) {
            continue;
        }
        let block_id = node.block_id().ok_or_else(unavailable)?;
        let projection = projection(document, block_id)?;
        let start = if index == first_path[0] {
            projection.offset(first).map_err(|_| unavailable())?
        } else {
            ScalarOffset(0)
        };
        let end = if index == last_path[0] {
            projection.offset(last).map_err(|_| unavailable())?
        } else {
            ScalarOffset(projection.text().chars().count())
        };
        if start == end {
            continue;
        }
        let expected_text = exact_text(&projection, start, end)?;
        ranges.push(AssistantSelectionRange {
            block_id: block_id.into(),
            start,
            end,
            expected_text,
        });
    }
    Ok(ranges)
}
impl Workspace {
    fn assistant_selection_document(
        &self,
        project: &str,
        id: &str,
        revision: i64,
    ) -> Result<Document> {
        let document = {
            let connection = self.connection()?;
            let owned: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM documents WHERE id=?1 AND project_id=?2)",
                [id, project],
                |row| row.get(0),
            )?;
            if !owned {
                return Err(StudioError::NotFound {
                    kind: "project document",
                    id: id.into(),
                });
            }
            let document = read_document(&connection, id)?;
            require_revision(&document, revision)?;
            let pending: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM rich_working_drafts WHERE document_id=?1)",
                [id],
                |row| row.get(0),
            )?;
            if pending {
                return Err(StudioError::Validation("Save or preserve the working draft before attaching or revealing a canonical selection".into()));
            }
            document
        };
        Ok(document)
    }
    /// Capture the original supported top-level ranges from exact directed native leaf carets.
    ///
    /// Flush the live editor first. This reads one canonical snapshot, creates no revision/note/
    /// proposal and sends nothing to a provider. Lists/tables/opaque blocks are not silently
    /// expanded into additional context. Reversed selections preserve the same document order.
    ///
    /// # Errors
    /// Refuses foreign/stale documents, invalid native carets and selections with no supported text.
    pub fn capture_assistant_selection(
        &self,
        project: &str,
        id: &str,
        revision: i64,
        anchor: &TextPoint,
        head: &TextPoint,
    ) -> Result<AssistantSelectionAttachment> {
        let current = self.assistant_selection_document(project, id, revision)?;
        let document = tree(&current)?;
        let ranges = capture_ranges(&document, anchor, head)?;
        let quote = quote(&ranges);
        if quote.is_empty() {
            return Err(unavailable());
        }
        Ok(AssistantSelectionAttachment {
            project_id: project.into(),
            document_id: id.into(),
            base_revision: revision,
            ranges,
            quote,
        })
    }
    /// Validate untrusted attachment data and return exact native reveal boundaries.
    ///
    /// No substring search or stale retargeting is used. At omitted inline atoms the affinities
    /// choose the minimal quoted range. The UI may keep its original in-process directed carets
    /// while the same revision remains current; this DTO is not a replay/selection capability.
    ///
    /// # Errors
    /// Refuses foreign/stale data, unsupported/duplicate/reordered blocks, altered quotes or ranges.
    pub fn assistant_selection_boundaries(
        &self,
        project: &str,
        attachment: &AssistantSelectionAttachment,
    ) -> Result<(TextPoint, TextPoint)> {
        if attachment.project_id != project {
            return Err(unavailable());
        }
        let current = self.assistant_selection_document(
            project,
            &attachment.document_id,
            attachment.base_revision,
        )?;
        let document = tree(&current)?;
        if attachment.ranges.is_empty()
            || attachment.ranges.len() > document.root().children().len()
        {
            return Err(unavailable());
        }
        let mut previous = None;
        let mut first = None;
        let mut last = None;
        for range in &attachment.ranges {
            let index = document
                .root()
                .children()
                .iter()
                .position(|node| {
                    node.block_id() == Some(range.block_id.as_str()) && supported(node)
                })
                .ok_or_else(unavailable)?;
            if previous.is_some_and(|previous| index <= previous) {
                return Err(unavailable());
            }
            previous = Some(index);
            let projection = projection(&document, &range.block_id)?;
            if exact_text(&projection, range.start, range.end)? != range.expected_text {
                return Err(StudioError::ExpectedTextMismatch);
            }
            let start = projection
                .point(range.start, true)
                .map_err(|_| unavailable())?;
            let end = projection
                .point(range.end, false)
                .map_err(|_| unavailable())?;
            if first.is_none() {
                first = Some(start);
            }
            last = Some(end);
        }
        let derived_quote = quote(&attachment.ranges);
        if derived_quote.is_empty() || derived_quote != attachment.quote {
            return Err(StudioError::ExpectedTextMismatch);
        }
        let (first, last) = first.zip(last).ok_or_else(unavailable)?;
        // A single source selection cannot omit an interior supported block or replace a full
        // interior block with a partial range. Revealing such forged data would expand scope.
        if capture_ranges(&document, &first, &last)? != attachment.ranges {
            return Err(StudioError::ExpectedTextMismatch);
        }
        Ok((first, last))
    }
}
