//! Regression probes for board-owned tool gestures; synthetic widget events, not OS input.
use super::*;
use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};
use serde_json::{Value, json};

fn fixture(
    content: Value,
) -> AppResult<(tempfile::TempDir, Workspace, CanvasState, egui::Context)> {
    let directory = tempfile::tempdir()?;
    let workspace = Workspace::open(directory.path().join("input.sqlite"))?;
    let project = workspace.create_project("Synthetic board input")?;
    let snapshot = workspace.create_project_canvas(&project.id, "Board", content)?;
    let state = CanvasState::from_snapshot(&project.id, snapshot)?;
    let ctx = egui::Context::default();
    ctx.data_mut(|data| {
        data.insert_temp(egui::Id::new("connected-capture-active"), true);
    });
    Ok((directory, workspace, state, ctx))
}

fn frame(
    state: &mut CanvasState,
    database: &Path,
    ctx: &egui::Context,
    events: Vec<Event>,
    time: f64,
) -> Rect {
    let mut input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 680.0))),
        events,
        time: Some(time),
        focused: true,
        ..Default::default()
    };
    state.prepare_input(ctx, &mut input, false);
    let tokens = theme::resolve(&ThemeSettings::default(), false);
    let mut outside = Rect::NOTHING;
    let mut output = ctx.run_ui(input, |root| {
        egui::Panel::left("test-navigation")
            .exact_size(140.0)
            .show(root, |ui| {
                outside = ui.button("Another pane").rect;
            });
        egui::CentralPanel::default().show(root, |ui| {
            state.show(ui, database, Language::English, tokens, false);
        });
    });
    output.textures_delta.clear();
    output.platform_output.commands.clear();
    outside
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn click(pos: Pos2) -> Vec<Event> {
    vec![
        Event::PointerMoved(pos),
        button(pos, true),
        button(pos, false),
    ]
}

fn canvas_observation(ctx: &egui::Context) -> AppResult<Value> {
    super::super::capture_connected_writing::test_observation(ctx)
        .1
        .remove("canvas")
        .ok_or_else(|| "the actual board pass must emit its observation".into())
}

fn rect(value: &Value) -> AppResult<Rect> {
    let values = value.as_array().ok_or("observed rectangle")?;
    Ok(Rect::from_min_max(
        egui::pos2(
            values[0].as_f64().ok_or("left")? as f32,
            values[1].as_f64().ok_or("top")? as f32,
        ),
        egui::pos2(
            values[2].as_f64().ok_or("right")? as f32,
            values[3].as_f64().ok_or("bottom")? as f32,
        ),
    ))
}

fn text_control(ctx: &egui::Context) -> AppResult<Rect> {
    super::super::capture_connected_writing::test_observation(ctx)
        .0
        .remove("canvas.add_text")
        .ok_or_else(|| "rendered Text card control".into())
}

fn control(ctx: &egui::Context, name: &str) -> AppResult<Rect> {
    super::super::capture_connected_writing::test_observation(ctx)
        .0
        .remove(name)
        .ok_or_else(|| format!("rendered control {name}").into())
}

fn settle_write(state: &mut CanvasState, ctx: &egui::Context) -> AppResult<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while state.pending.is_some() && Instant::now() < deadline {
        state.poll(ctx)?;
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(state.pending.is_none(), "board write did not complete");
    assert!(state.failure.is_none(), "{:?}", state.failure);
    Ok(())
}

#[test]
fn toolbar_toggles_and_other_panes_should_never_place_a_card() -> AppResult<()> {
    let (directory, workspace, mut state, ctx) = fixture(json!({"nodes":[],"edges":[]}))?;
    let database = directory.path().join("input.sqlite");
    frame(&mut state, &database, &ctx, vec![], 0.0);
    let outside = frame(&mut state, &database, &ctx, vec![], 0.1);
    let text = text_control(&ctx)?.center();

    frame(&mut state, &database, &ctx, click(text), 1.0);
    assert_eq!(state.tool, BoardTool::Text);
    assert!(state.text_editor.is_none(), "arming must not create a card");
    assert!(state.pending.is_none());

    frame(&mut state, &database, &ctx, click(outside.center()), 2.0);
    assert_eq!(state.tool, BoardTool::Text);
    assert!(state.text_editor.is_none(), "another pane owns this click");
    assert!(state.pending.is_none());
    assert!(state.press_origin.is_none());

    frame(&mut state, &database, &ctx, click(text), 3.0);
    assert_eq!(state.tool, BoardTool::Select);
    assert!(
        state.text_editor.is_none(),
        "disarming must not create a card"
    );
    let saved = workspace.project_canvas(&state.project_id, &state.snapshot.canvas.id)?;
    assert_eq!(saved.revision.info.revision, 1);
    assert_eq!(saved.revision.content, json!({"nodes":[],"edges":[]}));
    Ok(())
}

#[test]
fn board_drag_and_space_pan_should_not_place_but_a_subsequent_click_should() -> AppResult<()> {
    let (directory, workspace, mut state, ctx) = fixture(json!({"nodes":[],"edges":[]}))?;
    let database = directory.path().join("input.sqlite");
    frame(&mut state, &database, &ctx, vec![], 0.0);
    frame(&mut state, &database, &ctx, vec![], 0.1);
    let text = text_control(&ctx)?.center();
    frame(&mut state, &database, &ctx, click(text), 1.0);
    let viewport = rect(&canvas_observation(&ctx)?["viewport"])?;
    let start = viewport.center() - vec2(viewport.width() * 0.15, 0.0);
    let end = viewport.center() + vec2(viewport.width() * 0.15, 0.0);
    frame(
        &mut state,
        &database,
        &ctx,
        vec![Event::PointerMoved(start), button(start, true)],
        2.0,
    );
    frame(
        &mut state,
        &database,
        &ctx,
        vec![Event::PointerMoved(end)],
        2.1,
    );
    frame(&mut state, &database, &ctx, vec![button(end, false)], 2.2);
    assert!(
        state.text_editor.is_none(),
        "marquee movement is not a click"
    );
    assert!(state.pending.is_none());
    assert!(
        state.press_origin.is_none(),
        "a release consumes the old press"
    );
    assert_eq!(state.snapshot.revision.info.revision, 1);

    // Releasing Space before the mouse must not turn the completed pan into placement,
    // even when its pointer distance still qualifies as a click.
    let point = viewport.center();
    let space = |pressed| Event::Key {
        key: egui::Key::Space,
        physical_key: Some(egui::Key::Space),
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    frame(
        &mut state,
        &database,
        &ctx,
        vec![space(true), Event::PointerMoved(point), button(point, true)],
        2.4,
    );
    frame(&mut state, &database, &ctx, vec![space(false)], 2.5);
    frame(&mut state, &database, &ctx, vec![button(point, false)], 2.6);
    assert!(state.text_editor.is_none(), "Space owns the completed pan");
    assert!(state.pending.is_none());
    assert!(state.press_origin.is_none());

    frame(&mut state, &database, &ctx, click(viewport.center()), 4.0);
    assert!(
        state.text_editor.is_some(),
        "a real placement enters direct input"
    );
    settle_write(&mut state, &ctx)?;
    let saved = workspace.project_canvas(&state.project_id, &state.snapshot.canvas.id)?;
    assert_eq!(saved.revision.info.revision, 2);
    assert_eq!(
        saved.revision.content["nodes"].as_array().map(Vec::len),
        Some(1)
    );
    Ok(())
}

#[test]
fn select_mode_should_keep_empty_board_double_click_creation() -> AppResult<()> {
    let (directory, workspace, mut state, ctx) = fixture(json!({"nodes":[],"edges":[]}))?;
    let database = directory.path().join("input.sqlite");
    frame(&mut state, &database, &ctx, vec![], 0.0);
    frame(&mut state, &database, &ctx, vec![], 0.1);
    let point = rect(&canvas_observation(&ctx)?["viewport"])?.center();
    frame(&mut state, &database, &ctx, click(point), 1.0);
    assert!(state.text_editor.is_none());
    frame(&mut state, &database, &ctx, click(point), 1.1);
    assert_eq!(state.tool, BoardTool::Select);
    assert!(
        state.text_editor.is_some(),
        "the native double-click remains reachable"
    );
    settle_write(&mut state, &ctx)?;
    let saved = workspace.project_canvas(&state.project_id, &state.snapshot.canvas.id)?;
    assert_eq!(saved.revision.info.revision, 2);
    assert_eq!(
        saved.revision.content["nodes"].as_array().map(Vec::len),
        Some(1)
    );
    Ok(())
}

#[test]
fn connecting_overlapping_cards_should_use_the_topmost_painted_card() -> AppResult<()> {
    let source = json!({"nodes":[
        {"id":"bottom","type":"text","x":0,"y":0,"width":300,"height":180,"text":"Bottom"},
        {"id":"top","type":"text","x":0,"y":0,"width":300,"height":180,"text":"Top"},
        {"id":"target","type":"text","x":500,"y":0,"width":300,"height":180,"text":"Target"}
    ],"edges":[]});
    let (directory, workspace, mut state, ctx) = fixture(source.clone())?;
    let database = directory.path().join("input.sqlite");
    frame(&mut state, &database, &ctx, vec![], 0.0);
    frame(&mut state, &database, &ctx, vec![], 0.1);
    let controls = super::super::capture_connected_writing::test_observation(&ctx).0;
    let connect = controls.get("canvas.connect").ok_or("Connection control")?;
    frame(&mut state, &database, &ctx, click(connect.center()), 1.0);
    let observed = canvas_observation(&ctx)?;
    let nodes = observed["visible_nodes"]
        .as_array()
        .ok_or("visible cards")?;
    let top = nodes
        .iter()
        .find(|node| node["node_id"] == "top")
        .ok_or("top card")?;
    let target = nodes
        .iter()
        .find(|node| node["node_id"] == "target")
        .ok_or("target card")?;
    frame(
        &mut state,
        &database,
        &ctx,
        click(rect(&top["reserved"])?.center()),
        2.0,
    );
    assert_eq!(state.connect_source.as_deref(), Some("top"));
    assert!(state.pending.is_none());
    frame(
        &mut state,
        &database,
        &ctx,
        click(rect(&target["reserved"])?.center()),
        3.0,
    );
    settle_write(&mut state, &ctx)?;
    let saved = workspace.project_canvas(&state.project_id, &state.snapshot.canvas.id)?;
    assert_eq!(saved.revision.content["edges"][0]["fromNode"], "top");
    assert_eq!(saved.revision.content["edges"][0]["toNode"], "target");
    assert_eq!(saved.revision.content["nodes"], source["nodes"]);
    Ok(())
}

#[test]
fn explicit_handles_should_create_a_loop_between_different_sides() -> AppResult<()> {
    let source = json!({"nodes":[
        {"id":"card","type":"text","x":0,"y":0,"width":300,"height":180,"text":"Loop", "future":{"keep":true}}
    ],"edges":[]});
    let (directory, workspace, mut state, ctx) = fixture(source.clone())?;
    let database = directory.path().join("input.sqlite");
    frame(&mut state, &database, &ctx, vec![], 0.0);
    frame(&mut state, &database, &ctx, vec![], 0.1);
    let connect = control(&ctx, "canvas.connect")?.center();
    frame(&mut state, &database, &ctx, click(connect), 1.0);
    // Controls are emitted from the actual visible card geometry, including clipping.
    let right = control(&ctx, "canvas.handle.card.right")?.center();
    let top = control(&ctx, "canvas.handle.card.top")?.center();
    frame(&mut state, &database, &ctx, click(right), 2.0);
    assert_eq!(state.connect_source.as_deref(), Some("card"));
    assert_eq!(state.connect_source_side, Some(Side::Right));
    assert!(state.pending.is_none());
    frame(&mut state, &database, &ctx, click(top), 3.0);
    settle_write(&mut state, &ctx)?;
    let saved = workspace.project_canvas(&state.project_id, &state.snapshot.canvas.id)?;
    assert_eq!(saved.revision.info.revision, 2);
    assert_eq!(saved.revision.content["nodes"], source["nodes"]);
    assert_eq!(
        saved.revision.content["edges"].as_array().map(Vec::len),
        Some(1)
    );
    let edge = &saved.revision.content["edges"][0];
    assert_eq!(edge["fromNode"], "card");
    assert_eq!(edge["toNode"], "card");
    assert_eq!(edge["fromSide"], "right");
    assert_eq!(edge["toSide"], "top");
    Ok(())
}

fn labeled_graph() -> Value {
    json!({"nodes":[
        {"id":"a","type":"text","x":0,"y":0,"width":200,"height":180,"text":"A"},
        {"id":"b","type":"text","x":600,"y":0,"width":200,"height":180,"text":"B"}
    ],"edges":[{"id":"edge","fromNode":"a","toNode":"b","label":"Visible connection"}]})
}

#[test]
fn label_double_click_should_open_properties_without_creating_a_card() -> AppResult<()> {
    let source = labeled_graph();
    let (directory, workspace, mut state, ctx) = fixture(source.clone())?;
    let database = directory.path().join("input.sqlite");
    frame(&mut state, &database, &ctx, vec![], 0.0);
    frame(&mut state, &database, &ctx, vec![], 0.1);
    let label = control(&ctx, "canvas.connection.painted.edge")?.center();
    frame(&mut state, &database, &ctx, click(label), 1.0);
    assert!(state.view.selection().edges.contains("edge"));
    frame(&mut state, &database, &ctx, click(label), 1.1);
    assert!(
        state.connection_editor.is_some(),
        "the visible label opens its properties"
    );
    assert!(state.text_editor.is_none(), "a label is not empty board");
    assert!(state.pending.is_none());
    let saved = workspace.project_canvas(&state.project_id, &state.snapshot.canvas.id)?;
    assert_eq!(saved.revision.info.revision, 1);
    assert_eq!(saved.revision.content, source);
    Ok(())
}

#[test]
fn connection_menu_should_select_its_observed_row_without_mutating_the_board() -> AppResult<()> {
    let source = labeled_graph();
    let (directory, workspace, mut state, ctx) = fixture(source.clone())?;
    let database = directory.path().join("input.sqlite");
    frame(&mut state, &database, &ctx, vec![], 0.0);
    frame(&mut state, &database, &ctx, vec![], 0.1);
    let menu = control(&ctx, "canvas.connections")?.center();
    frame(&mut state, &database, &ctx, click(menu), 1.0);
    // Allow the popup's sizing pass to settle before using its observed row.
    frame(&mut state, &database, &ctx, vec![], 1.1);
    let row = control(&ctx, "canvas.connection.select.edge")?;
    frame(&mut state, &database, &ctx, vec![], 1.2);
    assert_eq!(control(&ctx, "canvas.connection.select.edge")?, row);
    frame(&mut state, &database, &ctx, click(row.center()), 2.0);
    assert!(state.view.selection().edges.contains("edge"));
    assert!(state.pending.is_none());
    assert!(state.text_editor.is_none());
    let saved = workspace.project_canvas(&state.project_id, &state.snapshot.canvas.id)?;
    assert_eq!(saved.revision.info.revision, 1);
    assert_eq!(saved.revision.content, source);
    Ok(())
}

#[test]
fn embedded_media_retry_should_not_complete_a_connection_or_retain_its_press() -> AppResult<()> {
    let source = json!({"nodes":[
        {"id":"source","type":"text","x":0,"y":0,"width":300,"height":220,"text":"Source"},
        {"id":"media","type":"file","x":500,"y":0,"width":300,"height":220,"file":"inert-missing.png"}
    ],"edges":[]});
    let (directory, workspace, mut state, ctx) = fixture(source.clone())?;
    let database = directory.path().join("input.sqlite");
    // The core correctly refuses missing resource references. Model that failure
    // only in this test's in-memory projection; the durable graph stays legal.
    let mut projected = state.snapshot.clone();
    projected.revision.content["nodes"][1]["x-writers-studio"] =
        json!({"resource":{"kind":"media","id":"synthetic-missing-media"}});
    state = CanvasState::from_snapshot(&state.project_id, projected)?;
    frame(&mut state, &database, &ctx, vec![], 0.0);
    frame(&mut state, &database, &ctx, vec![], 0.1);
    let connect = control(&ctx, "canvas.connect")?.center();
    frame(&mut state, &database, &ctx, click(connect), 1.0);

    // Exercise the real worker/error widget using an absent synthetic resource;
    // no original media, decoder fixture or fabricated hit rectangle is needed.
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut time = 1.1;
    let retry_key = "media.retry.synthetic-missing-media";
    while control(&ctx, retry_key).is_err() && Instant::now() < deadline {
        frame(&mut state, &database, &ctx, vec![], time);
        time += 0.1;
        std::thread::sleep(Duration::from_millis(5));
    }
    frame(&mut state, &database, &ctx, vec![], time);
    let retry = control(&ctx, retry_key)?.center();
    let observed = canvas_observation(&ctx)?;
    let source_card = observed["visible_nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|node| node["node_id"] == "source"))
        .ok_or("visible source card")?;
    frame(
        &mut state,
        &database,
        &ctx,
        click(rect(&source_card["reserved"])?.center()),
        time + 1.0,
    );
    assert_eq!(state.connect_source.as_deref(), Some("source"));
    frame(
        &mut state,
        &database,
        &ctx,
        vec![Event::PointerMoved(retry), button(retry, true)],
        time + 2.0,
    );
    assert!(
        state.press_origin.is_none(),
        "the embedded button owns its press"
    );
    frame(
        &mut state,
        &database,
        &ctx,
        vec![button(retry, false)],
        time + 2.1,
    );
    assert_eq!(state.connect_source.as_deref(), Some("source"));
    assert!(state.press_origin.is_none());
    assert!(
        state.pending.is_none(),
        "Retry must not also insert an edge"
    );
    assert!(state.connection_editor.is_none());
    let saved = workspace.project_canvas(&state.project_id, &state.snapshot.canvas.id)?;
    assert_eq!(saved.revision.info.revision, 1);
    assert_eq!(saved.revision.content, source);
    Ok(())
}
