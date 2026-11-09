//! Persistent passage-note presentation over guarded core commands and ephemeral native ranges.
mod labels;
mod presentation;
use super::*;
use craft_editor_egui::{
    DocPoint, Selection,
    decorations::{DecorationSource, TextDecoration},
};
use craft_editor_model::{
    BlockTextProjection, Document as RichTree, InlineAtomText, ReplayProof, Revision, ScalarOffset,
    TextPoint, TextProjectionOptions, scalar_to_byte,
};
use std::{collections::HashMap, sync::Arc};
use studio_core::{
    AnchorStatus, AnnotationAnchor, AnnotationPatch, AnnotationPreviewSnapshot, AnnotationStatus,
    PassageAnnotation,
};

#[derive(Clone)]
struct NoteDraft {
    original: PassageAnnotation,
    body: String,
    tags: String,
    catalog: Option<studio_core::ProjectLabelCatalog>,
}
struct NoteComposer {
    id: String,
    document_id: String,
    base_revision: i64,
    anchor: AnnotationAnchor,
    body: String,
    tags: String,
    catalog: Option<studio_core::ProjectLabelCatalog>,
    focus: bool,
    dismissed: bool,
}
impl NoteDraft {
    fn changed(&self) -> bool {
        self.body.trim() != self.original.note.trim() || self.tags != self.original.tags.join("\n")
    }
}
impl NoteComposer {
    fn has_content(&self) -> bool {
        !self.body.trim().is_empty() || !self.tags.trim().is_empty()
    }
}

struct ProjectMarksBrowser {
    project_id: String,
    records: Vec<studio_core::ProjectAnnotationSummary>,
    available_tags: Vec<String>,
    visible: Vec<usize>,
    filter_key: Option<(String, Option<String>, bool)>,
    paths: HashMap<String, String>,
    trashed: std::collections::HashSet<String>,
    query: String,
    tag: Option<String>,
    deleted: bool,
    open: bool,
    tag_action: Option<TagAction>,
    restore_confirmation: Option<studio_core::ProjectAnnotationSummary>,
}
struct TagAction {
    snapshot: studio_core::ProjectTagSnapshot,
    replacement: String,
    remove: bool,
    focus: bool,
    pending_drafts: Vec<(String, String)>,
}
enum ProjectMarkupAction {
    Open,
    Restore,
}

#[derive(Default)]
pub(super) struct NotesState {
    snapshot: Option<Arc<AnnotationPreviewSnapshot>>,
    preview: Vec<PassageAnnotation>,
    preview_key: Option<(Revision, u64)>,
    epoch: u64,
    invalid: bool,
    selected: Option<String>,
    drafts: HashMap<String, NoteDraft>,
    composer: Option<NoteComposer>,
    paused_composers: HashMap<String, NoteComposer>,
    failed_context: Option<(String, i64)>,
    close_confirmation: bool,
    delete_confirmation: Option<PassageAnnotation>,
    reveal_row: bool,
    last_error: Option<Box<dyn std::error::Error + Send + Sync>>,
    reported_error_epoch: Option<u64>,
    project_browser: Option<ProjectMarksBrowser>,
    labels: labels::LabelUi,
    decoration_cache: presentation::DecorationCache,
}
impl NotesState {
    pub(super) fn clear(&mut self) {
        let mut paused_composers = std::mem::take(&mut self.paused_composers);
        if let Some(composer) = self.composer.take() {
            paused_composers.insert(composer.document_id.clone(), composer);
        }
        let mut drafts = std::mem::take(&mut self.drafts);
        drafts.retain(|_, draft| draft.changed());
        *self = Self {
            drafts,
            paused_composers,
            project_browser: self.project_browser.take(),
            labels: std::mem::take(&mut self.labels),
            ..Self::default()
        };
    }
    pub(super) fn modal_open(&self) -> bool {
        self.labels.manager
            || self.labels.editor.is_some()
            || self.composer.is_some()
            || self.delete_confirmation.is_some()
            || self.close_confirmation
            || self
                .project_browser
                .as_ref()
                .is_some_and(|browser| browser.open)
    }
    pub(super) fn has_unsaved_drafts(&self) -> bool {
        self.drafts.values().any(|draft| draft.changed())
            || self
                .composer
                .as_ref()
                .is_some_and(|composer| composer.has_content())
            || self
                .paused_composers
                .values()
                .any(|composer| composer.has_content())
    }
    /// Return only document identities, never note bodies, for an explicit save/review prerequisite.
    pub(super) fn pending_documents(&self) -> Vec<String> {
        let mut documents = std::collections::BTreeSet::new();
        for draft in self.drafts.values().filter(|draft| draft.changed()) {
            documents.insert(draft.original.document_id.clone());
        }
        for composer in self.composer.iter().chain(self.paused_composers.values()) {
            if composer.has_content() {
                documents.insert(composer.document_id.clone());
            }
        }
        documents.into_iter().collect()
    }

    pub(super) fn select_pending_draft(&mut self, document_id: &str) {
        if self.composer.is_none()
            && let Some(mut composer) = self.paused_composers.remove(document_id)
        {
            composer.dismissed = false;
            composer.focus = true;
            self.composer = Some(composer);
        }
        self.selected = self
            .drafts
            .iter()
            .find(|(_, draft)| draft.original.document_id == document_id && draft.changed())
            .map(|(id, _)| id.clone());
        self.reveal_row = self.selected.is_some();
    }

    pub(super) fn request_close_confirmation(&mut self) {
        self.close_confirmation = true;
    }
    pub(super) fn close_confirmation_open(&self) -> bool {
        self.close_confirmation
    }
    pub(super) fn invalidate(&mut self) {
        self.invalid = true;
        self.failed_context = None;
        self.snapshot = None;
        self.preview_key = None;
    }

    /// Call outside canvas/input resolution after open, acknowledged save or note mutation.
    pub(super) fn prepare(&mut self, workspace: &Workspace, document: &Document) -> AppResult<()> {
        if document.rich_json.is_none() {
            self.clear();
            return Ok(());
        }
        if self
            .labels
            .catalog
            .as_ref()
            .is_none_or(|catalog| catalog.project_id() != document.project_id)
        {
            self.labels.catalog = None;
            match workspace.project_labels(&document.project_id) {
                Ok(catalog) => self.labels.catalog = Some(catalog),
                Err(error) => {
                    self.invalid = true;
                    self.snapshot = None;
                    return Err(error.into());
                }
            }
        }
        if self.snapshot.as_ref().is_some_and(|snapshot| {
            snapshot.document_id() == document.id && snapshot.base_revision() == document.revision
        }) {
            return Ok(());
        }
        if self
            .failed_context
            .as_ref()
            .is_some_and(|(id, revision)| id == &document.id && *revision == document.revision)
        {
            return Ok(());
        }
        let prior_revision = self
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.base_revision());
        let prior_id = self
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.document_id().to_owned());
        if prior_id.as_deref().is_some_and(|id| id != document.id)
            || self
                .preview
                .iter()
                .any(|note| note.document_id != document.id)
        {
            self.selected = None;
            self.preview.clear();
            self.preview_key = None;
            if let Some(composer) = self.composer.take() {
                self.paused_composers
                    .insert(composer.document_id.clone(), composer);
            }
            self.delete_confirmation = None;
        }
        let snapshot = match workspace.annotation_preview_snapshot(&document.id, document.revision)
        {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.invalid = true;
                self.snapshot = None;
                self.failed_context = Some((document.id.clone(), document.revision));
                return Err(error.into());
            }
        };
        if self.composer.is_none()
            && self
                .paused_composers
                .get(&document.id)
                .is_some_and(|composer| !composer.dismissed)
        {
            self.composer = self.paused_composers.remove(&document.id);
        }
        self.drafts.retain(|id, draft| {
            draft.original.document_id != document.id
                || draft.changed()
                || snapshot.stored_notes().iter().any(|note| &note.id == id)
        });
        for note in snapshot.stored_notes() {
            if let Some(draft) = self.drafts.get_mut(&note.id)
                && !draft.changed()
            {
                draft.original = note.clone();
                draft.body.clone_from(&note.note);
                draft.tags = note.tags.join("\n");
                draft.catalog = self.labels.catalog.clone();
            }
        }
        // A guarded local canonical save advances every note once. Rebase the
        // editor's metadata guard only when authored fields stayed identical.
        if prior_revision.is_some_and(|revision| revision.checked_add(1) == Some(document.revision))
        {
            for note in snapshot.stored_notes() {
                if let Some(draft) = self.drafts.get_mut(&note.id)
                    && draft.original.revision.checked_add(1) == Some(note.revision)
                    && draft.original.note == note.note
                    && draft.original.tags == note.tags
                    && draft.original.status == note.status
                {
                    draft.original = note.clone();
                }
            }
        }
        self.preview = snapshot.stored_notes().to_vec();
        self.snapshot = Some(Arc::new(snapshot));
        self.epoch = self
            .epoch
            .checked_add(1)
            .ok_or("note_preview_epoch_overflow")?;
        self.preview_key = None;
        self.invalid = false;
        self.failed_context = None;
        Ok(())
    }
    pub(super) fn frame<'a>(
        &'a mut self,
        proofs: &'a [ReplayProof],
        language: Language,
        tokens: Tokens,
    ) -> NotesFrame<'a> {
        NotesFrame {
            snapshot: self.snapshot.clone(),
            state: self,
            prior: proofs,
            language,
            tokens,
        }
    }
    pub(super) fn activate(&mut self, id: &str) {
        self.selected = Some(id.into());
        self.reveal_row = true;
    }
    fn acknowledge(&mut self, note: &PassageAnnotation) {
        if let Some(draft) = self.drafts.get_mut(&note.id)
            && draft.original.note == note.note
            && draft.original.tags == note.tags
        {
            draft.original = note.clone();
        }
        self.invalidate();
    }
}

pub(super) struct NotesFrame<'a> {
    state: &'a mut NotesState,
    snapshot: Option<Arc<AnnotationPreviewSnapshot>>,
    prior: &'a [ReplayProof],
    language: Language,
    tokens: Tokens,
}
impl NotesFrame<'_> {
    pub(super) fn take_error(&mut self) -> Option<Box<dyn std::error::Error + Send + Sync>> {
        self.state.last_error.take()
    }
}
impl DecorationSource for NotesFrame<'_> {
    fn decorations(
        &mut self,
        document: &RichTree,
        revision: Revision,
        pending: &[ReplayProof],
    ) -> craft_editor_egui::Result<Vec<TextDecoration>> {
        let key = (revision, self.state.epoch);
        if self.snapshot.is_none() {
            // Preparation already reported this context's acquisition failure.
            // Typing must not create a fresh modal for every new input revision.
            self.state.invalid = true;
            return Err(craft_editor_egui::Error::DecorationSource);
        }
        if self.state.preview_key != Some(key) {
            self.state.preview_key = Some(key);
            let result = (|| -> AppResult<Vec<PassageAnnotation>> {
                let snapshot = self.snapshot.as_ref().ok_or("note_preview_not_prepared")?;
                if snapshot.stored_notes().is_empty() {
                    return Ok(vec![]);
                }
                let proofs: Vec<_> = self.prior.iter().chain(pending).cloned().collect();
                Ok(snapshot.preview(&proofs, document.to_json()?)?)
            })();
            match result {
                Ok(notes) => {
                    self.state.preview = notes;
                    self.state.invalid = false;
                    self.state.reported_error_epoch = None;
                }
                Err(error) => {
                    self.state.invalid = true;
                    if self.state.reported_error_epoch != Some(self.state.epoch) {
                        self.state.last_error = Some(error);
                        self.state.reported_error_epoch = Some(self.state.epoch);
                    }
                }
            }
        }
        if self.state.invalid {
            return Err(craft_editor_egui::Error::DecorationSource);
        }
        self.state
            .decoration_cache
            .render(&presentation::PresentationFrame {
                document,
                revision,
                epoch: self.state.epoch,
                notes: &self.state.preview,
                selected: self.state.selected.as_deref(),
                catalog: self.state.labels.catalog.as_ref(),
                language: self.language,
                tokens: self.tokens,
            })
    }
}

fn selection_for(note: &PassageAnnotation, tree: &RichTree) -> AppResult<Selection> {
    let (first, last) = note.anchor.selection(tree)?;
    Ok(Selection {
        anchor: DocPoint {
            block_id: first.block_id,
            offset: first.offset,
        },
        head: DocPoint {
            block_id: last.block_id,
            offset: last.offset,
        },
    })
}
fn note_input(
    ui: &mut egui::Ui,
    body: &mut String,
    id: egui::Id,
    l: Language,
    t: Tokens,
    read_only: bool,
) -> egui::Response {
    let rows = body.lines().count().clamp(2, 6);
    let response = if read_only {
        let mut text: &str = body;
        ui.add(
            egui::TextEdit::multiline(&mut text)
                .id(id)
                .desired_width(f32::INFINITY)
                .desired_rows(rows)
                .margin(12),
        )
    } else {
        ui.add(
            egui::TextEdit::multiline(body)
                .id(id)
                .desired_width(f32::INFINITY)
                .desired_rows(rows)
                .margin(12),
        )
    };
    note_length(ui, body, l, t);
    response
}
fn note_row(
    ui: &mut egui::Ui,
    note: &PassageAnnotation,
    quote: &str,
    selected: bool,
    catalog: Option<&studio_core::ProjectLabelCatalog>,
    l: Language,
    t: Tokens,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 72.0), egui::Sense::click());
    if selected || response.hovered() {
        ui.painter()
            .rect_filled(rect, 5.0, if selected { t.accent_soft } else { t.surface });
    }
    ui.painter().rect_filled(
        egui::Rect::from_min_size(
            rect.left_top() + egui::vec2(1.0, 7.0),
            egui::vec2(3.0, rect.height() - 14.0),
        ),
        2.0,
        labels::primary(note, catalog, t),
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(egui::vec2(9.0, 5.0)))
            .layout(Layout::top_down(Align::Min)),
        |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.horizontal(|ui| {
                if note.anchor_status == AnchorStatus::Detached
                    || note.status == AnnotationStatus::NeedsAttention
                {
                    icons::icon(ui, Icon::Warning, 15.0, t.warning)
                        .on_hover_text(l.text("Detached passage — original quote retained"));
                } else if note.status == AnnotationStatus::Resolved {
                    icons::icon(ui, Icon::Check, 15.0, t.muted).on_hover_text(l.text("Resolved"));
                }
                let primary = if note.note.is_empty() {
                    quote
                } else {
                    &note.note
                };
                ui.add(
                    egui::Label::new(
                        RichText::new(excerpt(primary, 100))
                            .size(13.0)
                            .strong()
                            .color(t.text),
                    )
                    .selectable(false)
                    .truncate(),
                );
            });
            if !note.note.is_empty() {
                ui.add(
                    egui::Label::new(RichText::new(excerpt(quote, 100)).size(12.0).color(t.muted))
                        .selectable(false)
                        .truncate(),
                );
            }
            if !note.tags.is_empty() {
                labels::preview_chips(ui, &note.tags, catalog, l, t);
            }
        },
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            format!("{}: {}", quote, markup_summary(note, 240)),
        )
    });
    response.on_hover_text(format!(
        "{}\n{}\n{}",
        note.note,
        quote,
        note.tags.join(" · ")
    ))
}

fn markup_summary(note: &PassageAnnotation, limit: usize) -> String {
    let tags = note.tags.join(" · ");
    if note.note.is_empty() {
        tags
    } else if tags.is_empty() {
        excerpt(&note.note, limit)
    } else {
        format!("{} · {}", tags, excerpt(&note.note, limit))
    }
}
fn project_markup_row(
    ui: &mut egui::Ui,
    row: &studio_core::ProjectAnnotationSummary,
    path: &str,
    trashed: bool,
    catalog: Option<&studio_core::ProjectLabelCatalog>,
    l: Language,
    t: Tokens,
) -> Option<ProjectMarkupAction> {
    let note = &row.annotation;
    let mut action = None;
    let button_width = (ui.available_width() * 0.34).clamp(110.0, 200.0);
    let body_width = (ui.available_width() - button_width - ui.spacing().item_spacing.x).max(1.0);
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(body_width, 116.0),
            Layout::top_down(Align::Min),
            |ui| {
                ui.add(egui::Label::new(RichText::new(path).strong()).truncate())
                    .on_hover_text(path);
                ui.add(
                    egui::Label::new(excerpt(&displayed_quote(&note.anchor, None, l), 100))
                        .truncate(),
                );
                if !note.note.is_empty() {
                    ui.add(
                        egui::Label::new(
                            RichText::new(excerpt(&note.note, 90))
                                .size(12.0)
                                .color(t.text),
                        )
                        .truncate(),
                    );
                }
                if !note.tags.is_empty() {
                    labels::preview_chips(ui, &note.tags, catalog, l, t);
                }
                let warning = if trashed {
                    Some("Restore this document from Trash to open its markup.")
                } else if note.anchor_status == AnchorStatus::Detached {
                    Some("Detached passage — original quote retained")
                } else {
                    None
                };
                if let Some(warning) = warning {
                    ui.add(
                        egui::Label::new(
                            RichText::new(l.text(warning)).size(12.0).color(t.warning),
                        )
                        .truncate(),
                    )
                    .on_hover_text(l.text(warning));
                }
            },
        );
        ui.vertical(|ui| {
            ui.add_space(36.0);
            let label = l.text(if note.deleted {
                "Restore markup"
            } else if note.anchor_status == AnchorStatus::Attached {
                "Show passage"
            } else {
                "Open note"
            });
            let clicked = ui
                .add_enabled_ui(!trashed, |ui| {
                    ui.add_sized([button_width, 40.0], egui::Button::new(label).truncate())
                        .on_hover_text(label)
                        .clicked()
                })
                .inner;
            if clicked {
                action = Some(if note.deleted {
                    ProjectMarkupAction::Restore
                } else {
                    ProjectMarkupAction::Open
                });
            }
        });
    });
    ui.separator();
    action
}
fn visible_menu_anchor(visible: egui::Rect, caret: Option<egui::Rect>) -> egui::Pos2 {
    let inset = 4.0_f32
        .min(visible.width() * 0.25)
        .min(visible.height() * 0.25);
    visible.shrink(inset).clamp(caret.map_or_else(
        || visible.center(),
        |caret| caret.left_bottom() + egui::vec2(0.0, 4.0),
    ))
}
fn valid_note_body(body: &str) -> bool {
    !body.trim().is_empty()
        && body.trim().chars().count() <= studio_core::MAX_PROJECT_ANNOTATION_NOTE_CHARACTERS
}
fn parsed_tags(source: &str) -> Vec<String> {
    source
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_owned)
        .collect()
}
fn valid_tags(tags: &[String]) -> bool {
    let mut seen = std::collections::HashSet::new();
    tags.len() <= 32
        && tags.iter().all(|tag| {
            !tag.trim().is_empty()
                && tag.chars().count() <= 80
                && !tag.chars().any(char::is_control)
                && seen.insert(tag)
        })
}
fn valid_markup(body: &str, source: &str) -> bool {
    let tags = parsed_tags(source);
    valid_tags(&tags)
        && if tags.is_empty() {
            valid_note_body(body)
        } else {
            body.trim().chars().count() <= studio_core::MAX_PROJECT_ANNOTATION_NOTE_CHARACTERS
        }
}

fn note_length(ui: &mut egui::Ui, body: &str, l: Language, t: Tokens) {
    let count = body.trim().chars().count();
    let limit = studio_core::MAX_PROJECT_ANNOTATION_NOTE_CHARACTERS;
    if count < limit * 9 / 10 {
        return;
    }
    ui.label(
        RichText::new(l.format(
            "{count} / {limit} characters",
            &[("count", count.to_string()), ("limit", limit.to_string())],
        ))
        .size(12.0)
        .color(if count > limit { t.warning } else { t.muted }),
    );
}
fn displayed_quote(anchor: &AnnotationAnchor, tree: Option<&RichTree>, l: Language) -> String {
    let displayed = tree.and_then(|tree| {
        let (a, b) = anchor.selection(tree).ok()?;
        let projection = BlockTextProjection::with_options(
            tree,
            &anchor.block_id,
            TextProjectionOptions {
                block_separator: "\n",
                hard_break: "\n",
                inline_math: InlineAtomText::WithAffixes {
                    prefix: "$",
                    suffix: "$",
                },
                block_atom: "",
            },
        )
        .ok()?;
        let start = projection.offset(&a).ok()?;
        let end = projection.offset(&b).ok()?;
        let start = scalar_to_byte(projection.text(), start).ok()?.0;
        let end = scalar_to_byte(projection.text(), end).ok()?.0;
        projection.text().get(start..end).map(str::to_owned)
    });
    displayed.unwrap_or_else(|| {
        anchor
            .exact_text
            .replace('\u{fffc}', l.text("[Embedded content]"))
    })
}
fn excerpt(text: &str, limit: usize) -> String {
    let mut result: String = text.chars().take(limit).collect();
    if text.chars().count() > limit {
        result.push('…');
    }
    result
}
fn captured_selection(
    session: &craft_editor_egui::EditorSession,
) -> AppResult<(String, ScalarOffset, ScalarOffset, String)> {
    if session.is_composing() {
        return Err("annotation_composition_active".into());
    }
    let selection = session
        .selection()
        .ok_or("annotation_requires_text_selection")?;
    let anchor = AnnotationAnchor::from_selection(
        session.document(),
        &TextPoint {
            block_id: selection.anchor.block_id.clone(),
            offset: selection.anchor.offset,
        },
        &TextPoint {
            block_id: selection.head.block_id.clone(),
            offset: selection.head.offset,
        },
    )?;
    Ok((anchor.block_id, anchor.start, anchor.end, anchor.exact_text))
}

impl StudioApp {
    fn ensure_note_writable(&self) -> AppResult<()> {
        if self.is_recovery_read_only() {
            return Err(recovery::ReadOnlyRecoveryView.into());
        }
        if !self.can_edit_manuscript() {
            return Err(studio_core::StudioError::ReadOnly {
                document_id: self
                    .document
                    .as_ref()
                    .map(|document| document.id.clone())
                    .unwrap_or_default(),
            }
            .into());
        }
        Ok(())
    }
    pub(super) fn begin_note(&mut self) -> AppResult<()> {
        self.ensure_note_writable()?;
        if self.notes.invalid {
            return Err("annotation_preview_unverified".into());
        }
        if let Some(composer) = &mut self.notes.composer {
            composer.focus = true;
            return Ok(());
        }
        self.save()?;
        let document = self.document.as_ref().ok_or("missing_note_document")?;
        if let Some(mut composer) = self.notes.paused_composers.remove(&document.id) {
            composer.dismissed = false;
            composer.focus = true;
            self.notes.composer = Some(composer);
            return Ok(());
        }
        let session = &self.rich.as_ref().ok_or("missing_note_editor")?.session;
        let (id, start, end, quote) = captured_selection(session)?;
        let anchor = self.workspace.capture_annotation_anchor(
            &document.id,
            document.revision,
            &id,
            start,
            end,
            &quote,
        )?;
        self.notes.composer = Some(NoteComposer {
            id: uuid::Uuid::new_v4().to_string(),
            document_id: document.id.clone(),
            base_revision: document.revision,
            anchor,
            body: String::new(),
            tags: String::new(),
            catalog: self.notes.labels.catalog.clone(),
            focus: true,
            dismissed: false,
        });
        Ok(())
    }
    pub(super) fn open_mark_note(&mut self, id: &str) -> AppResult<()> {
        let canonical = self
            .notes
            .decoration_cache
            .target(id)
            .map(str::to_owned)
            .or_else(|| {
                self.notes
                    .preview
                    .iter()
                    .find(|note| note.id == id)
                    .map(|note| note.id.clone())
            })
            .ok_or("passage_decoration_context_changed")?;
        if self.notes.selected.as_deref() == Some(canonical.as_str()) {
            self.inspector = Inspector::Notes;
            self.inspector_visible = true;
            self.prefer_inspector = true;
            self.focus_mode = false;
            return Ok(());
        }
        self.activate_note(&canonical)
    }
    pub(super) fn save_focused_note(&mut self, ctx: &egui::Context) -> Option<AppResult<()>> {
        let focused = ctx.memory(|memory| memory.focused());
        let id = self
            .notes
            .drafts
            .keys()
            .find(|id| {
                focused == Some(egui::Id::new(("author-note-body", *id)))
                    || focused == Some(egui::Id::new(("author-note-tags", *id)))
            })?
            .clone();
        Some(self.save_note_draft(&id))
    }
    pub(super) fn activate_note(&mut self, id: &str) -> AppResult<()> {
        self.notes.activate(id);
        self.inspector = Inspector::Notes;
        self.inspector_visible = true;
        self.prefer_inspector = true;
        self.focus_mode = false;
        if self.notes.invalid {
            return Ok(());
        }
        if let Some(note) = self
            .notes
            .preview
            .iter()
            .find(|note| note.id == id && note.anchor_status == AnchorStatus::Attached)
            && let Some(rich) = &mut self.rich
        {
            rich.session
                .set_selection(selection_for(note, rich.session.document())?)?;
            rich.session.request_scroll_to_caret();
            self.focus_manuscript = true;
        }
        Ok(())
    }
    /// Explicit Save only; partial conflicts keep all still-unsaved bodies reachable.
    pub(super) fn save_note_drafts(&mut self) -> AppResult<()> {
        let blocked_document = self
            .is_recovery_read_only()
            .then(|| self.document.as_ref().map(|document| document.id.clone()))
            .flatten();
        let mut blocked = false;
        if blocked_document.is_none() {
            self.save()?;
        }
        let drafts: Vec<_> = self
            .notes
            .drafts
            .iter()
            .filter(|(_, draft)| draft.changed())
            .map(|(id, draft)| (id.clone(), draft.clone()))
            .collect();
        self.notes.invalidate();
        for (id, draft) in drafts {
            if blocked_document.as_ref() == Some(&draft.original.document_id) {
                blocked = true;
                continue;
            }
            let document = self.workspace.document(&draft.original.document_id)?;
            self.persist_note_draft(&id, document.revision, draft.original.revision, &draft)?;
            self.notes.drafts.remove(&id);
        }
        let active = self
            .notes
            .composer
            .as_ref()
            .filter(|composer| composer.has_content())
            .map(|composer| {
                (
                    composer.document_id.clone(),
                    composer.base_revision,
                    composer.anchor.clone(),
                    composer.body.clone(),
                    parsed_tags(&composer.tags),
                    composer.catalog.clone(),
                )
            });
        if let Some((id, revision, anchor, body, tags, catalog)) = active {
            if blocked_document.as_ref() == Some(&id) {
                blocked = true;
            } else {
                self.workspace.create_tagged_annotation_with_labels(
                    &id,
                    revision,
                    anchor,
                    &body,
                    &tags,
                    catalog.as_ref().ok_or("missing_composer_label_catalog")?,
                )?;
                self.notes.composer = None;
            }
        }
        let paused: Vec<_> = self
            .notes
            .paused_composers
            .iter()
            .filter(|(_, composer)| composer.has_content())
            .map(|(key, composer)| {
                (
                    key.clone(),
                    composer.document_id.clone(),
                    composer.base_revision,
                    composer.anchor.clone(),
                    composer.body.clone(),
                    parsed_tags(&composer.tags),
                    composer.catalog.clone(),
                )
            })
            .collect();
        for (key, id, revision, anchor, body, tags, catalog) in paused {
            if blocked_document.as_ref() == Some(&id) {
                blocked = true;
                continue;
            }
            self.workspace.create_tagged_annotation_with_labels(
                &id,
                revision,
                anchor,
                &body,
                &tags,
                catalog.as_ref().ok_or("missing_composer_label_catalog")?,
            )?;
            self.notes.paused_composers.remove(&key);
        }
        if self
            .document
            .as_ref()
            .is_some_and(|document| document.rich_json.is_some())
        {
            self.refresh_notes()?;
        }
        if blocked {
            return Err(recovery::ReadOnlyRecoveryView.into());
        }
        Ok(())
    }
    /// Called only after explicit Discard in the close confirmation.
    pub(super) fn discard_note_drafts(&mut self, ctx: &egui::Context) {
        for id in self.notes.drafts.keys() {
            egui::text_edit::TextEditState::default()
                .store(ctx, egui::Id::new(("author-note-body", id)));
            egui::text_edit::TextEditState::default()
                .store(ctx, egui::Id::new(("author-note-tags", id)));
        }
        if let Some(composer) = &self.notes.composer {
            egui::text_edit::TextEditState::default()
                .store(ctx, egui::Id::new(("note-composer-body", &composer.id)));
            egui::text_edit::TextEditState::default()
                .store(ctx, egui::Id::new(("note-composer-tags", &composer.id)));
        }
        for composer in self.notes.paused_composers.values() {
            egui::text_edit::TextEditState::default()
                .store(ctx, egui::Id::new(("note-composer-body", &composer.id)));
            egui::text_edit::TextEditState::default()
                .store(ctx, egui::Id::new(("note-composer-tags", &composer.id)));
        }
        self.notes.drafts.clear();
        self.notes.composer = None;
        self.notes.paused_composers.clear();
    }
    fn confirm_notes_close(&mut self, ctx: &egui::Context, t: Tokens) {
        let l = self.language;
        let mut save = false;
        let mut discard = false;
        let mut cancel = false;
        let response = components::modal(
            ctx,
            l,
            "unsaved-note-close",
            l.text("Save note drafts before closing?"),
            540.0,
            |ui| {
                ui.label(
                    l.text("Choose whether to save your note drafts before closing WriterCraft."),
                );
                components::footer(ui, |ui| {
                    save = components::action(ui, l.text("Save drafts"), true, Some(t)).clicked();
                    discard =
                        components::action(ui, l.text("Discard drafts"), true, None).clicked();
                    cancel = components::action(ui, l.text("Cancel"), true, None).clicked();
                });
            },
        );
        if save {
            let result = self.save_note_drafts();
            if result.is_ok() {
                self.notes.close_confirmation = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            self.record_error(result);
        } else if discard {
            self.discard_note_drafts(ctx);
            self.notes.close_confirmation = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if cancel || response.should_close() {
            self.notes.close_confirmation = false;
            if let Some(composer) = &mut self.notes.composer {
                composer.focus = true;
            }
        }
    }
    fn note_guard_after_flush(&mut self, before: &PassageAnnotation) -> AppResult<(i64, i64)> {
        self.ensure_note_writable()?;
        if self.notes.invalid {
            return Err("annotation_preview_unverified".into());
        }
        let prior = self.document.as_ref().ok_or("missing_note_document")?;
        if prior.id != before.document_id || prior.revision != before.base_revision {
            return Err("annotation_document_context_changed".into());
        }
        let prior_revision = prior.revision;
        self.save()?;
        let current = self.document.as_ref().ok_or("missing_note_document")?;
        let steps = current
            .revision
            .checked_sub(prior_revision)
            .ok_or("annotation_revision_moved_backwards")?;
        if steps > 1 {
            return Err("annotation_document_context_changed".into());
        }
        let expected = before
            .revision
            .checked_add(steps)
            .ok_or("annotation_revision_overflow")?;
        let stored = self.workspace.annotation(&before.id)?;
        if stored.revision != expected {
            return Err(studio_core::StudioError::AnnotationConflict {
                expected,
                actual: stored.revision,
            }
            .into());
        }
        if stored.note != before.note
            || stored.tags != before.tags
            || stored.status != before.status
        {
            return Err("annotation_authored_metadata_changed".into());
        }
        Ok((current.revision, expected))
    }
    fn refresh_notes(&mut self) -> AppResult<()> {
        self.refresh_label_catalog()?;
        let document = self.document.as_ref().ok_or("missing_note_document")?;
        self.notes.invalidate();
        self.notes.prepare(&self.workspace, document)
    }
    fn save_note_draft(&mut self, id: &str) -> AppResult<()> {
        self.refresh_label_catalog()?;
        let draft = self
            .notes
            .drafts
            .get(id)
            .cloned()
            .ok_or("missing_note_draft")?;
        let (doc_revision, note_revision) = self.note_guard_after_flush(&draft.original)?;
        let note = self.persist_note_draft(id, doc_revision, note_revision, &draft)?;
        self.notes.drafts.remove(id);
        self.notes.acknowledge(&note);
        self.refresh_notes()
    }
    fn persist_note_draft(
        &self,
        id: &str,
        doc_revision: i64,
        note_revision: i64,
        draft: &NoteDraft,
    ) -> AppResult<PassageAnnotation> {
        let tags = parsed_tags(&draft.tags);
        let patch = AnnotationPatch {
            note: Some(draft.body.clone()),
            status: None,
        };
        if tags == draft.original.tags {
            Ok(self
                .workspace
                .update_annotation(id, doc_revision, note_revision, patch)?)
        } else {
            Ok(self.workspace.update_annotation_markup_with_labels(
                id,
                doc_revision,
                note_revision,
                patch,
                Some(tags),
                draft
                    .catalog
                    .as_ref()
                    .ok_or("missing_draft_label_catalog")?,
            )?)
        }
    }
    fn set_note_status(
        &mut self,
        note: &PassageAnnotation,
        status: AnnotationStatus,
    ) -> AppResult<()> {
        let (doc_revision, note_revision) = self.note_guard_after_flush(note)?;
        let updated = self.workspace.update_annotation(
            &note.id,
            doc_revision,
            note_revision,
            AnnotationPatch {
                note: None,
                status: Some(status),
            },
        )?;
        self.notes.acknowledge(&updated);
        self.refresh_notes()
    }
    fn reattach_note_selection(&mut self, note: &PassageAnnotation) -> AppResult<()> {
        let (doc_revision, note_revision) = self.note_guard_after_flush(note)?;
        let session = &self.rich.as_ref().ok_or("missing_note_editor")?.session;
        let (id, start, end, quote) = captured_selection(session)?;
        let anchor = self.workspace.capture_annotation_anchor(
            &note.document_id,
            doc_revision,
            &id,
            start,
            end,
            &quote,
        )?;
        let updated =
            self.workspace
                .reattach_annotation(&note.id, doc_revision, note_revision, anchor)?;
        self.notes.acknowledge(&updated);
        self.refresh_notes()
    }
    fn note_action_unavailable_reason(&self) -> Option<&'static str> {
        if !self.can_edit_manuscript() {
            return Some("Read only");
        }
        if self.notes.invalid {
            return Some("Notes could not be refreshed. Your text is preserved.");
        }
        let Some(rich) = &self.rich else {
            return Some("Select a passage to add a note.");
        };
        if rich.session.is_composing() {
            return Some("Finish text composition before adding a note.");
        }
        captured_selection(&rich.session)
            .err()
            .map(|_| "Select a passage within one text block to add a note.")
    }

    pub(super) fn rich_note_selection_menu(&mut self, response: &egui::Response) {
        let l = self.language;
        let reason = self.note_action_unavailable_reason();
        let allowed = reason.is_none();
        let keyboard = response.has_focus()
            && response
                .ctx
                .input_mut(|input| input.consume_key(egui::Modifiers::SHIFT, egui::Key::F10));
        let popup_id = response.id.with("rich-selection-menu");
        let keyboard_id = popup_id.with("keyboard-anchor");
        if response.secondary_clicked() {
            response
                .ctx
                .data_mut(|data| data.remove::<bool>(keyboard_id));
        }
        if keyboard {
            response
                .ctx
                .data_mut(|data| data.insert_temp(keyboard_id, true));
            egui::Popup::open_id(&response.ctx, popup_id);
        }
        if !egui::Popup::is_id_open(&response.ctx, popup_id) {
            response
                .ctx
                .data_mut(|data| data.remove::<bool>(keyboard_id));
        }
        let mut popup = egui::Popup::context_menu(response).id(popup_id);
        if response
            .ctx
            .data(|data| data.get_temp::<bool>(keyboard_id).unwrap_or(false))
        {
            let visible = response
                .interact_rect
                .intersect(response.ctx.content_rect());
            let visible = if visible.is_positive() {
                visible
            } else {
                response.ctx.content_rect().shrink(24.0)
            };
            let caret = match self
                .rich
                .as_ref()
                .map(|rich| rich.session.canvas_caret_rect(response))
                .transpose()
            {
                Ok(caret) => caret.flatten(),
                Err(error) => {
                    self.record_error(Err(error.into()));
                    None
                }
            };
            popup = popup.at_position(visible_menu_anchor(visible, caret));
        }
        popup.show(|ui| {
            let review_allowed = self
                .rich
                .as_ref()
                .and_then(|rich| rich.session.selection())
                .is_some_and(|selection| selection.anchor != selection.head);
            if ui
                .add_enabled(
                    review_allowed,
                    egui::Button::new(l.text("Prepare an edit from selection")),
                )
                .clicked()
            {
                let result = self.capture_review_selection();
                self.record_error(result);
                ui.close();
            }
            if ui
                .add_enabled(allowed, egui::Button::new(l.text("Annotate passage…")))
                .on_disabled_hover_text(l.text(reason.unwrap_or("Add note")))
                .clicked()
            {
                let result = self.begin_note();
                self.record_error(result);
                ui.close();
            }
            if let Some(reason) = reason {
                ui.add(egui::Label::new(l.text(reason)).wrap());
            }
            if ui.button(l.text("Notes")).clicked() {
                self.inspector = Inspector::Notes;
                self.inspector_visible = true;
                self.prefer_inspector = true;
                self.focus_mode = false;
                ui.close();
            }
        });
    }

    /// A bounded list/detail inspector; the parent must not wrap this in another ScrollArea.
    pub(super) fn notes_panel(&mut self, ui: &mut egui::Ui, t: Tokens) {
        let l = self.language;
        let writable = self.can_edit_manuscript();
        if self
            .document
            .as_ref()
            .is_none_or(|doc| doc.rich_json.is_none())
        {
            ui.label(l.text("Select a passage to add a note."));
            return;
        }
        if self.notes.invalid {
            ui.colored_label(
                t.warning,
                l.text("Notes could not be refreshed. Your text is preserved."),
            );
        }
        let reason = self.note_action_unavailable_reason();
        let catalog = self.notes.labels.catalog.clone();
        let mut manage_labels = false;
        ui.horizontal(|ui| {
            if ui
                .add_enabled_ui(reason.is_none(), |ui| {
                    icons::labeled_button(ui, Icon::Plus, l.text("Add note"), false)
                })
                .inner
                .on_disabled_hover_text(l.text(reason.unwrap_or("Add note")))
                .clicked()
            {
                let result = self.begin_note();
                self.record_error(result);
            }
            if icons::icon_button(ui, Icon::Tag, l.text("Project markup…")).clicked() {
                let result = self.open_project_marks();
                self.record_error(result);
            }
            let more = icons::icon_button(ui, Icon::More, l.text("Markup actions"));
            egui::Popup::menu(&more).show(|ui| {
                if ui.button(l.text("Project labels…")).clicked() {
                    manage_labels = true;
                    ui.close();
                }
                if ui.button(l.text("Reload notes")).clicked() {
                    let result = self.refresh_notes();
                    self.record_error(result);
                    ui.close();
                }
            });
        });
        if manage_labels {
            let result = self.open_label_manager();
            self.record_error(result);
        }
        if self.notes.decoration_cache.has_overlaps() {
            ui.label(
                RichText::new(
                    l.text("For overlapping passages, the selected note's color is shown."),
                )
                .size(11.0)
                .color(t.muted),
            );
        }
        let pending = self
            .document
            .as_ref()
            .is_some_and(|document| self.notes.paused_composers.contains_key(&document.id));
        if pending
            && components::action(
                ui,
                l.text("Resume markup draft"),
                writable && !self.notes.invalid,
                None,
            )
            .clicked()
        {
            let result = self.begin_note();
            self.record_error(result);
        }
        if let Some(reason) = reason
            && self.notes.preview.is_empty()
        {
            ui.add(
                egui::Label::new(RichText::new(l.text(reason)).size(13.0).color(t.muted)).wrap(),
            );
        }
        ui.add_space(12.0);
        let mut activate = None;
        let list_height = (ui.available_height() - 260.0).clamp(72.0, 216.0);
        let mut list = egui::ScrollArea::vertical()
            .id_salt("document-notes-list")
            .max_height(list_height)
            .auto_shrink([false, true]);
        let rows: Vec<_> = self
            .notes
            .preview
            .iter()
            .filter(|note| note.status != AnnotationStatus::Resolved)
            .chain(
                self.notes
                    .preview
                    .iter()
                    .filter(|note| note.status == AnnotationStatus::Resolved),
            )
            .collect();
        if self.notes.reveal_row {
            if let Some(index) = rows
                .iter()
                .position(|note| Some(&note.id) == self.notes.selected.as_ref())
            {
                list = list.vertical_scroll_offset(
                    (index as f32 * (72.0 + ui.spacing().item_spacing.y) - list_height * 0.5
                        + 36.0)
                        .max(0.0),
                );
            }
            self.notes.reveal_row = false;
        }
        list.show_rows(ui, 72.0, rows.len(), |ui, range| {
            for note in &rows[range] {
                let selected = self.notes.selected.as_ref() == Some(&note.id);
                let tree = if self.notes.invalid {
                    None
                } else {
                    self.rich.as_ref().map(|rich| rich.session.document())
                };
                let quote = displayed_quote(&note.anchor, tree, l);
                if ui
                    .push_id(&note.id, |ui| {
                        note_row(ui, note, &quote, selected, catalog.as_ref(), l, t)
                    })
                    .inner
                    .clicked()
                {
                    activate = Some(note.id.clone());
                }
            }
        });
        if rows.is_empty() {
            ui.label(RichText::new(l.text("No notes in this document.")).color(t.muted));
        }
        drop(rows);
        if let Some(id) = activate {
            let result = self.activate_note(&id);
            self.record_error(result);
        }
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);
        let selected = self
            .notes
            .selected
            .as_ref()
            .and_then(|id| self.notes.preview.iter().find(|note| &note.id == id))
            .or_else(|| {
                self.notes
                    .preview
                    .iter()
                    .find(|note| note.status != AnnotationStatus::Resolved)
            })
            .or_else(|| self.notes.preview.first())
            .cloned();
        let Some(note) = selected else {
            ui.label(RichText::new(l.text("Select a passage to add a note.")).color(t.muted));
            return;
        };
        self.notes.selected = Some(note.id.clone());
        let mut save = false;
        let mut toggle = false;
        let mut reattach = false;
        let mut remove = false;
        let mut reveal = false;
        let mut reload_draft = false;
        let mut create_label = false;
        let quote = displayed_quote(
            &note.anchor,
            self.rich.as_ref().map(|rich| rich.session.document()),
            l,
        );
        egui::ScrollArea::vertical()
            .id_salt(("document-note-details", &note.id))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(egui::Label::new(RichText::new(excerpt(&quote, 180)).size(12.0).color(t.muted)).wrap()).on_hover_text(&quote);
                ui.add_space(8.0);
                if note.anchor_status == AnchorStatus::Detached || note.status == AnnotationStatus::NeedsAttention {
                    ui.colored_label(t.warning, l.text("The passage changed. The note is preserved; select text to attach it again."));
                    let allowed = writable && !self.notes.invalid && self.rich.as_ref().is_some_and(|rich| captured_selection(&rich.session).is_ok());
                    reattach = components::action(ui, l.text("Attach to selection"), allowed, None).clicked();
                    ui.add_space(12.0);
                }
                let draft = self.notes.drafts.entry(note.id.clone()).or_insert_with(|| NoteDraft {
                    original: note.clone(), body: note.note.clone(), tags: note.tags.join("\n"), catalog: catalog.clone(),
                });
                let conflicted = draft.original.revision != note.revision;
                let assignments_changed = parsed_tags(&draft.tags) != draft.original.tags;
                let labels_changed = assignments_changed && !labels::same_catalog(draft.catalog.as_ref(), catalog.as_ref());
                if conflicted {
                    ui.colored_label(t.warning, l.text("This note changed elsewhere. Your draft is preserved."));
                    ui.horizontal_wrapped(|ui| {
                        if components::action(ui, l.text("Copy draft"), true, None).clicked() {
                            ui.ctx().copy_text(format!("{}\n\n{}", draft.tags, draft.body));
                        }
                        reload_draft = components::action(ui, l.text("Discard draft and reload"), true, None).clicked();
                    });
                    ui.add_space(12.0);
                }
                if labels_changed {
                    ui.colored_label(t.warning, l.text("Project labels changed. Your draft is preserved."));
                    if ui.button(l.text("Review label changes")).clicked() { draft.catalog = catalog.clone(); }
                }
                create_label = labels::assignment_picker(ui, &mut draft.tags, catalog.as_ref(), l, t, !writable);
                let labels_available = !assignments_changed || labels::assignments_available(&draft.tags, catalog.as_ref());
                if !labels_available { ui.colored_label(t.warning, l.text("Remove or replace labels that are no longer available.")); }
                ui.add_space(8.0);
                ui.label(RichText::new(l.text("Author note")).size(13.0).strong());
                note_input(ui, &mut draft.body, egui::Id::new(("author-note-body", &note.id)), l, t, !writable);
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    if draft.changed() {
                        save = components::action(ui, l.text("Save markup"), valid_markup(&draft.body, &draft.tags) && !conflicted && !labels_changed && labels_available && !self.notes.invalid && writable, Some(t)).clicked();
                    }
                    reveal = ui.add_enabled_ui(note.anchor_status == AnchorStatus::Attached && !self.notes.invalid, |ui| icons::icon_button(ui, Icon::ArrowRight, l.text("Show passage"))).inner.clicked();
                    let more = icons::icon_button(ui, Icon::More, l.text("Markup actions"));
                    egui::Popup::menu(&more).show(|ui| {
                        toggle = ui.add_enabled(!self.notes.invalid && writable, egui::Button::new(l.text(if note.status == AnnotationStatus::Resolved {"Reopen note"} else {"Resolve note"}))).clicked();
                        remove = ui.add_enabled(!self.notes.invalid && writable, egui::Button::new(l.text("Delete markup"))).clicked();
                        if toggle || remove { ui.close(); }
                    });
                });
            });
        if create_label {
            let result =
                self.begin_label_creation(Some(labels::LabelTarget::Note(note.id.clone())));
            self.record_error(result);
        }
        if reload_draft {
            let id = egui::Id::new(("author-note-body", &note.id));
            egui::text_edit::TextEditState::default().store(ui.ctx(), id);
            egui::text_edit::TextEditState::default()
                .store(ui.ctx(), egui::Id::new(("author-note-tags", &note.id)));
            self.notes.drafts.insert(
                note.id.clone(),
                NoteDraft {
                    original: note.clone(),
                    body: note.note.clone(),
                    tags: note.tags.join("\n"),
                    catalog: catalog.clone(),
                },
            );
        }
        if save {
            let result = self.save_note_draft(&note.id);
            self.record_error(result);
        }
        if toggle {
            let result = self.set_note_status(
                &note,
                if note.status == AnnotationStatus::Resolved {
                    AnnotationStatus::Open
                } else {
                    AnnotationStatus::Resolved
                },
            );
            self.record_error(result);
        }
        if reattach {
            let result = self.reattach_note_selection(&note);
            self.record_error(result);
        }
        if reveal {
            let result = self.activate_note(&note.id);
            self.record_error(result);
        }
        if remove {
            self.notes.delete_confirmation = Some(note);
        }
    }

    pub(super) fn notes_dialogs(&mut self, ctx: &egui::Context, t: Tokens) {
        let l = self.language;
        let writable = self.can_edit_manuscript();
        if self.notes.close_confirmation {
            self.confirm_notes_close(ctx, t);
            return;
        }
        if self.label_dialogs(ctx, t) {
            return;
        }
        self.project_marks_dialog(ctx, t);
        if self
            .notes
            .project_browser
            .as_ref()
            .is_some_and(|browser| browser.open)
        {
            return;
        }
        if let Some(mut composer) = self.notes.composer.take() {
            let catalog = self.notes.labels.catalog.clone();
            let composer_id = composer.id.clone();
            let mut save = false;
            let mut cancel = false;
            let mut discard = false;
            let mut create_label = false;
            let mut review_labels = false;
            let response = components::modal(
                ctx,
                l,
                "new-passage-note",
                l.text("Passage markup"),
                540.0,
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt(("new-note-body-scroll", &composer.id))
                        .max_height((ctx.content_rect().height() - 300.0).clamp(0.0, 440.0))
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.label(RichText::new(l.text("Selected passage")).strong());
                            ui.label(excerpt(
                                &displayed_quote(
                                    &composer.anchor,
                                    self.rich.as_ref().map(|rich| rich.session.document()),
                                    l,
                                ),
                                480,
                            ));
                            ui.add_space(16.0);
                            ui.label(l.text("Add a note, tags, or both."));
                            ui.add_space(8.0);
                            if !labels::same_catalog(composer.catalog.as_ref(), catalog.as_ref()) {
                                ui.colored_label(
                                    t.warning,
                                    l.text("Project labels changed. Your draft is preserved."),
                                );
                                review_labels = ui.button(l.text("Review label changes")).clicked();
                            }
                            create_label = labels::assignment_picker(
                                ui,
                                &mut composer.tags,
                                catalog.as_ref(),
                                l,
                                t,
                                !writable,
                            );
                            if !labels::assignments_available(&composer.tags, catalog.as_ref()) {
                                ui.colored_label(
                                    t.warning,
                                    l.text(
                                        "Remove or replace labels that are no longer available.",
                                    ),
                                );
                            }
                            ui.add_space(8.0);
                            ui.label(l.text("Author note"));
                            let field = note_input(
                                ui,
                                &mut composer.body,
                                egui::Id::new(("note-composer-body", &composer.id)),
                                l,
                                t,
                                !writable,
                            );
                            if composer.focus {
                                field.request_focus();
                                composer.focus = false;
                            }
                            ui.add_space(12.0);
                            ui.label(l.text("Cancel keeps this draft available in Notes."));
                        });
                    components::footer(ui, |ui| {
                        save = components::action(
                            ui,
                            l.text("Save markup"),
                            valid_markup(&composer.body, &composer.tags)
                                && labels::same_catalog(
                                    composer.catalog.as_ref(),
                                    catalog.as_ref(),
                                )
                                && labels::assignments_available(&composer.tags, catalog.as_ref())
                                && !self.notes.invalid
                                && writable,
                            Some(t),
                        )
                        .clicked();
                        cancel = components::action(ui, l.text("Cancel"), true, None).clicked();
                        if composer.has_content() {
                            discard = components::action(ui, l.text("Discard draft"), true, None)
                                .clicked();
                        }
                    });
                },
            );
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::S))
                && valid_markup(&composer.body, &composer.tags)
                && labels::same_catalog(composer.catalog.as_ref(), catalog.as_ref())
                && labels::assignments_available(&composer.tags, catalog.as_ref())
                && !self.notes.invalid
                && writable
            {
                save = true;
            }
            if review_labels {
                composer.catalog = catalog.clone();
            }
            if save {
                let created = (|| -> AppResult<PassageAnnotation> {
                    self.refresh_label_catalog()?;
                    Ok(self.workspace.create_tagged_annotation_with_labels(
                        &composer.document_id,
                        composer.base_revision,
                        composer.anchor.clone(),
                        &composer.body,
                        &parsed_tags(&composer.tags),
                        composer
                            .catalog
                            .as_ref()
                            .ok_or("missing_composer_label_catalog")?,
                    )?)
                })();
                let result = match created {
                    Ok(note) => {
                        self.notes.activate(&note.id);
                        self.inspector = Inspector::Notes;
                        self.inspector_visible = true;
                        self.prefer_inspector = true;
                        self.focus_mode = false;
                        self.refresh_notes()
                    }
                    Err(error) => {
                        self.notes.composer = Some(composer);
                        Err(error)
                    }
                };
                self.record_error(result);
            } else if !cancel && !discard && !response.should_close() {
                self.notes.composer = Some(composer);
            } else {
                if composer.has_content() && !discard {
                    composer.dismissed = true;
                    self.notes
                        .paused_composers
                        .insert(composer.document_id.clone(), composer);
                    self.inspector = Inspector::Notes;
                    self.inspector_visible = true;
                    self.prefer_inspector = true;
                }
                self.focus_manuscript = true;
            }
            if create_label {
                let result =
                    self.begin_label_creation(Some(labels::LabelTarget::Composer(composer_id)));
                self.record_error(result);
            }
        }
        if let Some(note) = self.notes.delete_confirmation.take() {
            let mut confirm = false;
            let mut cancel = false;
            let response = components::modal(
                ctx,
                l,
                "delete-passage-note",
                l.text("Delete markup?"),
                440.0,
                |ui| {
                    ui.label(
                        RichText::new(excerpt(
                            &displayed_quote(
                                &note.anchor,
                                self.rich.as_ref().map(|rich| rich.session.document()),
                                l,
                            ),
                            180,
                        ))
                        .strong(),
                    );
                    ui.label(excerpt(&note.note, 180));
                    ui.add_space(12.0);
                    ui.label(l.text("The manuscript text will stay unchanged."));
                    ui.label(l.text("You can restore deleted markup from Project markup."));
                    components::footer(ui, |ui| {
                        confirm = components::action(
                            ui,
                            l.text("Delete markup"),
                            !self.notes.invalid && writable,
                            Some(t),
                        )
                        .clicked();
                        cancel = components::action(ui, l.text("Cancel"), true, None).clicked();
                    });
                },
            );
            if confirm {
                let result =
                    self.note_guard_after_flush(&note)
                        .and_then(|(doc_revision, note_revision)| {
                            self.workspace.delete_annotation(
                                &note.id,
                                doc_revision,
                                note_revision,
                            )?;
                            self.notes.drafts.remove(&note.id);
                            self.notes.selected = None;
                            self.refresh_notes()
                        });
                if result.is_err() {
                    self.notes.delete_confirmation = Some(note);
                }
                self.record_error(result);
            } else if !cancel && !response.should_close() {
                self.notes.delete_confirmation = Some(note);
            }
        }
    }
}

impl ProjectMarksBrowser {
    fn refresh(&mut self, workspace: &Workspace) -> AppResult<()> {
        self.records = workspace.project_annotations(&self.project_id, true)?;
        self.available_tags = self
            .records
            .iter()
            .filter(|row| !row.annotation.deleted)
            .flat_map(|row| row.annotation.tags.iter().cloned())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        self.filter_key = None;
        let tree = workspace.project_tree(&self.project_id)?;
        let by_id: HashMap<_, _> = tree
            .iter()
            .map(|node| (node.logical_id.as_str(), node))
            .collect();
        self.paths.clear();
        self.trashed.clear();
        for node in &tree {
            let Some(document) = &node.document_id else {
                continue;
            };
            let mut titles = Vec::new();
            let mut seen = std::collections::HashSet::new();
            let mut current = Some(node);
            while let Some(item) = current {
                if !seen.insert(item.logical_id.as_str()) {
                    break;
                }
                if item.metadata["trashedAt"]
                    .as_str()
                    .is_some_and(|value| !value.is_empty())
                {
                    self.trashed.insert(document.clone());
                }
                titles.push(item.metadata["title"].as_str().unwrap_or_default());
                current = item.metadata["parentId"]
                    .as_str()
                    .and_then(|id| by_id.get(id).copied());
            }
            titles.reverse();
            self.paths.insert(document.clone(), titles.join(" / "));
        }
        Ok(())
    }
    fn filter(&mut self) {
        let key = (self.query.clone(), self.tag.clone(), self.deleted);
        if self.filter_key.as_ref() == Some(&key) {
            return;
        }
        let query = self.query.to_lowercase();
        self.visible = self
            .records
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                (row.annotation.deleted == self.deleted
                    && self
                        .tag
                        .as_ref()
                        .is_none_or(|tag| row.annotation.tags.contains(tag))
                    && (query.is_empty()
                        || row.document_title.to_lowercase().contains(&query)
                        || row.annotation.note.to_lowercase().contains(&query)
                        || row
                            .annotation
                            .anchor
                            .exact_text
                            .to_lowercase()
                            .contains(&query)
                        || row
                            .annotation
                            .tags
                            .iter()
                            .any(|tag| tag.to_lowercase().contains(&query))))
                .then_some(index)
            })
            .collect();
        self.filter_key = Some(key);
    }
}

impl StudioApp {
    fn restore_project_markup(
        &mut self,
        browser: &mut ProjectMarksBrowser,
        row: &studio_core::ProjectAnnotationSummary,
    ) -> AppResult<()> {
        let note = self.workspace.restore_annotation_with_labels(
            &row.annotation.id,
            row.document_revision,
            row.annotation.revision,
            self.notes
                .labels
                .catalog
                .as_ref()
                .ok_or("missing_restore_label_catalog")?,
        )?;
        if let Some(record) = browser
            .records
            .iter_mut()
            .find(|record| record.annotation.id == note.id)
        {
            record.document_revision = note.base_revision;
            record.annotation = note;
        }
        browser.filter_key = None;
        let result = browser.refresh(&self.workspace);
        self.record_error(result);
        let result = if self.document.is_some() {
            self.refresh_notes()
        } else {
            self.refresh_label_catalog()
        };
        self.record_error(result);
        Ok(())
    }

    fn pending_project_markup_drafts(&self, project_id: &str) -> AppResult<Vec<(String, String)>> {
        let mut pending = Vec::new();
        for id in self.notes.pending_documents() {
            let document = self.workspace.document(&id)?;
            if document.project_id == project_id {
                pending.push((document.id, document.title));
            }
        }
        Ok(pending)
    }

    fn review_project_markup_drafts(&mut self, document_id: &str) -> AppResult<()> {
        let document = self.workspace.document(document_id)?;
        self.begin_reference_excursion(document)?;
        self.notes.select_pending_draft(document_id);
        self.inspector = Inspector::Notes;
        self.inspector_visible = true;
        self.prefer_inspector = true;
        self.focus_mode = false;
        Ok(())
    }

    /// Never let a bulk edit hide an unsaved draft by deleting its canonical mark.
    fn apply_project_tag_action(
        &mut self,
        project_id: &str,
        action: &mut TagAction,
    ) -> AppResult<bool> {
        action.pending_drafts = self.pending_project_markup_drafts(project_id)?;
        if !action.pending_drafts.is_empty() {
            return Ok(false);
        }
        if action.remove {
            self.workspace.remove_project_tag(&action.snapshot)?;
        } else {
            self.workspace
                .rename_project_tag(&action.snapshot, &action.replacement)?;
        }
        Ok(true)
    }

    /// Open the project-scoped manual markup browser after acknowledging current writing.
    pub(super) fn open_project_marks(&mut self) -> AppResult<()> {
        self.save_explicit()?;
        self.refresh_label_catalog()?;
        let project_id = self
            .project
            .as_ref()
            .ok_or("missing_markup_project")?
            .id
            .clone();
        if let Some(mut composer) = self.notes.composer.take() {
            composer.dismissed = true;
            self.notes
                .paused_composers
                .insert(composer.document_id.clone(), composer);
        }
        let mut browser = self
            .notes
            .project_browser
            .take()
            .filter(|browser| browser.project_id == project_id)
            .unwrap_or_else(|| ProjectMarksBrowser {
                project_id,
                records: Vec::new(),
                available_tags: Vec::new(),
                visible: Vec::new(),
                filter_key: None,
                paths: HashMap::new(),
                trashed: Default::default(),
                query: String::new(),
                tag: None,
                deleted: false,
                open: false,
                tag_action: None,
                restore_confirmation: None,
            });
        let result = browser.refresh(&self.workspace);
        browser.open = result.is_ok();
        self.notes.project_browser = Some(browser);
        result
    }

    fn project_marks_dialog(&mut self, ctx: &egui::Context, t: Tokens) {
        let Some(mut browser) = self.notes.project_browser.take() else {
            return;
        };
        if !browser.open {
            self.notes.project_browser = Some(browser);
            return;
        }
        let l = self.language;
        let catalog = self.notes.labels.catalog.clone();
        if let Some(row) = browser.restore_confirmation.take() {
            let mut restore = false;
            let mut cancel = false;
            let response = components::modal(
                ctx,
                l,
                "restore-markup-labels",
                l.text("Restore markup"),
                480.0,
                |ui| {
                    ui.label(l.text(
                        "Restoring this markup also restores these labels with their saved colors:",
                    ));
                    if let Some(catalog) = &catalog {
                        for label in catalog
                            .retired_labels()
                            .iter()
                            .filter(|label| row.annotation.tags.contains(&label.name))
                        {
                            labels::chip(ui, &label.name, labels::color(label.color, t), t);
                        }
                    }
                    components::footer(ui, |ui| {
                        restore = components::action(ui, l.text("Restore markup"), true, Some(t))
                            .clicked();
                        cancel = components::action(ui, l.text("Cancel"), true, None).clicked();
                    });
                },
            );
            if restore {
                let result = self.restore_project_markup(&mut browser, &row);
                if result.is_err() {
                    browser.restore_confirmation = Some(row);
                }
                self.record_error(result);
            } else if !cancel && !response.should_close() {
                browser.restore_confirmation = Some(row);
            }
            self.notes.project_browser = Some(browser);
            return;
        }
        if let Some(mut action) = browser.tag_action.take() {
            if !action.pending_drafts.is_empty() {
                let mut review = None;
                let mut cancel = false;
                let response = components::modal(
                    ctx,
                    l,
                    "project-tag-draft-preflight",
                    l.text("Review drafts"),
                    540.0,
                    |ui| {
                        ui.label(l.text("Save or discard the project's markup drafts before changing a project tag."));
                        egui::ScrollArea::vertical()
                            .max_height((ctx.content_rect().height() - 280.0).clamp(80.0, 320.0))
                            .show(ui, |ui| {
                                for (id, title) in &action.pending_drafts {
                                    ui.horizontal(|ui| {
                                        let label = l.text("Review drafts");
                                        let action_width = ui
                                            .painter()
                                            .layout_no_wrap(
                                                label.into(),
                                                egui::TextStyle::Button.resolve(ui.style()),
                                                t.text,
                                            )
                                            .size()
                                            .x
                                            + ui.spacing().button_padding.x * 2.0;
                                        ui.add_sized(
                                            [
                                                (ui.available_width()
                                                    - action_width.max(112.0)
                                                    - ui.spacing().item_spacing.x)
                                                    .max(1.0),
                                                40.0,
                                            ],
                                            egui::Label::new(title).truncate(),
                                        )
                                        .on_hover_text(title);
                                        if components::action(ui, label, true, None).clicked() {
                                            review = Some(id.clone());
                                        }
                                    });
                                }
                            });
                        components::footer(ui, |ui| {
                            cancel = components::action(ui, l.text("Cancel"), true, None).clicked();
                        });
                    },
                );
                if let Some(id) = review {
                    let result = self.review_project_markup_drafts(&id);
                    if result.is_ok() {
                        browser.open = false;
                    } else {
                        browser.tag_action = Some(action);
                    }
                    self.record_error(result);
                } else if !cancel && !response.should_close() {
                    browser.tag_action = Some(action);
                }
                self.notes.project_browser = Some(browser);
                return;
            }
            let mut apply = false;
            let mut cancel = false;
            let response = components::modal(
                ctx,
                l,
                "project-tag-action",
                l.text(if action.remove {
                    "Remove project tag?"
                } else {
                    "Rename project tag"
                }),
                540.0,
                |ui| {
                    ui.label(RichText::new(action.snapshot.tag()).strong());
                    ui.label(l.format(
                        "This changes {count} passages in this project.",
                        &[("count", action.snapshot.count().to_string())],
                    ));
                    ui.label(l.text("This includes markup in documents in Trash."));
                    if !action.remove && action.snapshot.deleted_count() > 0 {
                        ui.label(l.format(
                            "This also updates {count} deleted passages.",
                            &[("count", action.snapshot.deleted_count().to_string())],
                        ));
                    }
                    if action.remove {
                        ui.label(l.format("{count} passages have only this tag; their empty markup will move to deleted markup.", &[("count", action.snapshot.tag_only_count().to_string())]));
                    } else {
                        ui.add_space(12.0);
                        ui.label(l.text("New tag name"));
                        let field = components::singleline(
                            ui,
                            &mut action.replacement,
                            l.text("New tag name"),
                        );
                        if action.focus {
                            field.request_focus();
                            action.focus = false;
                        }
                        ui.label(l.text("Existing tag names cannot be merged by renaming."));
                    }
                    components::footer(ui, |ui| {
                        let valid = action.remove
                            || (action.replacement != action.snapshot.tag()
                                && valid_tags(&[action.replacement.clone()]));
                        apply = components::action(
                            ui,
                            l.text(if action.remove {
                                "Remove tag"
                            } else {
                                "Rename tag"
                            }),
                            valid,
                            Some(t),
                        )
                        .clicked();
                        cancel = components::action(ui, l.text("Cancel"), true, None).clicked();
                    });
                },
            );
            if apply {
                match self.apply_project_tag_action(&browser.project_id, &mut action) {
                    Ok(true) => {
                        browser.tag = if action.remove {
                            None
                        } else {
                            Some(action.replacement.clone())
                        };
                        browser.records.clear();
                        browser.visible.clear();
                        browser.available_tags.clear();
                        browser.filter_key = None;
                        let result = browser.refresh(&self.workspace);
                        self.record_error(result);
                        let result = if self.document.is_some() {
                            self.refresh_notes()
                        } else {
                            self.refresh_label_catalog()
                        };
                        self.record_error(result);
                    }
                    Ok(false) => browser.tag_action = Some(action),
                    Err(error) => {
                        browser.tag_action = Some(action);
                        self.record_error(Err(error));
                    }
                }
            } else if !cancel && !response.should_close() {
                browser.tag_action = Some(action);
            }
            self.notes.project_browser = Some(browser);
            return;
        }
        let mut chosen = None;
        let mut restore = None;
        let mut refresh = false;
        let mut tag_action = None;
        let mut close = false;
        let mut manage_labels = false;
        let response = components::modal(
            ctx,
            l,
            "project-markup",
            l.text("Project markup"),
            740.0,
            |ui| {
                if let Some(project) = &self.project {
                    ui.label(RichText::new(&project.title).strong());
                    ui.add_space(12.0);
                }
                components::singleline(
                    ui,
                    &mut browser.query,
                    l.text("Find in passages, notes and titles…"),
                );
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    egui::ComboBox::from_id_salt("project-markup-tag")
                        .selected_text(browser.tag.as_deref().unwrap_or(l.text("All tags")))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut browser.tag, None, l.text("All tags"));
                            for tag in &browser.available_tags {
                                ui.selectable_value(&mut browser.tag, Some(tag.clone()), tag);
                            }
                        });
                    ui.checkbox(&mut browser.deleted, l.text("Deleted markup"));
                    refresh = ui.button(l.text("Refresh markup")).clicked();
                    manage_labels = ui.button(l.text("Project labels…")).clicked();
                    if browser.tag.is_some() && !browser.deleted {
                        if ui.button(l.text("Rename tag…")).clicked() {
                            tag_action = Some(false);
                        }
                        if ui.button(l.text("Remove tag…")).clicked() {
                            tag_action = Some(true);
                        }
                    }
                });
                ui.add_space(12.0);
                browser.filter();
                let rows = &browser.visible;
                ui.label(l.format("Passages: {count}", &[("count", rows.len().to_string())]));
                egui::ScrollArea::vertical()
                    .id_salt("project-markup-results")
                    .max_height((ctx.content_rect().height() - 340.0).clamp(100.0, 440.0))
                    .auto_shrink([false, true])
                    .show_rows(ui, 124.0, rows.len(), |ui, range| {
                        for index in range {
                            let row = &browser.records[rows[index]];
                            let document_id = &row.annotation.document_id;
                            let path = browser
                                .paths
                                .get(document_id)
                                .map(String::as_str)
                                .unwrap_or(&row.document_title);
                            let action = ui
                                .push_id(&row.annotation.id, |ui| {
                                    project_markup_row(
                                        ui,
                                        row,
                                        path,
                                        browser.trashed.contains(document_id),
                                        catalog.as_ref(),
                                        l,
                                        t,
                                    )
                                })
                                .inner;
                            match action {
                                Some(ProjectMarkupAction::Open) => chosen = Some(row.clone()),
                                Some(ProjectMarkupAction::Restore) => restore = Some(row.clone()),
                                None => {}
                            }
                        }
                    });
                if rows.is_empty() {
                    ui.label(l.text("No matching markup in this project."));
                }
                components::footer(ui, |ui| {
                    close = components::action(ui, l.text("Done"), true, Some(t)).clicked();
                });
            },
        );
        if close || response.should_close() {
            browser.open = false;
            self.focus_manuscript = true;
        }
        if refresh {
            let result = browser.refresh(&self.workspace);
            self.record_error(result);
            let result = self.refresh_label_catalog();
            self.record_error(result);
        }
        if manage_labels {
            let result = self.open_label_manager();
            self.record_error(result);
        }
        if let Some(remove) = tag_action
            && let Some(tag) = &browser.tag
        {
            match self
                .workspace
                .project_tag_snapshot(&browser.project_id, tag)
            {
                Ok(snapshot) => match self.pending_project_markup_drafts(&browser.project_id) {
                    Ok(pending_drafts) => {
                        browser.tag_action = Some(TagAction {
                            replacement: tag.clone(),
                            snapshot,
                            remove,
                            focus: true,
                            pending_drafts,
                        })
                    }
                    Err(error) => self.record_error(Err(error)),
                },
                Err(error) => self.record_error(Err(error.into())),
            }
        }
        if let Some(row) = restore {
            let revives_labels = catalog.as_ref().is_some_and(|catalog| {
                catalog
                    .retired_labels()
                    .iter()
                    .any(|label| row.annotation.tags.contains(&label.name))
            });
            if revives_labels {
                browser.restore_confirmation = Some(row);
            } else {
                let result = self.restore_project_markup(&mut browser, &row);
                self.record_error(result);
            }
        }
        if let Some(row) = chosen {
            let result = (|| -> AppResult<()> {
                let current = self.workspace.annotation(&row.annotation.id)?;
                let document = self.workspace.document(&current.document_id)?;
                if document.project_id != browser.project_id {
                    return Err("markup_project_changed".into());
                }
                self.begin_reference_excursion(document)?;
                self.activate_note(&current.id)?;
                Ok(())
            })();
            if result.is_ok() {
                browser.open = false;
            }
            self.record_error(result);
        }
        self.notes.project_browser = Some(browser);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_editor_model::{Command, Editor, TextRange, Transaction};
    type TestResult = AppResult<()>;
    struct BackupNotesFixture {
        _directory: tempfile::TempDir,
        app: StudioApp,
        first: Document,
        first_note: PassageAnnotation,
        second: Document,
        second_note: PassageAnnotation,
    }
    fn backup_notes_fixture() -> AppResult<BackupNotesFixture> {
        let directory = tempfile::tempdir()?;
        let creation = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = StudioApp::new(
            &creation,
            directory.path().join("backup-notes.sqlite"),
            false,
            None,
        )?;
        let first_project = app.workspace.create_project("First project")?;
        let second_project = app.workspace.create_project("Second project")?;
        let first = app.workspace.create_rich_document(&first_project.id,"First document",RichTree::from_json(serde_json::json!({
            "type":"doc","attrs":{"schemaVersion":1},"content":[
                {"type":"paragraph","attrs":{"blockId":"first"},"content":[{"type":"text","text":"Я😀 echo"}]},
                {"type":"paragraph","attrs":{"blockId":"tail"},"content":[{"type":"text","text":"tail"}]}
            ]
        }))?.to_json()?)?;
        let second = app.workspace.create_rich_document(
            &second_project.id,
            "Second document",
            RichTree::plain("second", "other")?.to_json()?,
        )?;
        let first_anchor = app.workspace.capture_annotation_anchor(
            &first.id,
            first.revision,
            "first",
            ScalarOffset(3),
            ScalarOffset(7),
            "echo",
        )?;
        let second_anchor = app.workspace.capture_annotation_anchor(
            &second.id,
            second.revision,
            "second",
            ScalarOffset(0),
            ScalarOffset(5),
            "other",
        )?;
        let first_note = app.workspace.create_annotation(
            &first.id,
            first.revision,
            first_anchor,
            "Saved first note",
        )?;
        let second_note = app.workspace.create_annotation(
            &second.id,
            second.revision,
            second_anchor.clone(),
            "Saved second note",
        )?;
        app.open_project(first_project)?;
        app.notes.drafts.insert(
            first_note.id.clone(),
            NoteDraft {
                original: first_note.clone(),
                catalog: Some(app.workspace.project_labels(&first.project_id)?),
                tags: String::new(),
                body: "Edited first note 😀".into(),
            },
        );
        app.notes.paused_composers.insert(
            second.id.clone(),
            NoteComposer {
                id: "parked-composer".into(),
                catalog: Some(app.workspace.project_labels(&second.project_id)?),
                document_id: second.id.clone(),
                base_revision: second.revision,
                anchor: second_anchor,
                body: "Parked new note 中".into(),
                tags: String::new(),
                focus: false,
                dismissed: false,
            },
        );
        Ok(BackupNotesFixture {
            _directory: directory,
            app,
            first,
            first_note,
            second,
            second_note,
        })
    }

    #[test]
    fn backup_preflight_and_cancel_do_not_commit_parked_notes() -> TestResult {
        let mut fixture = backup_notes_fixture()?;
        assert!(!fixture.app.prepare_workspace_backup()?);
        assert!(fixture.app.storage.modal_open());
        assert!(!fixture.app.storage.busy());
        assert_eq!(fixture.app.notes.pending_documents().len(), 2);
        fixture.app.cancel_workspace_backup_preflight();
        assert!(!fixture.app.storage.modal_open());
        assert_eq!(
            fixture.app.workspace.annotation(&fixture.first_note.id)?,
            fixture.first_note
        );
        assert_eq!(
            fixture.app.workspace.annotations(&fixture.second.id)?,
            vec![fixture.second_note]
        );
        assert_eq!(
            fixture
                .app
                .notes
                .drafts
                .get(&fixture.first_note.id)
                .ok_or("missing first draft")?
                .body,
            "Edited first note 😀"
        );
        assert_eq!(
            fixture
                .app
                .notes
                .paused_composers
                .get(&fixture.second.id)
                .ok_or("missing parked composer")?
                .body,
            "Parked new note 中"
        );
        Ok(())
    }

    #[test]
    fn explicit_backup_save_resolves_pending_notes_across_projects() -> TestResult {
        let mut fixture = backup_notes_fixture()?;
        assert!(!fixture.app.prepare_workspace_backup()?);
        fixture.app.save_backup_note_prerequisites()?;
        assert!(!fixture.app.notes.has_unsaved_drafts());
        assert!(!fixture.app.storage.modal_open());
        assert_eq!(
            fixture
                .app
                .workspace
                .annotation(&fixture.first_note.id)?
                .note,
            "Edited first note 😀"
        );
        assert!(
            fixture
                .app
                .workspace
                .annotations(&fixture.second.id)?
                .iter()
                .any(|note| note.note == "Parked new note 中")
        );
        assert_eq!(
            fixture.app.workspace.document(&fixture.first.id)?,
            fixture.first
        );
        assert_eq!(
            fixture.app.workspace.document(&fixture.second.id)?,
            fixture.second
        );
        Ok(())
    }

    #[test]
    fn tag_only_drafts_survive_context_clear_backup_cancel_and_explicit_save() -> TestResult {
        let mut fixture = backup_notes_fixture()?;
        let first_labels = fixture
            .app
            .workspace
            .project_labels(&fixture.first.project_id)?;
        let first_labels = fixture.app.workspace.create_project_label(
            &first_labels,
            "  Ася  ",
            studio_core::ProjectLabelColor::Blue,
        )?;
        let first_labels = fixture.app.workspace.create_project_label(
            &first_labels,
            "motivation 😀",
            studio_core::ProjectLabelColor::Violet,
        )?;
        let second_labels = fixture
            .app
            .workspace
            .project_labels(&fixture.second.project_id)?;
        let second_labels = fixture.app.workspace.create_project_label(
            &second_labels,
            "Question",
            studio_core::ProjectLabelColor::Amber,
        )?;
        let draft = fixture
            .app
            .notes
            .drafts
            .get_mut(&fixture.first_note.id)
            .ok_or("missing draft")?;
        draft.body.clone_from(&fixture.first_note.note);
        draft.tags = "  Ася  \nmotivation 😀".into();
        draft.catalog = Some(first_labels);
        let composer = fixture
            .app
            .notes
            .paused_composers
            .get_mut(&fixture.second.id)
            .ok_or("missing composer")?;
        composer.body.clear();
        composer.tags = "Question".into();
        composer.catalog = Some(second_labels);
        composer.dismissed = true;
        fixture.app.notes.clear();
        assert_eq!(fixture.app.notes.pending_documents().len(), 2);
        assert!(!fixture.app.prepare_workspace_backup()?);
        fixture.app.cancel_workspace_backup_preflight();
        assert!(fixture.app.notes.has_unsaved_drafts());
        assert_eq!(
            fixture.app.workspace.annotation(&fixture.first_note.id)?,
            fixture.first_note
        );
        fixture.app.save_note_drafts()?;
        assert!(!fixture.app.notes.has_unsaved_drafts());
        assert_eq!(
            fixture
                .app
                .workspace
                .annotation(&fixture.first_note.id)?
                .tags,
            ["  Ася  ", "motivation 😀"]
        );
        assert!(
            fixture
                .app
                .workspace
                .annotations(&fixture.second.id)?
                .iter()
                .any(|note| note.note.is_empty() && note.tags == ["Question"])
        );
        Ok(())
    }

    #[test]
    fn concurrent_tag_edit_refuses_save_and_preserves_exact_tag_draft() -> TestResult {
        let mut fixture = backup_notes_fixture()?;
        let draft = fixture
            .app
            .notes
            .drafts
            .get_mut(&fixture.first_note.id)
            .ok_or("missing draft")?;
        draft.body.clone_from(&fixture.first_note.note);
        draft.tags = "Ася / motive 😀".into();
        let external = fixture.app.workspace.update_annotation_markup(
            &fixture.first_note.id,
            fixture.first.revision,
            fixture.first_note.revision,
            AnnotationPatch::default(),
            Some(vec!["Changed elsewhere".into()]),
        )?;
        assert!(fixture.app.save_note_draft(&fixture.first_note.id).is_err());
        assert_eq!(
            fixture.app.workspace.annotation(&fixture.first_note.id)?,
            external
        );
        assert_eq!(
            fixture
                .app
                .notes
                .drafts
                .get(&fixture.first_note.id)
                .ok_or("draft was lost")?
                .tags,
            "Ася / motive 😀"
        );
        Ok(())
    }

    #[test]
    fn changed_assignment_draft_cannot_recreate_a_concurrently_renamed_label() -> TestResult {
        let mut fixture = backup_notes_fixture()?;
        let catalog = fixture
            .app
            .workspace
            .project_labels(&fixture.first.project_id)?;
        let catalog = fixture.app.workspace.create_project_label(
            &catalog,
            "Motive",
            studio_core::ProjectLabelColor::Violet,
        )?;
        let draft = fixture
            .app
            .notes
            .drafts
            .get_mut(&fixture.first_note.id)
            .ok_or("missing draft")?;
        draft.catalog = Some(catalog);
        draft.tags = "Motive".into();
        let snapshot = fixture
            .app
            .workspace
            .project_tag_snapshot(&fixture.first.project_id, "Motive")?;
        fixture
            .app
            .workspace
            .rename_project_tag(&snapshot, "Choice")?;
        assert!(fixture.app.save_note_draft(&fixture.first_note.id).is_err());
        assert_eq!(
            fixture
                .app
                .notes
                .drafts
                .get(&fixture.first_note.id)
                .ok_or("lost draft")?
                .tags,
            "Motive"
        );
        assert!(
            fixture
                .app
                .workspace
                .project_labels(&fixture.first.project_id)?
                .labels()
                .iter()
                .all(|label| label.name != "Motive")
        );
        assert_eq!(
            fixture.app.workspace.annotation(&fixture.first_note.id)?,
            fixture.first_note
        );
        Ok(())
    }

    #[test]
    fn body_only_draft_saves_after_color_only_catalog_change_without_replacing_its_text()
    -> TestResult {
        let mut fixture = backup_notes_fixture()?;
        let catalog = fixture
            .app
            .workspace
            .project_labels(&fixture.first.project_id)?;
        let catalog = fixture.app.workspace.create_project_label(
            &catalog,
            "Motive",
            studio_core::ProjectLabelColor::Violet,
        )?;
        fixture.app.workspace.set_project_label_color(
            &catalog,
            "Motive",
            studio_core::ProjectLabelColor::Rose,
        )?;
        fixture.app.save_note_draft(&fixture.first_note.id)?;
        assert_eq!(
            fixture
                .app
                .workspace
                .annotation(&fixture.first_note.id)?
                .note,
            "Edited first note 😀"
        );
        assert!(
            fixture
                .app
                .workspace
                .annotation(&fixture.first_note.id)?
                .tags
                .is_empty()
        );
        Ok(())
    }

    #[test]
    fn clean_viewed_markup_adopts_project_tag_rename_without_a_false_conflict() -> TestResult {
        let mut fixture = backup_notes_fixture()?;
        fixture.app.notes.drafts.clear();
        let saved = fixture.app.workspace.update_annotation_markup(
            &fixture.first_note.id,
            fixture.first.revision,
            fixture.first_note.revision,
            AnnotationPatch {
                note: Some(String::new()),
                status: None,
            },
            Some(vec!["A".into()]),
        )?;
        fixture.app.refresh_notes()?;
        fixture.app.notes.drafts.insert(
            saved.id.clone(),
            NoteDraft {
                original: saved.clone(),
                catalog: Some(
                    fixture
                        .app
                        .workspace
                        .project_labels(&fixture.first.project_id)?,
                ),
                body: String::new(),
                tags: "A".into(),
            },
        );
        let mut action = TagAction {
            snapshot: fixture
                .app
                .workspace
                .project_tag_snapshot(&fixture.first.project_id, "A")?,
            replacement: "B".into(),
            remove: false,
            focus: false,
            pending_drafts: Vec::new(),
        };
        assert!(
            fixture
                .app
                .apply_project_tag_action(&fixture.first.project_id, &mut action)?
        );
        fixture.app.refresh_notes()?;
        let draft = fixture
            .app
            .notes
            .drafts
            .get(&saved.id)
            .ok_or("missing viewed draft")?;
        assert_eq!(draft.tags, "B");
        assert_eq!(draft.original, fixture.app.workspace.annotation(&saved.id)?);
        assert!(!draft.changed());
        let mut remove = TagAction {
            snapshot: fixture
                .app
                .workspace
                .project_tag_snapshot(&fixture.first.project_id, "B")?,
            remove: true,
            ..action
        };
        assert!(
            fixture
                .app
                .apply_project_tag_action(&fixture.first.project_id, &mut remove)?
        );
        fixture.app.refresh_notes()?;
        assert!(!fixture.app.notes.drafts.contains_key(&saved.id));
        Ok(())
    }

    #[test]
    fn project_tag_removal_requires_review_of_unsaved_tag_only_markup_before_mutation() -> TestResult
    {
        let mut fixture = backup_notes_fixture()?;
        let saved = fixture.app.workspace.update_annotation_markup(
            &fixture.first_note.id,
            fixture.first.revision,
            fixture.first_note.revision,
            AnnotationPatch {
                note: Some(String::new()),
                status: None,
            },
            Some(vec!["A".into()]),
        )?;
        fixture.app.refresh_notes()?;
        fixture.app.notes.drafts.insert(
            saved.id.clone(),
            NoteDraft {
                original: saved.clone(),
                catalog: Some(
                    fixture
                        .app
                        .workspace
                        .project_labels(&fixture.first.project_id)?,
                ),
                body: "Her unfinished motive 😀".into(),
                tags: "A".into(),
            },
        );
        let mut action = TagAction {
            snapshot: fixture
                .app
                .workspace
                .project_tag_snapshot(&fixture.first.project_id, "A")?,
            replacement: String::new(),
            remove: true,
            focus: false,
            pending_drafts: Vec::new(),
        };
        assert!(
            !fixture
                .app
                .apply_project_tag_action(&fixture.first.project_id, &mut action)?
        );
        assert_eq!(
            action.pending_drafts,
            [(fixture.first.id.clone(), fixture.first.title.clone())]
        );
        assert_eq!(fixture.app.workspace.annotation(&saved.id)?, saved);
        fixture
            .app
            .review_project_markup_drafts(&fixture.first.id)?;
        assert_eq!(
            fixture.app.notes.selected.as_deref(),
            Some(saved.id.as_str())
        );
        assert!(
            fixture
                .app
                .notes
                .preview
                .iter()
                .any(|note| note.id == saved.id)
        );
        assert_eq!(
            fixture
                .app
                .notes
                .drafts
                .get(&saved.id)
                .ok_or("missing local draft")?
                .body,
            "Her unfinished motive 😀"
        );
        fixture.app.save_note_draft(&saved.id)?;
        let mut reviewed = TagAction {
            snapshot: fixture
                .app
                .workspace
                .project_tag_snapshot(&fixture.first.project_id, "A")?,
            ..action
        };
        assert!(
            fixture
                .app
                .apply_project_tag_action(&fixture.first.project_id, &mut reviewed)?
        );
        let remaining = fixture.app.workspace.annotation(&saved.id)?;
        assert!(!remaining.deleted && remaining.tags.is_empty());
        assert_eq!(remaining.note, "Her unfinished motive 😀");
        Ok(())
    }

    #[test]
    fn dismissed_markup_draft_is_available_for_explicit_resume_without_a_new_selection()
    -> TestResult {
        let mut fixture = backup_notes_fixture()?;
        let mut composer = fixture
            .app
            .notes
            .paused_composers
            .remove(&fixture.second.id)
            .ok_or("missing composer")?;
        composer.dismissed = true;
        composer.tags = "Last light 😀".into();
        let composer_id = composer.id.clone();
        fixture
            .app
            .notes
            .paused_composers
            .insert(fixture.second.id.clone(), composer);
        fixture.app.open_document(fixture.second.clone())?;
        assert!(fixture.app.notes.composer.is_none());
        fixture.app.begin_note()?;
        let resumed = fixture
            .app
            .notes
            .composer
            .as_ref()
            .ok_or("draft was not resumed")?;
        assert_eq!(resumed.id, composer_id);
        assert_eq!(resumed.tags, "Last light 😀");
        assert!(!resumed.dismissed);
        Ok(())
    }

    #[test]
    fn failed_backup_note_save_keeps_exact_conflicting_body_and_preflight() -> TestResult {
        let mut fixture = backup_notes_fixture()?;
        fixture.app.notes.drafts.insert(
            fixture.second_note.id.clone(),
            NoteDraft {
                original: fixture.second_note.clone(),
                catalog: Some(
                    fixture
                        .app
                        .workspace
                        .project_labels(&fixture.second.project_id)?,
                ),
                tags: String::new(),
                body: "Keep my conflicting draft 😀".into(),
            },
        );
        assert!(!fixture.app.prepare_workspace_backup()?);
        let external = fixture.app.workspace.update_annotation(
            &fixture.second_note.id,
            fixture.second.revision,
            fixture.second_note.revision,
            AnnotationPatch {
                note: Some("Newer saved note".into()),
                status: None,
            },
        )?;
        assert!(fixture.app.save_backup_note_prerequisites().is_err());
        assert!(fixture.app.storage.modal_open());
        assert!(!fixture.app.storage.busy());
        assert_eq!(
            fixture.app.workspace.annotation(&fixture.second_note.id)?,
            external
        );
        assert_eq!(
            fixture
                .app
                .notes
                .drafts
                .get(&fixture.second_note.id)
                .ok_or("conflicting draft was lost")?
                .body,
            "Keep my conflicting draft 😀"
        );
        assert_eq!(
            fixture
                .app
                .notes
                .paused_composers
                .get(&fixture.second.id)
                .ok_or("parked composer was lost")?
                .body,
            "Parked new note 中"
        );
        Ok(())
    }

    #[test]
    fn backup_review_opens_the_affected_document_and_preserves_drafts() -> TestResult {
        let mut fixture = backup_notes_fixture()?;
        fixture.app.settings_open = true;
        assert!(!fixture.app.prepare_workspace_backup()?);
        fixture.app.review_backup_notes(&fixture.second.id)?;
        assert_eq!(
            fixture
                .app
                .document
                .as_ref()
                .map(|document| document.id.as_str()),
            Some(fixture.second.id.as_str())
        );
        assert!(!fixture.app.settings_open);
        assert!(!fixture.app.storage.modal_open());
        assert!(fixture.app.inspector_visible && fixture.app.inspector == Inspector::Notes);
        assert_eq!(
            fixture
                .app
                .notes
                .composer
                .as_ref()
                .ok_or("missing reviewed composer")?
                .body,
            "Parked new note 中"
        );
        assert_eq!(
            fixture.app.workspace.annotations(&fixture.second.id)?,
            vec![fixture.second_note]
        );
        Ok(())
    }

    #[test]
    fn unsupported_note_selection_explains_scope_without_expanding_it() -> TestResult {
        let mut fixture = backup_notes_fixture()?;
        for (anchor, head) in [(0, 2), (2, 0)] {
            fixture
                .app
                .rich
                .as_mut()
                .ok_or("missing rich session")?
                .session
                .set_selection(Selection {
                    anchor: DocPoint {
                        block_id: "first".into(),
                        offset: ScalarOffset(anchor),
                    },
                    head: DocPoint {
                        block_id: "first".into(),
                        offset: ScalarOffset(head),
                    },
                })?;
            assert!(fixture.app.note_action_unavailable_reason().is_none());
        }
        let selection = Selection {
            anchor: DocPoint {
                block_id: "first".into(),
                offset: ScalarOffset(0),
            },
            head: DocPoint {
                block_id: "tail".into(),
                offset: ScalarOffset(2),
            },
        };
        fixture
            .app
            .rich
            .as_mut()
            .ok_or("missing rich session")?
            .session
            .set_selection(selection.clone())?;
        assert_eq!(
            fixture.app.note_action_unavailable_reason(),
            Some("Select a passage within one text block to add a note.")
        );
        assert_eq!(
            fixture
                .app
                .rich
                .as_ref()
                .ok_or("missing rich session")?
                .session
                .selection(),
            Some(&selection)
        );
        assert_eq!(
            fixture.app.workspace.document(&fixture.first.id)?,
            fixture.first
        );
        Ok(())
    }

    fn fixture() -> AppResult<(Workspace, Document, PassageAnnotation, Editor)> {
        let workspace = Workspace::memory()?;
        let project = workspace.create_project("Synthetic note controller")?;
        let tree = RichTree::plain("p", "Я😀 echo echo")?;
        let document = workspace.create_rich_document(&project.id, "Notes", tree.to_json()?)?;
        let anchor = workspace.capture_annotation_anchor(
            &document.id,
            document.revision,
            "p",
            ScalarOffset(8),
            ScalarOffset(12),
            "echo",
        )?;
        let note = workspace.create_annotation(
            &document.id,
            document.revision,
            anchor,
            "Revise this passage",
        )?;
        Ok((workspace, document, note, Editor::new(tree, Revision(1))))
    }
    #[test]
    fn frame_maps_fresh_native_input_purely_without_metadata_writes() -> TestResult {
        let (workspace, document, note, mut editor) = fixture()?;
        let mut state = NotesState::default();
        state.prepare(&workspace, &document)?;
        let change = editor.apply(Transaction {
            base_revision: editor.revision(),
            commands: vec![Command::ReplaceText {
                range: TextRange {
                    block_id: "p".into(),
                    start: ScalarOffset(10),
                    end: ScalarOffset(10),
                },
                expected: "".into(),
                replacement: "中".into(),
            }],
        })?;
        let tokens = crate::theme::tokens(&egui::Context::default());
        let marks = state.frame(&[], Language::English, tokens).decorations(
            editor.document(),
            editor.revision(),
            &[change.replay_proof()],
        )?;
        assert_eq!(marks.len(), 1);
        assert_eq!(marks[0].range.head.offset, ScalarOffset(13));
        assert_eq!(workspace.annotation(&note.id)?, note);
        Ok(())
    }

    #[test]
    fn activating_partitioned_mark_reveals_its_full_anchor_then_allows_caret_placement()
    -> TestResult {
        let mut fixture = backup_notes_fixture()?;
        let anchor = fixture.app.workspace.capture_annotation_anchor(
            &fixture.first.id,
            fixture.first.revision,
            "first",
            ScalarOffset(0),
            ScalarOffset(7),
            "Я😀 echo",
        )?;
        let broad = fixture.app.workspace.create_annotation(
            &fixture.first.id,
            fixture.first.revision,
            anchor,
            "Context around the passage",
        )?;
        fixture.app.refresh_notes()?;
        fixture.app.notes.selected = Some(fixture.first_note.id.clone());
        let rich = fixture.app.rich.as_ref().ok_or("missing rich session")?;
        let tokens = crate::theme::tokens(&egui::Context::default());
        let marks = fixture
            .app
            .notes
            .frame(&[], Language::English, tokens)
            .decorations(rich.session.document(), rich.session.revision(), &[])?;
        let segment = marks
            .iter()
            .find(|mark| {
                fixture.app.notes.decoration_cache.target(&mark.id) == Some(broad.id.as_str())
            })
            .ok_or("missing visible broad segment")?;
        assert_eq!(segment.range.anchor.offset, ScalarOffset(0));
        assert_eq!(segment.range.head.offset, ScalarOffset(3));
        fixture.app.open_mark_note(&segment.id)?;
        assert_eq!(
            fixture.app.notes.selected.as_deref(),
            Some(broad.id.as_str())
        );
        let rich = fixture.app.rich.as_mut().ok_or("missing rich session")?;
        let full = selection_for(&broad, rich.session.document())?;
        assert_eq!(rich.session.selection(), Some(&full));
        let caret = Selection {
            anchor: DocPoint {
                block_id: "first".into(),
                offset: ScalarOffset(1),
            },
            head: DocPoint {
                block_id: "first".into(),
                offset: ScalarOffset(1),
            },
        };
        rich.session.set_selection(caret.clone())?;
        fixture.app.open_mark_note(&segment.id)?;
        assert_eq!(
            fixture
                .app
                .rich
                .as_ref()
                .ok_or("missing rich session")?
                .session
                .selection(),
            Some(&caret)
        );
        assert_eq!(fixture.app.workspace.annotation(&broad.id)?, broad);
        Ok(())
    }

    #[test]
    fn pointer_on_note_text_quote_or_preview_chip_activates_the_exact_note() -> TestResult {
        fn painted_text_center(shape: &egui::epaint::Shape, text: &str) -> Option<egui::Pos2> {
            match shape {
                egui::epaint::Shape::Text(shape) if shape.galley.job.text == text => {
                    Some(shape.visual_bounding_rect().center())
                }
                egui::epaint::Shape::Vec(shapes) => shapes
                    .iter()
                    .find_map(|shape| painted_text_center(shape, text)),
                _ => None,
            }
        }
        fn frame(
            ctx: &egui::Context,
            app: &mut StudioApp,
            note: &PassageAnnotation,
            events: Vec<egui::Event>,
            time: f64,
        ) -> AppResult<egui::FullOutput> {
            let mut clicked = false;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(640.0, 480.0),
                    )),
                    focused: true,
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |root| {
                    egui::CentralPanel::default().show(root, |ui| {
                        ui.set_max_width(340.0);
                        ui.style_mut().interaction.selectable_labels = true;
                        clicked |= note_row(
                            ui,
                            note,
                            "echo",
                            app.notes.selected.as_deref() == Some(note.id.as_str()),
                            app.notes.labels.catalog.as_ref(),
                            Language::English,
                            theme::tokens(ctx),
                        )
                        .clicked();
                    });
                },
            );
            output.textures_delta.clear();
            if clicked {
                app.activate_note(&note.id)?;
            }
            Ok(output)
        }
        let mut fixture = backup_notes_fixture()?;
        let note = fixture.app.workspace.update_annotation_markup(
            &fixture.first_note.id,
            fixture.first.revision,
            fixture.first_note.revision,
            AnnotationPatch::default(),
            Some(vec![
                "Motivation".into(),
                "This second label intentionally exceeds the remaining row width".into(),
                "Other".into(),
            ]),
        )?;
        fixture.app.refresh_notes()?;
        for target in [note.note.as_str(), "echo", "Motivation", "+2"] {
            let ctx = egui::Context::default();
            fixture.app.notes.selected = None;
            let rich = fixture.app.rich.as_mut().ok_or("missing rich session")?;
            let caret = Selection {
                anchor: DocPoint {
                    block_id: "first".into(),
                    offset: ScalarOffset(0),
                },
                head: DocPoint {
                    block_id: "first".into(),
                    offset: ScalarOffset(0),
                },
            };
            rich.session.set_selection(caret.clone())?;
            let output = frame(&ctx, &mut fixture.app, &note, vec![], 0.0)?;
            let point = output
                .shapes
                .iter()
                .find_map(|shape| painted_text_center(&shape.shape, target))
                .ok_or("missing painted click target")?;
            frame(
                &ctx,
                &mut fixture.app,
                &note,
                vec![egui::Event::PointerMoved(point)],
                0.1,
            )?;
            for (pressed, time) in [(true, 0.2), (false, 0.3)] {
                frame(
                    &ctx,
                    &mut fixture.app,
                    &note,
                    vec![egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    time,
                )?;
            }
            let rich = fixture.app.rich.as_ref().ok_or("missing rich session")?;
            if target == "+2" {
                assert!(egui::Popup::is_any_open(&ctx));
                assert!(fixture.app.notes.selected.is_none());
                assert_eq!(rich.session.selection(), Some(&caret));
            } else {
                assert_eq!(
                    fixture.app.notes.selected.as_deref(),
                    Some(note.id.as_str())
                );
                let full = selection_for(&note, rich.session.document())?;
                assert_eq!(rich.session.selection(), Some(&full), "{target}");
            }
        }
        assert_eq!(fixture.app.workspace.annotation(&note.id)?, note);
        Ok(())
    }

    #[test]
    fn refreshing_metadata_preserves_dirty_draft_and_detects_the_conflict() -> TestResult {
        let (workspace, document, note, _) = fixture()?;
        let mut state = NotesState::default();
        state.prepare(&workspace, &document)?;
        state.drafts.insert(
            note.id.clone(),
            NoteDraft {
                original: note.clone(),
                catalog: Some(workspace.project_labels(&document.project_id)?),
                tags: String::new(),
                body: "My unsaved draft".into(),
            },
        );
        workspace.update_annotation(
            &note.id,
            document.revision,
            note.revision,
            AnnotationPatch {
                note: Some("Changed elsewhere".into()),
                status: None,
            },
        )?;
        state.invalidate();
        state.prepare(&workspace, &document)?;
        let draft = state
            .drafts
            .get(&note.id)
            .ok_or("missing preserved draft")?;
        let current = state
            .preview
            .iter()
            .find(|current| current.id == note.id)
            .ok_or("missing current note")?;
        assert_eq!(draft.body, "My unsaved draft");
        assert_ne!(draft.original.revision, current.revision);
        Ok(())
    }
    #[test]
    fn changed_document_clears_selection_but_preserves_authored_drafts_for_return() -> TestResult {
        let (workspace, document, note, _) = fixture()?;
        let mut state = NotesState::default();
        state.prepare(&workspace, &document)?;
        state.activate(&note.id);
        state.drafts.insert(
            note.id.clone(),
            NoteDraft {
                original: note,
                catalog: Some(workspace.project_labels(&document.project_id)?),
                tags: String::new(),
                body: "Private draft".into(),
            },
        );
        let other = workspace.create_rich_document(
            &document.project_id,
            "Other",
            RichTree::plain("other", "Text")?.to_json()?,
        )?;
        state.prepare(&workspace, &other)?;
        assert!(state.selected.is_none());
        assert_eq!(state.drafts.len(), 1);
        assert!(state.has_unsaved_drafts());
        assert!(state.preview.is_empty());
        state.prepare(&workspace, &document)?;
        assert_eq!(
            state
                .drafts
                .values()
                .next()
                .ok_or("missing preserved draft")?
                .body,
            "Private draft"
        );
        Ok(())
    }
    #[test]
    fn displayed_note_quote_uses_authored_math_and_never_exposes_native_atom_scalars() -> TestResult
    {
        let tree = RichTree::from_json(
            serde_json::json!({"type":"doc","attrs":{"schemaVersion":4},"content":[{"type":"paragraph","attrs":{"blockId":"p"},"content":[{"type":"text","text":"中"},{"type":"inlineMath","attrs":{"latex":"x^2"}},{"type":"text","text":"文"}]}]}),
        )?;
        let anchor = AnnotationAnchor::from_selection(
            &tree,
            &TextPoint {
                block_id: "p".into(),
                offset: ScalarOffset(0),
            },
            &TextPoint {
                block_id: "p".into(),
                offset: ScalarOffset(3),
            },
        )?;
        assert_eq!(anchor.exact_text, "中\u{fffc}文");
        assert_eq!(
            displayed_quote(&anchor, Some(&tree), Language::English),
            "中$x^2$文"
        );
        for language in Language::ALL {
            assert!(!displayed_quote(&anchor, None, language).contains('\u{fffc}'));
        }
        Ok(())
    }
    fn field_frame(
        ctx: &egui::Context,
        body: &mut String,
        id: egui::Id,
        events: Vec<egui::Event>,
        time: f64,
    ) {
        ctx.memory_mut(|memory| memory.request_focus(id));
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(640.0, 480.0),
                )),
                time: Some(time),
                focused: true,
                events,
                ..Default::default()
            },
            |root| {
                egui::CentralPanel::default().show(root, |ui| {
                    note_input(ui, body, id, Language::English, theme::tokens(ctx), false);
                });
            },
        );
        output.textures_delta.clear();
    }
    #[test]
    fn full_unicode_paste_is_preserved_above_validation_limit() {
        let ctx = egui::Context::default();
        let id = egui::Id::new(("author-note-body", "large"));
        let mut body = String::new();
        field_frame(&ctx, &mut body, id, vec![], 0.0);
        let original = "Я😀中".repeat(studio_core::MAX_PROJECT_ANNOTATION_NOTE_CHARACTERS / 3 + 1);
        field_frame(
            &ctx,
            &mut body,
            id,
            vec![egui::Event::Paste(original.clone())],
            1.0,
        );
        assert_eq!(body, original);
        assert!(!valid_note_body(&body));
    }
    #[test]
    fn note_editor_accepts_the_full_source_capacity_using_unicode_scalars() {
        let limit = studio_core::MAX_PROJECT_ANNOTATION_NOTE_CHARACTERS;
        let body = "Я😀中".repeat(limit / 3);
        assert_eq!(body.chars().count(), limit);
        assert!(valid_note_body(&body));
        assert!(valid_note_body(&format!("  {body}\n")));
        assert!(!valid_note_body(&format!("{body}😀")));
        assert!(!valid_note_body(" \n\t"));
    }
    #[test]
    fn note_input_undo_history_cannot_restore_another_notes_body() {
        let ctx = egui::Context::default();
        let a = egui::Id::new(("author-note-body", "a"));
        let b = egui::Id::new(("author-note-body", "b"));
        let mut first = String::from("Alpha");
        field_frame(&ctx, &mut first, a, vec![], 0.0);
        field_frame(
            &ctx,
            &mut first,
            a,
            vec![egui::Event::Text("X".into())],
            1.0,
        );
        let mut second = String::from("Beta");
        field_frame(
            &ctx,
            &mut second,
            b,
            vec![egui::Event::Key {
                key: egui::Key::Z,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            }],
            2.0,
        );
        assert_eq!(second, "Beta");
        assert!(first.contains('X'));
    }
    #[test]
    fn failed_new_document_snapshot_never_exposes_previous_documents_notes_on_retry() -> TestResult
    {
        let (workspace, document, note, _) = fixture()?;
        let mut state = NotesState::default();
        state.prepare(&workspace, &document)?;
        state.activate(&note.id);
        let mut other = workspace.create_rich_document(
            &document.project_id,
            "Identical template",
            document.rich_json.clone().ok_or("missing tree")?,
        )?;
        other.revision += 1; // Simulate a stale captured document revision during an external update.
        assert!(state.prepare(&workspace, &other).is_err());
        assert!(state.preview.is_empty());
        assert!(state.selected.is_none());
        assert!(state.invalid);
        state.prepare(&workspace, &other)?;
        assert!(state.preview.is_empty());
        assert!(state.selected.is_none());
        assert!(state.invalid);
        Ok(())
    }
    #[test]
    fn failed_note_context_does_not_raise_new_errors_for_each_prose_revision() -> TestResult {
        let (workspace, document, _, mut editor) = fixture()?;
        let mut state = NotesState::default();
        let mut stale = document.clone();
        stale.revision += 1;
        assert!(state.prepare(&workspace, &stale).is_err());
        let t = theme::tokens(&egui::Context::default());
        for revision in 1..=3 {
            let mut frame = state.frame(&[], Language::English, t);
            assert!(
                frame
                    .decorations(editor.document(), Revision(revision), &[])
                    .is_err()
            );
            assert!(frame.take_error().is_none());
        }
        state.invalidate();
        state.prepare(&workspace, &document)?;
        editor.apply(Transaction {
            base_revision: editor.revision(),
            commands: vec![Command::ReplaceText {
                range: TextRange {
                    block_id: "p".into(),
                    start: ScalarOffset(0),
                    end: ScalarOffset(0),
                },
                expected: "".into(),
                replacement: "New ".into(),
            }],
        })?;
        for (index, revision) in (5..=7).enumerate() {
            let mut frame = state.frame(&[], Language::English, t);
            // Deliberately omit the required proofs: derived projection fails,
            // but only the first failure in this metadata epoch is reported.
            assert!(
                frame
                    .decorations(editor.document(), Revision(revision), &[])
                    .is_err()
            );
            assert_eq!(frame.take_error().is_some(), index == 0);
        }
        Ok(())
    }
    #[test]
    fn keyboard_note_menu_tracks_measured_caret_and_stays_in_visible_scrolled_editor() {
        let visible = egui::Rect::from_min_max(egui::pos2(300.0, 160.0), egui::pos2(900.0, 650.0));
        let caret = egui::Rect::from_min_size(egui::pos2(420.0, 360.0), egui::vec2(1.0, 24.0));
        assert_eq!(
            visible_menu_anchor(visible, Some(caret)),
            egui::pos2(420.0, 388.0)
        );
        let offscreen = caret.translate(egui::vec2(0.0, -2400.0));
        assert!(visible.contains(visible_menu_anchor(visible, Some(offscreen))));
        assert!(visible.contains(visible_menu_anchor(visible, None)));
    }
    #[test]
    fn saved_recovery_view_preserves_blocked_note_drafts_and_can_save_other_documents() -> TestResult
    {
        let directory = tempfile::tempdir()?;
        let context = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(context);
        let mut app = StudioApp::new(
            &creation,
            directory.path().join("readonly-recovery.sqlite"),
            false,
            None,
        )?;
        let project = app.workspace.create_project("Recovery guard")?;
        let a = app.workspace.create_rich_document(
            &project.id,
            "Saved",
            RichTree::plain("a", "echo")?.to_json()?,
        )?;
        let b = app.workspace.create_rich_document(
            &project.id,
            "Other",
            RichTree::plain("b", "other")?.to_json()?,
        )?;
        let aa = app.workspace.capture_annotation_anchor(
            &a.id,
            a.revision,
            "a",
            ScalarOffset(0),
            ScalarOffset(4),
            "echo",
        )?;
        let ba = app.workspace.capture_annotation_anchor(
            &b.id,
            b.revision,
            "b",
            ScalarOffset(0),
            ScalarOffset(5),
            "other",
        )?;
        let an = app
            .workspace
            .create_annotation(&a.id, a.revision, aa, "Saved note")?;
        let bn = app
            .workspace
            .create_annotation(&b.id, b.revision, ba, "Other note")?;
        let draft = app.workspace.save_rich_draft(
            &a.id,
            a.revision,
            None,
            RichTree::plain("a", "Unsaved preserved prose")?.to_json()?,
        )?;
        let snapshot = app.workspace.rich_recovery_snapshot(&a.id)?;
        app.open_saved_recovery(&snapshot)?;
        assert!(app.is_recovery_read_only());
        assert!(!app.can_edit_manuscript());
        app.notes.drafts.insert(
            an.id.clone(),
            NoteDraft {
                original: an.clone(),
                catalog: Some(app.workspace.project_labels(&a.project_id)?),
                tags: String::new(),
                body: "Preserved local note".into(),
            },
        );
        app.notes.drafts.insert(
            bn.id.clone(),
            NoteDraft {
                original: bn.clone(),
                catalog: Some(app.workspace.project_labels(&b.project_id)?),
                tags: String::new(),
                body: "Saved parked note".into(),
            },
        );
        assert!(app.begin_note().is_err());
        assert!(app.save_note_drafts().is_err());
        assert_eq!(app.workspace.annotation(&an.id)?, an);
        assert_eq!(app.workspace.annotation(&bn.id)?.note, "Saved parked note");
        assert_eq!(
            app.notes
                .drafts
                .get(&an.id)
                .ok_or("missing blocked draft")?
                .body,
            "Preserved local note"
        );
        assert_eq!(app.workspace.rich_draft(&a.id)?, Some(draft));
        assert_eq!(app.workspace.document(&a.id)?, a);
        Ok(())
    }
}
