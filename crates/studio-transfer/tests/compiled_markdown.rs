#![recursion_limit = "512"]
#![expect(
    clippy::unwrap_used,
    reason = "Synthetic compiled-manuscript contracts must fail immediately"
)]
//! Only generated synthetic writing, metadata and media bytes may enter these public fixtures.
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use studio_core::{
    ImportAssetSource, ProjectAttachment, ProjectExportSnapshot, ProjectImportWorker,
    ProjectTransferCancellation, ValidatedProjectImport,
};
use studio_transfer::{ManuscriptSelection, compile_manuscript, plan_manuscript};

struct Assets(HashMap<String, Vec<u8>>);
impl ImportAssetSource for Assets {
    fn open<'a>(
        &'a mut self,
        attachment: &ProjectAttachment,
    ) -> std::io::Result<Box<dyn Read + 'a>> {
        let bytes = self
            .0
            .get(&attachment.path)
            .ok_or_else(|| std::io::Error::other("missing synthetic bytes"))?;
        Ok(Box::new(Cursor::new(bytes.as_slice())))
    }
}
fn paragraph(id: &str, text: &str) -> Value {
    json!({"type":"paragraph","attrs":{"blockId":id},"content":[{"type":"text","text":text}]})
}
fn doc(blocks: Value) -> Value {
    json!({"type":"doc","attrs":{"schemaVersion":4},"content":blocks})
}
fn synthetic_png() -> Vec<u8> {
    vec![
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 207, 192, 240,
        31, 0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ]
}
fn fixture() -> (Value, Assets) {
    let png = synthetic_png();
    let media = json!([{"id":"image","path":"media/image.png","sha256":format!("{:x}",Sha256::digest(&png)),"byteSize":png.len(),"mediaType":"image/png","originalName":"é 😀.png","width":1,"height":1}]);
    let first = doc(json!([
        json!({"type":"heading","attrs":{"blockId":"body-heading","level":2},"content":[{"type":"text","text":"Authored section"}]}),
        json!({"type":"imageBlock","attrs":{"blockId":"picture","src":"/api/media/image/content","assetId":"image","alt":"Synthetic image"}}),
        json!({"type":"paragraph","attrs":{"blockId":"to-two"},"content":[{"type":"text","text":"next","marks":[{"type":"link","attrs":{"href":"/projects/source/studio/two"}}]}]}),
        json!({"type":"paragraph","attrs":{"blockId":"to-secret"},"content":[{"type":"text","text":"outside","marks":[{"type":"link","attrs":{"href":"/projects/source/studio/secret"}}]}]})
    ]));
    let second = doc(json!([paragraph("two-body", "Second chapter writing")]));
    let secret = doc(json!([paragraph("secret-body", "UNSELECTED_SECRET")]));
    let manifest = json!({
        "format":"writers-studio-project","formatVersion":8,
        "project":{"id":"source","title":"Compiled Автор 😀","kind":"free","language":"en","synopsis":"UNSELECTED_SYNOPSIS","structureRevision":1,"structure":{"schemaVersion":1,"template":"free","rootLabel":"Manuscript","nodeTypes":[{"key":"folder","label":"Folder","contentBearing":false,"canBeRoot":true,"allowedChildren":["document"]},{"key":"document","label":"Document","contentBearing":true,"canBeRoot":true,"allowedChildren":[]}]}},
        "nodes":[
            {"id":"part","kind":"folder","title":"Part","status":"draft","position":0,"currentRevision":0},
            {"id":"one","parentId":"part","kind":"document","title":"One","status":"draft","position":0,"currentRevision":1},
            {"id":"two","parentId":"part","kind":"document","title":"Two","status":"draft","position":1,"currentRevision":1},
            {"id":"secret","kind":"document","title":"Secret","status":"draft","position":1,"currentRevision":1}
        ],
        "documentRevisions":[
            {"id":"revision-one","nodeId":"one","revision":1,"baseRevision":0,"schemaVersion":4,"content":first,"plainText":"First","source":"manual"},
            {"id":"revision-two","nodeId":"two","revision":1,"baseRevision":0,"schemaVersion":4,"content":second,"plainText":"Second","source":"manual"},
            {"id":"revision-secret","nodeId":"secret","revision":1,"baseRevision":0,"schemaVersion":4,"content":secret,"plainText":"Secret","source":"manual"}
        ],
        "namedVersions":[],"documentAnnotations":[],"dictionaryTerms":[],"dictationSessions":[],
        "transcriptArtifacts":[],"transcriptDrafts":[],"transcriptDraftRevisions":[],"audio":[],
        "media":media,"canvases":[],"canvasRevisions":[],"assistantGuide":{"currentRevision":0,"active":false,"revisions":[]},
        "projectMemory":[],"projectMemoryRevisions":[]
    });
    (
        manifest,
        Assets(HashMap::from([("media/image.png".into(), png)])),
    )
}
fn imported(database: &Path) -> (ProjectImportWorker, String) {
    let (manifest, _) = fixture();
    imported_manifest(database, manifest)
}
fn imported_manifest(database: &Path, manifest: Value) -> (ProjectImportWorker, String) {
    let (_, mut assets) = fixture();
    let validated = ValidatedProjectImport::parse(&serde_json::to_vec(&manifest).unwrap()).unwrap();
    let mut worker = ProjectImportWorker::open(database).unwrap();
    let result = worker
        .import(
            &validated,
            &mut assets,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    (worker, result.project_id)
}
fn snapshot(worker: &ProjectImportWorker, project: &str) -> ProjectExportSnapshot {
    worker.workspace().project_export_snapshot(project).unwrap()
}
fn text(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}
fn files(root: &Path) -> Vec<PathBuf> {
    fn visit(root: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(&path, out);
            } else {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    visit(root, &mut out);
    out
}
fn sha(path: &Path) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
}

#[test]
fn subtree_compilation_should_emit_one_document_in_tree_order_with_nested_headings() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let (worker, project) = imported(&database);
    let snapshot = snapshot(&worker, &project);
    let destination = directory.path().join("manuscript");
    let receipt = compile_manuscript(
        &snapshot,
        &database,
        &destination,
        &ManuscriptSelection::Subtree("part".into()),
        &ProjectTransferCancellation::default(),
    )
    .unwrap();
    assert_eq!(
        receipt
            .entries
            .iter()
            .map(|entry| (
                entry.node_id.as_str(),
                entry.kind.as_str(),
                entry.heading_level
            ))
            .collect::<Vec<_>>(),
        vec![
            ("part", "folder", 1),
            ("one", "document", 2),
            ("two", "document", 2)
        ]
    );
    let body = text(&destination.join(&receipt.document));
    let part = body.find("# Part").unwrap();
    let one = body.find("## One").unwrap();
    let two = body.find("## Two").unwrap();
    assert!(part < one && one < two, "chapters must follow tree order");
    assert!(!body.contains("UNSELECTED_SECRET"));
    assert_eq!(body.matches("<a id=\"document-").count(), 2);
    assert_eq!(
        body.matches("<a id=").count(),
        2,
        "a reader manuscript keeps only chapter anchors, not per-block HTML markers"
    );
    assert!(
        body.lines().any(|line| line == "#### Authored section"),
        "body headings shift under the chapter"
    );
}

#[test]
fn selected_documents_only_should_exclude_other_manuscript_writing() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let (worker, project) = imported(&database);
    let snapshot = snapshot(&worker, &project);
    let destination = directory.path().join("selected");
    let receipt = compile_manuscript(
        &snapshot,
        &database,
        &destination,
        &ManuscriptSelection::Documents(vec!["two".into()]),
        &ProjectTransferCancellation::default(),
    )
    .unwrap();
    assert_eq!(receipt.entries.len(), 1);
    let body = text(&destination.join(&receipt.document));
    assert!(body.contains("Second chapter writing"));
    assert!(!body.contains("Authored section"));
    assert!(!body.contains("First"));
}

#[test]
fn referenced_assets_should_be_copied_adjacently_with_relative_links_and_matching_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let (worker, project) = imported(&database);
    let snapshot = snapshot(&worker, &project);
    let destination = directory.path().join("assets-export");
    let receipt = compile_manuscript(
        &snapshot,
        &database,
        &destination,
        &ManuscriptSelection::Subtree("part".into()),
        &ProjectTransferCancellation::default(),
    )
    .unwrap();
    let body = text(&destination.join(&receipt.document));
    let assets = receipt.assets_directory.clone().unwrap();
    let asset_files = files(&destination.join(&assets));
    assert_eq!(asset_files.len(), 1);
    let relative = asset_files[0]
        .strip_prefix(&destination)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    let encoded = relative
        .split('/')
        .map(percent_encode_path)
        .collect::<Vec<_>>()
        .join("/");
    assert!(
        body.contains(&format!("](<{encoded}>)")),
        "the document links the copied asset"
    );
    assert!(
        percent_decode_path(&encoded).contains("é 😀"),
        "Unicode original name stays readable in the linked path"
    );
    assert_eq!(
        std::fs::read(&asset_files[0]).unwrap(),
        synthetic_png(),
        "original artifact bytes are copied unchanged"
    );
    for file in &receipt.files {
        assert_eq!(sha(&destination.join(&file.path)), file.sha256);
    }
    assert!(!destination.join(".incomplete").exists());
}

#[test]
fn moving_the_document_and_assets_should_preserve_resolvable_relative_links() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let (worker, project) = imported(&database);
    let snapshot = snapshot(&worker, &project);
    let destination = directory.path().join("first");
    let receipt = compile_manuscript(
        &snapshot,
        &database,
        &destination,
        &ManuscriptSelection::Subtree("part".into()),
        &ProjectTransferCancellation::default(),
    )
    .unwrap();
    let moved = directory.path().join("moved");
    std::fs::create_dir(&moved).unwrap();
    std::fs::copy(
        destination.join(&receipt.document),
        moved.join(&receipt.document),
    )
    .unwrap();
    let assets = receipt.assets_directory.clone().unwrap();
    copy_directory(&destination.join(&assets), &moved.join(&assets));
    let body = text(&moved.join(&receipt.document));
    for line in body.lines() {
        if let Some(target) = line
            .split("](<")
            .nth(1)
            .and_then(|tail| tail.split(">)").next())
            && !target.starts_with('#')
            && !target.starts_with("http")
        {
            assert!(
                moved.join(percent_decode_path(target)).exists(),
                "moved link must resolve: {target}"
            );
        }
    }
}
fn percent_encode_path(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}
fn percent_decode_path(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) = (
                char::from(bytes[index + 1]).to_digit(16),
                char::from(bytes[index + 2]).to_digit(16),
            )
        {
            out.push((high * 16 + low) as u8);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| value.to_owned())
}
fn copy_directory(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_directory(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn existing_destination_should_be_refused_without_overwriting() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let (worker, project) = imported(&database);
    let snapshot = snapshot(&worker, &project);
    let destination = directory.path().join("occupied");
    std::fs::create_dir(&destination).unwrap();
    std::fs::write(destination.join("keep.txt"), b"keep").unwrap();
    let result = compile_manuscript(
        &snapshot,
        &database,
        &destination,
        &ManuscriptSelection::Subtree("part".into()),
        &ProjectTransferCancellation::default(),
    );
    assert!(result.is_err());
    assert_eq!(
        std::fs::read(destination.join("keep.txt")).unwrap(),
        b"keep"
    );
}

#[test]
fn cancelled_compilation_should_not_create_the_destination_or_report_completion() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let (worker, project) = imported(&database);
    let snapshot = snapshot(&worker, &project);
    let destination = directory.path().join("cancelled");
    let cancel = ProjectTransferCancellation::default();
    cancel.cancel();
    let result = compile_manuscript(
        &snapshot,
        &database,
        &destination,
        &ManuscriptSelection::Subtree("part".into()),
        &cancel,
    );
    assert!(result.is_err());
    assert!(!destination.exists());
}

#[test]
fn included_document_links_should_become_in_document_anchors_and_outside_links_should_be_disclosed()
{
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let (worker, project) = imported(&database);
    let snapshot = snapshot(&worker, &project);
    let destination = directory.path().join("links");
    let receipt = compile_manuscript(
        &snapshot,
        &database,
        &destination,
        &ManuscriptSelection::Subtree("part".into()),
        &ProjectTransferCancellation::default(),
    )
    .unwrap();
    let body = text(&destination.join(&receipt.document));
    assert!(
        body.contains("](<#document-two>)"),
        "a link to an included chapter resolves inside the single document"
    );
    assert!(receipt.conversions.iter().any(|conversion| {
        conversion.resource_id == "one"
            && conversion.feature == "document-link-outside-export-scope"
    }));
}

#[test]
fn plan_preview_should_match_the_compiled_inclusion_and_order() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let (worker, project) = imported(&database);
    let snapshot = snapshot(&worker, &project);
    let plan = plan_manuscript(&snapshot, &ManuscriptSelection::Subtree("part".into())).unwrap();
    assert_eq!(plan.entries.len(), 3);
    assert!(plan.conversions.iter().any(|conversion| {
        conversion.feature == "reader-export-is-not-a-canonical-project-backup"
    }));
    assert!(
        plan.conversions.iter().any(|conversion| {
            conversion.resource_id == "one"
                && conversion.feature == "document-link-outside-export-scope"
        }),
        "the preview must disclose outside-selection links before writing"
    );
}

#[test]
fn whole_project_selection_should_include_active_nodes_and_skip_deleted_writing() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let (mut manifest, _) = fixture();
    manifest["nodes"][2]["trashedAt"] = json!("2026-11-01T00:00:00Z");
    let (worker, project) = imported_manifest(&database, manifest);
    let snapshot = snapshot(&worker, &project);
    let destination = directory.path().join("whole");
    let receipt = compile_manuscript(
        &snapshot,
        &database,
        &destination,
        &ManuscriptSelection::Project,
        &ProjectTransferCancellation::default(),
    )
    .unwrap();
    assert_eq!(
        receipt
            .entries
            .iter()
            .map(|entry| entry.node_id.as_str())
            .collect::<Vec<_>>(),
        vec!["part", "one", "secret"],
        "deleted writing never appears in a reader manuscript"
    );
    let body = text(&destination.join(&receipt.document));
    assert!(!body.contains("Second chapter writing"));
    assert!(body.contains("Secret"));
}
