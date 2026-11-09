//! Hosted synthetic native-widget probes. These do not certify OS input or product parity.
use super::*;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use studio_core::ProjectTreeNode;

const ACTIVE: &str = "connected-capture-active";
const FRAME: &str = "connected-capture-frame";
const INITIAL: &str = "Synthetic card: Привет 😀 e\u{301}";
const LATER: &str = " — later draft 日本語";
const CLIPPED_TEXT: &str = "MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM";

#[derive(Clone, Default)]
struct Observation {
    controls: BTreeMap<String, Rect>,
    surfaces: BTreeMap<String, Value>,
}

/// The observer is inert in every ordinary application session.
pub(super) fn enabled(ctx: &egui::Context) -> bool {
    ctx.data(|data| {
        data.get_temp::<bool>(egui::Id::new(ACTIVE))
            .unwrap_or(false)
    })
}
/// Record the actual emitted control rectangle; never invent a click coordinate.
pub(super) fn control(ctx: &egui::Context, key: &str, rect: Rect) {
    if enabled(ctx) {
        ctx.data_mut(|data| {
            let frame = data.get_temp_mut_or_default::<Observation>(egui::Id::new(FRAME));
            frame.controls.insert(key.into(), rect);
        });
    }
}
/// Record bounded semantic state from the surface that was painted in this frame.
pub(super) fn observe(ctx: &egui::Context, surface: &str, state: Value) {
    if enabled(ctx) {
        ctx.data_mut(|data| {
            let frame = data.get_temp_mut_or_default::<Observation>(egui::Id::new(FRAME));
            frame.surfaces.insert(surface.into(), state);
        });
    }
}
fn frame(ctx: &egui::Context) -> Observation {
    ctx.data(|data| {
        data.get_temp::<Observation>(egui::Id::new(FRAME))
            .unwrap_or_default()
    })
}
fn digest(value: &Value) -> String {
    format!("{:x}", Sha256::digest(value.to_string().as_bytes()))
}
fn key(key: Key, modifiers: Modifiers) -> Vec<Event> {
    [true, false]
        .into_iter()
        .map(|pressed| Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        })
        .collect()
}
fn click(rect: Rect) -> Vec<Event> {
    click_at(rect.center())
}
fn click_at(pos: Pos2) -> Vec<Event> {
    vec![
        Event::PointerMoved(pos),
        Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
        Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    ]
}
/// Locate a board point outside every visible card so the armed text tool can place a card there.
fn empty_canvas_point(state: &Value) -> Option<Pos2> {
    let viewport = value_rect(&state["viewport"])?;
    let candidates = [
        viewport.center(),
        viewport.left_center() + vec2(viewport.width() * 0.25, 0.0),
        viewport.right_center() - vec2(viewport.width() * 0.25, 0.0),
        viewport.center_top() + vec2(0.0, viewport.height() * 0.25),
        viewport.center_bottom() - vec2(0.0, viewport.height() * 0.25),
        viewport.left_top() + vec2(viewport.width() * 0.2, viewport.height() * 0.2),
        viewport.right_bottom() - vec2(viewport.width() * 0.2, viewport.height() * 0.2),
    ];
    let visible = state["visible_nodes"].as_array();
    candidates.into_iter().find(|point| {
        !visible.is_some_and(|nodes| {
            nodes
                .iter()
                .any(|node| value_rect(&node["reserved"]).is_some_and(|rect| rect.contains(*point)))
        })
    })
}
fn rect_value(rect: Rect) -> Value {
    json!([rect.min.x, rect.min.y, rect.max.x, rect.max.y])
}
fn value_rect(value: &Value) -> Option<Rect> {
    let rect = value.as_array()?;
    if rect.len() != 4 {
        return None;
    }
    Some(Rect::from_min_max(
        egui::pos2(rect[0].as_f64()? as f32, rect[1].as_f64()? as f32),
        egui::pos2(rect[2].as_f64()? as f32, rect[3].as_f64()? as f32),
    ))
}
fn has_row(state: &Value, document: &str) -> bool {
    state["visible"]
        .as_array()
        .is_some_and(|rows| rows.iter().any(|row| row["document_id"] == document))
}
fn has_visible_node(state: &Value, id: &str) -> bool {
    state["visible_nodes"].as_array().is_some_and(|nodes| {
        nodes.iter().any(|node| {
            node["node_id"] == id
                && node["body_clip"].as_array().is_some_and(|rect| {
                    rect.len() == 4
                        && rect[0]
                            .as_f64()
                            .zip(rect[2].as_f64())
                            .is_some_and(|(a, b)| b > a)
                        && rect[1]
                            .as_f64()
                            .zip(rect[3].as_f64())
                            .is_some_and(|(a, b)| b > a)
                })
        })
    })
}

pub(super) struct ConnectedCapture {
    scenario: String,
    project: String,
    board: String,
    documents: Vec<Document>,
    card: Option<String>,
    seed: Value,
    phase: usize,
    clipped_camera_positioned: bool,
    events: Vec<Event>,
    steps: Vec<Value>,
    final_state: Value,
    terminal: bool,
    preservation_verified: bool,
    durability_evidence: Value,
    diagnostic: Option<Value>,
    failure: Option<String>,
}
impl ConnectedCapture {
    pub(super) fn initialize(
        app: &mut StudioApp,
        demo: bool,
        ctx: &egui::Context,
    ) -> AppResult<Option<Self>> {
        let Ok(scenario) = std::env::var("WRITERCRAFT_CAPTURE_SCENARIO") else {
            return Ok(None);
        };
        if scenario.is_empty() {
            return Ok(None);
        }
        if !matches!(
            scenario.as_str(),
            "canvas-inline" | "canvas-cancel" | "canvas-clipped-2x" | "hierarchy-search"
        ) {
            return Err("unknown_connected_capture_scenario".into());
        }
        let runner = PathBuf::from(
            std::env::var_os("RUNNER_TEMP").ok_or("connected_capture_requires_hosted_runner")?,
        )
        .canonicalize()?;
        let parent = app
            .database
            .parent()
            .ok_or("connected_capture_database_parent_missing")?
            .canonicalize()?;
        if !demo
            || app.capture.is_none()
            || std::env::var("GITHUB_ACTIONS").as_deref() != Ok("true")
            || !parent.starts_with(&runner)
            || !parent.file_name().is_some_and(|name| {
                name.to_string_lossy()
                    .starts_with("writercraft-startup-workspace-")
            })
        {
            return Err("connected_capture_requires_fresh_hosted_synthetic_workspace".into());
        }
        let project = app
            .workspace
            .create_project("Synthetic connected writing")?;
        let mut documents = Vec::new();
        for (index, title) in ["Opening", "Echo", "Echo"].into_iter().enumerate() {
            documents.push(app.workspace.create_rich_document(&project.id, title, json!({"type":"doc","attrs":{"schemaVersion":4},"content":[{"type":"paragraph","attrs":{"blockId":format!("capture-p-{index}")},"content":[{"type":"text","text":format!("Synthetic document identity {index}. Привет 😀 日本語") }]}]}))?);
        }
        let seed = json!({"id":"capture-seed","type":"text","x":-350,"y":0,"width":300,"height":180,"text":"Saved independent note","future":{"retain":true}});
        let board = app.workspace.create_project_canvas(
            &project.id,
            "Synthetic writing board",
            json!({"nodes":[seed.clone()],"edges":[],"future":{"retain":"exact"}}),
        )?;
        app.open_project(project.clone())?;
        let source: Vec<ProjectTreeNode> = [
            ("capture-root", None, None),
            ("capture-opening", Some("capture-root"), Some(0)),
            ("capture-chapter", Some("capture-root"), None),
            ("capture-echo-a", Some("capture-chapter"), Some(1)),
            ("capture-echo-b", Some("capture-chapter"), Some(2)),
        ].into_iter().enumerate().map(|(index,(id,parent,document))| ProjectTreeNode {
            logical_id:id.into(), document_id:document.map(|i| documents[i].id.clone()), ordinal:index as u64,
            metadata:json!({"title":if id=="capture-root" {"Synthetic manuscript"} else if id=="capture-chapter" {"Nested chapter"} else {id},"kind":"custom","parentId":parent,"position":index,"future":{"retain":true}}),
        }).collect();
        app.project_outline = project_navigation::ProjectOutline::prepare(&project.id, &source)?;
        app.navigator = true;
        app.focus_mode = false;
        app.inspector_visible = false;
        if scenario.starts_with("canvas-") {
            app.open_canvas(&board.canvas.id)?;
        }
        ctx.data_mut(|data| data.insert_temp(egui::Id::new(ACTIVE), true));
        if scenario == "canvas-clipped-2x" {
            app.canvas
                .as_mut()
                .ok_or("capture_canvas_missing")?
                .set_capture_camera(
                    ctx,
                    craft_canvas_model::WorldPoint {
                        x: 4096.0,
                        y: -2048.0,
                    },
                    2.0,
                )?;
        }
        Ok(Some(Self {
            scenario,
            project: project.id,
            board: board.canvas.id,
            documents,
            card: None,
            seed,
            phase: 0,
            clipped_camera_positioned: false,
            events: Vec::new(),
            steps: Vec::new(),
            final_state: Value::Null,
            terminal: false,
            preservation_verified: false,
            durability_evidence: Value::Null,
            diagnostic: None,
            failure: None,
        }))
    }
    pub(super) fn raw_input(&mut self, ctx: &egui::Context, input: &mut RawInput) {
        if !self.events.is_empty()
            && let Some(step) = self.steps.last_mut()
        {
            step["input_hook_frame"] = json!(ctx.cumulative_frame_nr());
        }
        input.events.append(&mut self.events);
        input.focused = true;
        ctx.data_mut(|data| data.insert_temp(egui::Id::new(FRAME), Observation::default()));
    }
    fn advance(&mut self, name: &str, state: &Value, events: Vec<Event>) {
        self.steps.push(json!({"step":name,"observed_state_sha256":digest(state),"phase":self.phase,"observed_native_state":state,"input_provenance":"scripted egui RawInput, not OS events"}));
        self.events = events;
        self.phase += 1;
    }
    fn click_control(&mut self, observation: &Observation, name: &str, state: &Value, key: &str) {
        if let Some(rect) = observation
            .controls
            .get(key)
            .filter(|rect| rect.is_positive())
        {
            self.advance(name, state, click(*rect));
        }
    }
    fn expected_text(&self) -> String {
        match self.scenario.as_str() {
            "canvas-cancel" => String::new(),
            "canvas-clipped-2x" => CLIPPED_TEXT.into(),
            _ => format!("{INITIAL}{LATER}"),
        }
    }
    fn verify_preservation(&mut self, app: &StudioApp, state: &Value) -> AppResult<()> {
        let workspace = Workspace::open(&app.database)?;
        for original in &self.documents {
            let reopened = workspace.document(&original.id)?;
            if reopened.rich_json != original.rich_json
                || reopened.revision != original.revision
                || reopened.project_id != self.project
            {
                return Err("connected_capture_changed_unrelated_manuscript".into());
            }
        }
        let saved = workspace.project_canvas(&self.project, &self.board)?;
        let nodes = saved.revision.content["nodes"]
            .as_array()
            .ok_or("connected_capture_missing_nodes")?;
        if nodes.first() != Some(&self.seed)
            || saved.revision.content["future"]["retain"] != "exact"
            || saved.revision.content["edges"] != json!([])
        {
            return Err("connected_capture_changed_unrelated_board_content".into());
        }
        if self.scenario.starts_with("canvas-") {
            let id = self
                .card
                .as_deref()
                .ok_or("connected_capture_missing_new_card")?;
            let node = nodes
                .iter()
                .find(|node| node["id"] == id)
                .ok_or("connected_capture_new_card_not_durable")?;
            let expected = self.expected_text();
            if nodes.len() != 2
                || node["text"] != expected
                || saved.revision.content != state["canonical"]
                || saved.revision.info.revision != state["revision"]
            {
                return Err("connected_capture_durable_content_does_not_match_native_state".into());
            }
        } else if nodes.len() != 1 {
            return Err("hierarchy_capture_changed_board_content".into());
        }
        let history = workspace.project_canvas_history(&self.project, &self.board, None, 16)?;
        let mut history_evidence = Vec::new();
        let mut saved_initial_text = false;
        for revision in &history {
            let historical =
                workspace.project_canvas_revision(&self.project, &self.board, revision.revision)?;
            let text = self
                .card
                .as_deref()
                .and_then(|id| {
                    historical.content["nodes"]
                        .as_array()?
                        .iter()
                        .find(|node| node["id"] == id)
                })
                .and_then(|node| node["text"].as_str());
            saved_initial_text |= text == Some(INITIAL);
            history_evidence.push(json!({"revision":revision.revision,"content_sha256":digest(&historical.content),"card_text":text}));
        }
        let expected_count = match self.scenario.as_str() {
            "canvas-inline" => 4,
            "canvas-cancel" => 2,
            "canvas-clipped-2x" => 3,
            _ => 1,
        };
        if history.len() != expected_count
            || (self.scenario == "canvas-inline" && !saved_initial_text)
            || (self.scenario == "canvas-cancel"
                && history_evidence.iter().any(|revision| {
                    revision["card_text"]
                        .as_str()
                        .is_some_and(|text| !text.is_empty())
                }))
        {
            return Err("connected_capture_unexpected_durable_history".into());
        }
        self.durability_evidence = json!({"independent_workspace_reread":true,"process_restart_verified":false,"history":history_evidence});
        self.preservation_verified = true;
        Ok(())
    }
    /// Move the already mounted card across the viewport before testing its input.
    /// Moving before creation is ineffective because Add centers a new card there.
    fn position_clipped_camera(
        &mut self,
        app: &mut StudioApp,
        ctx: &egui::Context,
        state: &Value,
    ) -> AppResult<()> {
        let Some(id) = state["editor"]["node_id"].as_str() else {
            return Ok(());
        };
        let Some(reserved) = state["visible_nodes"]
            .as_array()
            .and_then(|nodes| nodes.iter().find(|node| node["node_id"] == id))
            .and_then(|node| value_rect(&node["reserved"]))
        else {
            return Ok(());
        };
        let viewport = value_rect(&state["viewport"]).ok_or("capture_viewport_missing")?;
        let center_x = state["camera"]["center"][0]
            .as_f64()
            .ok_or("capture_camera_center_missing")?;
        let center_y = state["camera"]["center"][1]
            .as_f64()
            .ok_or("capture_camera_center_missing")?;
        let zoom = state["camera"]["zoom"]
            .as_f64()
            .filter(|zoom| zoom.is_finite() && *zoom > 0.0)
            .ok_or("capture_camera_zoom_invalid")?;
        let screen_shift = viewport.right() + reserved.width() * 0.25 - reserved.right();
        app.canvas
            .as_mut()
            .ok_or("capture_canvas_missing")?
            .set_capture_camera(
                ctx,
                craft_canvas_model::WorldPoint {
                    x: center_x - f64::from(screen_shift) / zoom,
                    y: center_y,
                },
                zoom,
            )?;
        self.steps.push(json!({
            "step":"position-existing-card-across-viewport",
            "observed_state_sha256":digest(state),
            "phase":self.phase,
            "observed_native_state":state,
            "input_provenance":"capture-only camera placement after actual card creation; not OS input"
        }));
        self.clipped_camera_positioned = true;
        ctx.request_repaint();
        Ok(())
    }
    fn drive_canvas(&mut self, observation: &Observation, state: &Value) {
        match self.phase {
            0 => self.click_control(observation, "arm-text-card-tool", state, "canvas.add_text"),
            1 if state["editor"].is_null() && state["pending"] == false => {
                if let Some(point) = empty_canvas_point(state) {
                    self.advance("place-text-card-on-empty-board", state, click_at(point));
                }
            }
            2 if state["editor"]["node_id"].as_str().is_some() => {
                self.card = state["editor"]["node_id"].as_str().map(str::to_owned);
                self.advance(
                    "new-card-input-mounted",
                    state,
                    vec![Event::Text(if self.scenario == "canvas-clipped-2x" {
                        CLIPPED_TEXT.into()
                    } else {
                        INITIAL.into()
                    })],
                );
            }
            3 if state["editor"]["text"]
                == if self.scenario == "canvas-clipped-2x" {
                    CLIPPED_TEXT
                } else {
                    INITIAL
                }
                && (self.scenario == "canvas-cancel"
                    || (state["editor"]["awaiting_insertion"] == false
                        && state["pending"] == false)) =>
            {
                if self.scenario == "canvas-cancel" {
                    self.advance(
                        "cancel-native-draft",
                        state,
                        key(Key::Escape, Modifiers::NONE),
                    );
                } else {
                    let mut events = key(Key::S, Modifiers::COMMAND);
                    if self.scenario != "canvas-clipped-2x" {
                        events.push(Event::Paste(LATER.into()));
                    }
                    self.advance(
                        if self.scenario == "canvas-clipped-2x" {
                            "save-clipped-native-input"
                        } else {
                            "save-before-later-native-paste"
                        },
                        state,
                        events,
                    );
                }
            }
            4 => {
                let expected = self.expected_text();
                let ready = if self.scenario == "canvas-cancel" {
                    state["editor"].is_null()
                } else {
                    state["editor"]["text"] == expected
                        && state["editor"]["dirty"] == false
                        && state["editor"]["submitted"] == false
                };
                if ready
                    && state["pending"] == false
                    && state["failed"] == false
                    && state["navigation_blocked"] == false
                {
                    self.advance("expected-native-and-acknowledged-state", state, vec![]);
                    self.terminal = true;
                }
            }
            _ => {}
        }
    }
    fn drive_hierarchy(&mut self, observation: &Observation, state: &Value) {
        let second = &self.documents[2].id;
        match self.phase {
            0 if has_row(state, &self.documents[0].id) => self.click_control(
                observation,
                "collapse-root",
                state,
                "hierarchy.disclosure.capture-root",
            ),
            1 if !has_row(state, &self.documents[0].id) => {
                self.click_control(observation, "focus-search", state, "navigator.search")
            }
            2 => self.advance(
                "type-duplicate-name-query",
                state,
                vec![Event::Text("Echo".into())],
            ),
            3 if state["query"] == "echo"
                && has_row(state, second)
                && has_row(state, &self.documents[1].id) =>
            {
                let mut events = key(Key::A, Modifiers::COMMAND);
                events.extend(key(Key::Backspace, Modifiers::NONE));
                self.advance("clear-search", state, events);
            }
            4 if state["query"] == "" && !has_row(state, second) => self.advance(
                "normal-collapse-retained",
                state,
                vec![Event::Text("Echo".into())],
            ),
            5 if has_row(state, second) && state["query"] == "echo" => self.click_control(
                observation,
                "choose-exact-second-duplicate",
                state,
                &format!("hierarchy.document.{second}"),
            ),
            6 if state["selected_document"] == *second => {
                self.advance("exact-document-opened", state, vec![]);
                self.terminal = true;
            }
            _ => {}
        }
    }
    pub(super) fn readiness(
        &mut self,
        app: &mut StudioApp,
        ctx: &egui::Context,
    ) -> super::native_capture::CaptureReadiness {
        let observation = frame(ctx);
        let surface = if self.scenario.starts_with("canvas-") {
            "canvas"
        } else {
            "hierarchy"
        };
        let mut state = observation
            .surfaces
            .get(surface)
            .cloned()
            .unwrap_or(Value::Null);
        if !state.is_null() {
            state["native_caret"] = ctx.output(|output| output.ime.map(|ime|json!({"input_rect":rect_value(ime.rect),"cursor_rect":rect_value(ime.cursor_rect)}))).unwrap_or(Value::Null);
            if !self.scenario.starts_with("canvas-") {
                state["rendered_target_rect"] = observation
                    .controls
                    .get(&format!("hierarchy.document.{}", self.documents[2].id))
                    .map(|rect| rect_value(*rect))
                    .unwrap_or(Value::Null);
            }
        }
        let previous_steps = self.steps.len();
        if state["project_id"] == self.project && self.failure.is_none() {
            if self.scenario == "canvas-clipped-2x"
                && self.phase == 2
                && !self.clipped_camera_positioned
            {
                if let Err(error) = self.position_clipped_camera(app, ctx, &state) {
                    self.failure = Some(error.to_string());
                }
            } else if self.scenario.starts_with("canvas-") {
                self.drive_canvas(&observation, &state);
            } else {
                self.drive_hierarchy(&observation, &state);
            }
            if self.terminal
                && !self.preservation_verified
                && let Err(error) = self.verify_preservation(app, &state)
            {
                self.failure = Some(error.to_string());
            }
        }
        if self.steps.len() > previous_steps
            && let Some(step) = self.steps.last_mut()
        {
            step["observed_frame"] = json!(ctx.cumulative_frame_nr());
        }
        self.final_state = state;
        let visible = if self.scenario.starts_with("canvas-") {
            self.card
                .as_deref()
                .is_some_and(|id| has_visible_node(&self.final_state, id))
        } else {
            has_row(&self.final_state, &self.documents[2].id)
                && value_rect(&self.final_state["rendered_target_rect"]).is_some_and(|rect| {
                    rect.is_positive() && ctx.content_rect().contains(rect.center())
                })
                && app
                    .document
                    .as_ref()
                    .is_some_and(|document| document.id == self.documents[2].id)
        };
        if self.scenario == "canvas-clipped-2x" && self.terminal {
            let clip = self.final_state["visible_nodes"]
                .as_array()
                .and_then(|nodes| {
                    nodes
                        .iter()
                        .find(|node| node["node_id"].as_str() == self.card.as_deref())
                })
                .and_then(|node| value_rect(&node["body_clip"]));
            let reserve = self.final_state["visible_nodes"]
                .as_array()
                .and_then(|nodes| {
                    nodes
                        .iter()
                        .find(|node| node["node_id"].as_str() == self.card.as_deref())
                })
                .and_then(|node| value_rect(&node["reserved"]));
            let cursor = value_rect(&self.final_state["native_caret"]["cursor_rect"]);
            let clipped = reserve
                .zip(clip)
                .is_some_and(|(reserve, clip)| reserve.width() > clip.width() + 20.0);
            let caret_visible = cursor
                .zip(clip)
                .is_some_and(|(cursor, clip)| clip.contains(cursor.center()));
            let zoom = self.final_state["camera"]["zoom"].as_f64();
            self.diagnostic = Some(
                json!({"name":"clipped-card-native-caret","status":if clipped && caret_visible && zoom==Some(2.0) {"passed"} else {"failed"},"actual_zoom":zoom,"card_is_clipped":clipped,"native_caret_visible":caret_visible,"clip":clip.map(rect_value),"cursor":cursor.map(rect_value),"os_ime_acceptance":false}),
            );
        }
        let bytes = Sha256::digest(self.final_state.to_string().as_bytes());
        let mut generation = [0_u8; 8];
        generation.copy_from_slice(&bytes[..8]);
        super::native_capture::CaptureReadiness {
            generation: u64::from_le_bytes(generation),
            scroll_offset_bits: 0,
            visible_atoms: usize::from(visible),
            target_is_text: false,
            visible_text_targets: 0,
            pending_atoms: usize::from(!self.events.is_empty()),
            failed_atoms: usize::from(self.failure.is_some()),
            target_visible: visible,
            settled: self.terminal
                && self.preservation_verified
                && self.failure.is_none()
                && self.events.is_empty(),
        }
    }
    pub(super) fn evidence(&self) -> Value {
        json!({"scenario":self.scenario,"expected_project_id":self.project,"expected_card_id":self.card,"expected_document_id":self.documents[2].id,"phase":self.phase,"steps":self.steps,"terminal":self.terminal,"content_contract_satisfied":self.preservation_verified && self.failure.is_none(),"failure":self.failure,"durability_evidence":self.durability_evidence,"diagnostic":self.diagnostic,"final_state":self.final_state,"fixture_source":"Persisted synthetic documents and board; imported-shape ProjectTreeNode projection. This is not importer or process-restart acceptance.","input_provenance":"scripted egui RawInput through compiled native widgets; no OS pasteboard/IME/accessibility claim","os_input_acceptance":false,"full_journey_acceptance":false})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn connected_observation_is_inert_until_explicitly_armed() {
        let ctx = egui::Context::default();
        control(
            &ctx,
            "private-control",
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(20.0, 20.0)),
        );
        observe(&ctx, "canvas", json!({"text":"must not be retained"}));
        assert!(frame(&ctx).controls.is_empty() && frame(&ctx).surfaces.is_empty());
    }
    #[test]
    fn target_identity_requires_nonempty_current_body_clip() {
        let state = json!({"visible_nodes":[{"node_id":"a","body_clip":[0,0,0,30]},{"node_id":"b","body_clip":[0,0,30,30]}]});
        assert!(
            !has_visible_node(&state, "a")
                && !has_visible_node(&state, "absent")
                && has_visible_node(&state, "b")
        );
    }
}
