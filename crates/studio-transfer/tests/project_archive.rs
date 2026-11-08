#![expect(
    clippy::unwrap_used,
    reason = "Synthetic portability contracts must fail immediately"
)]
//! Complete project-transfer tests use synthetic text and byte resources only.
use std::io::{Cursor, Write};
use std::path::Path;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use studio_core::{ProjectImportWorker, ProjectTransferCancellation};
use studio_transfer::{ProjectArchive, write_project_archive};
use zip::write::SimpleFileOptions;

fn fixture(version: u64) -> (Value, Vec<u8>) {
    let bytes = b"synthetic original artifact".to_vec();
    let content = json!({"type":"doc","attrs":{"schemaVersion":1,"opaque":true},"content":[{"type":"paragraph","attrs":{"blockId":"p","vendor":null},"content":[{"type":"text","text":"Автор 😀 synthetic"}]}]});
    let manifest = json!({"format":"writers-studio-project","formatVersion":version,"project":{"id":"source","title":"Synthetic project","kind":"free","language":"ru","structureRevision":1,"structure":{"schemaVersion":1,"template":"free","rootLabel":"Manuscript","nodeTypes":[{"key":"document","label":"Document","contentBearing":true,"canBeRoot":true,"allowedChildren":[]}]}},"nodes":[{"id":"document","kind":"document","title":"Chapter","status":"draft","position":0,"currentRevision":1}],"documentRevisions":[{"id":"revision","nodeId":"document","revision":1,"baseRevision":0,"schemaVersion":1,"content":content,"plainText":"Автор 😀 synthetic","source":"manual","vendor":{"keep":true}}],"namedVersions":[],"dictionaryTerms":[],"documentAnnotations":[],"dictationSessions":[],"transcriptArtifacts":[],"transcriptDrafts":[],"transcriptDraftRevisions":[],"audio":[],"media":if version>=3 {json!([{"id":"asset","path":"media/original.bin","sha256":format!("{:x}",Sha256::digest(&bytes)),"byteSize":bytes.len(),"mediaType":"application/x-fixture","originalName":"original.bin","width":null,"height":null}])}else{json!([])},"canvases":[],"canvasRevisions":[],"projectMemory":[],"projectMemoryRevisions":[],"opaqueUnknown":{"keep":true}});
    (manifest, bytes)
}
fn archive(path: &Path, manifest: &Value, bytes: &[u8], extra: bool) {
    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file("project.json", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(&serde_json::to_vec(manifest).unwrap())
        .unwrap();
    if !manifest["media"].as_array().unwrap().is_empty() {
        zip.start_file("media/original.bin", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes).unwrap();
    }
    if extra {
        zip.start_file("undeclared.bin", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"synthetic").unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn supported_envelopes_should_import_and_roundtrip_exact_semantics_and_original_bytes() {
    let directory = tempfile::tempdir().unwrap();
    for version in 1..=8 {
        let (manifest, bytes) = fixture(version);
        let source = directory.path().join(format!("source-{version}.wstudio"));
        archive(&source, &manifest, &bytes, false);
        let mut opened = ProjectArchive::open(&source).unwrap();
        let cancel = ProjectTransferCancellation::default();
        opened.verify_integrity(&cancel).unwrap();
        let mut worker = ProjectImportWorker::memory().unwrap();
        let result = opened.import(&mut worker, &cancel).unwrap();
        let snapshot = worker
            .workspace()
            .project_export_snapshot(&result.project_id)
            .unwrap();
        assert_eq!(snapshot.manifest(), &manifest);
        let mut output = Cursor::new(Vec::new());
        write_project_archive(&snapshot, &mut output, &cancel).unwrap();
        let exported = directory
            .path()
            .join(format!("roundtrip-{version}.wstudio"));
        std::fs::write(&exported, output.into_inner()).unwrap();
        let mut reopened = ProjectArchive::open(exported).unwrap();
        reopened.verify_integrity(&cancel).unwrap();
        assert_eq!(reopened.project().manifest(), &manifest);
    }
}

#[test]
fn source_path_replacement_should_not_change_the_already_opened_archive_identity() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.wstudio");
    let moved = directory.path().join("original-renamed.wstudio");
    let (manifest, bytes) = fixture(8);
    archive(&source, &manifest, &bytes, false);
    let mut opened = ProjectArchive::open(&source).unwrap();
    std::fs::rename(&source, moved).unwrap();
    std::fs::write(&source, b"unrelated replacement file").unwrap();
    let mut worker = ProjectImportWorker::memory().unwrap();
    let imported = opened
        .import(&mut worker, &ProjectTransferCancellation::default())
        .unwrap();
    assert_eq!(
        worker
            .workspace()
            .project_export_snapshot(&imported.project_id)
            .unwrap()
            .manifest(),
        &manifest
    );
    assert_eq!(
        std::fs::read(source).unwrap(),
        b"unrelated replacement file"
    );
}

#[test]
fn valid_zip_with_corrupt_declared_bytes_should_fail_verification_and_rollback_import() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("corrupt.wstudio");
    let (manifest, mut bytes) = fixture(8);
    bytes[0] ^= 1;
    archive(&source, &manifest, &bytes, false);
    let mut opened = ProjectArchive::open(source).unwrap();
    let cancel = ProjectTransferCancellation::default();
    assert!(opened.verify_integrity(&cancel).is_err());
    let mut worker = ProjectImportWorker::memory().unwrap();
    assert!(opened.import(&mut worker, &cancel).is_err());
    assert!(worker.workspace().list_projects().unwrap().is_empty());
}

#[test]
fn undeclared_members_should_fail_without_extraction_or_workspace_mutation() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("extra.wstudio");
    let (manifest, bytes) = fixture(8);
    archive(&source, &manifest, &bytes, true);
    assert!(ProjectArchive::open(&source).is_err());
    assert!(!directory.path().join("undeclared.bin").exists());
}

#[test]
fn legacy_only_writer_should_refuse_opaque_native_state_without_emitting_any_bytes() {
    let worker = ProjectImportWorker::memory().unwrap();
    let project = worker.workspace().create_project("Opaque source").unwrap();
    worker
        .workspace()
        .import_document(
            &project.id,
            "Original",
            "Preview only",
            json!({"futureSchema":99,"keep":{"notes":["synthetic"]}}),
        )
        .unwrap();
    let snapshot = worker
        .workspace()
        .project_export_snapshot(&project.id)
        .unwrap();
    let mut output = Cursor::new(Vec::new());
    assert!(
        write_project_archive(
            &snapshot,
            &mut output,
            &ProjectTransferCancellation::default()
        )
        .is_err()
    );
    assert!(output.get_ref().is_empty());
}

#[test]
fn unchanged_zip_roundtrip_should_preserve_exact_manifest_framing_and_terminal_newline() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("framed.wstudio");
    let (manifest, bytes) = fixture(8);
    let mut framed = b"\r\n\t ".to_vec();
    framed.extend(serde_json::to_vec_pretty(&manifest).unwrap());
    framed.extend_from_slice(b" \n\r\n\t");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    zip.start_file("project.json", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(&framed).unwrap();
    zip.start_file("media/original.bin", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(&bytes).unwrap();
    zip.finish().unwrap();
    let mut archive = ProjectArchive::open(&path).unwrap();
    assert_eq!(archive.project().original_manifest().as_bytes(), framed);
    let mut worker = ProjectImportWorker::memory().unwrap();
    let cancel = ProjectTransferCancellation::default();
    let imported = archive.import(&mut worker, &cancel).unwrap();
    let snapshot = worker
        .workspace()
        .project_export_snapshot(&imported.project_id)
        .unwrap();
    let mut output = Cursor::new(Vec::new());
    write_project_archive(&snapshot, &mut output, &cancel).unwrap();
    output.set_position(0);
    let mut exported = zip::ZipArchive::new(output).unwrap();
    let mut raw = Vec::new();
    std::io::Read::read_to_end(&mut exported.by_name("project.json").unwrap(), &mut raw).unwrap();
    assert_eq!(raw, framed);
}

#[test]
fn guarded_file_publication_should_verify_reopen_and_never_clobber_existing_or_reserved_paths() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let source = directory.path().join("source.wstudio");
    let (manifest, bytes) = fixture(8);
    archive(&source, &manifest, &bytes, false);
    let mut opened = ProjectArchive::open(&source).unwrap();
    let cancel = ProjectTransferCancellation::default();
    let mut worker = ProjectImportWorker::open(&database).unwrap();
    let imported = opened.import(&mut worker, &cancel).unwrap();
    let snapshot = worker
        .workspace()
        .project_export_snapshot(&imported.project_id)
        .unwrap();
    let destination = directory.path().join("published.wstudio");
    let receipt =
        studio_transfer::export_project_archive_file(&snapshot, &database, &destination, &cancel)
            .unwrap();
    let encoded = std::fs::read(&destination).unwrap();
    assert_eq!(receipt.sha256, format!("{:x}", Sha256::digest(&encoded)));
    assert_eq!(receipt.byte_size, u64::try_from(encoded.len()).unwrap());
    ProjectArchive::open(&destination)
        .unwrap()
        .verify_integrity(&cancel)
        .unwrap();
    assert!(
        studio_transfer::export_project_archive_file(&snapshot, &database, &destination, &cancel)
            .is_err()
    );
    assert_eq!(std::fs::read(&destination).unwrap(), encoded);
    for name in [
        "workspace.sqlite",
        "workspace.sqlite-wal",
        "workspace.sqlite-shm",
        "workspace.sqlite-journal",
        "appearance.json",
        "interface.json",
        "editor.json",
    ] {
        let target = directory.path().join(name);
        assert!(
            studio_transfer::export_project_archive_file(&snapshot, &database, &target, &cancel)
                .is_err()
        );
    }
    let cancelled = directory.path().join("cancelled.wstudio");
    cancel.cancel();
    assert!(
        studio_transfer::export_project_archive_file(&snapshot, &database, &cancelled, &cancel)
            .is_err()
    );
    assert!(!cancelled.exists());
    assert!(!std::fs::read_dir(directory.path()).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".writercraft-export-")
    }));
}
