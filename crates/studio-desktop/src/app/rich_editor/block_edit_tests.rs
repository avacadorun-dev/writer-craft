//! Compiled-widget probes for direct existing-block source editing.
//! Synthetic egui input through actual widgets; operating-system interaction remains separate.
use super::*;
use craft_editor_model::Document as RichTree;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};

const WINDOW: Vec2 = Vec2::new(1100.0, 820.0);

struct Fixture {
    _directory: tempfile::TempDir,
    app: StudioApp,
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
            directory.path().join("block-edit.sqlite"),
            false,
            None,
        )?;
        app.language = Language::English;
        app.navigator = true;
        app.inspector_visible = false;
        app.editor_preferences.values.spellcheck = false;
        let project = app.workspace.create_project("Block editing")?;
        let tree = RichTree::from_json(serde_json::json!({
            "type":"doc", "attrs":{"schemaVersion":4}, "content":[
                {"type":"paragraph","attrs":{"blockId":"p"},"content":[{"type":"text","text":"Opening prose."}]},
                {"type":"mermaidBlock","attrs":{"blockId":"diagram","code":"flowchart LR\nA-->B","caption":"Diagram"}},
                {"type":"codeBlock","attrs":{"blockId":"code","language":"rust"},"content":[{"type":"text","text":"fn main() {}"}]}
            ]
        }))?;
        let document =
            app.workspace
                .create_rich_document(&project.id, "Diagram", tree.to_json()?)?;
        app.open_project(project)?;
        app.open_document(document)?;
        Ok(Self {
            _directory: directory,
            app,
            ctx,
            frame: eframe::Frame::_new_kittest(),
            time: 0.0,
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

    fn render(&mut self, events: Vec<Event>) -> AppResult<egui::Response> {
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
        let mut response = None;
        let mut output = ctx.run_ui(input, |ui| {
            eframe::App::ui(&mut self.app, ui, &mut self.frame);
            response = self
                .rich()
                .ok()
                .and_then(|rich| ctx.read_response(rich.session.options.id));
        });
        output.textures_delta.clear();
        output.platform_output.commands.clear();
        assert!(
            self.app.error.is_none(),
            "Full-app block editing failed: {:?}",
            self.app.error.as_ref().map(|error| &error.details),
        );
        response.ok_or_else(|| "The actual manuscript response was not emitted".into())
    }

    fn settle(&mut self) -> AppResult<egui::Response> {
        let mut last = None;
        for _ in 0..12 {
            let response = self.render(vec![])?;
            if last.as_ref() == Some(&response.interact_rect) {
                return Ok(response);
            }
            last = Some(response.interact_rect);
        }
        Err("The manuscript surface did not settle within 12 frames".into())
    }

    fn selected_node_rect(&mut self) -> AppResult<Rect> {
        let target = self
            .rich()?
            .session
            .document()
            .node_ref(&NodePath(vec![1]))?;
        self.rich_mut()?.session.select_node(target)?;
        let response = self.settle()?;
        self.rich()?
            .session
            .canvas_selection_rect(&response)?
            .ok_or_else(|| "The selected diagram has no measured native frame".into())
    }

    fn click_at(&mut self, pos: Pos2) -> AppResult<()> {
        self.press_at(pos, PointerButton::Primary)
    }

    fn right_click_at(&mut self, pos: Pos2) -> AppResult<()> {
        self.press_at(pos, PointerButton::Secondary)
    }

    fn press_at(&mut self, pos: Pos2, button: PointerButton) -> AppResult<()> {
        self.render(vec![Event::PointerMoved(pos)])?;
        for pressed in [true, false] {
            self.render(vec![Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers: Modifiers::NONE,
            }])?;
        }
        Ok(())
    }
}

#[test]
fn single_click_selects_the_diagram_and_a_second_click_opens_its_source() -> AppResult<()> {
    let mut fixture = Fixture::new()?;
    let rect = fixture.selected_node_rect()?;
    // Return to ordinary prose so the gesture, not a seeded node selection, must select it.
    let point = DocPoint {
        block_id: "p".into(),
        offset: ScalarOffset(2),
    };
    fixture.rich_mut()?.session.set_selection(Selection {
        anchor: point.clone(),
        head: point,
    })?;
    fixture.settle()?;
    assert!(!fixture.rich()?.dialog_open());

    fixture.click_at(rect.center())?;
    assert!(
        fixture.rich()?.session.node_selection().is_some(),
        "the single click selects the rendered diagram"
    );
    assert!(
        !fixture.rich()?.dialog_open(),
        "the single click must not open a source dialog"
    );

    fixture.click_at(rect.center())?;
    assert!(
        fixture.rich()?.dialog_open(),
        "the immediate second click opens the existing source dialog"
    );
    Ok(())
}

#[test]
fn pressing_f2_on_a_focused_code_caret_opens_the_same_source_dialog() -> AppResult<()> {
    let mut fixture = Fixture::new()?;
    fixture.settle()?;
    let point = DocPoint {
        block_id: "code".into(),
        offset: ScalarOffset(3),
    };
    fixture.rich_mut()?.session.set_selection(Selection {
        anchor: point.clone(),
        head: point,
    })?;
    let ctx = fixture.ctx.clone();
    fixture.rich_mut()?.session.request_canvas_focus(&ctx);
    fixture.render(vec![])?;
    assert!(!fixture.rich()?.dialog_open());
    fixture.render(vec![Event::Key {
        key: Key::F2,
        physical_key: Some(Key::F2),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }])?;
    assert!(
        fixture.rich()?.dialog_open(),
        "F2 opens the existing source dialog for the focused code block"
    );
    Ok(())
}

#[test]
fn right_click_on_the_selected_diagram_offers_block_editing() -> AppResult<()> {
    let mut fixture = Fixture::new()?;
    let rect = fixture.selected_node_rect()?;
    let point = DocPoint {
        block_id: "p".into(),
        offset: ScalarOffset(2),
    };
    fixture.rich_mut()?.session.set_selection(Selection {
        anchor: point.clone(),
        head: point,
    })?;
    fixture.settle()?;
    fixture.click_at(rect.center())?;
    assert!(fixture.rich()?.session.node_selection().is_some());
    fixture.right_click_at(rect.center())?;
    let response = fixture
        .ctx
        .read_response(fixture.rich()?.session.options.id)
        .ok_or("The manuscript response was not emitted")?;
    let popup_id = response.id.with("rich-selection-menu");
    assert!(
        egui::Popup::is_id_open(&fixture.ctx, popup_id),
        "the right-click opens the manuscript context menu"
    );
    assert!(
        fixture.ctx.data(|data| data
            .get_temp::<bool>(popup_id.with("block-edit"))
            .unwrap_or(false)),
        "the object-specific Edit block entry survives the opening frame"
    );
    Ok(())
}
