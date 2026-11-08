#![expect(
    clippy::unwrap_used,
    reason = "Synthetic media capability contracts fail immediately"
)]
//! Media preview receives one exact resource capability, never a whole-project writing snapshot.
use std::io::{Cursor, Read};

use craft_editor_model::Document as RichTree;
use serde_json::json;
use sha2::{Digest, Sha256};
use studio_core::{
    ImportAssetSource, ProjectAttachment, ProjectImportWorker, ProjectTransferCancellation,
    ValidatedProjectImport, Workspace,
};
struct Asset(Vec<u8>);
impl ImportAssetSource for Asset {
    fn open<'a>(&'a mut self, _: &ProjectAttachment) -> std::io::Result<Box<dyn Read + 'a>> {
        Ok(Box::new(Cursor::new(self.0.as_slice())))
    }
}
fn import(database: &std::path::Path, bytes: Vec<u8>, mime: &str) -> (ProjectImportWorker, String) {
    let source = Workspace::memory().unwrap();
    let project = source.create_project("Synthetic media").unwrap();
    source
        .create_rich_document(
            &project.id,
            "Unrelated writing",
            RichTree::plain("p", "Private unrelated writing")
                .unwrap()
                .to_json()
                .unwrap(),
        )
        .unwrap();
    let mut manifest = source
        .project_export_snapshot(&project.id)
        .unwrap()
        .manifest()
        .clone();
    manifest["media"] = json!([{"id":"shared","path":"media/original.bin","sha256":format!("{:x}",Sha256::digest(&bytes)),"byteSize":bytes.len(),"mediaType":mime,"originalName":"original.bin","width":null,"height":null}]);
    let parsed = ValidatedProjectImport::parse(&serde_json::to_vec(&manifest).unwrap()).unwrap();
    let mut worker = ProjectImportWorker::open(database).unwrap();
    let imported = worker
        .import(
            &parsed,
            &mut Asset(bytes),
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    (worker, imported.project_id)
}
#[test]
fn one_media_capability_should_keep_original_alias_mime_hash_and_reject_another_project() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let (first, a) = import(
        &database,
        b"first immutable original".to_vec(),
        "application/x-synthetic",
    );
    let (second, b) = import(
        &database,
        b"second immutable original".to_vec(),
        "application/x-synthetic",
    );
    let (descriptor, source) = first
        .workspace()
        .project_media_source(&a, "shared")
        .unwrap();
    let (foreign, _) = second
        .workspace()
        .project_media_source(&b, "shared")
        .unwrap();
    assert_eq!(descriptor.logical_id, "shared");
    assert_eq!(descriptor.media_type, "application/x-synthetic");
    assert_eq!(source.project_id(), a);
    assert_eq!(
        source
            .load(
                &descriptor.logical_id,
                &ProjectTransferCancellation::default()
            )
            .unwrap(),
        b"first immutable original"
    );
    assert!(
        source
            .copy_to(
                &foreign,
                &mut Vec::new(),
                &ProjectTransferCancellation::default()
            )
            .is_err()
    );
    assert!(
        first
            .workspace()
            .project_media_source(&a, "missing")
            .is_err()
    );
}
#[test]
fn audio_original_above_image_limit_should_stream_without_an_image_policy_cut() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let mut bytes = vec![0_u8; 13 * 1024 * 1024];
    bytes[..3].copy_from_slice(b"ID3"); // Header-only synthetic preservation fixture, not decoder acceptance.
    let sha = format!("{:x}", Sha256::digest(&bytes));
    let (worker, project) = import(&database, bytes, "audio/mpeg");
    let (descriptor, source) = worker
        .workspace()
        .project_media_source(&project, "shared")
        .unwrap();
    assert_eq!(descriptor.byte_size, 13 * 1024 * 1024);
    assert_eq!(descriptor.sha256, sha);
    let mut copied = Vec::new();
    source
        .copy_to(
            &descriptor,
            &mut copied,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    assert_eq!(format!("{:x}", Sha256::digest(&copied)), sha);
}
#[test]
fn reading_one_media_should_not_parse_unrelated_manuscript_sources() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let (worker, project) = import(
        &database,
        b"owned original".to_vec(),
        "application/x-synthetic",
    );
    let opaque = worker
        .workspace()
        .import_document(
            &project,
            "Unrelated opaque",
            "Reading preview",
            json!({"future":"schema"}),
        )
        .unwrap();
    rusqlite::Connection::open(&database)
        .unwrap()
        .execute(
            "UPDATE documents SET source_json='broken unrelated JSON' WHERE id=?1",
            [opaque.id],
        )
        .unwrap();
    assert!(
        worker
            .workspace()
            .project_export_snapshot(&project)
            .is_err()
    );
    let (descriptor, source) = worker
        .workspace()
        .project_media_source(&project, "shared")
        .unwrap();
    assert_eq!(
        source
            .load(
                &descriptor.logical_id,
                &ProjectTransferCancellation::default()
            )
            .unwrap(),
        b"owned original"
    );
}
