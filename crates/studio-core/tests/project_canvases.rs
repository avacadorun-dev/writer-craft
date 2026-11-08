#![recursion_limit = "512"]
#![expect(
    clippy::unwrap_used,
    reason = "Synthetic Canvas persistence contracts fail immediately"
)]
//! Synthetic project Canvas lifecycle contracts; no owner archives or diagrams are used.
use std::io::Read;
use std::path::Path;

use serde_json::{Value, json};
use studio_core::{
    ImportAssetSource, ProjectAttachment, ProjectImportWorker, ProjectTransferCancellation,
    ValidatedProjectImport, Workspace,
};

struct NoAssets;
impl ImportAssetSource for NoAssets {
    fn open<'a>(&'a mut self, _: &ProjectAttachment) -> std::io::Result<Box<dyn Read + 'a>> {
        Err(std::io::Error::other("fixture has no binary artifacts"))
    }
}
fn content(text: &str) -> Value {
    json!({"nodes":[
        {"id":"card","type":"text","x":0,"y":0,"width":200,"height":120,"text":text,"x-writers-studio":{"custom":{"keep":true,"nullable":null}}},
        {"id":"file","type":"file","x":220,"y":0,"width":200,"height":120,"file":"writers-studio://documents/doc","x-writers-studio":{"resource":{"kind":"document","id":"doc"}}}
    ],"edges":[{"id":"edge","fromNode":"card","toNode":"file","label":"Relation","vendor":"unchanged"}],"x-writers-studio":{"schemaVersion":2,"vendor":{"retain":true}}})
}
fn fixture() -> Vec<u8> {
    let manifest = json!({
        "format":"writers-studio-project","formatVersion":8,
        "project":{"id":"source","title":"Synthetic Canvas project","kind":"free","language":"en","structureRevision":1,"structure":{"schemaVersion":1,"template":"free","rootLabel":"Writing","nodeTypes":[{"key":"document","label":"Document","contentBearing":true,"canBeRoot":true,"allowedChildren":[]}]}},
        "nodes":[{"id":"doc","kind":"document","title":"Writing","status":"draft","position":0,"currentRevision":1}],
        "documentRevisions":[{"id":"doc-r1","nodeId":"doc","revision":1,"baseRevision":0,"schemaVersion":1,"content":{"type":"doc","attrs":{"schemaVersion":1},"content":[{"type":"paragraph","attrs":{"blockId":"p"},"content":[{"type":"text","text":"Synthetic prose"}]}]},"plainText":"Synthetic prose","source":"manual"}],
        "namedVersions":[],"dictionaryTerms":[],"documentAnnotations":[],"dictationSessions":[],"transcriptArtifacts":[],"transcriptDrafts":[],"transcriptDraftRevisions":[],"audio":[],"media":[],
        "canvases":[{"id":"board","title":"Board Автор 😀","position":0,"currentRevision":2,"vendor":{"unknown":null,"precise":"HUGE"}}],
        "canvasRevisions":[
            {"id":"canvas-r1","canvasId":"board","revision":1,"baseRevision":0,"schemaVersion":2,"content":content("First graph"),"source":"manual","createdAt":"2026-11-08T00:00:00Z","unknownRevision":{"precise":"HUGE"}},
            {"id":"canvas-r2","canvasId":"board","revision":2,"baseRevision":1,"schemaVersion":2,"content":content("Current graph"),"source":"manual","unknownRevision":{"precise":"HUGE"}}
        ],"projectMemory":[],"projectMemoryRevisions":[]
    });
    serde_json::to_string_pretty(&manifest)
        .unwrap()
        .replace("\"HUGE\"", "1.2345678901234567890123456789e+100")
        .into_bytes()
}
fn imported(path: &Path) -> (ProjectImportWorker, String) {
    let validated = ValidatedProjectImport::parse(&fixture()).unwrap();
    let mut worker = ProjectImportWorker::open(path).unwrap();
    let result = worker
        .import(
            &validated,
            &mut NoAssets,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    (worker, result.project_id)
}
#[test]
fn selected_board_and_paged_history_should_survive_reopen_without_a_manifest_capture() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("canvas.sqlite");
    let (worker, project) = imported(&database);
    let boards = worker.workspace().project_canvases(&project).unwrap();
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0].id, "board");
    assert_eq!(boards[0].current_revision, 2);
    assert_eq!(
        worker
            .workspace()
            .project_canvas(&project, "board")
            .unwrap()
            .revision
            .content,
        content("Current graph")
    );
    let page = worker
        .workspace()
        .project_canvas_history(&project, "board", None, 1)
        .unwrap();
    assert_eq!(page.len(), 1);
    assert_eq!(page[0].revision, 2);
    assert_eq!(page[0].created_at, None);
    let next = worker
        .workspace()
        .project_canvas_history(&project, "board", Some(page[0].revision), 1)
        .unwrap();
    assert_eq!(next[0].revision, 1);
    assert!(next[0].created_at.is_some());
    drop(worker);
    let reopened = Workspace::open(database).unwrap();
    assert_eq!(
        reopened
            .project_canvas_revision(&project, "board", 1)
            .unwrap()
            .content,
        content("First graph")
    );
}
#[test]
fn save_and_restore_should_append_preserving_source_metadata_unknown_precision_and_old_graphs() {
    let directory = tempfile::tempdir().unwrap();
    let (worker, project) = imported(&directory.path().join("canvas.sqlite"));
    let saved = worker
        .workspace()
        .save_project_canvas(&project, "board", 2, content("Changed graph"))
        .unwrap();
    assert_eq!(saved.info.revision, 3);
    assert_eq!(saved.info.base_revision, 2);
    assert!(saved.info.created_at.is_some());
    assert!(
        worker
            .workspace()
            .save_project_canvas(&project, "board", 2, content("Stale graph"))
            .is_err()
    );
    let restored = worker
        .workspace()
        .restore_project_canvas_revision(&project, "board", 3, 1)
        .unwrap();
    assert_eq!(restored.info.revision, 4);
    assert_eq!(restored.info.base_revision, 3);
    assert_eq!(restored.info.source, "restore");
    assert_eq!(restored.content, content("First graph"));
    assert_eq!(
        worker
            .workspace()
            .project_canvas_revision(&project, "board", 2)
            .unwrap()
            .content,
        content("Current graph")
    );
    let renamed = worker
        .workspace()
        .rename_project_canvas(&project, "board", 4, "Renamed 日本語")
        .unwrap();
    assert_eq!(renamed.title, "Renamed 日本語");
    assert!(renamed.metadata["vendor"]["unknown"].is_null());
    let snapshot = worker
        .workspace()
        .project_export_snapshot(&project)
        .unwrap();
    assert_eq!(
        snapshot.manifest()["canvases"][0]["title"],
        "Renamed 日本語"
    );
    assert_eq!(snapshot.manifest()["canvases"][0]["currentRevision"], 4);
    assert_eq!(
        snapshot.manifest()["canvasRevisions"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    let encoded = std::str::from_utf8(snapshot.manifest_bytes()).unwrap();
    assert!(encoded.contains("1.2345678901234567890123456789e+100"));
    ValidatedProjectImport::parse(snapshot.manifest_bytes()).unwrap();
}
#[test]
fn unchanged_graph_save_should_not_invent_another_saved_revision() {
    let directory = tempfile::tempdir().unwrap();
    let (worker, project) = imported(&directory.path().join("canvas.sqlite"));
    let result = worker
        .workspace()
        .save_project_canvas(&project, "board", 2, content("Current graph"))
        .unwrap();
    assert_eq!(result.info.revision, 2);
    assert_eq!(
        worker
            .workspace()
            .project_canvas_history(&project, "board", None, 256)
            .unwrap()
            .len(),
        2
    );
}
#[test]
fn native_draft_recovery_should_compare_both_head_and_acknowledged_version() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("canvas.sqlite");
    let (worker, project) = imported(&database);
    let first = worker
        .workspace()
        .save_project_canvas_draft(&project, "board", 2, None, content("Unfinished one"))
        .unwrap();
    assert!(
        worker
            .workspace()
            .save_project_canvas_draft(&project, "board", 2, None, content("Lost update"))
            .is_err()
    );
    let second = worker
        .workspace()
        .save_project_canvas_draft(
            &project,
            "board",
            2,
            Some(&first.version),
            content("Unfinished two"),
        )
        .unwrap();
    assert!(
        worker
            .workspace()
            .discard_project_canvas_draft(&project, "board", &first.version)
            .is_err()
    );
    let snapshot = worker
        .workspace()
        .project_export_snapshot(&project)
        .unwrap();
    assert!(snapshot.requires_canonical_companion());
    assert_eq!(
        snapshot.native_state()["canvasDrafts"][0]["content"],
        content("Unfinished two")
    );
    drop(worker);
    let reopened = Workspace::open(&database).unwrap();
    assert_eq!(
        reopened
            .project_canvas(&project, "board")
            .unwrap()
            .draft
            .unwrap()
            .version,
        second.version
    );
    let recovered = reopened
        .recover_project_canvas_draft(&project, "board", 2, &second.version)
        .unwrap();
    assert_eq!(recovered.info.revision, 3);
    assert_eq!(recovered.content, content("Unfinished two"));
    assert!(
        reopened
            .project_canvas(&project, "board")
            .unwrap()
            .draft
            .is_none()
    );
}
#[test]
fn stale_recovery_should_keep_both_new_saved_graph_and_older_draft_until_explicit_discard() {
    let directory = tempfile::tempdir().unwrap();
    let (worker, project) = imported(&directory.path().join("canvas.sqlite"));
    let draft = worker
        .workspace()
        .save_project_canvas_draft(&project, "board", 2, None, content("Older unfinished"))
        .unwrap();
    worker
        .workspace()
        .save_project_canvas(&project, "board", 2, content("New saved graph"))
        .unwrap();
    assert!(
        worker
            .workspace()
            .recover_project_canvas_draft(&project, "board", 3, &draft.version)
            .is_err()
    );
    assert!(
        worker
            .workspace()
            .save_project_canvas_draft(
                &project,
                "board",
                3,
                Some(&draft.version),
                content("New draft")
            )
            .is_err()
    );
    let board = worker
        .workspace()
        .project_canvas(&project, "board")
        .unwrap();
    assert_eq!(board.revision.content, content("New saved graph"));
    assert_eq!(board.draft.unwrap().content, content("Older unfinished"));
    worker
        .workspace()
        .discard_project_canvas_draft(&project, "board", &draft.version)
        .unwrap();
    let newer = worker
        .workspace()
        .save_project_canvas_draft(&project, "board", 3, None, content("New draft"))
        .unwrap();
    assert_ne!(draft.version, newer.version);
    assert!(
        worker
            .workspace()
            .discard_project_canvas_draft(&project, "board", &draft.version)
            .is_err()
    );
    assert_eq!(
        worker
            .workspace()
            .project_canvas(&project, "board")
            .unwrap()
            .draft
            .unwrap()
            .version,
        newer.version
    );
}
#[test]
fn invalid_references_or_geometry_should_fail_without_mutating_saved_or_draft_state() {
    let directory = tempfile::tempdir().unwrap();
    let (worker, project) = imported(&directory.path().join("canvas.sqlite"));
    let mut foreign = content("Foreign reference");
    foreign["nodes"][1]["x-writers-studio"]["resource"]["id"] = "another-project-document".into();
    assert!(
        worker
            .workspace()
            .save_project_canvas(&project, "board", 2, foreign)
            .is_err()
    );
    let mut invalid = content("Invalid geometry");
    invalid["nodes"][0]["width"] = 39.into();
    assert!(
        worker
            .workspace()
            .save_project_canvas_draft(&project, "board", 2, None, invalid)
            .is_err()
    );
    assert!(
        worker
            .workspace()
            .save_project_canvas(&project, "board", u64::MAX, content("Overflow"))
            .is_err()
    );
    assert!(
        worker
            .workspace()
            .project_canvas_history(&project, "board", None, 0)
            .is_err()
    );
    assert!(
        worker
            .workspace()
            .project_canvas_history(&project, "board", None, 257)
            .is_err()
    );
    let board = worker
        .workspace()
        .project_canvas(&project, "board")
        .unwrap();
    assert_eq!(board.canvas.current_revision, 2);
    assert!(board.draft.is_none());
}
#[test]
fn native_board_creation_should_export_and_never_cross_project_ownership() {
    let workspace = Workspace::memory().unwrap();
    let first = workspace.create_project("First").unwrap();
    let other = workspace.create_project("Other").unwrap();
    let graph =
        json!({"nodes":[],"edges":[],"x-writers-studio":{"schemaVersion":2,"unknown":true}});
    let created = workspace
        .create_project_canvas(&first.id, "New board", graph.clone())
        .unwrap();
    assert_eq!(created.revision.content, graph);
    assert_eq!(created.canvas.current_revision, 1);
    assert!(
        workspace
            .project_canvas(&other.id, &created.canvas.id)
            .is_err()
    );
    assert!(
        workspace
            .rename_project_canvas(&other.id, &created.canvas.id, 1, "Foreign")
            .is_err()
    );
    assert!(workspace.project_canvases(&other.id).unwrap().is_empty());
    let document = workspace
        .create_document(&first.id, "Native writing", "Synthetic")
        .unwrap();
    let mut linked = content("Native link");
    linked["nodes"][1]["x-writers-studio"]["resource"]["id"] = document.id.into();
    workspace
        .save_project_canvas(&first.id, &created.canvas.id, 1, linked)
        .unwrap();
    let snapshot = workspace.project_export_snapshot(&first.id).unwrap();
    assert_eq!(snapshot.manifest()["canvases"].as_array().unwrap().len(), 1);
    ValidatedProjectImport::parse(snapshot.manifest_bytes()).unwrap();
}
