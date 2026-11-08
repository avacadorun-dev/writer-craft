#![recursion_limit = "512"]
#![expect(
    clippy::unwrap_used,
    reason = "Synthetic export contracts must fail immediately"
)]
//! Only generated synthetic writing, metadata and media bytes may enter these public fixtures.
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use studio_core::{
    ImportAssetSource, ProjectAttachment, ProjectExportSnapshot, ProjectImportWorker,
    ProjectTransferCancellation, ValidatedProjectImport,
};
use studio_transfer::{
    MarkdownExportSelection, export_markdown_folder, export_markdown_folder_with_progress,
};

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
fn plain(text: &str) -> Value {
    json!({"type":"doc","attrs":{"schemaVersion":4},"content":[paragraph("p",text)]})
}
fn all_nodes() -> Value {
    let mut blocks = vec![
        json!({"type":"heading","attrs":{"blockId":"heading","level":2},"content":[{"type":"text","text":"Visible heading"}]}),
        json!({"type":"paragraph","attrs":{"blockId":"p","sourceRefs":["outside-segment"]},"content":[{"type":"text","text":"selected saved writing"}]}),
        json!({"type":"paragraph","attrs":{"blockId":"marked"},"content":[
            {"type":"text","text":" bold ","marks":[{"type":"bold"}]},
            {"type":"text","text":"italic","marks":[{"type":"italic"}]},
            {"type":"text","text":"underlined","marks":[{"type":"underline"}]},
            {"type":"text","text":"strike","marks":[{"type":"strike"}]},
            {"type":"text","text":"tick`code","marks":[{"type":"code"}]},
            {"type":"text","text":"PDF original","marks":[{"type":"link","attrs":{"href":"/api/media/pdf/content","title":"Source"}}]},
            {"type":"hardBreak"},
            {"type":"inlineMath","attrs":{"latex":"a+b","description":"Inline formula"}}
        ]}),
        json!({"type":"blockquote","attrs":{"blockId":"quote"},"content":[paragraph("quoted","Quoted writing")]}),
        json!({"type":"bulletList","attrs":{"blockId":"bullets"},"content":[{"type":"listItem","content":[paragraph("bullet","Bullet")]}]}),
        json!({"type":"orderedList","attrs":{"blockId":"ordered","start":3,"type":"a"},"content":[{"type":"listItem","content":[paragraph("numbered","Numbered")]}]}),
        json!({"type":"codeBlock","attrs":{"blockId":"code","language":"rust"},"content":[{"type":"text","text":"let x = ```;"}]}),
        json!({"type":"horizontalRule","attrs":{"blockId":"rule"}}),
        json!({"type":"sceneBreak","attrs":{"blockId":"scene"}}),
        json!({"type":"imageBlock","attrs":{"blockId":"image","src":"/api/media/image/content","assetId":"image","alt":"Synthetic image","caption":"Image caption","width":120,"height":100}}),
        json!({"type":"mermaidBlock","attrs":{"blockId":"diagram","code":"flowchart LR\nA-->B","caption":"Diagram caption"}}),
        json!({"type":"callout","attrs":{"blockId":"callout","kind":"warning"},"content":[paragraph("callout-text","Callout writing")]}),
        json!({"type":"mathBlock","attrs":{"blockId":"formula","latex":"E=mc^2","description":"Energy formula"}}),
    ];
    blocks.push(json!({"type":"table","attrs":{"blockId":"table"},"content":[
        {"type":"tableRow","content":[{"type":"tableHeader","attrs":{"colspan":1,"rowspan":1,"colwidth":[180],"align":"center"},"content":[paragraph("head","Header")]}]},
        {"type":"tableRow","content":[{"type":"tableCell","attrs":{"colspan":1,"rowspan":1,"align":"right"},"content":[paragraph("cell","Cell | value")]}]}
    ]}));
    json!({"type":"doc","attrs":{"schemaVersion":4,"vendor":{"retain":true}},"content":blocks})
}
fn fixture() -> (Value, Assets) {
    let png = vec![
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 207, 192, 240,
        31, 0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    let pdf = b"%PDF-1.4\nSynthetic fixture only\n%%EOF".to_vec();
    let audio = b"RIFF\x04\x00\x00\x00WAVEsynthetic original".to_vec();
    let records=[("image","media/image.png","image/png","é 😀.png",&png),("pdf","media/file.pdf","application/pdf","CON.pdf",&pdf)].into_iter().map(|(id,path,mime,name,bytes)|json!({"id":id,"path":path,"sha256":format!("{:x}",Sha256::digest(bytes)),"byteSize":bytes.len(),"mediaType":mime,"originalName":name,"width":if id=="image" {Some(1)}else{None},"height":if id=="image" {Some(1)}else{None}})).collect::<Vec<_>>();
    let manifest = json!({
        "format":"writers-studio-project","formatVersion":8,
        "project":{"id":"source","title":"Synthetic Автор 日本語 😀","kind":"free","language":"en","synopsis":"UNSELECTED_SYNOPSIS","structureRevision":1,"structure":{"schemaVersion":1,"template":"free","rootLabel":"Manuscript","nodeTypes":[{"key":"folder","label":"Folder","contentBearing":false,"canBeRoot":true,"allowedChildren":["document"]},{"key":"document","label":"Document","contentBearing":true,"canBeRoot":true,"allowedChildren":[]}]}},
        "nodes":[
            {"id":"folder","kind":"folder","title":"Tree","status":"draft","position":0,"currentRevision":0},
            {"id":"selected","parentId":"folder","kind":"document","title":"é 😀","status":"draft","position":0,"currentRevision":2,"sourceFrontmatter":"---\noriginal: kept\n---\n"},
            {"id":"secret","parentId":"folder","kind":"document","title":"e\u{301} 😀","status":"draft","position":1,"currentRevision":1}
        ],
        "documentRevisions":[
            {"id":"history","nodeId":"selected","revision":1,"baseRevision":0,"schemaVersion":4,"content":plain("selected original history"),"plainText":"selected original history","source":"manual"},
            {"id":"head","nodeId":"selected","revision":2,"baseRevision":1,"schemaVersion":4,"content":all_nodes(),"plainText":"selected saved writing","source":"manual"},
            {"id":"secret-head","nodeId":"secret","revision":1,"baseRevision":0,"schemaVersion":4,"content":plain("UNSELECTED_BODY"),"plainText":"UNSELECTED_BODY","source":"manual"}
        ],
        "namedVersions":[{"id":"named","nodeId":"selected","revision":1,"name":"Original"}],
        "documentAnnotations":[{"id":"source-note","nodeId":"selected","kind":"note","status":"resolved","anchorStatus":"attached","anchor":{"blockId":"p","start":0,"end":8,"exactText":"selected","prefix":"","suffix":" saved writing"},"note":"Selected source note","targetSnapshot":{},"baseRevision":2,"revision":1}],
        "dictionaryTerms":[],
        "dictationSessions":[{"id":"audio-session","destinationNodeId":"selected","language":"en","status":"review_ready","durationMs":100,"recordedDurationMs":100,"manifest":{},"commandsEnabled":false,"commandPrefix":""},{"id":"outside-session","destinationNodeId":"secret","language":"en","durationMs":0,"recordedDurationMs":0,"manifest":{"private":"UNSELECTED_SOURCE_SESSION"}}],
        "transcriptArtifacts":[{"id":"outside-artifact","sessionId":"outside-session","kind":"batch_source","text":"UNSELECTED_TRANSCRIPT","segments":[{"id":"outside-segment","text":"UNSELECTED_SOURCE_SEGMENT"}]}],"transcriptDrafts":[],"transcriptDraftRevisions":[],
        "audio":[{"sessionId":"audio-session","path":"audio/original.wav","sha256":format!("{:x}",Sha256::digest(&audio)),"byteSize":audio.len(),"durationMs":100,"mediaType":"audio/wav"}],
        "media":records,
        "canvases":[{"id":"board","title":"Board é 😀","position":0,"currentRevision":1}],
        "canvasRevisions":[{"id":"canvas-revision","canvasId":"board","revision":1,"baseRevision":0,"schemaVersion":2,"source":"manual","content":{"nodes":[{"id":"image-card","type":"file","x":0,"y":0,"width":200,"height":120,"file":"writers-studio://media/image","x-writers-studio":{"resource":{"kind":"media","id":"image"},"vendor":"keep"}},{"id":"prose-card","type":"file","x":220,"y":0,"width":200,"height":120,"file":"writers-studio://documents/selected","x-writers-studio":{"resource":{"kind":"document","id":"selected"}}},{"id":"opaque-file","type":"file","x":440,"y":0,"width":200,"height":120,"file":"unowned/local-reference.pdf","x-writers-studio":{"resource":null}}],"edges":[],"x-writers-studio":{"schemaVersion":2,"vendor":{"retain":true}}}}],
        "assistantGuide":{"currentRevision":1,"active":true,"revisions":[{"id":"guide","revision":1,"content":"UNSELECTED_GUIDE","source":"manual"}]},
        "projectMemory":[],"projectMemoryRevisions":[],
        "unknownWriting":"UNSELECTED_UNKNOWN",
        "externalRecoveryCheckpoint":{"text":"UNSELECTED_EXTERNAL_RECOVERY","authority":"forged"}
    });
    (
        manifest,
        Assets(HashMap::from([
            ("media/image.png".into(), png),
            ("media/file.pdf".into(), pdf),
            ("audio/original.wav".into(), audio),
        ])),
    )
}
fn imported(database: &Path) -> (ProjectImportWorker, String) {
    let (manifest, mut assets) = fixture();
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
fn output_text(root: &Path) -> String {
    fn visit(root: &Path, files: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(&path, files);
            } else {
                files.push(path);
            }
        }
    }
    let mut files = Vec::new();
    visit(root, &mut files);
    files
        .iter()
        .map(|path| String::from_utf8_lossy(&std::fs::read(path).unwrap()).into_owned())
        .collect::<Vec<_>>()
        .join("\n")
}
#[test]
fn complete_export_should_keep_rich_reading_views_original_artifacts_and_canonical_histories() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let (worker, project) = imported(&database);
    let snapshot = snapshot(&worker, &project);
    let document_id = worker
        .workspace()
        .resolve_project_resource(&project, "node", "selected")
        .unwrap();
    let canonical = worker
        .workspace()
        .document(&document_id)
        .unwrap()
        .rich_json
        .unwrap();
    let tree = craft_editor_model::Document::from_json(canonical.clone()).unwrap();
    let image = tree
        .root()
        .children()
        .iter()
        .find(|node| node.node_type == "imageBlock")
        .unwrap();
    assert_eq!(image.string_attr("assetId"), Some("image"));
    let images = worker
        .workspace()
        .image_asset_snapshot(&project, &tree)
        .unwrap();
    assert!(images.failures().is_empty());
    let native = worker
        .workspace()
        .resolve_project_resource(&project, "media", "image")
        .unwrap();
    assert_ne!(native, "image");
    assert_eq!(images.metadata("image").unwrap().id, native);
    let expected = fixture().1;
    assert_eq!(
        images.load(image).unwrap().as_slice(),
        expected.0["media/image.png"].as_slice()
    );
    let capsule = worker
        .workspace()
        .project_images(&project)
        .unwrap()
        .export_images(&["image".into()], 1024 * 1024)
        .unwrap();
    assert_eq!(capsule[0].source_id, "image");
    assert_eq!(capsule[0].bytes, expected.0["media/image.png"]);
    assert_eq!(
        worker
            .workspace()
            .document(&document_id)
            .unwrap()
            .rich_json
            .unwrap(),
        canonical
    );
    let destination = directory.path().join("portable");
    let receipt = export_markdown_folder(
        &snapshot,
        &database,
        &destination,
        &MarkdownExportSelection::Project,
        &ProjectTransferCancellation::default(),
    )
    .unwrap();
    assert!(!destination.join(".incomplete").exists());
    let completion: Value = serde_json::from_slice(
        &std::fs::read(destination.join(".writercraft/export.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(completion["state"], "complete");
    assert_eq!(
        completion["files"].as_array().unwrap().len(),
        receipt.files.len()
    );
    let companion: Value = serde_json::from_slice(
        &std::fs::read(destination.join(".writercraft/project.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(companion["sourceManifest"], *snapshot.manifest());
    assert_eq!(companion["nativeState"]["authority"], "untrusted-on-import");
    let mut portable_names = HashSet::new();
    for file in &receipt.files {
        let bytes = std::fs::read(destination.join(&file.path)).unwrap();
        assert_eq!(file.sha256, format!("{:x}", Sha256::digest(&bytes)));
        assert_eq!(file.byte_size, u64::try_from(bytes.len()).unwrap());
        assert!(
            portable_names.insert(
                file.path
                    .to_lowercase()
                    .trim_end_matches(['.', ' '])
                    .to_owned()
            )
        );
    }
    for attachment in snapshot.attachments() {
        assert!(
            receipt
                .files
                .iter()
                .any(|file| file.path.starts_with("assets/") && file.sha256 == attachment.sha256)
        );
    }
    let document = receipt
        .files
        .iter()
        .filter(|file| file.path.starts_with("documents/") && file.path.ends_with(".md"))
        .map(|file| std::fs::read_to_string(destination.join(&file.path)).unwrap())
        .find(|body| body.contains("selected saved writing"))
        .unwrap();
    for visible in [
        "original: kept",
        "## Visible heading",
        "**bold**",
        "*italic*",
        "<u>underlined</u>",
        "~~strike~~",
        "> [!WARNING]",
        "3. ",
        "Image caption",
        "flowchart LR",
        "E=mc^2",
        "Header",
        "Cell",
        "../",
    ] {
        assert!(
            document.contains(visible),
            "missing {visible} in {document}"
        );
    }
    assert!(!document.contains("asset://"));
    assert!(!document.contains("/api/media/"));
    let canvas = receipt
        .files
        .iter()
        .find(|file| file.path.ends_with(".canvas"))
        .unwrap();
    let canvas_value: Value =
        serde_json::from_slice(&std::fs::read(destination.join(&canvas.path)).unwrap()).unwrap();
    for card in canvas_value["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|card| !card["x-writers-studio"]["resource"].is_null())
    {
        let file = card["file"].as_str().unwrap();
        assert!(
            destination
                .join(&canvas.path)
                .parent()
                .unwrap()
                .join(file)
                .is_file(),
            "{file}"
        );
    }
    for feature in [
        "formula-reader-support-required",
        "mermaid-reader-support-required",
        "underline-html-reader-support",
        "table-widths-and-cell-metadata-in-companion",
        "canvas-extensions-and-original-paths-in-companion",
        "unresolved-canvas-file-source-retained-in-companion",
    ] {
        assert!(
            receipt
                .conversions
                .iter()
                .any(|item| item.feature == feature),
            "{feature}"
        );
    }
}
#[test]
fn selected_export_should_filter_every_unrelated_document_and_recovery_domain() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let (worker, project) = imported(&database);
    for (logical, text) in [
        ("selected", "selected unfinished draft"),
        ("secret", "UNSELECTED_NATIVE_DRAFT"),
    ] {
        let native = worker
            .workspace()
            .resolve_project_resource(&project, "node", logical)
            .unwrap();
        worker
            .workspace()
            .save_rich_draft(
                &native,
                if logical == "selected" { 2 } else { 1 },
                None,
                plain(text),
            )
            .unwrap();
    }
    let destination = directory.path().join("selected");
    let receipt = export_markdown_folder(
        &snapshot(&worker, &project),
        &database,
        &destination,
        &MarkdownExportSelection::Documents(vec!["selected".into()]),
        &ProjectTransferCancellation::default(),
    )
    .unwrap();
    assert_eq!(receipt.scope, "selected-documents");
    let text = output_text(&destination);
    for secret in [
        "UNSELECTED_BODY",
        "UNSELECTED_SYNOPSIS",
        "UNSELECTED_GUIDE",
        "UNSELECTED_UNKNOWN",
        "UNSELECTED_NATIVE_DRAFT",
        "UNSELECTED_EXTERNAL_RECOVERY",
        "UNSELECTED_SOURCE_SESSION",
        "UNSELECTED_TRANSCRIPT",
        "UNSELECTED_SOURCE_SEGMENT",
    ] {
        assert!(!text.contains(secret), "leaked {secret}");
    }
    assert!(text.contains("selected unfinished draft"));
    assert!(text.contains("outside-segment"));
    assert!(
        receipt
            .conversions
            .iter()
            .any(|item| item.feature == "out-of-scope-source-references-retained-opaque")
    );
    assert!(text.contains("Selected source note"));
    assert!(text.contains("selected original history"));
    assert!(
        !receipt
            .files
            .iter()
            .any(|file| file.path.starts_with("canvases/"))
    );
    assert_eq!(
        receipt
            .files
            .iter()
            .filter(|file| file.path.starts_with("assets/"))
            .count(),
        3
    );
}
#[test]
fn captured_revision_should_remain_consistent_after_a_later_save() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let (worker, project) = imported(&database);
    let snapshot = snapshot(&worker, &project);
    let native = worker
        .workspace()
        .resolve_project_resource(&project, "node", "selected")
        .unwrap();
    worker
        .workspace()
        .save_rich_document(&native, 2, plain("LATER_CANONICAL_EDIT"))
        .unwrap();
    let destination = directory.path().join("captured");
    export_markdown_folder(
        &snapshot,
        &database,
        &destination,
        &MarkdownExportSelection::Project,
        &ProjectTransferCancellation::default(),
    )
    .unwrap();
    let text = output_text(&destination);
    assert!(text.contains("selected saved writing"));
    assert!(!text.contains("LATER_CANONICAL_EDIT"));
}
#[test]
fn existing_outputs_and_future_sqlite_sidecars_should_be_refused_before_any_creation() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let (worker, project) = imported(&database);
    let snapshot = snapshot(&worker, &project);
    let existing = directory.path().join("existing");
    std::fs::create_dir(&existing).unwrap();
    std::fs::write(existing.join("unrelated.txt"), b"preserve unrelated bytes").unwrap();
    let cancel = ProjectTransferCancellation::default();
    assert!(
        export_markdown_folder(
            &snapshot,
            &database,
            &existing,
            &MarkdownExportSelection::Project,
            &cancel
        )
        .is_err()
    );
    assert_eq!(
        std::fs::read(existing.join("unrelated.txt")).unwrap(),
        b"preserve unrelated bytes"
    );
    for name in [
        "workspace.sqlite",
        "workspace.sqlite-wal",
        "workspace.sqlite-shm",
        "workspace.sqlite-journal",
        "appearance.json",
        "interface.json",
        "editor.json",
        "WORKSPACE.SQLITE-WAL. ",
    ] {
        let target = directory.path().join(name);
        assert!(
            export_markdown_folder(
                &snapshot,
                &database,
                &target,
                &MarkdownExportSelection::Project,
                &cancel
            )
            .is_err(),
            "{name}"
        );
        assert!(!target.is_dir(), "{name}");
    }
}
#[test]
fn cancellation_after_first_reading_file_should_leave_explicit_incomplete_output() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let (worker, project) = imported(&database);
    let destination = directory.path().join("cancelled");
    let cancel = ProjectTransferCancellation::default();
    let control = cancel.clone();
    assert!(
        export_markdown_folder_with_progress(
            &snapshot(&worker, &project),
            &database,
            &destination,
            &MarkdownExportSelection::Project,
            &cancel,
            &mut |progress| {
                if progress.completed >= 1 {
                    control.cancel();
                }
            }
        )
        .is_err()
    );
    assert!(destination.join(".incomplete").is_file());
    assert!(!destination.join(".writercraft/export.json").exists());
    assert!(
        std::fs::read_dir(destination.join("documents"))
            .unwrap()
            .next()
            .is_some()
    );
}
#[test]
fn invalid_selection_should_not_leave_any_output() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let (worker, project) = imported(&database);
    let snapshot = snapshot(&worker, &project);
    for identities in [
        vec![],
        vec!["selected".into(), "selected".into()],
        vec!["folder".into()],
        vec!["missing".into()],
    ] {
        let destination = directory.path().join("invalid");
        assert!(
            export_markdown_folder(
                &snapshot,
                &database,
                &destination,
                &MarkdownExportSelection::Documents(identities),
                &ProjectTransferCancellation::default()
            )
            .is_err()
        );
        assert!(!destination.exists());
    }
}
#[test]
fn opaque_import_should_export_a_named_reading_preview_and_preserve_original_json() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let worker = ProjectImportWorker::open(&database).unwrap();
    let project = worker.workspace().create_project("Opaque fixture").unwrap();
    worker
        .workspace()
        .import_document(
            &project.id,
            "Original",
            "Readable historical projection",
            json!({"futureSchema":99,"original":"OPAQUE_CANONICAL_BYTES"}),
        )
        .unwrap();
    let destination = directory.path().join("opaque");
    let receipt = export_markdown_folder(
        &snapshot(&worker, &project.id),
        &database,
        &destination,
        &MarkdownExportSelection::Project,
        &ProjectTransferCancellation::default(),
    )
    .unwrap();
    assert!(
        receipt
            .conversions
            .iter()
            .any(|item| item.feature == "opaque-import-reading-preview-original-json-in-companion")
    );
    let text = output_text(&destination);
    assert!(text.contains("Readable historical projection"));
    assert!(text.contains("OPAQUE_CANONICAL_BYTES"));
}

#[test]
fn changed_output_bytes_should_never_receive_a_completion_marker() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let (worker, project) = imported(&database);
    let destination = directory.path().join("tampered");
    let mut first = None;
    let mut tampered = false;
    let result = export_markdown_folder_with_progress(
        &snapshot(&worker, &project),
        &database,
        &destination,
        &MarkdownExportSelection::Project,
        &ProjectTransferCancellation::default(),
        &mut |progress| {
            if first.is_none() {
                first = Some(progress.path.clone());
            }
            if progress.path.starts_with("assets/") && !tampered {
                std::fs::write(
                    destination.join(first.as_ref().unwrap()),
                    b"externally changed bytes",
                )
                .unwrap();
                tampered = true;
            }
        },
    );
    assert!(tampered);
    assert!(result.is_err());
    assert!(destination.join(".incomplete").is_file());
    assert!(!destination.join(".writercraft/export.json").exists());
}

#[test]
fn native_stream_folder_should_render_all_source_blocks_and_restore_saved_history_and_artifacts() {
    use studio_core::ProjectPortableExportWorker;
    use studio_transfer::{CanonicalProjectFolder, export_canonical_project_folder};
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("source.sqlite");
    let (worker, project) = imported(&database);
    let mut exporter = ProjectPortableExportWorker::open(&database).unwrap();
    let captured = exporter
        .prepare(&project, &ProjectTransferCancellation::default())
        .unwrap();
    let output = directory.path().join("Readable project Автор 😀");
    let receipt = export_canonical_project_folder(
        &captured,
        &database,
        &output,
        &ProjectTransferCancellation::default(),
    )
    .unwrap();
    assert_eq!(receipt.scope, "complete-project");
    assert!(!output.join(".incomplete").exists());
    let mut reader = CanonicalProjectFolder::open(&output).unwrap();
    let canonical: Value = serde_json::from_slice(reader.root_bytes()).unwrap();
    assert_eq!(canonical["formatVersion"], 2);
    let mut restored = ProjectImportWorker::open(directory.path().join("restored.sqlite")).unwrap();
    let result = reader
        .restore(&mut restored, &ProjectTransferCancellation::default())
        .unwrap();
    let native = restored
        .workspace()
        .resolve_project_resource(&result.project_id, "node", "selected")
        .unwrap();
    assert_eq!(
        restored.workspace().rich_revisions(&native).unwrap().len(),
        2
    );
    let snapshot = restored
        .workspace()
        .project_export_snapshot(&result.project_id)
        .unwrap();
    assert_eq!(snapshot.attachments().len(), 3);
    assert_eq!(
        snapshot
            .assets()
            .load("image", &ProjectTransferCancellation::default())
            .unwrap(),
        fixture().1.0["media/image.png"]
    );
    assert_eq!(
        restored
            .workspace()
            .project_canvas(&result.project_id, "board")
            .unwrap()
            .revision
            .content["nodes"][0]["file"],
        "writers-studio://media/image"
    );
    let text = output_text(&output);
    for required in [
        "## Visible heading",
        "```mermaid",
        "flowchart LR",
        "E=mc^2",
        "> [!WARNING]",
        "Image caption",
        "```rust",
        "Cell \\| value",
        "original: kept",
    ] {
        assert!(
            text.contains(required),
            "Missing reading feature {required:?}"
        );
    }
    assert!(receipt.conversion_count > 0);
}
#[test]
fn changed_reading_file_should_stop_native_restore_before_any_project_is_created() {
    use studio_core::ProjectPortableExportWorker;
    use studio_transfer::{CanonicalProjectFolder, export_canonical_project_folder};
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("source.sqlite");
    let (_worker, project) = imported(&database);
    let captured = ProjectPortableExportWorker::open(&database)
        .unwrap()
        .prepare(&project, &ProjectTransferCancellation::default())
        .unwrap();
    let output = directory.path().join("Export");
    export_canonical_project_folder(
        &captured,
        &database,
        &output,
        &ProjectTransferCancellation::default(),
    )
    .unwrap();
    let mut reader = CanonicalProjectFolder::open(&output).unwrap();
    std::fs::write(output.join("README.md"), "Changed reading view").unwrap();
    let mut restored = ProjectImportWorker::memory().unwrap();
    assert!(
        reader
            .restore(&mut restored, &ProjectTransferCancellation::default())
            .is_err()
    );
    assert!(restored.workspace().list_projects().unwrap().is_empty());
}
#[test]
fn native_folder_should_refuse_incomplete_or_unexpected_members_and_preserve_existing_destinations()
{
    use studio_core::ProjectPortableExportWorker;
    use studio_transfer::{CanonicalProjectFolder, export_canonical_project_folder};
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("source.sqlite");
    let (_worker, project) = imported(&database);
    let captured = ProjectPortableExportWorker::open(&database)
        .unwrap()
        .prepare(&project, &ProjectTransferCancellation::default())
        .unwrap();
    let output = directory.path().join("Export");
    export_canonical_project_folder(
        &captured,
        &database,
        &output,
        &ProjectTransferCancellation::default(),
    )
    .unwrap();
    let original = std::fs::read(output.join("README.md")).unwrap();
    assert!(
        export_canonical_project_folder(
            &captured,
            &database,
            &output,
            &ProjectTransferCancellation::default()
        )
        .is_err()
    );
    assert_eq!(std::fs::read(output.join("README.md")).unwrap(), original);
    std::fs::write(output.join(".incomplete"), "not complete").unwrap();
    assert!(CanonicalProjectFolder::open(&output).is_err());
    std::fs::remove_file(output.join(".incomplete")).unwrap();
    std::fs::write(output.join("unlisted.txt"), "Unlisted data").unwrap();
    assert!(
        CanonicalProjectFolder::open(&output)
            .unwrap()
            .preflight(&ProjectTransferCancellation::default())
            .is_err()
    );
}
#[cfg(unix)]
#[test]
fn opened_native_folder_should_stay_pinned_when_its_ambient_path_is_replaced() {
    use studio_core::ProjectPortableExportWorker;
    use studio_transfer::{CanonicalProjectFolder, export_canonical_project_folder};
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("source.sqlite");
    let (_worker, project) = imported(&database);
    let captured = ProjectPortableExportWorker::open(&database)
        .unwrap()
        .prepare(&project, &ProjectTransferCancellation::default())
        .unwrap();
    let output = directory.path().join("Export");
    export_canonical_project_folder(
        &captured,
        &database,
        &output,
        &ProjectTransferCancellation::default(),
    )
    .unwrap();
    let mut reader = CanonicalProjectFolder::open(&output).unwrap();
    std::fs::rename(&output, directory.path().join("Moved original")).unwrap();
    std::fs::create_dir(&output).unwrap();
    std::fs::write(output.join(".incomplete"), "Replacement path").unwrap();
    assert!(
        reader
            .preflight(&ProjectTransferCancellation::default())
            .is_ok()
    );
    assert!(CanonicalProjectFolder::open(&output).is_err());
}
#[cfg(unix)]
#[test]
fn native_folder_should_refuse_inner_symlink_even_when_target_bytes_match() {
    use studio_core::ProjectPortableExportWorker;
    use studio_transfer::{CanonicalProjectFolder, export_canonical_project_folder};
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("source.sqlite");
    let (_worker, project) = imported(&database);
    let captured = ProjectPortableExportWorker::open(&database)
        .unwrap()
        .prepare(&project, &ProjectTransferCancellation::default())
        .unwrap();
    let output = directory.path().join("Export");
    export_canonical_project_folder(
        &captured,
        &database,
        &output,
        &ProjectTransferCancellation::default(),
    )
    .unwrap();
    let mut reader = CanonicalProjectFolder::open(&output).unwrap();
    let outside = directory.path().join("outside.md");
    std::fs::rename(output.join("README.md"), &outside).unwrap();
    std::os::unix::fs::symlink(&outside, output.join("README.md")).unwrap();
    assert!(
        reader
            .preflight(&ProjectTransferCancellation::default())
            .is_err()
    );
}
