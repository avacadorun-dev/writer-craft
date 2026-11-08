#![expect(
    clippy::unwrap_used,
    reason = "Synthetic canonical-reader contracts fail immediately"
)]
//! Native archive input is pinned, bounded and hash-verified before semantic project activation.
use std::fs::File;
use std::io::{Read, Write};

use craft_editor_model::Document as RichTree;
use studio_core::{
    PreparedPortableProject, ProjectImportWorker, ProjectPortableExportWorker,
    ProjectTransferCancellation, Workspace,
};
use studio_transfer::{CanonicalProjectArchive, export_canonical_project_archive_file};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};
fn prepare(database: &std::path::Path) -> PreparedPortableProject {
    let workspace = Workspace::open(database).unwrap();
    let project = workspace.create_project("Native source").unwrap();
    workspace
        .create_rich_document(
            &project.id,
            "Text",
            RichTree::plain("p", "Native preserved writing")
                .unwrap()
                .to_json()
                .unwrap(),
        )
        .unwrap();
    let mut worker = ProjectPortableExportWorker::open(database).unwrap();
    worker
        .prepare(&project.id, &ProjectTransferCancellation::default())
        .unwrap()
}
fn write(path: &std::path::Path, prepared: &PreparedPortableProject, large: bool) {
    let mut zip = ZipWriter::new(File::create(path).unwrap());
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .large_file(large);
    zip.start_file("project.json", options).unwrap();
    zip.write_all(prepared.root_bytes()).unwrap();
    prepared
        .visit_objects(&mut |object| {
            zip.start_file(format!("objects/{}", object.sha256()), options)
                .unwrap();
            prepared.copy_object(object, &mut zip, &ProjectTransferCancellation::default())
        })
        .unwrap();
    zip.finish().unwrap();
}
#[test]
fn native_file_should_restore_through_verified_objects_and_independent_project_namespaces() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("source.sqlite");
    let prepared = prepare(&database);
    let archive_path = directory.path().join("project.writercraft");
    export_canonical_project_archive_file(
        &prepared,
        &database,
        &archive_path,
        &ProjectTransferCancellation::default(),
    )
    .unwrap();
    let mut archive = CanonicalProjectArchive::open(&archive_path).unwrap();
    assert_eq!(archive.root_bytes(), prepared.root_bytes());
    let mut worker = ProjectImportWorker::memory().unwrap();
    let a = archive
        .restore(&mut worker, &ProjectTransferCancellation::default())
        .unwrap();
    let b = archive
        .restore(&mut worker, &ProjectTransferCancellation::default())
        .unwrap();
    assert_ne!(a.project_id, b.project_id);
    assert_eq!(
        worker.workspace().list_documents(&a.project_id).unwrap()[0].body,
        "Native preserved writing"
    );
    assert_eq!(
        worker.workspace().list_documents(&b.project_id).unwrap()[0].body,
        "Native preserved writing"
    );
}
#[test]
fn renamed_source_path_should_not_replace_the_opened_native_file() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("source.sqlite");
    let prepared = prepare(&database);
    let path = directory.path().join("project.writercraft");
    write(&path, &prepared, false);
    let mut opened = CanonicalProjectArchive::open(&path).unwrap();
    std::fs::rename(&path, directory.path().join("original.writercraft")).unwrap();
    std::fs::write(&path, b"unrelated replacement").unwrap();
    let mut worker = ProjectImportWorker::memory().unwrap();
    let imported = opened
        .restore(&mut worker, &ProjectTransferCancellation::default())
        .unwrap();
    assert_eq!(
        worker
            .workspace()
            .list_documents(&imported.project_id)
            .unwrap()[0]
            .body,
        "Native preserved writing"
    );
    assert_eq!(std::fs::read(path).unwrap(), b"unrelated replacement");
}
#[test]
fn corrupted_payload_with_valid_zip_crc_should_fail_before_workspace_activation() {
    let directory = tempfile::tempdir().unwrap();
    let prepared = prepare(&directory.path().join("source.sqlite"));
    let good = directory.path().join("good.writercraft");
    write(&good, &prepared, false);
    let mut input = ZipArchive::new(File::open(good).unwrap()).unwrap();
    let bad = directory.path().join("bad.writercraft");
    let mut output = ZipWriter::new(File::create(&bad).unwrap());
    let mut changed = false;
    for index in 0..input.len() {
        let mut entry = input.by_index(index).unwrap();
        let name = entry.name().to_owned();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        if !changed && name.starts_with("objects/") && !bytes.is_empty() {
            bytes[0] ^= 1;
            changed = true;
        }
        output
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        output.write_all(&bytes).unwrap();
    }
    output.finish().unwrap();
    assert!(changed);
    let mut archive = CanonicalProjectArchive::open(bad).unwrap();
    let mut worker = ProjectImportWorker::memory().unwrap();
    worker.workspace().create_project("Existing").unwrap();
    assert!(
        archive
            .restore(&mut worker, &ProjectTransferCancellation::default())
            .is_err()
    );
    assert_eq!(worker.workspace().list_projects().unwrap().len(), 1);
}
#[test]
fn forced_zip64_entry_sizes_should_work_and_unsafe_extra_members_should_refuse() {
    let directory = tempfile::tempdir().unwrap();
    let prepared = prepare(&directory.path().join("source.sqlite"));
    let path = directory.path().join("zip64.writercraft");
    write(&path, &prepared, true);
    let mut opened = CanonicalProjectArchive::open(&path).unwrap();
    let mut worker = ProjectImportWorker::memory().unwrap();
    opened
        .restore(&mut worker, &ProjectTransferCancellation::default())
        .unwrap();
    let unsafe_path = directory.path().join("unsafe.writercraft");
    let mut zip = ZipWriter::new(File::create(&unsafe_path).unwrap());
    zip.start_file("project.json", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(prepared.root_bytes()).unwrap();
    zip.start_file("../escape", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"never extracted").unwrap();
    zip.finish().unwrap();
    assert!(CanonicalProjectArchive::open(&unsafe_path).is_err());
    assert!(!directory.path().join("escape").exists());
}
