//! Presentation-only return bookmarks shared by transfer and future connected writing journeys.
//! No media cache, manuscript clone, sealed session proof or canonical write authority is retained.
use super::*;

pub(super) struct NavigationReturn {
    project: Option<String>,
    content: ReturnContent,
    library: bool,
    focus_mode: bool,
    navigator: bool,
    inspector: Inspector,
    inspector_visible: bool,
    prefer_inspector: bool,
}
enum ReturnContent {
    None,
    Document {
        id: String,
        revision: i64,
        #[cfg(feature = "editor-preview")]
        rich: Option<RichReturn>,
    },
    #[cfg(feature = "editor-preview")]
    Canvas {
        id: String,
        revision: u64,
        identity: [u8; 32],
        camera: craft_canvas_model::Camera,
        selection: craft_canvas_model::Selection,
    },
}
#[cfg(feature = "editor-preview")]
struct RichReturn {
    identity: [u8; 32],
    selection: Option<craft_editor_egui::SelectionState>,
    scroll: Option<f32>,
}
impl NavigationReturn {
    /// Capture only acknowledged current view state. Caller must preserve work before navigation.
    pub(super) fn capture(app: &StudioApp) -> AppResult<Self> {
        if app.dirty.is_some() {
            return Err("save_before_navigation_return_capture".into());
        }
        #[cfg(feature = "editor-preview")]
        let content = if let Some(board) = &app.canvas {
            let (camera, selection) = board.navigation_view()?;
            ReturnContent::Canvas {
                id: board.snapshot.canvas.id.clone(),
                revision: board.snapshot.revision.info.revision,
                identity: content_identity(&board.snapshot.revision.content)?,
                camera,
                selection,
            }
        } else {
            document_return(app)?
        };
        #[cfg(not(feature = "editor-preview"))]
        let content = document_return(app)?;
        Ok(Self {
            project: app.project.as_ref().map(|project| project.id.clone()),
            content,
            library: app.library,
            focus_mode: app.focus_mode,
            navigator: app.navigator,
            inspector: app.inspector,
            inspector_visible: app.inspector_visible,
            prefer_inspector: app.prefer_inspector,
        })
    }
    /// Reopen actual current data. Restore selection/camera only for exact saved content; changed
    /// origins remain latest-data views and are reported to the caller for localized disclosure.
    pub(super) fn restore(&self, app: &mut StudioApp) -> AppResult<bool> {
        app.save_explicit()?;
        let Some(project_id) = self.project.as_ref() else {
            app.library = self.library;
            return Ok(true);
        };
        let project = app
            .workspace
            .list_projects()?
            .into_iter()
            .find(|project| &project.id == project_id)
            .ok_or("navigation_origin_project_missing")?;
        if app.project.as_ref().map(|current| current.id.as_str()) != Some(project_id.as_str()) {
            app.open_project(project)?;
        }
        #[cfg(feature = "editor-preview")]
        if app.recovery.is_some() {
            return Err("navigation_origin_requires_recovery_review".into());
        }
        let unchanged = match &self.content {
            ReturnContent::None => true,
            ReturnContent::Document {
                id,
                revision,
                #[cfg(feature = "editor-preview")]
                rich,
            } => {
                let document = app.workspace.document(id)?;
                if document.project_id.as_str() != project_id.as_str() {
                    return Err("navigation_origin_document_scope_changed".into());
                }
                let same_revision = document.revision == *revision;
                app.open_document(document)?;
                #[cfg(feature = "editor-preview")]
                {
                    if app.recovery.is_some() {
                        return Err("navigation_origin_requires_recovery_review".into());
                    }
                    match (rich.as_ref(), app.rich.as_mut()) {
                        (Some(saved), Some(current))
                            if same_revision
                                && content_identity(current.session.document().root())?
                                    == saved.identity =>
                        {
                            // This validates typed positions against the reopened exact tree. It
                            // deliberately does not restore a foreign sealed session bookmark.
                            current
                                .session
                                .restore_selection_state(saved.selection.clone())?;
                            if let Some(scroll) = saved.scroll {
                                current.restore_navigation_scroll(scroll)?;
                            }
                            true
                        }
                        (None, None) => same_revision,
                        _ => false,
                    }
                }
                #[cfg(not(feature = "editor-preview"))]
                {
                    same_revision
                }
            }
            #[cfg(feature = "editor-preview")]
            ReturnContent::Canvas {
                id,
                revision,
                identity,
                camera,
                selection,
            } => {
                app.open_canvas(id)?;
                let board = app
                    .canvas
                    .as_mut()
                    .ok_or("navigation_origin_board_missing")?;
                if board.project_id.as_str() != project_id.as_str() {
                    return Err("navigation_origin_board_scope_changed".into());
                }
                let same = board.snapshot.revision.info.revision == *revision
                    && content_identity(&board.snapshot.revision.content)? == *identity;
                if same {
                    board.restore_navigation_view(*camera, selection.clone())?;
                }
                same
            }
        };
        app.library = self.library;
        app.focus_mode = self.focus_mode;
        app.navigator = self.navigator;
        app.inspector = self.inspector;
        app.inspector_visible = self.inspector_visible;
        app.prefer_inspector = self.prefer_inspector;
        Ok(unchanged)
    }
}
fn document_return(app: &StudioApp) -> AppResult<ReturnContent> {
    let Some(document) = &app.document else {
        return Ok(ReturnContent::None);
    };
    #[cfg(feature = "editor-preview")]
    let rich = app
        .rich
        .as_ref()
        .map(|rich| -> AppResult<RichReturn> {
            if rich.pending_proof_count() != 0 || rich.session.is_composing() {
                return Err("acknowledge_input_before_navigation_return_capture".into());
            }
            Ok(RichReturn {
                identity: content_identity(rich.session.document().root())?,
                selection: rich.session.selection_state()?,
                scroll: rich.navigation_scroll_offset(),
            })
        })
        .transpose()?;
    Ok(ReturnContent::Document {
        id: document.id.clone(),
        revision: document.revision,
        #[cfg(feature = "editor-preview")]
        rich,
    })
}
#[cfg(feature = "editor-preview")]
fn content_identity(value: &impl serde::Serialize) -> AppResult<[u8; 32]> {
    use sha2::{Digest, Sha256};
    struct HashWriter(Sha256);
    impl Write for HashWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = HashWriter(Sha256::new());
    serde_json::to_writer(&mut writer, value)?;
    Ok(writer.0.finalize().into())
}

#[cfg(all(test, feature = "editor-preview"))]
mod tests {
    use super::*;
    use craft_editor_egui::{DocPoint, Selection};
    use craft_editor_model::ScalarOffset;
    type TestResult = AppResult<()>;
    fn fixture() -> AppResult<(tempfile::TempDir, StudioApp, Project, Document, Document)> {
        let directory = tempfile::tempdir()?;
        let creation = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = StudioApp::new(
            &creation,
            directory.path().join("return.sqlite"),
            false,
            None,
        )?;
        let project = app.workspace.create_project("Return context")?;
        let tree = craft_editor_model::Document::plain("p", "ABC😀 saved")?.to_json()?;
        let origin = app
            .workspace
            .create_rich_document(&project.id, "Origin", tree.clone())?;
        let other = app
            .workspace
            .create_rich_document(&project.id, "Other", tree)?;
        app.open_project(project.clone())?;
        app.open_document(origin.clone())?;
        Ok((directory, app, project, origin, other))
    }
    #[test]
    fn exact_manuscript_return_should_restore_typed_selection_but_not_invent_an_unmeasured_scroll()
    -> TestResult {
        let (_directory, mut app, _project, origin, other) = fixture()?;
        let point = DocPoint {
            block_id: "p".into(),
            offset: ScalarOffset(4),
        };
        let selection = Selection {
            anchor: point.clone(),
            head: point,
        };
        app.rich
            .as_mut()
            .ok_or("rich")?
            .session
            .set_selection(selection.clone())?;
        assert!(
            app.rich
                .as_ref()
                .ok_or("rich")?
                .navigation_scroll_offset()
                .is_none()
        );
        let bookmark = NavigationReturn::capture(&app)?;
        app.open_document(other)?;
        assert!(bookmark.restore(&mut app)?);
        assert_eq!(app.document.as_ref().ok_or("document")?.id, origin.id);
        assert_eq!(
            app.rich.as_ref().ok_or("rich")?.session.selection(),
            Some(&selection)
        );
        Ok(())
    }
    #[test]
    fn stale_manuscript_return_should_open_latest_content_without_reapplying_old_scalar_positions()
    -> TestResult {
        let (_directory, mut app, _project, origin, other) = fixture()?;
        let point = DocPoint {
            block_id: "p".into(),
            offset: ScalarOffset(3),
        };
        app.rich
            .as_mut()
            .ok_or("rich")?
            .session
            .set_selection(Selection {
                anchor: point.clone(),
                head: point,
            })?;
        let bookmark = NavigationReturn::capture(&app)?;
        app.open_document(other)?;
        app.workspace.save_rich_document(
            &origin.id,
            origin.revision,
            craft_editor_model::Document::plain("p", "Different saved content")?.to_json()?,
        )?;
        assert!(!bookmark.restore(&mut app)?);
        assert_eq!(
            app.document.as_ref().ok_or("document")?.body,
            "Different saved content"
        );
        assert!(
            app.rich
                .as_ref()
                .ok_or("rich")?
                .session
                .selection()
                .is_none()
        );
        Ok(())
    }
    #[test]
    fn board_return_should_restore_camera_selection_only_for_the_same_acknowledged_graph()
    -> TestResult {
        let (_directory, mut app, project, _origin, other) = fixture()?;
        let graph = serde_json::json!({"nodes":[{"id":"card","type":"text","x":0,"y":0,"width":200,"height":120,"text":"Saved card"}],"edges":[],"x-writers-studio":{"schemaVersion":2}});
        let board = app
            .workspace
            .create_project_canvas(&project.id, "Board", graph.clone())?;
        app.open_canvas(&board.canvas.id)?;
        let camera = craft_canvas_model::Camera::new(
            craft_canvas_model::WorldPoint::new(100.0, 200.0)?,
            1.5,
        )?;
        let selection = craft_canvas_model::Selection {
            nodes: ["card".into()].into_iter().collect(),
            edges: Default::default(),
        };
        app.canvas
            .as_mut()
            .ok_or("canvas")?
            .restore_navigation_view(camera, selection.clone())?;
        let bookmark = NavigationReturn::capture(&app)?;
        app.open_document(other)?;
        assert!(bookmark.restore(&mut app)?);
        assert_eq!(
            app.canvas.as_ref().ok_or("canvas")?.navigation_view()?,
            (camera, selection)
        );
        let mut changed = graph;
        changed["nodes"][0]["text"] = serde_json::json!("New saved card");
        app.workspace
            .save_project_canvas(&project.id, &board.canvas.id, 1, changed)?;
        assert!(!bookmark.restore(&mut app)?);
        assert_eq!(
            app.canvas
                .as_ref()
                .ok_or("canvas")?
                .snapshot
                .revision
                .info
                .revision,
            2
        );
        Ok(())
    }
}
