//! Recoverable local markup actions. Manuscript navigation and project label definitions are separate.
use super::*;
use studio_core::ProjectLabelCatalog;

#[derive(Clone)]
pub(super) struct QuickMarkupUndo {
    after: PassageAnnotation,
    prior_tags: Option<Vec<String>>,
    catalog: ProjectLabelCatalog,
    document_title: String,
}

impl StudioApp {
    /// Remove a local mark in one metadata transaction, retaining its authored draft for recovery.
    pub(in crate::app) fn quick_remove_note(&mut self, note: &PassageAnnotation) -> AppResult<()> {
        let draft = self.notes.drafts.get(&note.id).cloned();
        let original = draft.as_ref().map_or(note, |draft| &draft.original);
        let (document_revision, annotation_revision) = self.note_guard_after_flush(original)?;
        let document = self.document.as_ref().ok_or("missing_note_document")?;
        // Capture Undo's vocabulary without requiring an unchanged catalog for plain deletion.
        let undo_catalog = self.workspace.project_labels(&document.project_id)?;
        let tags = draft.as_ref().map(|draft| parsed_tags(&draft.tags));
        let changed_tags = tags
            .as_deref()
            .filter(|tags| *tags != original.tags.as_slice());
        let labels = if changed_tags.is_some() {
            Some(
                draft
                    .as_ref()
                    .and_then(|draft| draft.catalog.as_ref())
                    .ok_or("missing_draft_label_catalog")?,
            )
        } else {
            None
        };
        let body = draft
            .as_ref()
            .filter(|draft| draft.body != original.note)
            .map(|draft| draft.body.as_str());
        let after = self.workspace.remove_annotation_with_draft(
            &note.id,
            document_revision,
            annotation_revision,
            body,
            changed_tags,
            labels,
        )?;
        let undo = QuickMarkupUndo {
            after,
            prior_tags: None,
            catalog: undo_catalog,
            document_title: document.title.clone(),
        };
        self.finish_quick_removal(undo, Self::refresh_notes);
        Ok(())
    }

    /// Unassign one label immediately; no project definition or other passage is removed.
    pub(in crate::app) fn quick_remove_label(
        &mut self,
        note: &PassageAnnotation,
        name: &str,
    ) -> AppResult<()> {
        let draft = self.notes.drafts.get(&note.id).cloned();
        let original = draft.as_ref().map_or(note, |draft| &draft.original);
        let prior_tags = draft
            .as_ref()
            .map_or_else(|| original.tags.clone(), |draft| parsed_tags(&draft.tags));
        let catalog = if prior_tags != original.tags {
            draft.as_ref().and_then(|draft| draft.catalog.clone())
        } else {
            self.notes.labels.catalog.clone()
        }
        .ok_or("missing_draft_label_catalog")?;
        let (document_revision, annotation_revision) = self.note_guard_after_flush(original)?;
        let document_title = self
            .document
            .as_ref()
            .ok_or("missing_note_document")?
            .title
            .clone();
        let body = draft
            .as_ref()
            .filter(|draft| draft.body != original.note)
            .map(|draft| draft.body.as_str());
        let after = self.workspace.remove_annotation_label_with_draft(
            &note.id,
            document_revision,
            annotation_revision,
            name,
            body,
            Some(&prior_tags),
            &catalog,
        )?;
        let undo = QuickMarkupUndo {
            after,
            prior_tags: Some(prior_tags),
            catalog,
            document_title,
        };
        self.finish_quick_removal(undo, Self::refresh_notes);
        Ok(())
    }

    fn finish_quick_removal(
        &mut self,
        undo: QuickMarkupUndo,
        refresh: impl FnOnce(&mut Self) -> AppResult<()>,
    ) {
        // Receipt acknowledgement is independent of the fallible read that follows it.
        // A stale UI/refresh error must never retry the already committed mutation.
        self.notes.drafts.remove(&undo.after.id);
        if undo.after.deleted {
            self.notes.preview.retain(|note| note.id != undo.after.id);
            if self.notes.selected.as_deref() == Some(undo.after.id.as_str()) {
                self.notes.selected = None;
            }
        }
        self.notes.acknowledge(&undo.after);
        self.status = if undo.prior_tags.is_some() {
            "Label removed"
        } else {
            "Mark removed"
        }
        .into();
        self.notes.quick_undo = Some(undo);
        let result = refresh(self);
        self.record_error(result);
    }

    /// Undo only the most recent local removal; durable older tombstones remain in Project markup.
    pub(in crate::app) fn undo_quick_markup(&mut self) -> AppResult<()> {
        let undo = self
            .notes
            .quick_undo
            .clone()
            .ok_or("There is no recent markup removal to undo")?;
        if self
            .document
            .as_ref()
            .is_none_or(|document| document.id != undo.after.document_id)
        {
            return Err("Return to this document to undo markup removal".into());
        }
        if self
            .notes
            .drafts
            .get(&undo.after.id)
            .is_some_and(NoteDraft::changed)
        {
            return Err("Save or cancel the pending note edits before Undo".into());
        }
        let restored =
            if undo.after.deleted {
                self.ensure_note_writable()?;
                self.save()?;
                let current = self.document.as_ref().ok_or("missing_note_document")?;
                self.workspace.restore_annotation_with_labels(
                    &undo.after.id,
                    current.revision,
                    undo.after.revision,
                    &undo.catalog,
                )?
            } else {
                let (document_revision, annotation_revision) =
                    self.note_guard_after_flush(&undo.after)?;
                self.workspace.update_annotation_markup_with_labels(
                    &undo.after.id,
                    document_revision,
                    annotation_revision,
                    AnnotationPatch::default(),
                    Some(undo.prior_tags.clone().ok_or(
                        "The original label assignment is unavailable; use Project markup",
                    )?),
                    &undo.catalog,
                )?
            };
        // Clear the consumed receipt before refresh: a subsequent click cannot replay Undo.
        self.notes.quick_undo = None;
        self.notes.drafts.remove(&restored.id);
        self.notes.acknowledge(&restored);
        self.status = "Markup restored".into();
        let result = self.refresh_notes();
        self.record_error(result);
        Ok(())
    }

    pub(in crate::app) fn quick_undo_control(&mut self, ui: &mut egui::Ui, t: Tokens) {
        let Some(undo) = self.notes.quick_undo.as_ref() else {
            return;
        };
        let l = self.language;
        let current = self
            .document
            .as_ref()
            .is_some_and(|document| document.id == undo.after.document_id);
        let caption = if undo.prior_tags.is_some() {
            "Label removed"
        } else {
            "Mark removed"
        };
        let help = if current {
            "Restore this mark or label assignment. Earlier removed marks remain in Project markup."
        } else {
            "Return to this document to undo markup removal"
        };
        let mut clicked = false;
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(l.text(caption)).small().color(t.muted))
                .on_hover_text(format!(
                    "{}\n{}",
                    undo.document_title, undo.after.anchor.exact_text
                ));
            clicked = ui
                .add_enabled(current, egui::Button::new(l.text("Undo")))
                .on_hover_text(l.text(help))
                .clicked();
        });
        if clicked {
            let result = self.undo_quick_markup();
            self.record_error(result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::ProjectLabelColor;

    struct Fixture {
        _directory: tempfile::TempDir,
        app: StudioApp,
        document: Document,
        note: PassageAnnotation,
        ctx: egui::Context,
        frame: eframe::Frame,
        time: f64,
    }

    impl Fixture {
        fn new() -> AppResult<Self> {
            let directory = tempfile::tempdir()?;
            let ctx = egui::Context::default();
            let creation = eframe::CreationContext::_new_kittest(ctx.clone());
            let mut app = StudioApp::new(
                &creation,
                directory.path().join("quick-notes.sqlite"),
                false,
                None,
            )?;
            app.language = Language::English;
            let project = app.workspace.create_project("The Last Light")?;
            let body = format!(
                "The light remains. {}",
                "Asya studies the signal log. ".repeat(180)
            );
            let document = app.workspace.create_rich_document(
                &project.id,
                "Night watch",
                RichTree::plain("p", &body)?.to_json()?,
            )?;
            let anchor = app.workspace.capture_annotation_anchor(
                &document.id,
                document.revision,
                "p",
                ScalarOffset(4),
                ScalarOffset(9),
                "light",
            )?;
            let note = app.workspace.create_tagged_annotation(
                &document.id,
                document.revision,
                anchor,
                "Initial note",
                &["Asya".into(), "Chronology".into()],
            )?;
            app.open_project(project)?;
            app.open_document(document.clone())?;
            app.refresh_notes()?;
            Ok(Self {
                _directory: directory,
                app,
                document,
                note,
                ctx,
                frame: eframe::Frame::_new_kittest(),
                time: 0.0,
            })
        }

        fn draft(&mut self, note: &PassageAnnotation, body: &str) -> AppResult<()> {
            self.app.notes.drafts.insert(
                note.id.clone(),
                NoteDraft {
                    original: note.clone(),
                    body: body.into(),
                    tags: note.tags.join("\n"),
                    catalog: Some(
                        self.app
                            .workspace
                            .project_labels(&self.document.project_id)?,
                    ),
                },
            );
            Ok(())
        }

        fn render(&mut self) {
            self.time += 0.1;
            let mut input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1440.0, 940.0),
                )),
                focused: true,
                time: Some(self.time),
                ..Default::default()
            };
            eframe::App::raw_input_hook(&mut self.app, &self.ctx, &mut input);
            let mut output = self.ctx.run_ui(input, |ui| {
                eframe::App::ui(&mut self.app, ui, &mut self.frame)
            });
            output.textures_delta.clear();
            output.platform_output.commands.clear();
            assert!(
                self.app.error.is_none(),
                "{:?}",
                self.app.error.as_ref().map(|error| &error.details)
            );
        }

        fn settled_manuscript_scroll(&mut self) -> AppResult<f32> {
            let mut previous = None;
            let mut stable_frames = 0;
            // Native caret reveal uses egui's animated scroll target (up to 0.3 s).
            // Observe completion instead of treating the second 0.1 s frame as a bookmark.
            for _ in 0..16 {
                self.render();
                let current = self
                    .app
                    .rich
                    .as_ref()
                    .ok_or("rich session")?
                    .measured_navigation_scroll_offset()
                    .ok_or("measured manuscript scroll")?;
                if current.is_finite() && current > 0.0 && previous == Some(current) {
                    stable_frames += 1;
                    if stable_frames == 3 {
                        return Ok(current);
                    }
                } else {
                    stable_frames = 0;
                }
                previous = Some(current);
            }
            Err("manuscript scroll did not settle within the bounded frame budget".into())
        }
    }

    #[test]
    fn quick_remove_and_undo_preserve_latest_draft_exact_identity_and_live_manuscript_position()
    -> AppResult<()> {
        let mut fixture = Fixture::new()?;
        fixture.render();
        fixture.render();
        let point = DocPoint {
            block_id: "p".into(),
            offset: ScalarOffset(fixture.document.body.chars().count()),
        };
        let selection = Selection {
            anchor: point.clone(),
            head: point,
        };
        let rich = fixture.app.rich.as_mut().ok_or("rich session")?;
        rich.session.set_selection(selection.clone())?;
        rich.session.request_scroll_to_caret();
        let scroll = fixture.settled_manuscript_scroll()?;
        // A no-action control frame must preserve the measured baseline before removal.
        fixture.render();
        let rich = fixture.app.rich.as_ref().ok_or("rich session")?;
        let revision = rich.session.revision();
        assert_eq!(rich.measured_navigation_scroll_offset(), Some(scroll));
        assert_eq!(rich.session.selection(), Some(&selection));
        let note = fixture.note.clone();
        fixture.draft(&note, "She keeps the log to protect Lev.")?;
        fixture.app.quick_remove_note(&note)?;
        assert!(!fixture.app.notes.drafts.contains_key(&note.id));
        assert!(fixture.app.notes.quick_undo.is_some());
        assert!(
            fixture
                .app
                .workspace
                .annotations(&fixture.document.id)?
                .is_empty()
        );
        // Keep observing long enough to catch a new delayed scroll from deletion itself.
        for _ in 0..5 {
            fixture.render();
            let rich = fixture.app.rich.as_ref().ok_or("rich session")?;
            assert_eq!(rich.session.selection(), Some(&selection));
            assert_eq!(rich.session.revision(), revision);
            assert_eq!(rich.measured_navigation_scroll_offset(), Some(scroll));
        }
        fixture.app.undo_quick_markup()?;
        let restored = fixture.app.workspace.annotation(&note.id)?;
        assert_eq!(restored.id, note.id);
        assert_eq!(restored.note, "She keeps the log to protect Lev.");
        assert_eq!(restored.tags, note.tags);
        assert_eq!(restored.anchor, note.anchor);
        for _ in 0..5 {
            fixture.render();
            let rich = fixture.app.rich.as_ref().ok_or("rich session")?;
            assert_eq!(rich.session.selection(), Some(&selection));
            assert_eq!(rich.session.revision(), revision);
            assert_eq!(rich.measured_navigation_scroll_offset(), Some(scroll));
        }
        assert_eq!(
            fixture.app.workspace.document(&fixture.document.id)?,
            fixture.document
        );
        assert!(fixture.app.notes.quick_undo.is_none());
        assert!(fixture.app.undo_quick_markup().is_err());
        Ok(())
    }

    #[test]
    fn label_undo_preserves_saved_body_and_refuses_to_overwrite_a_new_pending_draft()
    -> AppResult<()> {
        let mut fixture = Fixture::new()?;
        let note = fixture.note.clone();
        fixture.draft(&note, "Latest authored motivation")?;
        fixture.app.quick_remove_label(&note, "Asya")?;
        let removed = fixture.app.workspace.annotation(&note.id)?;
        assert_eq!(removed.tags, ["Chronology"]);
        assert_eq!(removed.note, "Latest authored motivation");
        fixture.draft(&removed, "New unsaved wording after removal")?;
        assert!(fixture.app.undo_quick_markup().is_err());
        assert_eq!(
            fixture.app.notes.drafts[&note.id].body,
            "New unsaved wording after removal"
        );
        assert!(fixture.app.notes.quick_undo.is_some());
        assert_eq!(fixture.app.workspace.annotation(&note.id)?, removed);
        // Explicitly cancel this later draft, then Undo only the earlier assignment removal.
        fixture.app.notes.drafts.remove(&note.id);
        fixture.app.undo_quick_markup()?;
        let restored = fixture.app.workspace.annotation(&note.id)?;
        assert_eq!(restored.tags, note.tags);
        assert_eq!(restored.note, removed.note);
        assert_eq!(restored.anchor, note.anchor);
        assert_eq!(
            fixture.app.workspace.document(&fixture.document.id)?,
            fixture.document
        );
        Ok(())
    }

    #[test]
    fn stale_label_or_note_actions_keep_pending_draft_and_previous_undo_receipt() -> AppResult<()> {
        let mut fixture = Fixture::new()?;
        let note = fixture.note.clone();
        let other = fixture.app.workspace.create_tagged_annotation(
            &fixture.document.id,
            1,
            note.anchor.clone(),
            "Earlier correction",
            &note.tags,
        )?;
        fixture.app.refresh_notes()?;
        fixture.app.quick_remove_note(&other)?;
        fixture.draft(&note, "Pending wording stays local")?;
        let catalog = fixture
            .app
            .workspace
            .project_labels(&fixture.document.project_id)?;
        fixture.app.workspace.set_project_label_color(
            &catalog,
            "Asya",
            ProjectLabelColor::Violet,
        )?;
        assert!(fixture.app.quick_remove_label(&note, "Asya").is_err());
        assert_eq!(fixture.app.workspace.annotation(&note.id)?, note);
        assert_eq!(fixture.app.workspace.annotation_history(&note.id)?.len(), 1);
        let external = fixture.app.workspace.update_annotation(
            &note.id,
            1,
            note.revision,
            AnnotationPatch {
                note: Some("External metadata revision".into()),
                status: None,
            },
        )?;
        assert!(fixture.app.quick_remove_note(&note).is_err());
        assert_eq!(fixture.app.workspace.annotation(&note.id)?, external);
        assert_eq!(
            fixture.app.notes.drafts[&note.id].body,
            "Pending wording stays local"
        );
        assert_eq!(
            fixture
                .app
                .notes
                .quick_undo
                .as_ref()
                .map(|undo| undo.after.id.as_str()),
            Some(other.id.as_str())
        );
        assert_eq!(fixture.app.workspace.annotation_history(&note.id)?.len(), 2);
        Ok(())
    }

    #[test]
    fn a_committed_removal_keeps_undo_and_clears_its_draft_even_if_refresh_fails() -> AppResult<()>
    {
        let mut fixture = Fixture::new()?;
        let note = fixture.note.clone();
        fixture.draft(&note, "Latest authored recovery text")?;
        let catalog = fixture
            .app
            .workspace
            .project_labels(&fixture.document.project_id)?;
        let after = fixture.app.workspace.remove_annotation_with_draft(
            &note.id,
            1,
            note.revision,
            Some("Latest authored recovery text"),
            None,
            None,
        )?;
        fixture.app.finish_quick_removal(
            QuickMarkupUndo {
                after,
                prior_tags: None,
                catalog,
                document_title: fixture.document.title.clone(),
            },
            |_| Err("synthetic refresh failure".into()),
        );
        assert!(fixture.app.error.is_some());
        assert!(fixture.app.notes.invalid);
        assert!(!fixture.app.notes.drafts.contains_key(&note.id));
        assert!(fixture.app.notes.quick_undo.is_some());
        assert!(fixture.app.quick_remove_note(&note).is_err());
        assert_eq!(fixture.app.workspace.annotation_history(&note.id)?.len(), 2);
        fixture.app.undo_quick_markup()?;
        let restored = fixture.app.workspace.annotation(&note.id)?;
        assert_eq!(restored.note, "Latest authored recovery text");
        assert_eq!(restored.id, note.id);
        assert_eq!(fixture.app.workspace.annotation_history(&note.id)?.len(), 3);
        assert!(fixture.app.notes.quick_undo.is_none());
        Ok(())
    }
}
