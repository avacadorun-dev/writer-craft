#![expect(
    clippy::unwrap_used,
    reason = "Synthetic semantic canonical contracts fail immediately"
)]
//! Complete native saved state restores independently; external drafts/reviews never gain authority.
use std::collections::BTreeMap;
use std::io::{Cursor, Read};

use craft_editor_model::{
    Command, Document as RichTree, Revision, ScalarOffset, TextRange, Transaction,
};
use serde_json::{Value, json};
use sha2::Digest;
use studio_core::{
    AnchorStatus, AnnotationStatus, PortableObjectReference, PortableObjectSource,
    PreparedPortableProject, ProjectImportWorker, ProjectPortableExportWorker,
    ProjectTransferCancellation, VerifiedPortableProjectStream, Workspace,
};
struct Objects(BTreeMap<String, Vec<u8>>);
impl PortableObjectSource for Objects {
    fn verify_transport(&mut self) -> std::io::Result<()> {
        Ok(())
    }
    fn visit_objects(
        &mut self,
        visitor: &mut dyn FnMut(PortableObjectReference) -> studio_core::Result<()>,
    ) -> studio_core::Result<()> {
        for (sha, bytes) in &self.0 {
            visitor(PortableObjectReference::new(sha, bytes.len() as u64)?)?;
        }
        Ok(())
    }
    fn open<'a>(
        &'a mut self,
        object: &PortableObjectReference,
    ) -> std::io::Result<Box<dyn Read + 'a>> {
        let bytes = self
            .0
            .get(object.sha256())
            .ok_or_else(|| std::io::Error::other("Missing synthetic object"))?;
        Ok(Box::new(Cursor::new(bytes.as_slice())))
    }
}
fn preflight(
    project: &PreparedPortableProject,
    root: Option<Vec<u8>>,
) -> VerifiedPortableProjectStream {
    let mut objects = BTreeMap::new();
    project
        .visit_objects(&mut |reference| {
            let mut bytes = Vec::new();
            project.copy_object(
                reference,
                &mut bytes,
                &ProjectTransferCancellation::default(),
            )?;
            objects.insert(reference.sha256().into(), bytes);
            Ok(())
        })
        .unwrap();
    VerifiedPortableProjectStream::preflight(
        root.as_deref().unwrap_or(project.root_bytes()),
        &mut Objects(objects),
        &ProjectTransferCancellation::default(),
    )
    .unwrap()
}
fn rich(text: &str) -> Value {
    let mut rich = RichTree::plain("p", text).unwrap().to_json().unwrap();
    rich["attrs"]["vendor"] = json!({"keep":[1,null,true]});
    rich
}
fn graph(text: &str) -> Value {
    json!({"nodes":[{"id":"card","type":"text","x":0,"y":0,"width":240,"height":160,"text":text,"vendor":{"keep":true}}],"edges":[],"x-writers-studio":{"schemaVersion":2,"vendor":{"keep":true}}})
}
fn setup(database: &std::path::Path) -> (Workspace, String, String, String, String, Vec<u8>) {
    let workspace = Workspace::open(database).unwrap();
    let project = workspace.create_project("Canonical 作者 😀").unwrap();
    let document = workspace
        .create_rich_document(&project.id, "Writing", rich("echo 😀 echo"))
        .unwrap();
    let anchor = workspace
        .capture_annotation_anchor(
            &document.id,
            1,
            "p",
            ScalarOffset(7),
            ScalarOffset(11),
            "echo",
        )
        .unwrap();
    let note = workspace
        .create_annotation(&document.id, 1, anchor, "Remember the second occurrence")
        .unwrap();
    workspace
        .apply_rich_transaction(
            &document.id,
            Transaction {
                base_revision: Revision(1),
                commands: vec![Command::ReplaceText {
                    range: TextRange {
                        block_id: "p".into(),
                        start: ScalarOffset(7),
                        end: ScalarOffset(11),
                    },
                    expected: "echo".into(),
                    replacement: "changed".into(),
                }],
            },
        )
        .unwrap();
    workspace
        .update_annotation(
            &note.id,
            2,
            2,
            studio_core::AnnotationPatch {
                note: None,
                status: Some(AnnotationStatus::NeedsAttention),
            },
        )
        .unwrap();
    let board = workspace
        .create_project_canvas(&project.id, "Board", graph("First"))
        .unwrap();
    workspace
        .save_project_canvas(&project.id, &board.canvas.id, 1, graph("Saved"))
        .unwrap();
    workspace
        .save_project_canvas_draft(
            &project.id,
            &board.canvas.id,
            2,
            None,
            graph("Unfinished graph"),
        )
        .unwrap();
    workspace
        .stage_rich_proposal(
            &project.id,
            &document.id,
            Transaction {
                base_revision: Revision(2),
                commands: vec![Command::ReplaceText {
                    range: TextRange {
                        block_id: "p".into(),
                        start: ScalarOffset(0),
                        end: ScalarOffset(4),
                    },
                    expected: "echo".into(),
                    replacement: "Proposed".into(),
                }],
            },
            "Synthetic staged change",
        )
        .unwrap();
    workspace
        .save_rich_draft(&document.id, 2, None, rich("Unfinished writing"))
        .unwrap();
    let mut image_bytes = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image::ImageBuffer::from_pixel(
        2,
        2,
        image::Rgba([20, 40, 80, 255]),
    ))
    .write_to(&mut image_bytes, image::ImageFormat::Png)
    .unwrap();
    workspace
        .import_image(
            &project.id,
            "synthetic.png",
            Some("image/png"),
            image_bytes.get_ref(),
        )
        .unwrap();
    (
        workspace,
        project.id,
        document.id,
        note.id,
        board.canvas.id,
        image_bytes.into_inner(),
    )
}
#[test]
fn native_saved_notes_history_canvas_and_artifacts_should_restore_with_fresh_handles_and_quarantined_work()
 {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("source.sqlite");
    let (source, project, old_document, old_note, board, image_bytes) = setup(&database);
    let old_image = source
        .project_export_snapshot(&project)
        .unwrap()
        .attachments()[0]
        .logical_id
        .clone();
    let mut exporter = ProjectPortableExportWorker::open(&database).unwrap();
    let prepared = exporter
        .prepare(&project, &ProjectTransferCancellation::default())
        .unwrap();
    let stream = preflight(&prepared, None);
    let target = directory.path().join("target.sqlite");
    let mut importer = ProjectImportWorker::open(&target).unwrap();
    let imported = importer
        .restore_stream(&stream, &ProjectTransferCancellation::default())
        .unwrap();
    let document = importer
        .workspace()
        .resolve_project_resource(&imported.project_id, "document", &old_document)
        .unwrap();
    assert_ne!(document, old_document);
    assert_eq!(
        importer.workspace().document(&document).unwrap().body,
        "echo 😀 changed"
    );
    assert_eq!(
        importer
            .workspace()
            .rich_revisions(&document)
            .unwrap()
            .len(),
        2
    );
    let notes = importer.workspace().annotations(&document).unwrap();
    assert_eq!(notes.len(), 1);
    assert_ne!(notes[0].id, old_note);
    assert_eq!(notes[0].anchor.exact_text, "changed");
    assert_eq!(notes[0].anchor.start, ScalarOffset(7));
    assert_eq!(notes[0].anchor.end, ScalarOffset(14));
    assert_eq!(notes[0].anchor_status, AnchorStatus::Attached);
    assert_eq!(notes[0].status, AnnotationStatus::NeedsAttention);
    assert_eq!(
        importer
            .workspace()
            .annotation_history(&notes[0].id)
            .unwrap()
            .len(),
        3
    );
    let canvas = importer
        .workspace()
        .project_canvas(&imported.project_id, &board)
        .unwrap();
    assert_eq!(canvas.revision.content, graph("Saved"));
    assert_eq!(canvas.canvas.current_revision, 2);
    assert!(canvas.draft.is_none());
    assert!(
        importer
            .workspace()
            .rich_draft(&document)
            .unwrap()
            .is_none()
    );
    assert!(
        importer
            .workspace()
            .rich_proposals(&imported.project_id, &document, None, 10)
            .unwrap()
            .is_empty()
    );
    let (descriptor, source) = importer
        .workspace()
        .project_media_source(&imported.project_id, &old_image)
        .unwrap();
    assert_eq!(
        source
            .load(
                &descriptor.logical_id,
                &ProjectTransferCancellation::default()
            )
            .unwrap(),
        image_bytes
    );
    let evidence = importer
        .workspace()
        .project_evidence_references(&imported.project_id, &Default::default())
        .unwrap();
    assert!(
        evidence
            .iter()
            .any(|reference| reference.domain == "portable.record.richDraftEvidence")
    );
    assert!(
        evidence
            .iter()
            .any(|reference| reference.domain == "portable.record.richReviewEvidence")
    );
    let snapshot = importer
        .workspace()
        .project_export_snapshot(&imported.project_id)
        .unwrap();
    assert!(snapshot.has_retained_evidence());
    assert!(snapshot.canonical_manifest_bytes().is_err());
    drop(importer);
    let reopened = Workspace::open(target).unwrap();
    assert_eq!(
        reopened.document(&document).unwrap().body,
        "echo 😀 changed"
    );
}
#[test]
fn repeated_restore_should_keep_logical_ids_independent_projects_and_flat_evidence_counts() {
    let directory = tempfile::tempdir().unwrap();
    let source_db = directory.path().join("source.sqlite");
    let (_, project, old_document, _, board, _) = setup(&source_db);
    let mut exporter = ProjectPortableExportWorker::open(&source_db).unwrap();
    let prepared = exporter
        .prepare(&project, &ProjectTransferCancellation::default())
        .unwrap();
    let mut root: Value = serde_json::from_slice(prepared.root_bytes()).unwrap();
    root["futureRoot"] = json!({"keep":[true,null,"opaque"]});
    let stream = preflight(&prepared, Some(serde_json::to_vec(&root).unwrap()));
    let database = directory.path().join("copies.sqlite");
    let mut importer = ProjectImportWorker::open(&database).unwrap();
    let first = importer
        .restore_stream(&stream, &ProjectTransferCancellation::default())
        .unwrap();
    let second = importer
        .restore_stream(&stream, &ProjectTransferCancellation::default())
        .unwrap();
    assert_ne!(first.project_id, second.project_id);
    let a = importer
        .workspace()
        .resolve_project_resource(&first.project_id, "document", &old_document)
        .unwrap();
    let b = importer
        .workspace()
        .resolve_project_resource(&second.project_id, "document", &old_document)
        .unwrap();
    assert_ne!(a, b);
    let first_count = importer
        .workspace()
        .project_evidence_references(&first.project_id, &Default::default())
        .unwrap()
        .len();
    let mut exporter = ProjectPortableExportWorker::open(&database).unwrap();
    let prepared_again = exporter
        .prepare(&first.project_id, &ProjectTransferCancellation::default())
        .unwrap();
    let stream_again = preflight(&prepared_again, None);
    let third = importer
        .restore_stream(&stream_again, &ProjectTransferCancellation::default())
        .unwrap();
    assert_eq!(
        importer
            .workspace()
            .project_evidence_references(&third.project_id, &Default::default())
            .unwrap()
            .len(),
        first_count
    );
    assert_eq!(
        importer
            .workspace()
            .project_canvases(&third.project_id)
            .unwrap()[0]
            .id,
        board
    );
    assert!(
        importer
            .workspace()
            .resolve_project_resource(&third.project_id, "document", &a)
            .is_err()
    );
    assert!(
        importer
            .workspace()
            .project_evidence_references(&third.project_id, &Default::default())
            .unwrap()
            .iter()
            .all(|reference| !reference.domain.contains("nested"))
    );
}
#[test]
fn cancelled_or_semantically_foreign_records_should_never_publish_partial_projects() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("source.sqlite");
    let (_, project, _, _, _, _) = setup(&database);
    let mut exporter = ProjectPortableExportWorker::open(&database).unwrap();
    let prepared = exporter
        .prepare(&project, &ProjectTransferCancellation::default())
        .unwrap();
    let stream = preflight(&prepared, None);
    let mut importer = ProjectImportWorker::memory().unwrap();
    importer
        .workspace()
        .create_project("Keep existing")
        .unwrap();
    let cancel = ProjectTransferCancellation::default();
    cancel.cancel();
    assert!(importer.restore_stream(&stream, &cancel).is_err());
    assert_eq!(importer.workspace().list_projects().unwrap().len(), 1);
    let mut root: Value = serde_json::from_slice(prepared.root_bytes()).unwrap();
    root["components"]
        .as_array_mut()
        .unwrap()
        .retain(|component| component["domain"] != "nodes");
    let incomplete = preflight(&prepared, Some(serde_json::to_vec(&root).unwrap()));
    assert!(
        importer
            .restore_stream(&incomplete, &ProjectTransferCancellation::default())
            .is_err()
    );
    assert_eq!(importer.workspace().list_projects().unwrap().len(), 1);
}
struct NoAssets;
impl studio_core::ImportAssetSource for NoAssets {
    fn open<'a>(
        &'a mut self,
        _: &studio_core::ProjectAttachment,
    ) -> std::io::Result<Box<dyn Read + 'a>> {
        Err(std::io::Error::other("This source fixture has no assets"))
    }
}
fn legacy_source(note: &str) -> String {
    let content = rich("echo");
    let schema = content["attrs"]["schemaVersion"].as_u64().unwrap_or(1);
    let manifest = json!({"format":"writers-studio-project","formatVersion":8,
        "project":{"id":"legacy-project","title":"Legacy source","kind":"custom","language":"en","structureRevision":1,"structure":{"schemaVersion":1,"template":"custom","rootLabel":"Writing","nodeTypes":[{"key":"folder","label":"Folder","canBeRoot":true,"contentBearing":false,"allowedChildren":["document"]},{"key":"document","label":"Chapter","canBeRoot":false,"contentBearing":true,"allowedChildren":[]}]}},
        "nodes":[{"id":"folder","kind":"folder","title":"Part","status":"draft","position":0,"currentRevision":0},{"id":"doc","parentId":"folder","kind":"document","title":"Chapter","status":"editing","position":1,"currentRevision":1}],
        "documentRevisions":[{"id":"revision","nodeId":"doc","revision":1,"baseRevision":0,"schemaVersion":schema,"content":content,"plainText":"echo","source":"manual"}],
        "documentAnnotations":[{"id":"source-note","nodeId":"doc","kind":"note","status":"open","anchorStatus":"attached","anchor":{"blockId":"p","start":0,"end":4,"exactText":"echo","prefix":"","suffix":""},"note":note,"baseRevision":1,"revision":1}],
        "namedVersions":[],"dictionaryTerms":[],"dictationSessions":[],"transcriptArtifacts":[],"transcriptDrafts":[],"transcriptDraftRevisions":[],"audio":[],"media":[],"canvases":[],"canvasRevisions":[],"projectMemory":[],"projectMemoryRevisions":[],"vendorNumber":"HUGE"});
    format!(
        " \n{}\r\n",
        serde_json::to_string_pretty(&manifest)
            .unwrap()
            .replace("\"HUGE\"", "1.2345678901234567890123456789e+100")
    )
}
#[test]
fn source_original_tree_precision_and_newer_note_baseline_should_survive_native_restore() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("legacy.sqlite");
    let raw = legacy_source("Old source note");
    let source = studio_core::ValidatedProjectImport::parse(raw.as_bytes()).unwrap();
    let mut importer = ProjectImportWorker::open(&database).unwrap();
    let original = importer
        .import(
            &source,
            &mut NoAssets,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    let original_document = importer
        .workspace()
        .resolve_project_resource(&original.project_id, "document", "doc")
        .unwrap();
    let note = importer
        .workspace()
        .annotations(&original_document)
        .unwrap()
        .remove(0);
    importer
        .workspace()
        .update_annotation(
            &note.id,
            1,
            1,
            studio_core::AnnotationPatch {
                note: Some("New native note".into()),
                status: None,
            },
        )
        .unwrap();
    let mut exporter = ProjectPortableExportWorker::open(&database).unwrap();
    let prepared = exporter
        .prepare(
            &original.project_id,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    let stream = preflight(&prepared, None);
    let restored = importer
        .restore_stream(&stream, &ProjectTransferCancellation::default())
        .unwrap();
    let document = importer
        .workspace()
        .resolve_project_resource(&restored.project_id, "document", "doc")
        .unwrap();
    assert_ne!(document, original_document);
    let tree = importer
        .workspace()
        .project_tree(&restored.project_id)
        .unwrap();
    assert_eq!(tree.len(), 2);
    assert_eq!(tree[1].metadata["parentId"], "folder");
    assert_eq!(tree[1].metadata["kind"], "document");
    assert_eq!(
        importer.workspace().annotations(&document).unwrap()[0].note,
        "New native note"
    );
    let snapshot = importer
        .workspace()
        .project_export_snapshot(&restored.project_id)
        .unwrap();
    assert_eq!(snapshot.manifest_bytes(), raw.as_bytes());
    assert!(
        snapshot.requires_canonical_companion(),
        "New note state must not qualify for an old source-only export"
    );
    assert!(!snapshot.has_retained_evidence());
}
#[test]
fn format2_should_preserve_source_24000_character_notes_and_roll_back_24001() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("long.sqlite");
    let raw = legacy_source(&"Я".repeat(studio_core::MAX_PROJECT_ANNOTATION_NOTE_CHARACTERS));
    let parsed = studio_core::ValidatedProjectImport::parse(raw.as_bytes()).unwrap();
    let mut importer = ProjectImportWorker::open(&database).unwrap();
    let source = importer
        .import(
            &parsed,
            &mut NoAssets,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    let mut exporter = ProjectPortableExportWorker::open(&database).unwrap();
    let prepared = exporter
        .prepare(&source.project_id, &ProjectTransferCancellation::default())
        .unwrap();
    let stream = preflight(&prepared, None);
    let restored = importer
        .restore_stream(&stream, &ProjectTransferCancellation::default())
        .unwrap();
    let document = importer
        .workspace()
        .resolve_project_resource(&restored.project_id, "document", "doc")
        .unwrap();
    assert_eq!(
        importer.workspace().annotations(&document).unwrap()[0].note,
        "Я".repeat(24_000)
    );
    let mut objects = BTreeMap::new();
    prepared
        .visit_objects(&mut |reference| {
            let mut bytes = Vec::new();
            prepared.copy_object(
                reference,
                &mut bytes,
                &ProjectTransferCancellation::default(),
            )?;
            objects.insert(reference.sha256().to_owned(), bytes);
            Ok(())
        })
        .unwrap();
    let mut root: Value = serde_json::from_slice(prepared.root_bytes()).unwrap();
    let component = root["components"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|component| component["domain"] == "nativeAnnotations")
        .unwrap();
    let old = component["sha256"].as_str().unwrap().to_owned();
    let mut record: Value = serde_json::from_slice(&objects[&old]).unwrap();
    let state_sha = record["state"]["sha256"].as_str().unwrap();
    let mut state: Value = serde_json::from_slice(&objects[state_sha]).unwrap();
    state["note"] = json!("Я".repeat(studio_core::MAX_PROJECT_ANNOTATION_NOTE_CHARACTERS + 1));
    let state = serde_json::to_vec(&state).unwrap();
    let state_sha = format!("{:x}", sha2::Sha256::digest(&state));
    record["state"] = json!({"sha256":state_sha,"byteSize":state.len()});
    objects.insert(state_sha, state);
    let mut shard = serde_json::to_vec(&record).unwrap();
    shard.push(b'\n');
    let sha = format!("{:x}", sha2::Sha256::digest(&shard));
    component["sha256"] = json!(sha);
    component["byteSize"] = json!(shard.len());
    objects.remove(&old);
    objects.insert(sha, shard);
    let invalid = VerifiedPortableProjectStream::preflight(
        &serde_json::to_vec(&root).unwrap(),
        &mut Objects(objects),
        &ProjectTransferCancellation::default(),
    )
    .unwrap();
    let before = importer.workspace().list_projects().unwrap().len();
    assert!(
        importer
            .restore_stream(&invalid, &ProjectTransferCancellation::default())
            .is_err()
    );
    assert_eq!(importer.workspace().list_projects().unwrap().len(), before);
}
#[test]
fn native_restored_artifacts_should_remain_declared_in_legacy_reading_snapshots() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("image.sqlite");
    let source = Workspace::open(&database).unwrap();
    let project = source.create_project("Image source").unwrap();
    source
        .create_rich_document(&project.id, "Text", rich("Preserved source"))
        .unwrap();
    let mut png = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image::ImageBuffer::from_pixel(
        1,
        1,
        image::Rgba([1, 2, 3, 255]),
    ))
    .write_to(&mut png, image::ImageFormat::Png)
    .unwrap();
    let image = source
        .import_image(
            &project.id,
            "original.png",
            Some("image/png"),
            png.get_ref(),
        )
        .unwrap();
    let mut exporter = ProjectPortableExportWorker::open(&database).unwrap();
    let prepared = exporter
        .prepare(&project.id, &ProjectTransferCancellation::default())
        .unwrap();
    let stream = preflight(&prepared, None);
    let mut importer = ProjectImportWorker::memory().unwrap();
    let imported = importer
        .restore_stream(&stream, &ProjectTransferCancellation::default())
        .unwrap();
    let snapshot = importer
        .workspace()
        .project_export_snapshot(&imported.project_id)
        .unwrap();
    assert_eq!(snapshot.manifest()["media"].as_array().unwrap().len(), 1);
    assert_eq!(snapshot.manifest()["media"][0]["id"], image.id);
    assert_eq!(snapshot.manifest()["media"][0]["sha256"], image.sha256);
    assert_eq!(snapshot.attachments().len(), 1);
    studio_core::ValidatedProjectImport::parse(snapshot.manifest_bytes()).unwrap();
}
