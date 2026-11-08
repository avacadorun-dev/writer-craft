#![expect(
    clippy::unwrap_used,
    reason = "Synthetic portability contracts must fail immediately"
)]
//! Synthetic complete-domain migration contracts; never use owner archives or recordings.
use std::collections::HashMap;
use std::io::{Cursor, Read};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use studio_core::{
    ImportAssetSource, ProjectAttachment, ProjectImportWorker, ProjectTransferCancellation,
    ValidatedProjectImport,
};

struct Assets(HashMap<String, Vec<u8>>);
impl ImportAssetSource for Assets {
    fn open<'a>(
        &'a mut self,
        attachment: &ProjectAttachment,
    ) -> std::io::Result<Box<dyn Read + 'a>> {
        let bytes = self.0.get(&attachment.path).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "synthetic missing bytes")
        })?;
        Ok(Box::new(Cursor::new(bytes.as_slice())))
    }
}
fn text(body: &str) -> Value {
    json!({"type":"doc","attrs":{"schemaVersion":1,"unknownRoot":{"keep":true}},"content":[{"type":"paragraph","attrs":{"blockId":"p","sourceRefs":["source-segment"],"vendor":null},"content":[{"type":"text","text":body,"marks":[{"type":"bold"}]}]}]})
}
fn fixture() -> (Vec<u8>, Assets) {
    let bytes = b"synthetic opaque bytes".to_vec();
    let audio = b"synthetic immutable original audio evidence".to_vec();
    let mut manifest = json!({
        "format":"writers-studio-project","formatVersion":8,"exportedAt":"2026-11-08T00:00:00Z",
        "project":{"id":"source-project","title":"Book Автор 😀","kind":"free","language":"ru","synopsis":"Synthetic only","structureRevision":1,"structure":{"schemaVersion":1,"template":"free","rootLabel":"Manuscript","nodeTypes":[{"key":"folder","label":"Folder","contentBearing":false,"canBeRoot":true,"allowedChildren":["document"]},{"key":"document","label":"Document","contentBearing":true,"canBeRoot":true,"allowedChildren":[]}]},"createdAt":"2026-11-08T00:00:00Z","updatedAt":"2026-11-08T00:00:00Z","vendorDecimal":"HUGE_DECIMAL"},
        "nodes":[{"id":"folder","kind":"folder","title":"Container","status":"draft","position":0,"currentRevision":0,"vendor":{"keep":1}},{"id":"doc","parentId":"folder","kind":"document","title":"é 😀","status":"editing","position":1,"currentRevision":2,"sourceFrontmatter":"---\ntitle: original\n---\n","opaque":null}],
        "documentRevisions":[{"id":"revision-1","nodeId":"doc","revision":1,"baseRevision":0,"schemaVersion":1,"content":text("Original first."),"plainText":"Original first.","source":"manual","createdAt":"2026-11-08T00:00:00Z","opaque":true},{"id":"revision-2","nodeId":"doc","revision":2,"baseRevision":1,"schemaVersion":1,"content":text("First second."),"plainText":"First second.","source":"assistant","createdAt":"2026-11-08T00:01:00Z"}],
        "namedVersions":[{"id":"named","nodeId":"doc","revision":1,"name":"First snapshot","note":"Keep","createdAt":"2026-11-08T00:00:00Z"}],
        "dictionaryTerms":[{"id":"term","canonical":"Second","variants":["second"],"kind":"term","language":"en","pronunciation":"","active":true,"revision":1,"source":"manual"}],
        "documentAnnotations":[{"id":"note","nodeId":"doc","kind":"note","status":"open","anchorStatus":"attached","anchor":{"blockId":"p","start":6,"end":12,"exactText":"second","prefix":"First ","suffix":"."},"note":"Synthetic note","targetSnapshot":{},"baseRevision":2,"revision":1,"vendor":{"keep":true}},{"id":"dictionary-link","nodeId":"doc","kind":"dictionary_link","dictionaryTermId":"term","status":"open","anchor":{"blockId":"p","start":6,"end":12,"exactText":"second","prefix":"First ","suffix":"."},"targetSnapshot":{"canonical":"Second"},"baseRevision":2,"revision":1}],
        "dictationSessions":[{"id":"session","destinationNodeId":"doc","language":"ru","status":"review_ready","durationMs":500,"recordedDurationMs":500,"manifest":{"segments":["source-segment"]},"commandsEnabled":true,"commandPrefix":"command"}],
        "transcriptArtifacts":[{"id":"artifact","sessionId":"session","kind":"batch_source","text":"Synthetic source","segments":[{"id":"source-segment","text":"Synthetic source","start":0,"end":500}],"provider":"synthetic","model":"fixture","pipelineVersion":"1","language":"ru"}],
        "transcriptDrafts":[{"id":"draft","sessionId":"session","sourceArtifactId":"artifact","revision":1,"baseSourceText":"Synthetic source","content":text("Synthetic source"),"plainText":"Synthetic source","manuallyEdited":false}],
        "transcriptDraftRevisions":[{"id":"draft-rev","sessionId":"session","revision":1,"content":text("Synthetic source"),"plainText":"Synthetic source","source":"manual"}],
        "audio":[{"sessionId":"session","path":"audio/source.webm","sha256":format!("{:x}",Sha256::digest(&audio)),"byteSize":audio.len(),"durationMs":500,"mediaType":"audio/webm"}],
        "media":[{"id":"asset","path":"media/asset.bin","sha256":format!("{:x}",Sha256::digest(&bytes)),"byteSize":bytes.len(),"mediaType":"application/x-synthetic","originalName":"asset 😀.bin","sourcePath":"Materials/asset.bin","width":null,"height":null,"opaque":{"keep":true}}],
        "canvases":[{"id":"board","title":"Board","position":0,"currentRevision":1,"vendor":{"keep":true}}],
        "canvasRevisions":[{"id":"canvas-rev","canvasId":"board","revision":1,"baseRevision":0,"schemaVersion":2,"source":"manual","createdAt":"2026-11-08T00:00:00Z","content":{"nodes":[{"id":"text-card","type":"text","x":0,"y":0,"width":200,"height":120,"text":"Card","color":"2","x-writers-studio":{"card":{"typeId":"card-type","fields":{"document":"doc"}},"vendor":null}},{"id":"file-card","type":"file","x":220,"y":0,"width":200,"height":120,"file":"writers-studio://media/asset","x-writers-studio":{"resource":{"kind":"media","id":"asset"}}}],"edges":[{"id":"edge","fromNode":"text-card","toNode":"file-card","label":"Relation","opaque":true}],"x-writers-studio":{"schemaVersion":2,"activeView":"mind-map","opaque":null}}}],
        "canvasSchema":{"currentRevision":1,"revisions":[{"revision":1,"baseRevision":0,"source":"manual","schema":{"revision":1,"cardTypes":[{"id":"card-type","name":"Card","category":"custom","color":"2","icon":"file","fields":[{"id":"document","name":"Document","type":"document"}]}],"relationTypes":[]}}]},
        "assistantGuide":{"currentRevision":1,"active":true,"revisions":[{"id":"guide-1","revision":1,"content":"Synthetic guide","source":"manual","opaque":true}]},
        "projectMemory":[{"id":"memory","kind":"fact","statement":"Synthetic fact","scope":"node","scopeNodeId":"doc","sourceReferences":[{"kind":"document","id":"doc"}],"tags":["fixture"],"active":true,"alwaysRelevant":false,"revision":1,"source":"manual"}],
        "projectMemoryRevisions":[{"id":"memory-rev","memoryId":"memory","revision":1,"snapshot":{"id":"memory","kind":"fact","statement":"Synthetic fact","scope":"node","scopeNodeId":"doc"},"source":"manual"}],
        "vendorInteger":"HUGE_INTEGER","vendorUnknown":{"keep":[true,null,"original"]},
        "externalRecoveryCheckpoint":{"documentId":"doc","checkpointEvidence":{"forged":true}}
    });
    manifest["documentRevisions"][1]["content"]["attrs"]["precise"] = "HUGE_INTEGER".into();
    let raw = serde_json::to_string_pretty(&manifest)
        .unwrap()
        .replace("\"HUGE_INTEGER\"", "18446744073709551616000123")
        .replace("\"HUGE_DECIMAL\"", "1.2345678901234567890123456789e+100");
    (
        raw.into_bytes(),
        Assets(HashMap::from([
            ("media/asset.bin".into(), bytes),
            ("audio/source.webm".into(), audio),
        ])),
    )
}

#[test]
fn complete_project_should_preserve_exact_source_and_all_artifacts_in_independent_namespaces() {
    let (raw, mut assets) = fixture();
    let validated = ValidatedProjectImport::parse(&raw).unwrap();
    let mut worker = ProjectImportWorker::memory().unwrap();
    let cancel = ProjectTransferCancellation::default();
    let first = worker.import(&validated, &mut assets, &cancel).unwrap();
    let second = worker.import(&validated, &mut assets, &cancel).unwrap();
    assert_ne!(first.project_id, second.project_id);
    let snapshot = worker
        .workspace()
        .project_export_snapshot(&first.project_id)
        .unwrap();
    assert_eq!(snapshot.manifest_bytes(), raw);
    assert_eq!(snapshot.attachments().len(), 2);
    assert_eq!(
        snapshot.assets().load("asset", &cancel).unwrap(),
        b"synthetic opaque bytes"
    );
    let a = worker
        .workspace()
        .resolve_project_resource(&first.project_id, "node", "doc")
        .unwrap();
    let b = worker
        .workspace()
        .resolve_project_resource(&second.project_id, "node", "doc")
        .unwrap();
    assert_ne!(a, b);
    assert_eq!(worker.workspace().rich_revisions(&a).unwrap().len(), 2);
    assert_eq!(
        worker.workspace().annotations(&a).unwrap()[0].note,
        "Synthetic note"
    );
    assert!(worker.workspace().rich_draft(&a).unwrap().is_none());
}

#[test]
fn known_rich_edit_should_preserve_large_numbers_raw_unknown_fields_and_original_history() {
    let (raw, mut assets) = fixture();
    let validated = ValidatedProjectImport::parse(&raw).unwrap();
    let mut worker = ProjectImportWorker::memory().unwrap();
    let imported = worker
        .import(
            &validated,
            &mut assets,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    let native = worker
        .workspace()
        .resolve_project_resource(&imported.project_id, "node", "doc")
        .unwrap();
    let mut current = worker
        .workspace()
        .document(&native)
        .unwrap()
        .rich_json
        .unwrap();
    current["content"][0]["content"][0]["text"] = "First changed.".into();
    worker
        .workspace()
        .save_rich_document(&native, 2, current)
        .unwrap();
    let snapshot = worker
        .workspace()
        .project_export_snapshot(&imported.project_id)
        .unwrap();
    let encoded = std::str::from_utf8(snapshot.manifest_bytes()).unwrap();
    assert!(encoded.contains("18446744073709551616000123"));
    assert!(encoded.contains("1.2345678901234567890123456789e+100"));
    assert_eq!(
        snapshot.manifest()["documentRevisions"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        snapshot.manifest()["documentRevisions"][1],
        validated.manifest()["documentRevisions"][1]
    );
    ValidatedProjectImport::parse(snapshot.manifest_bytes()).unwrap();
}

#[test]
fn malformed_reference_or_duplicate_key_should_fail_before_any_project_is_created() {
    let (raw, _) = fixture();
    let duplicate = std::str::from_utf8(&raw).unwrap().replace(
        "\"vendorUnknown\": {",
        "\"vendorUnknown\": {\"duplicate\":1,\"duplicate\":2,",
    );
    assert!(ValidatedProjectImport::parse(duplicate.as_bytes()).is_err());
    let mut manifest: Value = serde_json::from_slice(&raw).unwrap();
    manifest["canvasRevisions"][0]["content"]["nodes"][1]["x-writers-studio"]["resource"]["id"] =
        "foreign".into();
    assert!(ValidatedProjectImport::parse(&serde_json::to_vec(&manifest).unwrap()).is_err());
}

#[test]
fn corrupt_or_cancelled_attachment_should_roll_back_all_rows() {
    let (raw, mut assets) = fixture();
    let validated = ValidatedProjectImport::parse(&raw).unwrap();
    assets.0.insert("media/asset.bin".into(), b"bad".to_vec());
    let mut worker = ProjectImportWorker::memory().unwrap();
    assert!(
        worker
            .import(
                &validated,
                &mut assets,
                &ProjectTransferCancellation::default()
            )
            .is_err()
    );
    assert!(worker.workspace().list_projects().unwrap().is_empty());
    let cancel = ProjectTransferCancellation::default();
    cancel.cancel();
    assert!(worker.import(&validated, &mut assets, &cancel).is_err());
    assert!(worker.workspace().list_projects().unwrap().is_empty());
}

#[test]
fn external_checkpoint_should_remain_quarantined_and_never_become_trusted_recovery() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite");
    let (raw, mut assets) = fixture();
    let validated = ValidatedProjectImport::parse(&raw).unwrap();
    let mut worker = ProjectImportWorker::open(&path).unwrap();
    let imported = worker
        .import(
            &validated,
            &mut assets,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    drop(worker);
    let db = rusqlite::Connection::open(path).unwrap();
    let trusted: i64 = db
        .query_row("SELECT count(*) FROM rich_draft_annotations", [], |row| {
            row.get(0)
        })
        .unwrap();
    let quarantined: i64 = db
        .query_row(
            "SELECT count(*) FROM project_quarantined_recovery WHERE project_id=?1",
            [&imported.project_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!((trusted, quarantined), (0, 1));
}

#[test]
fn legacy_utf16_bug_note_should_stay_readable_detached_without_invented_native_coordinates() {
    let (raw, mut assets) = fixture();
    let mut manifest: Value = serde_json::from_slice(&raw).unwrap();
    manifest["documentRevisions"][1]["content"] = text("😀 second.");
    manifest["documentRevisions"][1]["plainText"] = "😀 second.".into();
    manifest["documentAnnotations"] = json!([{"id":"utf16-note","nodeId":"doc","kind":"note","status":"resolved","anchorStatus":"attached","anchor":{"blockId":"p","start":3,"end":9,"exactText":"second","prefix":"😀 ","suffix":"."},"note":"Keep exact legacy note","targetSnapshot":{},"baseRevision":2,"revision":1,"vendor":{"preserve":true}}]);
    let validated = ValidatedProjectImport::parse(&serde_json::to_vec(&manifest).unwrap()).unwrap();
    let mut worker = ProjectImportWorker::memory().unwrap();
    let imported = worker
        .import(
            &validated,
            &mut assets,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    let projections = worker
        .workspace()
        .imported_project_annotations(&imported.project_id)
        .unwrap();
    assert_eq!(
        projections[0].anchor_status,
        studio_core::AnchorStatus::Detached
    );
    assert!(projections[0].native_anchor.is_none());
    assert!(projections[0].native_annotation_id.is_none());
    assert_eq!(projections[0].metadata["status"], "resolved");
    assert_eq!(projections[0].metadata["note"], "Keep exact legacy note");
    assert_eq!(
        worker
            .workspace()
            .project_export_snapshot(&imported.project_id)
            .unwrap()
            .manifest(),
        &manifest
    );
}

#[test]
fn midstream_cancellation_should_rollback_without_exposing_a_partial_project() {
    struct CancelReader<'a> {
        bytes: Cursor<&'a [u8]>,
        cancel: ProjectTransferCancellation,
    }
    impl Read for CancelReader<'_> {
        fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
            let count = output.len().min(4);
            let read = self.bytes.read(&mut output[..count])?;
            if read > 0 {
                self.cancel.cancel();
            }
            Ok(read)
        }
    }
    struct CancelAssets {
        assets: Assets,
        cancel: ProjectTransferCancellation,
    }
    impl ImportAssetSource for CancelAssets {
        fn open<'a>(
            &'a mut self,
            attachment: &ProjectAttachment,
        ) -> std::io::Result<Box<dyn Read + 'a>> {
            let bytes = self
                .assets
                .0
                .get(&attachment.path)
                .ok_or_else(|| std::io::Error::other("missing synthetic bytes"))?;
            Ok(Box::new(CancelReader {
                bytes: Cursor::new(bytes.as_slice()),
                cancel: self.cancel.clone(),
            }))
        }
    }
    let (raw, assets) = fixture();
    let validated = ValidatedProjectImport::parse(&raw).unwrap();
    let cancel = ProjectTransferCancellation::default();
    let mut source = CancelAssets {
        assets,
        cancel: cancel.clone(),
    };
    let mut worker = ProjectImportWorker::memory().unwrap();
    assert!(worker.import(&validated, &mut source, &cancel).is_err());
    assert!(worker.workspace().list_projects().unwrap().is_empty());
}

#[test]
fn dedicated_import_connection_should_not_block_wal_reads_or_show_uncommitted_rows() {
    use std::sync::mpsc;
    use std::time::Duration;
    struct WaitingReader<'a> {
        bytes: Cursor<&'a [u8]>,
        entered: Option<mpsc::Sender<()>>,
        release: Option<mpsc::Receiver<()>>,
    }
    impl Read for WaitingReader<'_> {
        fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
            if let Some(entered) = self.entered.take() {
                entered.send(()).map_err(std::io::Error::other)?;
            }
            if let Some(release) = self.release.take() {
                release.recv().map_err(std::io::Error::other)?;
            }
            self.bytes.read(output)
        }
    }
    struct WaitingAssets {
        assets: Assets,
        entered: Option<mpsc::Sender<()>>,
        release: Option<mpsc::Receiver<()>>,
    }
    impl ImportAssetSource for WaitingAssets {
        fn open<'a>(
            &'a mut self,
            attachment: &ProjectAttachment,
        ) -> std::io::Result<Box<dyn Read + 'a>> {
            let bytes = self
                .assets
                .0
                .get(&attachment.path)
                .ok_or_else(|| std::io::Error::other("missing synthetic bytes"))?;
            Ok(Box::new(WaitingReader {
                bytes: Cursor::new(bytes.as_slice()),
                entered: self.entered.take(),
                release: self.release.take(),
            }))
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite");
    let main = studio_core::Workspace::open(&path).unwrap();
    let (raw, assets) = fixture();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let import_path = path.clone();
    let import = std::thread::spawn(move || {
        let validated = ValidatedProjectImport::parse(&raw).unwrap();
        let mut worker = ProjectImportWorker::open(import_path).unwrap();
        let mut source = WaitingAssets {
            assets,
            entered: Some(entered_tx),
            release: Some(release_rx),
        };
        worker.import(
            &validated,
            &mut source,
            &ProjectTransferCancellation::default(),
        )
    });
    entered_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let (read_tx, read_rx) = mpsc::channel();
    let read = std::thread::spawn(move || read_tx.send(main.list_projects()).unwrap());
    let result = read_rx.recv_timeout(Duration::from_secs(2));
    release_tx.send(()).unwrap();
    assert!(result.unwrap().unwrap().is_empty());
    read.join().unwrap();
    import.join().unwrap().unwrap();
}

#[test]
fn changed_native_note_and_tombstone_should_require_and_survive_the_canonical_companion() {
    let (raw, mut assets) = fixture();
    let validated = ValidatedProjectImport::parse(&raw).unwrap();
    let mut worker = ProjectImportWorker::memory().unwrap();
    let imported = worker
        .import(
            &validated,
            &mut assets,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    let initial = worker
        .workspace()
        .project_export_snapshot(&imported.project_id)
        .unwrap();
    assert!(!initial.requires_canonical_companion());
    let note = worker
        .workspace()
        .resolve_project_resource(&imported.project_id, "annotation", "note")
        .unwrap();
    worker
        .workspace()
        .update_annotation(
            &note,
            2,
            1,
            studio_core::AnnotationPatch {
                note: Some("New native note".into()),
                status: None,
            },
        )
        .unwrap();
    let updated = worker
        .workspace()
        .project_export_snapshot(&imported.project_id)
        .unwrap();
    assert!(updated.requires_canonical_companion());
    let canonical: Value =
        serde_json::from_slice(&updated.canonical_manifest_bytes().unwrap()).unwrap();
    assert_eq!(
        canonical["nativeState"]["annotations"][0]["state"]["note"],
        "New native note"
    );
    let native_document = worker
        .workspace()
        .resolve_project_resource(&imported.project_id, "node", "doc")
        .unwrap();
    assert_eq!(
        canonical["nativeState"]["documentAliases"][&native_document],
        "doc"
    );
    assert_eq!(
        canonical["nativeState"]["annotationHistory"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        canonical["sourceManifest"]["documentAnnotations"][0]["note"],
        "Synthetic note"
    );
    let initial_canonical: Value =
        serde_json::from_slice(&initial.canonical_manifest_bytes().unwrap()).unwrap();
    assert_eq!(
        initial_canonical["nativeState"]["annotations"][0]["state"]["note"],
        "Synthetic note"
    );
    worker.workspace().delete_annotation(&note, 2, 2).unwrap();
    let deleted = worker
        .workspace()
        .project_export_snapshot(&imported.project_id)
        .unwrap();
    let canonical: Value =
        serde_json::from_slice(&deleted.canonical_manifest_bytes().unwrap()).unwrap();
    assert_eq!(
        canonical["nativeState"]["annotations"][0]["state"]["deleted"],
        true
    );
}
