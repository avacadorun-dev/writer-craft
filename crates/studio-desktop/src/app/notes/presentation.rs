//! Nonoverlapping display segments retain an explicit route to each original exact note.
use super::*;

#[derive(Default)]
pub(super) struct DecorationCache {
    key: Option<Key>,
    items: Vec<TextDecoration>,
    targets: HashMap<String, String>,
    overlaps: bool,
}

#[derive(PartialEq, Eq)]
struct Key {
    revision: Revision,
    epoch: u64,
    selected: Option<String>,
    language: Language,
    tokens: Tokens,
}

struct Leaf {
    id: String,
    length: usize,
}
struct Span {
    start: usize,
    end: usize,
    priority: usize,
}

fn leaves(node: &craft_editor_model::Node, result: &mut Vec<Leaf>) -> AppResult<()> {
    if matches!(
        node.node_type.as_str(),
        "paragraph" | "heading" | "codeBlock"
    ) {
        result.push(Leaf {
            id: node.block_id().ok_or("missing_decoration_leaf")?.into(),
            length: node.inline_text().chars().count(),
        });
    } else {
        for child in node.children() {
            leaves(child, result)?;
        }
    }
    Ok(())
}

impl DecorationCache {
    pub fn has_overlaps(&self) -> bool {
        self.overlaps
    }
    pub fn target(&self, id: &str) -> Option<&str> {
        self.targets.get(id).map(String::as_str)
    }

    pub fn render(
        &mut self,
        frame: &PresentationFrame<'_>,
    ) -> craft_editor_egui::Result<Vec<TextDecoration>> {
        let key = Key {
            revision: frame.revision,
            epoch: frame.epoch,
            selected: frame.selected.map(str::to_owned),
            language: frame.language,
            tokens: frame.tokens,
        };
        if self.key.as_ref() == Some(&key) {
            return Ok(self.items.clone());
        }
        self.key = None;
        self.items.clear();
        self.targets.clear();
        self.overlaps = false;
        let mut text_leaves = Vec::new();
        leaves(frame.document.root(), &mut text_leaves)
            .map_err(|_| craft_editor_egui::Error::InvalidDecoration)?;
        let indices: HashMap<_, _> = text_leaves
            .iter()
            .enumerate()
            .map(|(index, leaf)| (leaf.id.as_str(), index))
            .collect();
        let mut notes: Vec<_> = frame
            .notes
            .iter()
            .filter(|note| {
                note.anchor_status == AnchorStatus::Attached
                    && note.status != AnnotationStatus::Resolved
            })
            .collect();
        notes.sort_by(|a, b| {
            (Some(a.id.as_str()) != frame.selected)
                .cmp(&(Some(b.id.as_str()) != frame.selected))
                .then_with(|| a.id.cmp(&b.id))
        });
        let mut spans: Vec<Vec<Span>> = (0..text_leaves.len()).map(|_| Vec::new()).collect();
        for (priority, note) in notes.iter().enumerate() {
            // This verifies the original full attachment through core's exact block projection.
            let range = selection_for(note, frame.document)
                .map_err(|_| craft_editor_egui::Error::InvalidDecoration)?;
            let first = *indices
                .get(range.anchor.block_id.as_str())
                .ok_or(craft_editor_egui::Error::InvalidDecoration)?;
            let last = *indices
                .get(range.head.block_id.as_str())
                .ok_or(craft_editor_egui::Error::InvalidDecoration)?;
            if first > last {
                return Err(craft_editor_egui::Error::InvalidDecoration);
            }
            for index in first..=last {
                let start = if index == first {
                    range.anchor.offset.0
                } else {
                    0
                };
                let end = if index == last {
                    range.head.offset.0
                } else {
                    text_leaves[index].length
                };
                if start > end || end > text_leaves[index].length {
                    return Err(craft_editor_egui::Error::InvalidDecoration);
                }
                if start != end {
                    spans[index].push(Span {
                        start,
                        end,
                        priority,
                    });
                }
            }
        }
        for (leaf, spans) in text_leaves.iter().zip(spans) {
            let mut boundaries: Vec<_> = spans
                .iter()
                .flat_map(|span| [span.start, span.end])
                .collect();
            boundaries.sort_unstable();
            boundaries.dedup();
            let mut segments: Vec<(usize, usize, usize)> = Vec::new();
            for pair in boundaries.windows(2) {
                self.overlaps |= spans
                    .iter()
                    .filter(|span| span.start <= pair[0] && span.end >= pair[1])
                    .take(2)
                    .count()
                    > 1;
                let Some(priority) = spans
                    .iter()
                    .filter(|span| span.start <= pair[0] && span.end >= pair[1])
                    .map(|span| span.priority)
                    .min()
                else {
                    continue;
                };
                if let Some(previous) = segments.last_mut()
                    && previous.1 == pair[0]
                    && previous.2 == priority
                {
                    previous.1 = pair[1];
                } else {
                    segments.push((pair[0], pair[1], priority));
                }
            }
            for (start, end, priority) in segments {
                let note = notes[priority];
                let id = format!("passage-display-{}", self.items.len());
                self.targets.insert(id.clone(), note.id.clone());
                self.items.push(TextDecoration {
                    id,
                    range: Selection {
                        anchor: DocPoint {
                            block_id: leaf.id.clone(),
                            offset: ScalarOffset(start),
                        },
                        head: DocPoint {
                            block_id: leaf.id.clone(),
                            offset: ScalarOffset(end),
                        },
                    },
                    color: labels::primary(note, frame.catalog, frame.tokens),
                    label: frame.language.format(
                        "Passage markup: {markup}",
                        &[("markup", markup_summary(note, 240))],
                    ),
                });
            }
        }
        self.key = Some(key);
        Ok(self.items.clone())
    }
}

pub(super) struct PresentationFrame<'a> {
    pub document: &'a RichTree,
    pub revision: Revision,
    pub epoch: u64,
    pub notes: &'a [PassageAnnotation],
    pub selected: Option<&'a str>,
    pub catalog: Option<&'a studio_core::ProjectLabelCatalog>,
    pub language: Language,
    pub tokens: Tokens,
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::ProjectLabelColor;

    #[test]
    fn overlapping_blockquote_marks_keep_exact_full_targets_and_refresh_color_without_text_changes()
    -> AppResult<()> {
        let workspace = Workspace::memory()?;
        let project = workspace.create_project("Label presentation")?;
        let tree = RichTree::from_json(
            serde_json::json!({"type":"doc","attrs":{"schemaVersion":1},"content":[{"type":"blockquote","attrs":{"blockId":"q"},"content":[{"type":"paragraph","attrs":{"blockId":"a"},"content":[{"type":"text","text":"echo 😀"}]},{"type":"paragraph","attrs":{"blockId":"b"},"content":[{"type":"text","text":"echo end"}]}]}]}),
        )?;
        let document = workspace.create_rich_document(&project.id, "Scene", tree.to_json()?)?;
        let broad = AnnotationAnchor::from_selection(
            &tree,
            &TextPoint {
                block_id: "a".into(),
                offset: ScalarOffset(0),
            },
            &TextPoint {
                block_id: "b".into(),
                offset: ScalarOffset(4),
            },
        )?;
        let narrow = AnnotationAnchor::from_selection(
            &tree,
            &TextPoint {
                block_id: "b".into(),
                offset: ScalarOffset(0),
            },
            &TextPoint {
                block_id: "b".into(),
                offset: ScalarOffset(4),
            },
        )?;
        let first = workspace.create_tagged_annotation(
            &document.id,
            document.revision,
            broad,
            "Across two leaves",
            &["Motive".into()],
        )?;
        let second = workspace.create_tagged_annotation(
            &document.id,
            document.revision,
            narrow,
            "Second occurrence",
            &["Question".into()],
        )?;
        let catalog = workspace.project_labels(&project.id)?;
        let catalog =
            workspace.set_project_label_color(&catalog, "Motive", ProjectLabelColor::Violet)?;
        let catalog =
            workspace.set_project_label_color(&catalog, "Question", ProjectLabelColor::Orange)?;
        let notes = vec![first.clone(), second.clone()];
        let tokens = crate::theme::tokens(&egui::Context::default());
        let mut cache = DecorationCache::default();
        let frame = PresentationFrame {
            document: &tree,
            revision: Revision(1),
            epoch: 1,
            notes: &notes,
            selected: Some(&second.id),
            catalog: Some(&catalog),
            language: Language::English,
            tokens,
        };
        let marks = cache.render(&frame)?;
        assert!(cache.has_overlaps());
        assert_eq!(marks.len(), 2);
        assert_eq!(
            marks
                .iter()
                .map(|mark| &mark.id)
                .collect::<std::collections::HashSet<_>>()
                .len(),
            marks.len()
        );
        let targeted = marks
            .iter()
            .find(|mark| mark.range.anchor.block_id == "b")
            .ok_or("missing second leaf mark")?;
        assert_eq!(targeted.range.anchor.offset, ScalarOffset(0));
        assert_eq!(targeted.range.head.offset, ScalarOffset(4));
        assert_eq!(cache.target(&targeted.id), Some(second.id.as_str()));
        assert_eq!(
            targeted.color,
            labels::color(ProjectLabelColor::Orange, tokens)
        );
        let recolored =
            workspace.set_project_label_color(&catalog, "Question", ProjectLabelColor::Rose)?;
        let recolored_frame = PresentationFrame {
            epoch: 2,
            catalog: Some(&recolored),
            ..frame
        };
        let fresh = cache.render(&recolored_frame)?;
        assert_eq!(
            fresh
                .iter()
                .find(|mark| mark.range.anchor.block_id == "b")
                .ok_or("missing refreshed color")?
                .color,
            labels::color(ProjectLabelColor::Rose, tokens)
        );
        assert_eq!(workspace.annotation(&first.id)?, first);
        assert_eq!(workspace.annotation(&second.id)?, second);
        assert_eq!(workspace.document(&document.id)?, document);
        let full = selection_for(&first, &tree)?;
        assert_eq!(full.anchor.block_id, "a");
        assert_eq!(full.head.block_id, "b");
        Ok(())
    }
}
