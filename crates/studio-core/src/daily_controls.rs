//! Source-backed daily manuscript queries and guarded title metadata.
use super::{
    Document, Result, StudioError, Workspace, read_document, require_project, validate_title,
};
use craft_editor_model::{Document as RichDocument, Node, ScalarOffset, TextPoint};
use rusqlite::{TransactionBehavior, params};
use std::ops::Range;

/// Read-only manuscript statistics; private annotation bodies are never supplied here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DocumentStatistics {
    /// Whitespace-separated words, matching the original editor.
    pub words: usize,
    /// Unicode scalars, including the supplied projection's whitespace.
    pub characters: usize,
    /// Whole estimated minutes at the original 180 words per minute.
    pub reading_minutes: usize,
}

// Source String.trim and \s share ECMAScript whitespace; Rust differs for U+0085/U+FEFF.
pub(crate) fn is_ecmascript_whitespace(character: char) -> bool {
    matches!(character,
        '\u{0009}'..='\u{000d}' | ' ' | '\u{00a0}' | '\u{1680}' |
        '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' |
        '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

/// Count the supplied reader projection without modifying or normalizing it.
pub fn document_statistics(text: &str) -> DocumentStatistics {
    let words = text
        .split(is_ecmascript_whitespace)
        .filter(|word| !word.is_empty())
        .count();
    DocumentStatistics {
        words,
        characters: text.chars().count(),
        reading_minutes: words.div_ceil(180),
    }
}

/// Count a rich manuscript using the original Tiptap editor.getText projection.
/// Each nonzero-position block contributes two newlines, including containers;
/// hard breaks contribute one newline. Formula/image/diagram attributes and private
/// notes contribute no reader text. This query never replaces canonical JSON.
pub fn rich_document_statistics(document: &RichDocument) -> DocumentStatistics {
    fn append(node: &Node, first_top_level: bool, output: &mut String) {
        let inline = matches!(node.node_type.as_str(), "text" | "hardBreak" | "inlineMath");
        if !inline && !first_top_level {
            output.push_str("\n\n");
        }
        match node.node_type.as_str() {
            "text" => output.push_str(node.text.as_deref().unwrap_or_default()),
            "hardBreak" => output.push('\n'),
            _ => {
                for child in node.children() {
                    append(child, false, output);
                }
            }
        }
    }
    let mut text = String::new();
    for (index, node) in document.blocks().iter().enumerate() {
        append(node, index == 0, &mut text);
    }
    document_statistics(&text)
}

/// Exact semantic endpoints of the first case-insensitive document match.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindMatch {
    /// Inclusive native text-leaf boundary.
    pub anchor: TextPoint,
    /// Exclusive native text-leaf boundary.
    pub head: TextPoint,
}

/// Find the first literal match, returning Unicode-scalar boundaries in the original text.
/// Unicode lowercase expansion is mapped back to whole original scalars.
/// An empty query has no match. No regex or replacement is performed.
pub fn find_first_text_match(text: &str, query: &str) -> Option<Range<usize>> {
    if query.is_empty() {
        return None;
    }
    let needle = query.to_lowercase();
    find_first_folded_match(text, &needle)
}

fn find_first_folded_match(text: &str, needle: &str) -> Option<Range<usize>> {
    let folded = text.to_lowercase();
    let start = folded.find(needle)?;
    let end = start + needle.len();
    let mut folded_byte = 0;
    let mut scalar_start = None;
    for (index, character) in text.chars().enumerate() {
        let length: usize = character.to_lowercase().map(char::len_utf8).sum();
        let next = folded_byte + length;
        if scalar_start.is_none() && start < next {
            scalar_start = Some(index);
        }
        if end <= next {
            return scalar_start.map(|start| start..index + 1);
        }
        folded_byte = next;
    }
    None
}

/// Find within native paragraph/heading/code leaves in document order, including
/// nested list/callout/table leaves. Styled runs share one projection. Paragraph
/// and opaque-atom boundaries are not silently removed or searched as source text.
///
/// # Errors
/// Returns a validation error if a matching text leaf lacks its stable identity.
pub fn find_first_document_match(
    document: &RichDocument,
    query: &str,
) -> Result<Option<FindMatch>> {
    fn visit(node: &Node, needle: &str) -> Result<Option<FindMatch>> {
        if matches!(
            node.node_type.as_str(),
            "paragraph" | "heading" | "codeBlock"
        ) {
            let text = node.inline_text();
            let Some(range) = find_first_folded_match(&text, needle) else {
                return Ok(None);
            };
            let id = node.block_id().ok_or_else(|| {
                StudioError::Validation("A searchable text block has no stable identity".into())
            })?;
            return Ok(Some(FindMatch {
                anchor: TextPoint {
                    block_id: id.into(),
                    offset: ScalarOffset(range.start),
                },
                head: TextPoint {
                    block_id: id.into(),
                    offset: ScalarOffset(range.end),
                },
            }));
        }
        for child in node.children() {
            if let Some(found) = visit(child, needle)? {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }
    // U+FFFC denotes a whole opaque inline atom, never searchable author source.
    if query.is_empty() || query.contains('\u{fffc}') {
        return Ok(None);
    }
    let needle = query.to_lowercase();
    visit(document.root(), &needle)
}

impl Workspace {
    /// Rename document metadata under its owning project and an exact captured title.
    /// Content, rich JSON, source provenance, drafts, text revision and history stay intact.
    ///
    /// # Errors
    /// Rejects empty/oversized titles, another project or a stale title. Imported
    /// content remains read-only while its independent title metadata can be renamed.
    pub fn rename_document(
        &self,
        project_id: &str,
        document_id: &str,
        expected_title: &str,
        title: &str,
    ) -> Result<Document> {
        let title = validate_title(title, 200)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_project(&transaction, project_id)?;
        let document = read_document(&transaction, document_id)?;
        if document.project_id != project_id {
            return Err(StudioError::NotFound {
                kind: "Document",
                id: document_id.into(),
            });
        }
        if document.title != expected_title {
            return Err(StudioError::Validation(
                "The document title changed; reopen title editing before renaming".into(),
            ));
        }
        transaction.execute(
            "UPDATE documents SET title=?1 WHERE id=?2 AND project_id=?3 AND title=?4",
            params![title, document_id, project_id, expected_title],
        )?;
        let renamed = read_document(&transaction, document_id)?;
        transaction.commit()?;
        Ok(renamed)
    }
}
