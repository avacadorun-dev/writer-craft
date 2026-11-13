//! Full-app widget event regressions. These hosted probes do not replace native OS acceptance.
use super::*;
use craft_editor_egui::{DocPoint, Selection};
use craft_editor_model::{Document as RichDocument, ScalarOffset};
use egui::{Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect};

struct Fixture {
    _directory: tempfile::TempDir,
    app: StudioApp,
    ctx: egui::Context,
    frame: eframe::Frame,
    time: f64,
    first: Document,
    second: Document,
    parent: String,
}

impl Fixture {
    fn new() -> AppResult<Self> {
        let directory = tempfile::tempdir()?;
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = StudioApp::new(
            &creation,
            directory.path().join("tree-input.sqlite"),
            false,
            None,
        )?;
        app.language = Language::English;
        app.navigator = true;
        let project = app.workspace.create_project("The Last Light")?;
        let first = app.workspace.create_rich_document(
            &project.id,
            "Arrival scene",
            RichDocument::plain("p", "The light remains.")?.to_json()?,
        )?;
        let second = app.workspace.create_rich_document(
            &project.id,
            "Archive scene",
            RichDocument::plain("p", "Nina opens the signal log.")?.to_json()?,
        )?;
        let nodes = app.workspace.initialize_project_tree(&project.id)?;
        let parent = app.workspace.create_project_tree_node(
            &project.id,
            app.workspace.project_structure(&project.id)?.revision,
            None,
            "folder",
            "Chapter one",
        )?;
        let first_node = nodes
            .iter()
            .find(|node| node.document_id.as_deref() == Some(first.id.as_str()))
            .ok_or("first tree node")?;
        app.workspace.move_project_tree_node(
            &project.id,
            first_node,
            Some(&parent.logical_id),
            None,
        )?;
        app.open_project(project)?;
        app.open_document(first.clone())?;
        ctx.data_mut(|data| {
            data.insert_temp(egui::Id::new("connected-capture-active"), true);
        });
        let mut fixture = Self {
            _directory: directory,
            app,
            ctx,
            frame: eframe::Frame::_new_kittest(),
            time: 0.0,
            first,
            second,
            parent: parent.logical_id,
        };
        fixture.render(vec![], Modifiers::NONE);
        fixture.render(vec![], Modifiers::NONE);
        Ok(fixture)
    }

    fn render(&mut self, events: Vec<Event>, _modifiers: Modifiers) -> egui::FullOutput {
        self.time += 0.1;
        let mut input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1440.0, 940.0))),
            focused: true,
            time: Some(self.time),
            events,
            ..Default::default()
        };
        eframe::App::raw_input_hook(&mut self.app, &self.ctx, &mut input);
        let mut output = self.ctx.run_ui(input, |ui| {
            eframe::App::ui(&mut self.app, ui, &mut self.frame);
        });
        output.textures_delta.clear();
        output.platform_output.commands.clear();
        assert!(
            self.app.error.is_none(),
            "full application event failed: {:?}",
            self.app.error.as_ref().map(|error| &error.details)
        );
        output
    }

    fn click(&mut self, point: Pos2, button: PointerButton) {
        self.render(
            vec![Event::PointerMoved(point), pointer(point, button, true)],
            Modifiers::NONE,
        );
        self.render(vec![pointer(point, button, false)], Modifiers::NONE);
    }

    fn title(&mut self, document: &Document) -> AppResult<(Rect, Rect)> {
        let painted = self.render(vec![], Modifiers::NONE);
        let row = super::super::capture_connected_writing::test_observation(&self.ctx)
            .0
            .get(&format!("hierarchy.document.{}", document.id))
            .copied()
            .ok_or("actual rendered document row")?;
        let title = painted_text(&painted, &document.title, Some(row))
            .ok_or("actual title galley inside its tree row")?;
        Ok((row, title))
    }

    fn node_id(&self, document: &str) -> AppResult<String> {
        self.app
            .tree_ui
            .nodes
            .iter()
            .find(|node| node.document_id.as_deref() == Some(document))
            .map(|node| node.logical_id.clone())
            .ok_or_else(|| "mapped document node".into())
    }
}

fn pointer(pos: Pos2, button: PointerButton, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn key(key: Key, pressed: bool, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: Some(key),
        pressed,
        repeat: false,
        modifiers,
    }
}

fn painted_text(output: &egui::FullOutput, text: &str, within: Option<Rect>) -> Option<Rect> {
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
    output.shapes.iter().find_map(|shape| {
        visit(
            &shape.shape,
            text,
            within.map_or(shape.clip_rect, |row| row.intersect(shape.clip_rect)),
        )
    })
}

fn selection(start: usize, end: usize) -> Selection {
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

#[test]
fn clicking_title_opens_its_document_and_keeps_shift_f10_in_tree_after_editor_frames()
-> AppResult<()> {
    let mut fixture = Fixture::new()?;
    let second = fixture.second.clone();
    let (_, title) = fixture.title(&second)?;
    fixture.click(title.center(), PointerButton::Primary);
    // Navigation emits the new manuscript in the click frame and again afterward.
    fixture.render(vec![], Modifiers::NONE);
    assert_eq!(
        fixture.app.document.as_ref().map(|document| &document.id),
        Some(&second.id)
    );
    assert_eq!(
        fixture.app.tree_ui.selected.as_deref(),
        Some(fixture.node_id(&second.id)?.as_str())
    );
    let manuscript = fixture
        .app
        .rich
        .as_ref()
        .ok_or("rich editor")?
        .session
        .options
        .id;
    assert_ne!(
        fixture.ctx.memory(|memory| memory.focused()),
        Some(manuscript)
    );
    fixture.render(
        vec![key(Key::F10, true, Modifiers::SHIFT)],
        Modifiers::SHIFT,
    );
    fixture.render(
        vec![key(Key::F10, false, Modifiers::SHIFT)],
        Modifiers::NONE,
    );
    let painted = fixture.render(vec![], Modifiers::NONE);
    let rename = painted_text(&painted, "Rename…", None).ok_or("persistent tree Rename menu")?;
    let later = fixture.render(vec![], Modifiers::NONE);
    assert!(
        painted_text(&later, "Rename…", None).is_some(),
        "tree menu vanished after editor emission"
    );
    fixture.click(rename.center(), PointerButton::Primary);
    let dialog = fixture
        .app
        .tree_ui
        .dialog
        .as_ref()
        .ok_or("tree rename dialog")?;
    assert!(matches!(dialog.operation, Operation::Rename));
    assert_eq!(
        dialog
            .original
            .as_ref()
            .and_then(|node| node.document_id.as_deref()),
        Some(second.id.as_str())
    );
    assert_eq!(fixture.app.workspace.document(&second.id)?, second);
    Ok(())
}

#[test]
fn clicking_current_title_repeatedly_keeps_live_rich_selection_and_actual_undo() -> AppResult<()> {
    let mut fixture = Fixture::new()?;
    let first = fixture.first.clone();
    let rich = fixture.app.rich.as_mut().ok_or("rich editor")?;
    rich.session.set_selection(selection(
        first.body.chars().count(),
        first.body.chars().count(),
    ))?;
    rich.session.request_canvas_focus(&fixture.ctx);
    fixture.render(vec![], Modifiers::NONE);
    fixture.render(vec![Event::Text(" Tonight.".into())], Modifiers::NONE);
    let rich = fixture.app.rich.as_mut().ok_or("rich editor")?;
    assert_eq!(
        rich.session.document().plain_text(),
        format!("{} Tonight.", first.body)
    );
    assert!(
        rich.session.can_undo(),
        "native typing must establish real history"
    );
    let selected = selection(4, 9);
    rich.session.set_selection(selected.clone())?;
    let revision = rich.session.revision();
    let content = rich.session.document().to_json()?;
    for _ in 0..2 {
        let (_, title) = fixture.title(&first)?;
        fixture.click(title.center(), PointerButton::Primary);
        fixture.render(vec![], Modifiers::NONE);
        let rich = fixture.app.rich.as_ref().ok_or("rich editor")?;
        assert_eq!(rich.session.selection(), Some(&selected));
        assert_eq!(rich.session.revision(), revision);
        assert_eq!(rich.session.document().to_json()?, content);
        assert!(
            rich.session.can_undo(),
            "same-row browsing reconstructed the rich session"
        );
    }
    assert_eq!(
        fixture.app.workspace.document(&first.id)?.body,
        format!("{} Tonight.", first.body)
    );
    let rich = fixture.app.rich.as_mut().ok_or("rich editor")?;
    rich.session.undo()?;
    assert_eq!(rich.session.document().plain_text(), first.body);
    assert!(rich.session.can_redo());
    Ok(())
}

#[test]
fn right_click_on_unselected_title_renames_that_row_without_opening_its_document() -> AppResult<()>
{
    let mut fixture = Fixture::new()?;
    let second = fixture.second.clone();
    let (_, title) = fixture.title(&second)?;
    fixture.click(title.center(), PointerButton::Secondary);
    let painted = fixture.render(vec![], Modifiers::NONE);
    let rename = painted_text(&painted, "Rename…", None).ok_or("context Rename action")?;
    assert_eq!(
        fixture.app.document.as_ref().map(|document| &document.id),
        Some(&fixture.first.id)
    );
    fixture.click(rename.center(), PointerButton::Primary);
    let dialog = fixture
        .app
        .tree_ui
        .dialog
        .as_ref()
        .ok_or("clicked row rename dialog")?;
    assert!(matches!(dialog.operation, Operation::Rename));
    assert_eq!(
        dialog
            .original
            .as_ref()
            .and_then(|node| node.document_id.as_deref()),
        Some(second.id.as_str())
    );
    assert_eq!(
        fixture.app.document.as_ref().map(|document| &document.id),
        Some(&fixture.first.id)
    );
    assert_eq!(fixture.app.workspace.document(&second.id)?, second);
    Ok(())
}

#[test]
fn dragging_title_or_row_padding_owns_outline_payload_and_escape_release_never_moves_content()
-> AppResult<()> {
    use super::super::project_navigation::OutlineDrag;
    for over_text in [true, false] {
        let mut fixture = Fixture::new()?;
        let first = fixture.first.clone();
        let before = fixture.app.workspace.project_tree(&first.project_id)?;
        let history = fixture.app.workspace.rich_revisions(&first.id)?;
        let (row, title) = fixture.title(&first)?;
        let start = if over_text {
            title.center()
        } else {
            row.right_center() - egui::vec2(7.0, 0.0)
        };
        assert!(row.contains(start));
        assert_eq!(title.contains(start), over_text);
        let end = start + egui::vec2(-24.0, 8.0);
        fixture.render(
            vec![
                Event::PointerMoved(start),
                pointer(start, PointerButton::Primary, true),
            ],
            Modifiers::NONE,
        );
        fixture.render(vec![Event::PointerMoved(end)], Modifiers::NONE);
        let payload = egui::DragAndDrop::payload::<OutlineDrag>(&fixture.ctx)
            .ok_or("row must start a real outline drag")?;
        assert_eq!(payload.project, first.project_id);
        assert_eq!(
            payload.original.document_id.as_deref(),
            Some(first.id.as_str())
        );
        fixture.render(
            vec![
                key(Key::Escape, true, Modifiers::NONE),
                pointer(end, PointerButton::Primary, false),
            ],
            Modifiers::NONE,
        );
        fixture.render(
            vec![key(Key::Escape, false, Modifiers::NONE)],
            Modifiers::NONE,
        );
        assert!(!egui::DragAndDrop::has_payload_of_type::<OutlineDrag>(
            &fixture.ctx
        ));
        assert_eq!(
            fixture.app.workspace.project_tree(&first.project_id)?,
            before
        );
        assert_eq!(fixture.app.workspace.document(&first.id)?, first);
        assert_eq!(fixture.app.workspace.rich_revisions(&first.id)?, history);
    }
    Ok(())
}

#[test]
fn releasing_a_full_app_drag_into_a_populated_folder_commits_once_without_changing_manuscripts()
-> AppResult<()> {
    use super::super::project_navigation::OutlineDrag;
    let mut fixture = Fixture::new()?;
    let source = fixture.second.clone();
    let project = source.project_id.clone();
    let before = fixture.app.workspace.project_tree(&project)?;
    let first_history = fixture.app.workspace.rich_revisions(&fixture.first.id)?;
    let source_history = fixture.app.workspace.rich_revisions(&source.id)?;
    let source_id = fixture.node_id(&source.id)?;
    let (_, title) = fixture.title(&source)?;
    let target = super::super::capture_connected_writing::test_observation(&fixture.ctx)
        .0
        .get(&format!("hierarchy.node.{}", fixture.parent))
        .copied()
        .ok_or("painted populated folder row")?
        .center();
    fixture.render(
        vec![
            Event::PointerMoved(title.center()),
            pointer(title.center(), PointerButton::Primary, true),
        ],
        Modifiers::NONE,
    );
    fixture.render(vec![Event::PointerMoved(target)], Modifiers::NONE);
    let payload = egui::DragAndDrop::payload::<OutlineDrag>(&fixture.ctx)
        .ok_or("held movement must start the outline drag")?;
    assert_eq!(payload.original.logical_id, source_id);
    let hover = fixture.render(vec![], Modifiers::NONE);
    assert!(painted_text(&hover, "Move inside Chapter one", None).is_some());
    assert_eq!(fixture.app.workspace.project_tree(&project)?, before);

    fixture.render(
        vec![pointer(target, PointerButton::Primary, false)],
        Modifiers::NONE,
    );
    assert!(!egui::DragAndDrop::has_payload_of_type::<OutlineDrag>(
        &fixture.ctx
    ));
    let after = fixture.app.workspace.project_tree(&project)?;
    assert_eq!(after.len(), before.len());
    let moved = after
        .iter()
        .find(|node| node.logical_id == source_id)
        .ok_or("same moved scene identity")?;
    assert_eq!(moved.document_id.as_deref(), Some(source.id.as_str()));
    assert_eq!(moved.metadata["parentId"], fixture.parent);
    let first = after
        .iter()
        .find(|node| node.document_id.as_deref() == Some(fixture.first.id.as_str()))
        .ok_or("existing folder scene")?;
    assert!(
        moved.metadata["position"]
            .as_f64()
            .ok_or("moved position")?
            > first.metadata["position"]
                .as_f64()
                .ok_or("existing position")?
    );
    for node in before.iter().filter(|node| node.logical_id != source_id) {
        assert_eq!(
            after
                .iter()
                .find(|current| current.logical_id == node.logical_id),
            Some(node)
        );
    }
    assert_eq!(fixture.app.workspace.document(&source.id)?, source);
    assert_eq!(
        fixture.app.workspace.document(&fixture.first.id)?,
        fixture.first
    );
    assert_eq!(
        fixture.app.workspace.rich_revisions(&source.id)?,
        source_history
    );
    assert_eq!(
        fixture.app.workspace.rich_revisions(&fixture.first.id)?,
        first_history
    );
    assert_eq!(fixture.app.document.as_ref(), Some(&fixture.first));
    // The release receipt is consumed; a later frame cannot repeat the tree mutation.
    fixture.render(vec![], Modifiers::NONE);
    assert_eq!(fixture.app.workspace.project_tree(&project)?, after);
    Ok(())
}

#[test]
fn browsing_current_document_refuses_an_externally_trashed_ancestor_without_replacing_editor()
-> AppResult<()> {
    let mut fixture = Fixture::new()?;
    let selected = selection(4, 9);
    fixture
        .app
        .rich
        .as_mut()
        .ok_or("rich editor")?
        .session
        .set_selection(selected.clone())?;
    let id = fixture.node_id(&fixture.first.id)?;
    let parent = fixture
        .app
        .workspace
        .project_tree(&fixture.first.project_id)?
        .into_iter()
        .find(|node| node.logical_id == fixture.parent)
        .ok_or("parent folder")?;
    fixture
        .app
        .workspace
        .trash_project_tree_node(&fixture.first.project_id, &parent)?;
    // The reused session has the same document/revision. Exercise its fresh-tree guard
    // directly as well as the public Browse route; a same-ID shortcut must not bypass it.
    let error = fixture
        .app
        .open_tree_document(fixture.first.clone(), true)
        .err()
        .ok_or("hidden current document must refuse editor reuse")?;
    assert!(error.downcast_ref::<TrashedDocument>().is_some());
    let error = fixture
        .app
        .tree_action(TreeAction::Browse { id, mapped: true })
        .err()
        .ok_or("hidden current document must refuse browsing")?;
    assert!(error.downcast_ref::<TrashedDocument>().is_some());
    assert_eq!(fixture.app.document.as_ref(), Some(&fixture.first));
    let rich = fixture.app.rich.as_ref().ok_or("preserved rich editor")?;
    assert_eq!(rich.session.selection(), Some(&selected));
    assert_eq!(
        rich.session.document().to_json()?,
        fixture.first.rich_json.clone().ok_or("rich source")?
    );
    Ok(())
}
