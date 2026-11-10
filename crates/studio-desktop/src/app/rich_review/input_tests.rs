//! Hosted controller/widget regressions; these fixtures are not native acceptance evidence.
use super::*;
use craft_editor_model::ScalarOffset;
use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect};
use studio_core::{AnchorStatus, AnnotationPatch, PassageAnnotation};

struct Fixture {
    _directory: tempfile::TempDir,
    app: StudioApp,
    ctx: egui::Context,
    first: Document,
    second: Document,
    first_mark: PassageAnnotation,
    second_mark: PassageAnnotation,
    time: f64,
}

impl Fixture {
    fn new() -> AppResult<Self> {
        let directory = tempfile::tempdir()?;
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = StudioApp::new(
            &creation,
            directory.path().join("review-input.sqlite"),
            false,
            None,
        )?;
        app.language = Language::English;
        let project = app.workspace.create_project("The Last Light")?;
        let first = app.workspace.create_rich_document(
            &project.id,
            "Signal log",
            RichTree::plain("p", "Я😀 echo echo")?.to_json()?,
        )?;
        let second = app.workspace.create_rich_document(
            &project.id,
            "Archive",
            RichTree::plain("q", "Nina opens the log.")?.to_json()?,
        )?;
        let first_anchor = app.workspace.capture_annotation_anchor(
            &first.id,
            1,
            "p",
            ScalarOffset(3),
            ScalarOffset(7),
            "echo",
        )?;
        let second_anchor = app.workspace.capture_annotation_anchor(
            &second.id,
            1,
            "q",
            ScalarOffset(0),
            ScalarOffset(4),
            "Nina",
        )?;
        let first_mark = app.workspace.create_annotation(
            &first.id,
            1,
            first_anchor,
            "Clarify the first signal",
        )?;
        let second_mark = app.workspace.create_annotation(
            &second.id,
            1,
            second_anchor,
            "Clarify Nina's decision",
        )?;
        app.open_project(project)?;
        app.open_document(first.clone())?;
        Ok(Self {
            _directory: directory,
            app,
            ctx,
            first,
            second,
            first_mark,
            second_mark,
            time: 0.0,
        })
    }

    fn select(&mut self, block: &str, start: usize, end: usize) -> AppResult<()> {
        self.app
            .rich
            .as_mut()
            .ok_or("rich session")?
            .session
            .set_selection(Selection {
                anchor: DocPoint {
                    block_id: block.into(),
                    offset: ScalarOffset(start),
                },
                head: DocPoint {
                    block_id: block.into(),
                    offset: ScalarOffset(end),
                },
            })?;
        Ok(())
    }

    fn begin_queue(&mut self) -> AppResult<()> {
        self.app.begin_markup_review(
            vec![self.first_mark.id.clone(), self.second_mark.id.clone()],
            &self.first_mark.id,
        )
    }

    fn selection(&self) -> AppResult<&Selection> {
        self.app
            .rich
            .as_ref()
            .and_then(|rich| rich.session.selection())
            .ok_or_else(|| "selected passage".into())
    }

    fn render_panel(&mut self, events: Vec<Event>) -> egui::FullOutput {
        self.time += 0.1;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 600.0))),
            focused: true,
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let mut output = self.ctx.run_ui(input, |ui| {
            let tokens = crate::theme::tokens(ui.ctx());
            self.app.rich_review_panel(ui, tokens);
        });
        output.textures_delta.clear();
        output.platform_output.commands.clear();
        assert!(
            self.app.error.is_none(),
            "{:?}",
            self.app.error.as_ref().map(|error| &error.details)
        );
        output
    }
}

fn painted_text(output: &egui::FullOutput, text: &str) -> Option<Rect> {
    fn visit(shape: &egui::epaint::Shape, text: &str, clip: Rect) -> Option<Rect> {
        match shape {
            egui::epaint::Shape::Text(shape) if shape.galley.job.text == text => {
                let rect = shape.visual_bounding_rect().intersect(clip);
                (rect.is_positive() && rect.is_finite()).then_some(rect)
            }
            egui::epaint::Shape::Vec(shapes) => {
                shapes.iter().find_map(|shape| visit(shape, text, clip))
            }
            _ => None,
        }
    }
    output
        .shapes
        .iter()
        .find_map(|shape| visit(&shape.shape, text, shape.clip_rect))
}

#[test]
fn repeated_capture_and_cross_document_queue_navigation_keep_each_authored_replacement()
-> AppResult<()> {
    let mut fixture = Fixture::new()?;
    fixture.select("p", 8, 12)?;
    fixture.app.capture_review_selection()?;
    fixture.app.rich_review.replacements[0] = "The second signal was deliberate".into();
    fixture.app.rich_review.reason = "Compare the missing entry".into();
    fixture.app.capture_review_selection()?;
    assert_eq!(
        fixture.app.rich_review.replacements,
        ["The second signal was deliberate"]
    );
    assert!(fixture.app.rich_review.parked.is_empty());
    fixture.begin_queue()?;
    fixture.app.rich_review.replacements[0] = "The first signal was mistaken".into();
    fixture.app.step_markup_review(true)?;
    assert_eq!(
        fixture.app.document.as_ref().map(|document| &document.id),
        Some(&fixture.second.id)
    );
    fixture.app.rich_review.replacements[0] = "The archivist".into();
    for _ in 0..2 {
        fixture.app.step_markup_review(false)?;
        assert_eq!(
            fixture.app.rich_review.replacements,
            ["The first signal was mistaken"]
        );
        assert_eq!(fixture.selection()?.anchor.offset, ScalarOffset(3));
        fixture.app.step_markup_review(true)?;
        assert_eq!(fixture.app.rich_review.replacements, ["The archivist"]);
        assert_eq!(fixture.selection()?.anchor.block_id, "q");
    }
    fixture.app.step_markup_review(false)?;
    fixture.select("p", 8, 12)?;
    fixture.app.capture_review_selection()?;
    assert_eq!(
        fixture.app.rich_review.replacements,
        ["The second signal was deliberate"]
    );
    assert_eq!(fixture.app.rich_review.reason, "Compare the missing entry");
    assert_eq!(fixture.app.rich_review.parked.len(), 2);
    assert_eq!(
        fixture.app.workspace.document(&fixture.first.id)?,
        fixture.first
    );
    assert_eq!(
        fixture.app.workspace.document(&fixture.second.id)?,
        fixture.second
    );
    assert!(
        fixture
            .app
            .workspace
            .rich_proposals(&fixture.first.project_id, &fixture.first.id, None, 16)?
            .is_empty()
    );
    Ok(())
}

#[test]
fn visible_apply_above_long_wording_reuses_the_prepared_receipt_without_duplicate_proposals()
-> AppResult<()> {
    let mut fixture = Fixture::new()?;
    fixture.select("p", 8, 12)?;
    fixture.app.capture_review_selection()?;
    let wording = "The last signal came from the lighthouse. ".repeat(90);
    fixture.app.rich_review.replacements[0] = wording.clone();
    fixture.app.stage_rich_selected_edit()?;
    let id = fixture
        .app
        .rich_review
        .prepared
        .as_ref()
        .ok_or("prepared receipt")?
        .proposal
        .summary
        .id
        .clone();
    fixture.app.stage_rich_selected_edit()?;
    assert_eq!(
        fixture.app.workspace.document(&fixture.first.id)?,
        fixture.first
    );
    assert_eq!(
        fixture
            .app
            .workspace
            .rich_proposals(&fixture.first.project_id, &fixture.first.id, None, 16)?
            .len(),
        1
    );
    // Measure the top of the wording pane, without auto-scrolling an unfocused test caret.
    fixture.app.rich_review.focus = false;
    fixture.render_panel(vec![]);
    let output = fixture.render_panel(vec![]);
    let apply = painted_text(&output, "Apply changes").ok_or("visible primary Apply label")?;
    let editor = painted_text(&output, "Proposed text").ok_or("visible wording heading")?;
    assert!(
        apply.bottom() < editor.top(),
        "Apply must precede the local wording scroll area"
    );
    assert!(Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 600.0)).contains_rect(apply));
    let point = apply.center();
    fixture.render_panel(vec![
        Event::PointerMoved(point),
        Event::PointerButton {
            pos: point,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
    ]);
    fixture.render_panel(vec![Event::PointerButton {
        pos: point,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    let proposals = fixture.app.workspace.rich_proposals(
        &fixture.first.project_id,
        &fixture.first.id,
        None,
        16,
    )?;
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].id, id);
    assert_eq!(proposals[0].status, ProposalStatus::Applied);
    assert_eq!(
        fixture.app.workspace.document(&fixture.first.id)?.body,
        format!("Я😀 echo {wording}")
    );
    assert_eq!(
        fixture
            .app
            .workspace
            .rich_revisions(&fixture.first.id)?
            .len(),
        2
    );
    assert!(fixture.app.rich_review.prepared.is_none());
    Ok(())
}

#[test]
fn review_capture_refreshes_a_same_revision_reattachment_to_the_second_identical_occurrence()
-> AppResult<()> {
    let mut fixture = Fixture::new()?;
    // The application has already cached the first occurrence at this document revision.
    fixture.app.refresh_notes()?;
    let anchor = fixture.app.workspace.capture_annotation_anchor(
        &fixture.first.id,
        1,
        "p",
        ScalarOffset(8),
        ScalarOffset(12),
        "echo",
    )?;
    let moved = fixture.app.workspace.reattach_annotation(
        &fixture.first_mark.id,
        1,
        fixture.first_mark.revision,
        anchor,
    )?;
    fixture.begin_queue()?;
    let attachment = fixture
        .app
        .rich_review
        .attachment
        .as_ref()
        .ok_or("fresh review attachment")?;
    assert_eq!(attachment.base_revision, 1);
    assert_eq!(attachment.ranges[0].start, ScalarOffset(8));
    assert_eq!(attachment.ranges[0].end, ScalarOffset(12));
    assert_eq!(
        fixture.app.rich_review.markup_revision,
        Some(moved.revision)
    );
    assert_eq!(
        (
            fixture.selection()?.anchor.offset,
            fixture.selection()?.head.offset
        ),
        (ScalarOffset(8), ScalarOffset(12))
    );
    assert_eq!(
        fixture.app.workspace.document(&fixture.first.id)?,
        fixture.first
    );
    Ok(())
}

#[test]
fn queue_skips_deleted_and_detached_entries_without_trapping_navigation_or_guessing_a_quote()
-> AppResult<()> {
    let mut fixture = Fixture::new()?;
    let removed = fixture.app.workspace.create_annotation(
        &fixture.first.id,
        1,
        fixture.first_mark.anchor.clone(),
        "Completed correction",
    )?;
    let third = fixture.app.workspace.create_rich_document(
        &fixture.first.project_id,
        "Retired draft",
        RichTree::plain("r", "echo")?.to_json()?,
    )?;
    let anchor = fixture.app.workspace.capture_annotation_anchor(
        &third.id,
        1,
        "r",
        ScalarOffset(0),
        ScalarOffset(4),
        "echo",
    )?;
    let detached = fixture
        .app
        .workspace
        .create_annotation(&third.id, 1, anchor, "Old passage")?;
    fixture.app.begin_markup_review(
        vec![
            fixture.first_mark.id.clone(),
            removed.id.clone(),
            detached.id.clone(),
            fixture.second_mark.id.clone(),
        ],
        &fixture.first_mark.id,
    )?;
    fixture.app.rich_review.replacements[0] = "Keep this unfinished wording".into();
    fixture
        .app
        .workspace
        .delete_annotation(&removed.id, 1, removed.revision)?;
    fixture.app.workspace.save_rich_document(
        &third.id,
        1,
        RichTree::plain("r", "A different passage; echo appears later.")?.to_json()?,
    )?;
    assert_eq!(
        fixture
            .app
            .workspace
            .annotation(&detached.id)?
            .anchor_status,
        AnchorStatus::Detached
    );
    fixture.app.step_markup_review(true)?;
    let queue = fixture
        .app
        .rich_review
        .queue
        .as_ref()
        .ok_or("markup queue")?;
    assert_eq!(queue.index, 3);
    assert_eq!(queue.skipped, 2);
    assert!(queue.unavailable.contains(&removed.id) && queue.unavailable.contains(&detached.id));
    assert_eq!(
        fixture.app.document.as_ref().map(|document| &document.id),
        Some(&fixture.second.id)
    );
    assert_eq!(fixture.selection()?.anchor.block_id, "q");
    fixture.app.step_markup_review(false)?;
    assert_eq!(
        fixture.app.rich_review.replacements,
        ["Keep this unfinished wording"]
    );
    let selection = fixture.selection()?.clone();
    fixture.app.workspace.delete_annotation(
        &fixture.second_mark.id,
        1,
        fixture.second_mark.revision,
    )?;
    fixture.app.step_markup_review(true)?;
    fixture.app.step_markup_review(false)?;
    let queue = fixture
        .app
        .rich_review
        .queue
        .as_ref()
        .ok_or("markup queue")?;
    assert_eq!(queue.index, 0);
    assert!(!queue.has_neighbor(true));
    assert_eq!(fixture.selection()?, &selection);
    assert_eq!(
        fixture.app.workspace.document(&fixture.first.id)?,
        fixture.first
    );
    Ok(())
}

#[test]
fn stale_annotation_or_document_apply_preserves_wording_receipt_and_current_canonical_text()
-> AppResult<()> {
    let mut fixture = Fixture::new()?;
    fixture.begin_queue()?;
    fixture.app.rich_review.replacements[0] = "Unapplied first signal".into();
    fixture.app.stage_rich_selected_edit()?;
    let first_id = fixture
        .app
        .rich_review
        .prepared
        .as_ref()
        .ok_or("first prepared receipt")?
        .proposal
        .summary
        .id
        .clone();
    fixture.app.workspace.update_annotation(
        &fixture.first_mark.id,
        1,
        fixture.first_mark.revision,
        AnnotationPatch {
            note: Some("Changed correction meaning".into()),
            status: None,
        },
    )?;
    assert!(fixture.app.apply_inline_review(false).is_err());
    assert_eq!(
        fixture.app.rich_review.replacements,
        ["Unapplied first signal"]
    );
    assert_eq!(
        fixture.app.workspace.document(&fixture.first.id)?,
        fixture.first
    );
    assert_eq!(
        fixture
            .app
            .workspace
            .rich_proposal(&fixture.first.project_id, &first_id)?
            .summary
            .status,
        ProposalStatus::Staged
    );
    fixture.app.step_markup_review(true)?;
    fixture.app.capture_review_selection()?;
    fixture.app.rich_review.replacements[0] = "Unapplied archive wording".into();
    fixture.app.stage_rich_selected_edit()?;
    let second_id = fixture
        .app
        .rich_review
        .prepared
        .as_ref()
        .ok_or("second prepared receipt")?
        .proposal
        .summary
        .id
        .clone();
    let mut formatting = fixture
        .second
        .rich_json
        .clone()
        .ok_or("structured source")?;
    formatting["content"][0]["content"][0]["marks"] = serde_json::json!([{"type":"italic"}]);
    let changed = fixture
        .app
        .workspace
        .save_rich_document(&fixture.second.id, 1, formatting)?;
    fixture.app.open_document(changed.clone())?;
    assert!(fixture.app.apply_inline_review(false).is_err());
    assert_eq!(
        fixture.app.rich_review.replacements,
        ["Unapplied archive wording"]
    );
    assert_eq!(
        fixture
            .app
            .rich_review
            .prepared
            .as_ref()
            .map(|prepared| &prepared.proposal.summary.id),
        Some(&second_id)
    );
    assert_eq!(fixture.app.workspace.document(&fixture.second.id)?, changed);
    let proposals = fixture.app.workspace.rich_proposals(
        &fixture.second.project_id,
        &fixture.second.id,
        None,
        16,
    )?;
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].status, ProposalStatus::Staged);
    Ok(())
}

#[test]
fn removing_review_markup_offers_resume_only_for_a_retained_unapplied_replacement() -> AppResult<()>
{
    for apply_first in [false, true] {
        let mut fixture = Fixture::new()?;
        fixture.begin_queue()?;
        fixture.app.rich_review.replacements[0] = "A deliberate signal".into();
        if apply_first {
            fixture.app.apply_inline_review(false)?;
        }
        let canonical = fixture.app.workspace.document(&fixture.first.id)?;
        fixture.app.rich_review.focus = false;
        fixture.render_panel(vec![]);
        let output = fixture.render_panel(vec![]);
        let remove = painted_text(&output, "Delete markup")
            .ok_or("visible direct markup removal")?
            .center();
        fixture.render_panel(vec![
            Event::PointerMoved(remove),
            Event::PointerButton {
                pos: remove,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ]);
        fixture.render_panel(vec![Event::PointerButton {
            pos: remove,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]);
        let output = fixture.render_panel(vec![]);
        assert_eq!(painted_text(&output, "Resume edit").is_some(), !apply_first);
        assert_eq!(fixture.app.rich_review.paused, !apply_first);
        assert_eq!(
            fixture.app.workspace.document(&fixture.first.id)?,
            canonical
        );
        assert_eq!(fixture.app.rich_review.attachment.is_some(), !apply_first);
        if !apply_first {
            assert_eq!(
                fixture.app.rich_review.replacements,
                ["A deliberate signal"]
            );
        }
    }
    Ok(())
}

fn click_review_label(fixture: &mut Fixture, label: &str) -> AppResult<()> {
    fixture.app.rich_review.focus = false;
    fixture.render_panel(vec![]);
    let output = fixture.render_panel(vec![]);
    let pos = painted_text(&output, label)
        .ok_or_else(|| format!("missing action: {label}"))?
        .center();
    fixture.render_panel(vec![
        Event::PointerMoved(pos),
        Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
    ]);
    fixture.render_panel(vec![Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    Ok(())
}

#[test]
fn applied_note_can_be_resolved_with_one_visible_action_then_next_end_and_reopen() -> AppResult<()>
{
    let mut fixture = Fixture::new()?;
    fixture.begin_queue()?;
    // Refining only the first two scalars of the marked occurrence must keep its note/queue.
    fixture.select("p", 3, 5)?;
    fixture.app.capture_review_selection()?;
    assert_eq!(
        fixture.app.rich_review.markup_id.as_deref(),
        Some(fixture.first_mark.id.as_str())
    );
    assert!(fixture.app.rich_review.queue.is_some());
    fixture.app.rich_review.replacements[0] = "an".into();
    click_review_label(&mut fixture, "Apply changes")?;
    let first_applied = fixture.app.workspace.document(&fixture.first.id)?;
    assert_eq!(first_applied.body, "Я😀 anho echo");
    click_review_label(&mut fixture, "Resolve note and next")?;
    let resolved = fixture.app.workspace.annotation(&fixture.first_mark.id)?;
    assert_eq!(resolved.status, studio_core::AnnotationStatus::Resolved);
    assert!(!resolved.deleted);
    assert_eq!(resolved.note, fixture.first_mark.note);
    assert_eq!(
        fixture.app.workspace.document(&fixture.first.id)?,
        first_applied
    );
    assert_eq!(
        fixture.app.document.as_ref().map(|d| &d.id),
        Some(&fixture.second.id)
    );
    fixture.app.step_markup_review(false)?;
    assert_eq!(
        fixture.app.document.as_ref().map(|d| &d.id),
        Some(&fixture.second.id)
    );
    fixture.app.rich_review.replacements[0] = "The archivist".into();
    click_review_label(&mut fixture, "Apply changes")?;
    let second_applied = fixture.app.workspace.document(&fixture.second.id)?;
    click_review_label(&mut fixture, "Resolve note")?;
    let output = fixture.render_panel(vec![]);
    assert!(painted_text(&output, "End of review list").is_some());
    assert!(fixture.app.rich_review.queue_finished);
    assert_eq!(
        fixture
            .app
            .workspace
            .annotation(&fixture.second_mark.id)?
            .status,
        studio_core::AnnotationStatus::Resolved
    );
    click_review_label(&mut fixture, "Reopen note")?;
    assert_eq!(
        fixture
            .app
            .workspace
            .annotation(&fixture.second_mark.id)?
            .status,
        studio_core::AnnotationStatus::Open
    );
    assert_eq!(
        fixture.app.workspace.document(&fixture.second.id)?,
        second_applied
    );
    assert!(!fixture.app.rich_review.queue_finished);
    Ok(())
}

#[test]
fn unrelated_selection_cannot_resolve_the_previous_review_note() -> AppResult<()> {
    let mut fixture = Fixture::new()?;
    fixture.begin_queue()?;
    fixture.select("p", 8, 12)?;
    fixture.app.capture_review_selection()?;
    assert!(fixture.app.rich_review.markup_id.is_none());
    assert!(fixture.app.rich_review.queue.is_none());
    fixture.app.rich_review.replacements[0] = "different".into();
    fixture.app.apply_inline_review(false)?;
    assert!(fixture.app.rich_review.applied_markup.is_none());
    assert!(fixture.app.resolve_applied_review_note(true).is_err());
    assert_eq!(
        fixture
            .app
            .workspace
            .annotation(&fixture.first_mark.id)?
            .status,
        studio_core::AnnotationStatus::Open
    );
    Ok(())
}

#[test]
fn stale_note_refuses_completion_without_replaying_applied_text_or_losing_its_receipt()
-> AppResult<()> {
    let mut fixture = Fixture::new()?;
    fixture.begin_queue()?;
    fixture.app.rich_review.replacements[0] = "a signal".into();
    fixture.app.apply_inline_review(false)?;
    let canonical = fixture.app.workspace.document(&fixture.first.id)?;
    let note = fixture.app.workspace.annotation(&fixture.first_mark.id)?;
    let changed = fixture.app.workspace.update_annotation(
        &note.id,
        canonical.revision,
        note.revision,
        AnnotationPatch {
            note: Some("A later editorial question".into()),
            status: None,
        },
    )?;
    assert!(fixture.app.resolve_applied_review_note(true).is_err());
    assert_eq!(fixture.app.workspace.annotation(&note.id)?, changed);
    assert_eq!(
        fixture.app.workspace.document(&fixture.first.id)?,
        canonical
    );
    assert_eq!(
        fixture
            .app
            .rich_review
            .selected
            .as_ref()
            .map(|p| p.summary.status),
        Some(ProposalStatus::Applied)
    );
    assert!(!fixture.app.rich_review.queue_finished);
    Ok(())
}
