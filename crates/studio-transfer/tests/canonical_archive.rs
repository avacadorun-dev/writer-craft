#![expect(
    clippy::unwrap_used,
    reason = "Synthetic canonical publication contracts fail immediately"
)]
//! Native canonical format remains separate from legacy `.wstudio` compatibility.
use std::fs::File;
use std::io::Read;

use craft_editor_model::Document as RichTree;
use sha2::{Digest, Sha256};
use studio_core::{
    PortableProjectHeader, ProjectPortableExportWorker, ProjectTransferCancellation, Workspace,
};
use studio_transfer::{ProjectArchive, export_canonical_project_archive_file};
use zip::ZipArchive;

#[test]
fn canonical_zip_should_publish_a_verified_native_root_and_exact_objects_without_replacing_anything()
 {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let workspace = Workspace::open(&database).unwrap();
    let project = workspace.create_project("Portable 😀").unwrap();
    workspace
        .create_rich_document(
            &project.id,
            "Chapter",
            RichTree::plain("p", "Native author data")
                .unwrap()
                .to_json()
                .unwrap(),
        )
        .unwrap();
    let mut worker = ProjectPortableExportWorker::open(&database).unwrap();
    let prepared = worker
        .prepare(&project.id, &ProjectTransferCancellation::default())
        .unwrap();
    let destination = directory.path().join("project.writercraft");
    let receipt = export_canonical_project_archive_file(
        &prepared,
        &database,
        &destination,
        &ProjectTransferCancellation::default(),
    )
    .unwrap();
    let bytes = std::fs::read(&destination).unwrap();
    assert_eq!(receipt.byte_size, bytes.len() as u64);
    assert_eq!(receipt.sha256, format!("{:x}", Sha256::digest(&bytes)));
    let mut archive = ZipArchive::new(File::open(&destination).unwrap()).unwrap();
    assert_eq!(archive.len() as u64, prepared.object_count() + 1);
    let mut root = Vec::new();
    archive
        .by_name("project.json")
        .unwrap()
        .read_to_end(&mut root)
        .unwrap();
    PortableProjectHeader::parse(&root).unwrap();
    assert_eq!(root, prepared.root_bytes());
    prepared
        .visit_objects(&mut |reference| {
            let path = format!("objects/{}", reference.sha256());
            let mut entry = archive.by_name(&path).unwrap();
            let mut actual = Vec::new();
            entry.read_to_end(&mut actual)?;
            assert_eq!(actual.len() as u64, reference.byte_size());
            assert_eq!(format!("{:x}", Sha256::digest(&actual)), reference.sha256());
            Ok(())
        })
        .unwrap();
    assert!(
        ProjectArchive::open(&destination).is_err(),
        "A native format2 export must not impersonate the source-compatible archive"
    );
    assert!(
        export_canonical_project_archive_file(
            &prepared,
            &database,
            &destination,
            &ProjectTransferCancellation::default()
        )
        .is_err()
    );
    assert_eq!(std::fs::read(&destination).unwrap(), bytes);
}
#[test]
fn protected_paths_and_cancelled_output_should_preserve_database_and_leave_no_completed_file() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("chosen.sqlite");
    let workspace = Workspace::open(&database).unwrap();
    let project = workspace.create_project("Empty native project").unwrap();
    let mut worker = ProjectPortableExportWorker::open(&database).unwrap();
    let prepared = worker
        .prepare(&project.id, &ProjectTransferCancellation::default())
        .unwrap();
    for target in [
        database.clone(),
        directory.path().join("chosen.sqlite-wal"),
        directory.path().join("chosen.sqlite-shm"),
    ] {
        assert!(
            export_canonical_project_archive_file(
                &prepared,
                &database,
                &target,
                &ProjectTransferCancellation::default()
            )
            .is_err()
        );
    }
    let cancel = ProjectTransferCancellation::default();
    cancel.cancel();
    let destination = directory.path().join("cancelled.writercraft");
    assert!(
        export_canonical_project_archive_file(&prepared, &database, &destination, &cancel).is_err()
    );
    assert!(!destination.exists());
    assert_eq!(
        workspace.list_projects().unwrap()[0].title,
        "Empty native project"
    );
    let leftovers = std::fs::read_dir(directory.path())
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(".writercraft-canonical-"))
        .collect::<Vec<_>>();
    assert!(leftovers.is_empty());
}
