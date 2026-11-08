//! Passive system spelling over exact, visible native reader text.
use super::*;
use craft_editor_egui::{
    DocPoint, Selection,
    decorations::{DecorationSource, TextDecoration, TextUnderline},
};
use craft_editor_model::{Document as RichTree, Node, ReplayProof, Revision, ScalarOffset};
use std::{
    collections::{BTreeMap, VecDeque},
    ops::Range,
};

const CHUNK_UTF16: usize = 1024;
const CACHE_ENTRIES: usize = 64;
const FRAME_CALLS: usize = 8;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Availability {
    #[default]
    Pending,
    Ready,
    Unavailable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CheckError {
    Unavailable,
    InvalidRange,
}
trait Checker {
    fn check(&mut self, text: &str, start_utf16: usize)
    -> Result<Option<Range<usize>>, CheckError>;
}

struct NativeChecker;
#[cfg(target_os = "macos")]
impl Checker for NativeChecker {
    fn check(
        &mut self,
        text: &str,
        start_utf16: usize,
    ) -> Result<Option<Range<usize>>, CheckError> {
        use objc2_app_kit::NSSpellChecker;
        use objc2_foundation::NSString;
        if objc2::MainThreadMarker::new().is_none() {
            return Err(CheckError::Unavailable);
        }
        let checker = NSSpellChecker::sharedSpellChecker();
        if checker.availableLanguages().is_empty() {
            return Err(CheckError::Unavailable);
        }
        // This is process-local language detection, not a mutation of the user's
        // operating-system language preferences or a guess from the interface locale.
        checker.setAutomaticallyIdentifiesLanguages(true);
        let string = NSString::from_str(text);
        let start = isize::try_from(start_utf16).map_err(|_| CheckError::InvalidRange)?;
        let range = checker.checkSpellingOfString_startingAt(&string, start);
        if range.length == 0 || range.location == usize::MAX || range.location < start_utf16 {
            return Ok(None);
        }
        let end = range
            .location
            .checked_add(range.length)
            .ok_or(CheckError::InvalidRange)?;
        if end > text.encode_utf16().count() {
            return Err(CheckError::InvalidRange);
        }
        Ok(Some(range.location..end))
    }
}
#[cfg(not(target_os = "macos"))]
impl Checker for NativeChecker {
    fn check(&mut self, _: &str, _: usize) -> Result<Option<Range<usize>>, CheckError> {
        Err(CheckError::Unavailable)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Chunk {
    block_id: String,
    start: usize,
    text: String,
}
struct Entry {
    chunk: Chunk,
    next_utf16: usize,
    complete: bool,
    ranges: Vec<Selection>,
}
#[derive(Default)]
pub(super) struct ProofingState {
    revision: Option<Revision>,
    cache: VecDeque<Entry>,
    availability: Availability,
    pending: bool,
}
impl ProofingState {
    pub(super) fn availability(&self) -> Availability {
        self.availability
    }
    pub(super) fn pending(&self) -> bool {
        self.pending
    }
    pub(super) fn frame<'a>(
        &'a mut self,
        notes: &'a mut dyn DecorationSource,
        enabled: bool,
        language: Language,
        tokens: Tokens,
    ) -> ProofingFrame<'a> {
        ProofingFrame {
            state: self,
            notes,
            enabled,
            language,
            tokens,
        }
    }

    fn analyze(
        &mut self,
        document: &RichTree,
        revision: Revision,
        visible: &[Selection],
        enabled: bool,
        checker: &mut impl Checker,
    ) -> Result<Vec<Selection>, CheckError> {
        self.pending = false;
        if !enabled {
            self.cache.clear();
            self.revision = None;
            return Ok(vec![]);
        }
        if self.revision != Some(revision) {
            self.cache.clear();
            self.revision = Some(revision);
        }
        let chunks = visible_chunks(document, visible)?;
        let started = Instant::now();
        let mut calls = 0;
        let mut ranges = Vec::new();
        for chunk in chunks {
            let known = self.cache.iter().position(|entry| entry.chunk == chunk);
            let mut entry = known
                .and_then(|index| self.cache.remove(index))
                .unwrap_or_else(|| Entry {
                    chunk,
                    next_utf16: 0,
                    complete: false,
                    ranges: vec![],
                });
            while !entry.complete
                && calls < FRAME_CALLS
                && started.elapsed() < Duration::from_millis(3)
            {
                calls += 1;
                match checker.check(&entry.chunk.text, entry.next_utf16)? {
                    None => entry.complete = true,
                    Some(range) => {
                        if range.start < entry.next_utf16 || range.start >= range.end {
                            return Err(CheckError::InvalidRange);
                        }
                        let scalars = utf16_to_scalar_range(&entry.chunk.text, range.clone())?;
                        entry.ranges.push(Selection {
                            anchor: DocPoint {
                                block_id: entry.chunk.block_id.clone(),
                                offset: ScalarOffset(entry.chunk.start + scalars.start),
                            },
                            head: DocPoint {
                                block_id: entry.chunk.block_id.clone(),
                                offset: ScalarOffset(entry.chunk.start + scalars.end),
                            },
                        });
                        entry.next_utf16 = range.end;
                        if entry.next_utf16 >= entry.chunk.text.encode_utf16().count() {
                            entry.complete = true;
                        }
                    }
                }
                self.availability = Availability::Ready;
            }
            self.pending |= !entry.complete;
            for range in &entry.ranges {
                if !ranges.contains(range) {
                    ranges.push(range.clone());
                }
            }
            self.cache.push_back(entry);
            while self.cache.len() > CACHE_ENTRIES {
                self.cache.pop_front();
            }
        }
        Ok(ranges)
    }
}

pub(super) struct ProofingFrame<'a> {
    state: &'a mut ProofingState,
    notes: &'a mut dyn DecorationSource,
    enabled: bool,
    language: Language,
    tokens: Tokens,
}
impl DecorationSource for ProofingFrame<'_> {
    fn decorations(
        &mut self,
        document: &RichTree,
        revision: Revision,
        pending: &[ReplayProof],
    ) -> craft_editor_egui::Result<Vec<TextDecoration>> {
        self.notes.decorations(document, revision, pending)
    }
    fn underlines_in_view(
        &mut self,
        document: &RichTree,
        revision: Revision,
        _: &[ReplayProof],
        visible: &[Selection],
    ) -> craft_editor_egui::Result<Vec<TextUnderline>> {
        match self.state.analyze(
            document,
            revision,
            visible,
            self.enabled,
            &mut NativeChecker,
        ) {
            Ok(ranges) => Ok(ranges
                .into_iter()
                .map(|range| TextUnderline {
                    range,
                    color: self.tokens.warning,
                    label: self.language.text("Possible spelling mistake").into(),
                })
                .collect()),
            Err(_) => {
                // A spelling service is presentation, never canonical mutation authority.
                // Refuse stale/invalid results and expose availability without alert spam.
                self.state.availability = Availability::Unavailable;
                self.state.pending = false;
                self.state.cache.clear();
                Ok(vec![])
            }
        }
    }
}

/// Map only exact UTF-16 boundaries. A native result inside a surrogate is invalid.
fn utf16_to_scalar_range(text: &str, range: Range<usize>) -> Result<Range<usize>, CheckError> {
    let mut utf16 = 0;
    let mut start = None;
    let mut end = None;
    for (scalar, character) in text.chars().enumerate() {
        if utf16 == range.start {
            start = Some(scalar);
        }
        if utf16 == range.end {
            end = Some(scalar);
        }
        utf16 += character.len_utf16();
    }
    if utf16 == range.start {
        start = Some(text.chars().count());
    }
    if utf16 == range.end {
        end = Some(text.chars().count());
    }
    match (start, end) {
        (Some(start), Some(end)) if start < end => Ok(start..end),
        _ => Err(CheckError::InvalidRange),
    }
}

fn visible_chunks(document: &RichTree, visible: &[Selection]) -> Result<Vec<Chunk>, CheckError> {
    let mut windows: BTreeMap<&str, Range<usize>> = BTreeMap::new();
    for range in visible {
        if range.anchor.block_id != range.head.block_id {
            continue;
        }
        let a = range.anchor.offset.0.min(range.head.offset.0);
        let b = range.anchor.offset.0.max(range.head.offset.0);
        windows
            .entry(&range.anchor.block_id)
            .and_modify(|known| {
                known.start = known.start.min(a);
                known.end = known.end.max(b);
            })
            .or_insert(a..b);
    }
    let mut output = Vec::new();
    for (id, window) in windows {
        let node = document.block(id).map_err(|_| CheckError::InvalidRange)?;
        if !matches!(node.node_type.as_str(), "paragraph" | "heading") {
            continue;
        }
        let left = window.start.saturating_sub(96);
        let right = window.end.saturating_add(96);
        let (text, reached_end) = projected_window(node, left..right);
        // Do not spellcheck a chopped word at either viewport margin.
        let mut chars: Vec<char> = text.chars().collect();
        let mut start = left;
        if left > 0 {
            let skip = chars
                .iter()
                .position(|character| lexical_boundary(*character))
                .unwrap_or(chars.len());
            chars.drain(..skip);
            start += skip;
        }
        if !reached_end
            && let Some(last) = chars
                .iter()
                .rposition(|character| lexical_boundary(*character))
        {
            chars.truncate(last + 1);
        }
        let mut text = String::new();
        let mut count = 0;
        let mut chunk_start = start;
        for (index, character) in chars.iter().copied().enumerate() {
            text.push(character);
            count += character.len_utf16();
            if count >= CHUNK_UTF16 && lexical_boundary(character) {
                if count <= CHUNK_UTF16 + 192 {
                    output.push(Chunk {
                        block_id: id.into(),
                        start: chunk_start,
                        text: std::mem::take(&mut text),
                    });
                } else {
                    // A pathological unbroken lexical token must not make an
                    // operating-system spelling request unbounded or mark a fragment.
                    text.clear();
                }
                chunk_start = start + index + 1;
                count = 0;
            }
        }
        if !text.trim().is_empty() && count <= CHUNK_UTF16 + 192 {
            output.push(Chunk {
                block_id: id.into(),
                start: chunk_start,
                text,
            });
        }
    }
    Ok(output)
}

// Scheduling boundaries are deliberately independent of language-specific spelling.
// In particular, combining marks stay with their word instead of splitting its chunk.
fn lexical_boundary(character: char) -> bool {
    character.is_whitespace()
        || matches!(
            character,
            '.' | ','
                | ';'
                | ':'
                | '!'
                | '?'
                | '('
                | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | '、'
                | '。'
                | '，'
                | '；'
                | '：'
                | '！'
                | '？'
        )
}

/// Reader scalars retain their canonical positions. Inline code and nontext atoms
/// become spaces in this disposable analysis window, so words cannot join over them.
fn projected_window(node: &Node, window: Range<usize>) -> (String, bool) {
    let mut offset = 0;
    let mut output = String::new();
    let mut reached_end = true;
    for inline in node.children() {
        if offset >= window.end {
            reached_end = false;
            break;
        }
        let excluded = inline
            .marks
            .as_deref()
            .unwrap_or_default()
            .iter()
            .any(|mark| mark.mark_type == "code");
        if inline.node_type == "text" {
            for character in inline.text.as_deref().unwrap_or_default().chars() {
                if offset >= window.end {
                    reached_end = false;
                    break;
                }
                if offset >= window.start {
                    output.push(if excluded { ' ' } else { character });
                }
                offset += 1;
            }
        } else {
            if offset >= window.start {
                output.push(' ');
            }
            offset += 1;
        }
    }
    (output, reached_end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    type TestResult = Result<(), Box<dyn std::error::Error>>;
    fn range(start: usize, end: usize) -> Selection {
        Selection {
            anchor: DocPoint {
                block_id: "p".into(),
                offset: ScalarOffset(start),
            },
            head: DocPoint {
                block_id: "p".into(),
                offset: ScalarOffset(end),
            },
        }
    }
    struct FakeChecker {
        calls: usize,
    }
    impl Checker for FakeChecker {
        fn check(&mut self, text: &str, start: usize) -> Result<Option<Range<usize>>, CheckError> {
            self.calls += 1;
            let prefix: usize = text
                .chars()
                .take_while(|character| *character != 'b')
                .map(char::len_utf16)
                .sum();
            Ok((start <= prefix && text.contains("bad")).then_some(prefix..prefix + 3))
        }
    }
    #[test]
    fn native_utf16_ranges_never_split_an_emoji() {
        assert_eq!(utf16_to_scalar_range("Я😀 bad", 4..7), Ok(3..6));
        assert_eq!(
            utf16_to_scalar_range("Я😀 bad", 2..7),
            Err(CheckError::InvalidRange)
        );
    }
    #[test]
    fn inline_code_and_atoms_keep_offsets_but_never_enter_proofing() -> TestResult {
        let document = RichTree::from_json(
            json!({"type":"doc","attrs":{"schemaVersion":4},"content":[{"type":"paragraph","attrs":{"blockId":"p"},"content":[
                {"type":"text","text":"Я😀 "},{"type":"text","text":"bad","marks":[{"type":"code"}]},
                {"type":"inlineMath","attrs":{"blockId":"math","latex":"bad"}},
                {"type":"text","text":" bad"}
            ]}]}),
        )?;
        let chunks = visible_chunks(&document, &[range(0, 11)]).map_err(|_| "projection")?;
        assert_eq!(chunks[0].text, "Я😀      bad");
        let canonical = document.to_json()?;
        let mut state = ProofingState::default();
        let mut checker = FakeChecker { calls: 0 };
        let marks = state
            .analyze(&document, Revision(0), &[range(0, 11)], true, &mut checker)
            .map_err(|_| "analysis")?;
        assert_eq!(marks, vec![range(8, 11)]);
        assert_eq!(document.to_json()?, canonical);
        Ok(())
    }
    #[test]
    fn cached_results_are_invalidated_by_edit_and_disabled_spellcheck() -> TestResult {
        let old = RichTree::plain("p", "bad text")?;
        let new = RichTree::plain("p", "good text")?;
        let mut state = ProofingState::default();
        let mut checker = FakeChecker { calls: 0 };
        let first = state
            .analyze(&old, Revision(0), &[range(0, 8)], true, &mut checker)
            .map_err(|_| "first")?;
        assert_eq!(first, vec![range(0, 3)]);
        let calls = checker.calls;
        assert_eq!(
            state
                .analyze(&old, Revision(0), &[range(0, 8)], true, &mut checker)
                .map_err(|_| "cached")?,
            first
        );
        assert_eq!(checker.calls, calls);
        assert!(
            state
                .analyze(&new, Revision(1), &[range(0, 9)], true, &mut checker)
                .map_err(|_| "updated")?
                .is_empty()
        );
        assert!(
            state
                .analyze(&old, Revision(2), &[range(0, 8)], false, &mut checker)
                .map_err(|_| "disabled")?
                .is_empty()
        );
        assert!(state.cache.is_empty());
        Ok(())
    }
    #[test]
    fn scheduling_keeps_a_decomposed_word_whole_at_the_chunk_boundary() -> TestResult {
        let text = format!("{} e\u{301}clair next", "a ".repeat(511));
        let document = RichTree::plain("p", &text)?;
        let chunks = visible_chunks(&document, &[range(0, text.chars().count())])
            .map_err(|_| "chunk projection")?;
        assert!(
            chunks
                .iter()
                .any(|chunk| chunk.text.ends_with("e\u{301}clair "))
        );
        assert!(!chunks.iter().any(|chunk| chunk.text.ends_with("e\u{301}")));
        assert_eq!(
            chunks
                .iter()
                .map(|chunk| chunk.text.as_str())
                .collect::<String>(),
            text
        );
        assert_eq!(document.block_text("p")?, text);
        Ok(())
    }

    #[test]
    fn code_blocks_and_nonvisible_content_are_not_analyzed() -> TestResult {
        let document = RichTree::from_json(
            json!({"type":"doc","attrs":{"schemaVersion":1},"content":[
                {"type":"codeBlock","attrs":{"blockId":"p","language":"text"},"content":[{"type":"text","text":"bad"}]}
            ]}),
        )?;
        let mut state = ProofingState::default();
        let mut checker = FakeChecker { calls: 0 };
        assert!(
            state
                .analyze(&document, Revision(0), &[range(0, 3)], true, &mut checker)
                .map_err(|_| "code")?
                .is_empty()
        );
        assert_eq!(checker.calls, 0);
        assert!(
            state
                .analyze(&document, Revision(0), &[], true, &mut checker)
                .map_err(|_| "invisible")?
                .is_empty()
        );
        Ok(())
    }
}
