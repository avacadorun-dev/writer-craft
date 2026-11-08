#![expect(
    clippy::unwrap_used,
    reason = "Synthetic saved-reading contracts fail immediately"
)]
//! Reader views belong to the same saved canonical snapshot, never current UI drafts.
use craft_editor_model::Document as RichTree;
use serde_json::{Value, json};
use std::io::Read;
use studio_core::{
    ImportAssetSource, ProjectAttachment, ProjectImportWorker, ProjectPortableExportWorker,
    ProjectTransferCancellation, ValidatedProjectImport, Workspace,
};
struct NoAssets;
impl ImportAssetSource for NoAssets {
    fn open<'a>(&'a mut self, _: &ProjectAttachment) -> std::io::Result<Box<dyn Read + 'a>> {
        Err(std::io::Error::other("No synthetic assets declared"))
    }
}
fn rich(body: &str) -> Value {
    RichTree::plain("p", body).unwrap().to_json().unwrap()
}
fn graph(body: &str) -> Value {
    json!({"nodes":[{"id":"card","type":"text","x":0,"y":0,"width":200,"height":100,"text":body}],"edges":[],"x-writers-studio":{"schemaVersion":2}})
}
#[test]
fn saved_reading_should_stay_at_capture_despite_drafts_and_later_saves() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let workspace = Workspace::open(&database).unwrap();
    let project = workspace.create_project("Captured").unwrap();
    let doc = workspace
        .create_rich_document(&project.id, "Writing", rich("Saved prose"))
        .unwrap();
    workspace
        .save_rich_draft(&doc.id, 1, None, rich("Unpublished draft"))
        .unwrap();
    let board = workspace
        .create_project_canvas(&project.id, "Board", graph("Saved graph"))
        .unwrap();
    workspace
        .save_project_canvas_draft(
            &project.id,
            &board.canvas.id,
            1,
            None,
            graph("Unpublished graph"),
        )
        .unwrap();
    let mut worker = ProjectPortableExportWorker::open(&database).unwrap();
    let captured = worker
        .prepare(&project.id, &ProjectTransferCancellation::default())
        .unwrap();
    workspace
        .save_rich_document(&doc.id, 1, rich("Later saved prose"))
        .unwrap();
    workspace
        .save_project_canvas(&project.id, &board.canvas.id, 1, graph("Later saved graph"))
        .unwrap();
    let reading = captured.reading_document(&doc.id).unwrap();
    assert_eq!(reading.body, "Saved prose");
    assert_eq!(reading.rich_content, Some(rich("Saved prose")));
    assert_eq!(
        captured.reading_canvas(&board.canvas.id).unwrap(),
        graph("Saved graph")
    );
    let mut nodes = Vec::new();
    captured
        .visit_reading_nodes(&mut |node| {
            nodes.push(node);
            Ok(())
        })
        .unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].current_revision, 1);
    assert!(
        captured
            .reading_document("another-project-resource")
            .is_err()
    );
    assert!(captured.reading_canvas("another-project-resource").is_err());
}
#[test]
fn source_hierarchy_frontmatter_and_latest_saved_board_should_survive_the_reading_projection() {
    let original = json!({"format":"writers-studio-project","formatVersion":8,
      "project":{"id":"source","title":"Source","kind":"free","language":"en","structureRevision":1,"structure":{"schemaVersion":1,"template":"free","rootLabel":"Writing","nodeTypes":[{"key":"folder","label":"Folder","contentBearing":false,"canBeRoot":true,"allowedChildren":["document"]},{"key":"document","label":"Document","contentBearing":true,"canBeRoot":true,"allowedChildren":[]}]}},
      "nodes":[{"id":"folder","kind":"folder","title":"Folder","status":"draft","position":2,"currentRevision":0},{"id":"doc","kind":"document","parentId":"folder","title":"Chapter Автор 😀","status":"draft","position":1,"currentRevision":2,"sourceFrontmatter":"---\ntitle: exact source\n---\n"}],
      "documentRevisions":[{"id":"r1","nodeId":"doc","revision":1,"baseRevision":0,"schemaVersion":1,"content":rich("First"),"plainText":"First","source":"manual"},{"id":"r2","nodeId":"doc","revision":2,"baseRevision":1,"schemaVersion":1,"content":rich("Current"),"plainText":"Current","source":"manual"}],
      "canvases":[{"id":"board","title":"Source board","position":5,"currentRevision":2}],
      "canvasRevisions":[{"id":"c1","canvasId":"board","revision":1,"baseRevision":0,"schemaVersion":2,"content":graph("First"),"source":"manual"},{"id":"c2","canvasId":"board","revision":2,"baseRevision":1,"schemaVersion":2,"content":graph("Current"),"source":"manual"}]
    });
    let source = ValidatedProjectImport::parse(&serde_json::to_vec(&original).unwrap()).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let mut importer = ProjectImportWorker::open(&database).unwrap();
    let imported = importer
        .import(
            &source,
            &mut NoAssets,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    let mut worker = ProjectPortableExportWorker::open(&database).unwrap();
    let captured = worker
        .prepare(
            &imported.project_id,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    let mut nodes = Vec::new();
    captured
        .visit_reading_nodes(&mut |node| {
            nodes.push(node);
            Ok(())
        })
        .unwrap();
    assert_eq!(
        nodes
            .iter()
            .map(|n| n.logical_id.as_str())
            .collect::<Vec<_>>(),
        ["folder", "doc"]
    );
    assert_eq!(nodes[1].parent_id.as_deref(), Some("folder"));
    assert_eq!(nodes[1].title, "Chapter Автор 😀");
    assert_eq!(nodes[1].current_revision, 2);
    assert!(captured.reading_document("folder").is_err());
    let doc = captured.reading_document("doc").unwrap();
    assert_eq!(doc.body, "Current");
    assert_eq!(doc.source_frontmatter, "---\ntitle: exact source\n---\n");
    let mut boards = Vec::new();
    captured
        .visit_reading_canvases(&mut |board| {
            boards.push(board);
            Ok(())
        })
        .unwrap();
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0].current_revision, 2);
    assert_eq!(captured.reading_canvas("board").unwrap(), graph("Current"));
}
#[test]
fn opaque_source_should_export_a_named_read_only_preview_without_inventing_rich_content() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let workspace = Workspace::open(&database).unwrap();
    let project = workspace.create_project("Opaque").unwrap();
    let doc = workspace
        .import_document(
            &project.id,
            "Read-only source",
            "Preview only",
            json!({"type":"future-document","unknown":{"keep":true}}),
        )
        .unwrap();
    let mut worker = ProjectPortableExportWorker::open(&database).unwrap();
    let captured = worker
        .prepare(&project.id, &ProjectTransferCancellation::default())
        .unwrap();
    let mut nodes = Vec::new();
    captured
        .visit_reading_nodes(&mut |node| {
            nodes.push(node);
            Ok(())
        })
        .unwrap();
    assert!(nodes[0].read_only_preview);
    let reading = captured.reading_document(&doc.id).unwrap();
    assert_eq!(reading.body, "Preview only");
    assert!(reading.rich_content.is_none());
}

#[test]
fn explicit_export_target_should_refuse_a_changed_displayed_title_before_preparation() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let workspace = Workspace::open(&database).unwrap();
    let project = workspace.create_project("Displayed project").unwrap();
    let mut worker = ProjectPortableExportWorker::open(&database).unwrap();
    assert!(
        worker
            .prepare_named(
                &project.id,
                "A different displayed project",
                &ProjectTransferCancellation::default()
            )
            .is_err()
    );
    assert!(
        worker
            .prepare_named(
                &project.id,
                &project.title,
                &ProjectTransferCancellation::default()
            )
            .is_ok()
    );
    assert_eq!(workspace.list_projects().unwrap(), vec![project]);
}
