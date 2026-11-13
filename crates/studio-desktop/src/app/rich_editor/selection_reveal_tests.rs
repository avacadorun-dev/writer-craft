//! Full-application hosted probes for selection reveal. Native OS acceptance remains separate.
//! Mount only after the exact CraftEditor selection-reveal revision has been qualified.
use super::super::navigation_return::NavigationReturn;
use super::*;
use craft_editor_model::Document as RichTree;
use egui::{Event, Modifiers, Pos2, RawInput, Rect, Vec2};

const WINDOW: Vec2 = Vec2::new(1100.0, 820.0);
const RANGE_BLOCK: &str = "p18";
const FAR_BLOCK: &str = "p36";
const RANGE_TEXT: &str = "At the pier, Mira unfolded the tide chart and covered the damaged date with her thumb. Lev waited beside the lamp, pretending not to notice the blue pencil mark.";

fn point(block: &str, offset: usize) -> DocPoint {
    DocPoint {
        block_id: block.into(),
        offset: ScalarOffset(offset),
    }
}

fn range(block: &str, reverse: bool) -> Selection {
    let start = point(block, 0);
    let end = point(block, RANGE_TEXT.chars().count());
    if reverse {
        Selection {
            anchor: end,
            head: start,
        }
    } else {
        Selection {
            anchor: start,
            head: end,
        }
    }
}

struct Observation {
    response: egui::Response,
    selected: Option<Rect>,
    offset: f32,
}

impl Observation {
    fn viewport(&self) -> Rect {
        self.response.interact_rect
    }

    fn selected(&self) -> AppResult<Rect> {
        self.selected
            .ok_or_else(|| "The completed manuscript frame has no measured selection".into())
    }
}

struct Fixture {
    _directory: tempfile::TempDir,
    app: StudioApp,
    ctx: egui::Context,
    frame: eframe::Frame,
    time: f64,
    origin: studio_core::Document,
    reference: studio_core::Document,
}

impl Fixture {
    /// Deliberately does not render: an unmeasured origin must have no scroll bookmark.
    fn new() -> AppResult<Self> {
        let directory = tempfile::tempdir()?;
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = StudioApp::new(
            &creation,
            directory.path().join("selection-reveal.sqlite"),
            false,
            None,
        )?;
        app.language = Language::English;
        app.navigator = true;
        app.inspector_visible = false;
        app.editor_preferences.values.spellcheck = false;
        let project = app.workspace.create_project("Breakwater notebook")?;
        let paragraphs: Vec<_> = (0..48).map(|index| {
            let prose = if index == 18 || index == 36 {
                RANGE_TEXT.to_owned()
            } else {
                let detail = [
                    "Mira counted the lamps along the breakwater. One window remained dark, although the keeper had promised to leave it lit.",
                    "Lev carried the repaired radio under his coat. He stopped whenever the wind sounded like a voice coming through its silent speaker.",
                    "The notebook smelled of salt and lamp oil. Between the tide tables, someone had pressed a narrow strip of blue paper.",
                    "At the foot of the stairs they found a dry footprint. It faced the sea, while every other mark led back toward the village.",
                ][index % 4];
                format!("Watch {}. {detail}", index + 1)
            };
            serde_json::json!({
                "type":"paragraph", "attrs":{"blockId":format!("p{index}")},
                "content":[{"type":"text","text":prose}]
            })
        }).collect();
        let tree = RichTree::from_json(serde_json::json!({
            "type":"doc", "attrs":{"schemaVersion":1}, "content":paragraphs
        }))?;
        let origin =
            app.workspace
                .create_rich_document(&project.id, "Night watch", tree.to_json()?)?;
        let reference = app.workspace.create_rich_document(
            &project.id,
            "Tide notebook",
            RichTree::plain("reference", "The noon tide reached the third step. The blue mark belongs to the previous keeper.")?.to_json()?,
        )?;
        app.open_project(project)?;
        app.open_document(origin.clone())?;
        Ok(Self {
            _directory: directory,
            app,
            ctx,
            frame: eframe::Frame::_new_kittest(),
            time: 0.0,
            origin,
            reference,
        })
    }

    fn rich(&self) -> AppResult<&RichState> {
        self.app
            .rich
            .as_ref()
            .ok_or_else(|| "Missing manuscript session".into())
    }

    fn rich_mut(&mut self) -> AppResult<&mut RichState> {
        self.app
            .rich
            .as_mut()
            .ok_or_else(|| "Missing manuscript session".into())
    }

    fn render(&mut self, events: Vec<Event>) -> AppResult<Observation> {
        self.time += 0.05;
        let mut input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, WINDOW)),
            focused: true,
            time: Some(self.time),
            events,
            ..Default::default()
        };
        eframe::App::raw_input_hook(&mut self.app, &self.ctx, &mut input);
        let ctx = self.ctx.clone();
        let mut observation = None;
        let mut output = ctx.run_ui(input, |ui| {
            eframe::App::ui(&mut self.app, ui, &mut self.frame);
            // Read this emitted pass before end_pass swaps widget registries. Reading afterward
            // can pair the previous Response origin with current engine geometry during animation.
            observation = Some((|| -> AppResult<Observation> {
                let rich = self.rich()?;
                let response = ctx
                    .read_response(rich.session.options.id)
                    .ok_or("The actual manuscript response was not emitted")?;
                let selected = rich.session.canvas_selection_rect(&response)?;
                let offset = rich
                    .measured_navigation_scroll_offset()
                    .ok_or("The completed manuscript frame has no measured scroll offset")?;
                assert!(offset.is_finite() && offset >= 0.0);
                assert!(response.interact_rect.is_finite() && response.interact_rect.is_positive());
                Ok(Observation {
                    response,
                    selected,
                    offset,
                })
            })());
        });
        output.textures_delta.clear();
        output.platform_output.commands.clear();
        assert!(
            self.app.error.is_none(),
            "Full-app selection reveal failed: {:?}",
            self.app.error.as_ref().map(|error| &error.details),
        );
        observation.ok_or("The full application pass was not observed")?
    }

    fn settle(&mut self) -> AppResult<Observation> {
        let mut previous = None;
        let mut stable = 0;
        for _ in 0..16 {
            let observed = self.render(vec![])?;
            let key = (observed.offset, observed.viewport(), observed.selected);
            if previous == Some(key) {
                stable += 1;
                if stable == 3 {
                    return Ok(observed);
                }
            } else {
                stable = 0;
            }
            previous = Some(key);
        }
        Err(format!("The full-app viewport did not settle within 16 frames: {previous:?}").into())
    }

    fn assert_author_state(
        &self,
        tree: &RichTree,
        revision: Revision,
        selection: &Selection,
        focus: Option<egui::Id>,
    ) -> AppResult<()> {
        let rich = self.rich()?;
        assert_eq!(rich.session.document(), tree);
        assert_eq!(rich.session.revision(), revision);
        assert_eq!(rich.session.selection(), Some(selection));
        assert_eq!(self.ctx.memory(|memory| memory.focused()), focus);
        assert_eq!(rich.pending_proof_count(), 0);
        assert!(
            !rich.session.can_undo(),
            "Presentation must not manufacture model history"
        );
        assert_eq!(self.app.workspace.document(&self.origin.id)?, self.origin);
        Ok(())
    }

    /// Observe an actual host scroll animation before its target becomes visible.
    fn start_far_reveal(&mut self, selection: &Selection) -> AppResult<Observation> {
        let rich = self.rich_mut()?;
        rich.session.set_selection(selection.clone())?;
        rich.session.cancel_scroll_request();
        let initial = self.render(vec![])?;
        assert!(initial.selected()?.top() > initial.viewport().bottom());
        self.rich_mut()?.session.request_scroll_to_selection();
        let mut samples = Vec::new();
        for _ in 0..8 {
            let observed = self.render(vec![])?;
            samples.push((observed.offset, observed.selected));
            if observed.offset > initial.offset + 0.5
                && observed.selected()?.top() > observed.viewport().bottom()
            {
                return Ok(observed);
            }
        }
        Err(format!(
            "No real in-flight scroll sample was observed: start {}, samples {samples:?}",
            initial.offset
        )
        .into())
    }

    fn assert_offset_for_five_frames(&mut self, saved: f32) -> AppResult<()> {
        assert!(
            saved > 0.0,
            "The fixture must capture a real nonzero bookmark"
        );
        for frame in 0..5 {
            let observed = self.render(vec![])?;
            assert!(
                (observed.offset - saved).abs() <= 0.1,
                "Exact restore lost to an old reveal/animation in frame {frame}: {} versus {saved}",
                observed.offset
            );
        }
        Ok(())
    }
}

#[test]
fn inspector_reflow_reveals_whole_forward_and_reverse_ranges_below_the_real_toolbar()
-> AppResult<()> {
    for reverse in [false, true] {
        let mut fixture = Fixture::new()?;
        fixture.settle()?;
        let selected = range(RANGE_BLOCK, reverse);
        let rich = fixture.rich_mut()?;
        let tree = rich.session.document().clone();
        let revision = rich.session.revision();
        rich.session.set_selection(selected.clone())?;
        rich.session.cancel_scroll_request();
        let manuscript = rich.session.options.id;
        fixture
            .ctx
            .memory_mut(|memory| memory.request_focus(manuscript));
        let measured = fixture.render(vec![])?;
        let below_offset = measured.selected()?.bottom() - measured.response.rect.top()
            + measured.viewport().height() * 0.5;
        fixture
            .rich_mut()?
            .restore_navigation_scroll(below_offset)?;
        let below = fixture.settle()?;
        assert!(below.offset > 0.0);
        assert!(
            below.selected()?.bottom() < below.viewport().top(),
            "Start below the complete marked passage"
        );

        // No explicit reveal request here: opening the real inspector must handle reflow.
        fixture.app.inspector = Inspector::Notes;
        fixture.app.inspector_visible = true;
        fixture.app.prefer_inspector = true;
        let shown = fixture.settle()?;
        assert!(
            shown.viewport().width() + 100.0 < measured.viewport().width(),
            "The fixture must exercise a real narrower manuscript pane"
        );
        let bounds = shown.selected()?;
        assert!(
            bounds.height()
                > fixture.app.font_size * fixture.app.editor_preferences.values.line_height,
            "The fixture must cover multiple measured visual lines: {bounds:?}"
        );
        assert!(
            shown.viewport().contains_rect(bounds),
            "Complete {reverse:?} selection {bounds:?} is clipped by the real post-toolbar viewport {:?}",
            shown.viewport()
        );
        fixture.assert_author_state(&tree, revision, &selected, Some(manuscript))?;

        fixture.render(vec![
            Event::PointerMoved(shown.viewport().center()),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(0.0, shown.viewport().height() + bounds.height() + 32.0),
                phase: egui::TouchPhase::Move,
                modifiers: Modifiers::NONE,
            },
        ])?;
        let manual = fixture.settle()?;
        assert!(
            manual.offset + 1.0 < shown.offset,
            "A consumed reveal must leave later wheel input manual"
        );
        assert!(
            !manual.viewport().intersects(manual.selected()?),
            "The wheel probe must leave the selected passage outside the viewport"
        );
        fixture.assert_offset_for_five_frames(manual.offset)?;
        fixture.assert_author_state(&tree, revision, &selected, Some(manuscript))?;
    }
    Ok(())
}

#[test]
fn exact_navigation_restore_wins_over_pending_reveal_and_host_animation_without_false_zero()
-> AppResult<()> {
    let mut fixture = Fixture::new()?;
    fixture.app.inspector = Inspector::Notes;
    fixture.app.inspector_visible = true;
    fixture.app.prefer_inspector = true;
    assert!(
        fixture
            .rich()?
            .measured_navigation_scroll_offset()
            .is_none()
    );
    let unmeasured = NavigationReturn::capture(&fixture.app)?;
    fixture.settle()?;
    fixture.rich_mut()?.restore_navigation_scroll(420.0)?;
    let later = fixture.settle()?;
    assert!((later.offset - 420.0).abs() <= 0.1);
    fixture.app.open_document(fixture.reference.clone())?;
    fixture.render(vec![])?;
    assert!(unmeasured.restore(&mut fixture.app)?);
    assert!(
        fixture
            .rich()?
            .measured_navigation_scroll_offset()
            .is_none()
    );
    assert!(
        fixture.rich()?.navigation_scroll_offset.is_none(),
        "An unmeasured bookmark must not manufacture a zero offset"
    );
    fixture.assert_offset_for_five_frames(later.offset)?;

    let selected = range(FAR_BLOCK, false);
    let manuscript = fixture.rich()?.session.options.id;
    fixture
        .ctx
        .memory_mut(|memory| memory.request_focus(manuscript));
    let tree = fixture.rich()?.session.document().clone();
    let revision = fixture.rich()?.session.revision();
    let moving = fixture.start_far_reveal(&selected)?;
    assert!(moving.offset > later.offset);
    let bookmark = NavigationReturn::capture(&fixture.app)?;
    let saved = moving.offset;
    let painted_selection = moving.selected()?;

    // Both the engine flag and a previously submitted egui animation are live here.
    fixture.rich_mut()?.session.request_scroll_to_selection();
    fixture.rich_mut()?.restore_navigation_scroll(saved)?;
    fixture.assert_offset_for_five_frames(saved)?;
    assert_eq!(
        fixture.render(vec![])?.selected()?,
        painted_selection,
        "Restore must return the painted passage to the same position, not the animation's next offset"
    );
    fixture.assert_author_state(&tree, revision, &selected, Some(manuscript))?;

    let moving_again = fixture.start_far_reveal(&selected)?;
    assert!(moving_again.offset > saved + 0.5);
    fixture.app.open_document(fixture.reference.clone())?;
    fixture.render(vec![])?;
    // The raw bookmark restores view state; its caller owns focus (return_to_writing requests it).
    let host_focus = fixture.ctx.memory(|memory| memory.focused());
    assert!(bookmark.restore(&mut fixture.app)?);
    assert_eq!(
        fixture.app.document.as_ref().map(|document| &document.id),
        Some(&fixture.origin.id)
    );
    fixture.assert_offset_for_five_frames(saved)?;
    assert_eq!(fixture.render(vec![])?.selected()?, painted_selection);
    fixture.assert_author_state(&tree, revision, &selected, host_focus)?;
    Ok(())
}
