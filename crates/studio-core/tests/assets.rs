#![expect(
    clippy::unwrap_used,
    reason = "Synthetic image fixtures must fail immediately"
)]

use std::io::Cursor;

use craft_editor_model::{Command, Document as RichTree, Editor, NodePath, Revision, Transaction};
use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use studio_core::{
    ImageAssetFailureKind, MAX_IMAGE_BYTES, MAX_IMAGE_DIMENSION, StudioError, Workspace,
};

fn picture(format: ImageFormat, width: u32, height: u32) -> Vec<u8> {
    let picture = DynamicImage::ImageRgb8(RgbImage::from_pixel(width, height, Rgb([60, 100, 180])));
    let mut encoded = Cursor::new(Vec::new());
    picture.write_to(&mut encoded, format).unwrap();
    encoded.into_inner()
}
fn fixture() -> (Workspace, String) {
    let workspace = Workspace::memory().unwrap();
    let project = workspace.create_project("Images").unwrap();
    (workspace, project.id)
}
fn document(node: craft_editor_model::Node) -> RichTree {
    RichTree::from_root(craft_editor_model::Node::block(
        "doc",
        Map::from_iter([("schemaVersion".into(), json!(2))]),
        vec![node],
    ))
    .unwrap()
}

#[test]
fn all_original_backend_image_formats_are_durable_with_derived_mime_and_exact_checksum() {
    let (workspace, project) = fixture();
    for (format, mime) in [
        (ImageFormat::Png, "image/png"),
        (ImageFormat::Jpeg, "image/jpeg"),
        (ImageFormat::Gif, "image/gif"),
        (ImageFormat::WebP, "image/webp"),
    ] {
        let bytes = picture(format, 3, 2);
        let asset = workspace
            .import_image(&project, "../../cover.png", Some(mime), &bytes)
            .unwrap();
        assert_eq!((asset.width, asset.height), (3, 2));
        assert_eq!(asset.media_type, mime);
        assert_eq!(asset.original_name, "cover.png");
        assert_eq!(asset.byte_size, bytes.len() as u64);
        assert_eq!(asset.sha256, format!("{:x}", Sha256::digest(&bytes)));
        assert_eq!(
            workspace.image_asset_bytes(&project, &asset.id).unwrap(),
            bytes
        );
        assert_eq!(workspace.image_asset(&project, &asset.id).unwrap(), asset);
    }
}

#[test]
fn explicit_file_import_survives_source_removal_and_workspace_reopening() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let source = directory.path().join("Author image.png");
    let bytes = picture(ImageFormat::Png, 3, 2);
    std::fs::write(&source, &bytes).unwrap();
    let (project, asset) = {
        let workspace = Workspace::open(&database).unwrap();
        let project = workspace.create_project("Durable").unwrap();
        let asset = workspace.import_image_file(&project.id, &source).unwrap();
        assert!(workspace.list_documents(&project.id).unwrap().is_empty());
        (project, asset)
    };
    std::fs::remove_file(source).unwrap();
    let workspace = Workspace::open(database).unwrap();
    assert_eq!(
        workspace.image_asset(&project.id, &asset.id).unwrap(),
        asset
    );
    assert_eq!(
        workspace.image_asset_bytes(&project.id, &asset.id).unwrap(),
        bytes
    );
}

#[test]
fn empty_oversized_invalid_unsupported_and_mime_spoofed_images_are_refused() {
    let (workspace, project) = fixture();
    for bytes in [
        Vec::new(),
        vec![0; MAX_IMAGE_BYTES + 1],
        b"broken image".to_vec(),
        b"<svg xmlns='http://www.w3.org/2000/svg'/>".to_vec(),
        b"\x89PNG\r\n\x1a\ntruncated".to_vec(),
    ] {
        assert!(
            workspace
                .import_image(&project, "image.png", None, &bytes)
                .is_err()
        );
    }
    let png = picture(ImageFormat::Png, 1, 1);
    assert!(
        workspace
            .import_image(&project, "spoof.jpg", Some("image/jpeg"), &png)
            .is_err()
    );
    assert!(
        workspace
            .import_image_file(&project, std::env::temp_dir())
            .is_err()
    );
    let asset = workspace
        .import_image(&project, "image.png", Some(" IMAGE/PNG "), &png)
        .unwrap();
    assert_eq!(asset.media_type, "image/png");
}

#[test]
fn original_per_axis_dimension_limit_is_retained_without_a_smaller_native_import_limit() {
    let (workspace, project) = fixture();
    let accepted = picture(ImageFormat::Png, MAX_IMAGE_DIMENSION, 1);
    let asset = workspace
        .import_image(&project, "wide.png", None, &accepted)
        .unwrap();
    assert_eq!(asset.width, 20_000);
    let rejected = picture(ImageFormat::Png, MAX_IMAGE_DIMENSION + 1, 1);
    assert!(
        workspace
            .import_image(&project, "too-wide.png", None, &rejected)
            .is_err()
    );
}

#[test]
fn project_ownership_and_source_binding_prevent_cross_project_or_wrong_source_rendering() {
    let (workspace, project) = fixture();
    let other = workspace.create_project("Other").unwrap();
    let bytes = picture(ImageFormat::Png, 2, 2);
    let asset = workspace
        .import_image(&project, "image.png", None, &bytes)
        .unwrap();
    assert!(workspace.image_asset(&other.id, &asset.id).is_err());
    assert!(workspace.image_asset_bytes(&other.id, &asset.id).is_err());
    assert!(
        workspace
            .image_node(&other.id, &asset.id, "image", "Alt", "Caption")
            .is_err()
    );
    let node = workspace
        .image_node(&project, &asset.id, "image", "Alt", "Caption")
        .unwrap();
    let mut bad = node.clone();
    bad.attrs
        .as_mut()
        .unwrap()
        .insert("src".into(), json!("https://example.invalid/another.png"));
    bad.attrs
        .as_mut()
        .unwrap()
        .insert("blockId".into(), json!("bad"));
    let root = craft_editor_model::Node::block(
        "doc",
        Map::from_iter([("schemaVersion".into(), json!(2))]),
        vec![node.clone(), bad.clone()],
    );
    let tree = RichTree::from_root(root).unwrap();
    let snapshot = workspace.image_asset_snapshot(&project, &tree).unwrap();
    assert_eq!(*snapshot.load(&node).unwrap(), bytes);
    assert!(snapshot.load(&bad).is_err());
    assert_eq!(
        snapshot.failures()[0].kind,
        ImageAssetFailureKind::UnresolvedReference
    );
    let foreign = workspace.image_asset_snapshot(&other.id, &tree).unwrap();
    assert!(foreign.load(&node).is_err());
}

#[test]
fn renderer_snapshot_is_scoped_lazy_and_can_outlive_the_workspace_borrow() {
    let (workspace, project) = fixture();
    let bytes = picture(ImageFormat::Png, 2, 3);
    let asset = workspace
        .import_image(&project, "image.png", None, &bytes)
        .unwrap();
    let node = workspace
        .image_node(&project, &asset.id, "image", "Alt", "Caption")
        .unwrap();
    let snapshot = workspace
        .image_asset_snapshot(&project, &document(node.clone()))
        .unwrap();
    let unrelated = workspace
        .import_image(&project, "unreferenced.png", None, &bytes)
        .unwrap();
    let unrelated_node = workspace
        .image_node(&project, &unrelated.id, "other-image", "", "")
        .unwrap();
    assert!(snapshot.load(&unrelated_node).is_err());
    assert_eq!(snapshot.project_id(), project);
    drop(workspace);
    assert_eq!(
        std::thread::spawn(move || snapshot.load(&node).unwrap())
            .join()
            .unwrap()
            .as_slice(),
        bytes
    );
}

#[test]
fn legacy_owned_content_routes_resolve_without_fetching_or_rewriting_canonical_attributes() {
    let (workspace, project) = fixture();
    let bytes = picture(ImageFormat::Png, 1, 1);
    let asset = workspace
        .import_image(&project, "image.png", None, &bytes)
        .unwrap();
    let mut node = workspace
        .image_node(&project, &asset.id, "image", "Alt", "Caption")
        .unwrap();
    let attrs = node.attrs.as_mut().unwrap();
    attrs.insert(
        "src".into(),
        json!(format!("/api/media/{}/content", asset.id)),
    );
    attrs.insert("vendor".into(), json!({"keep":[null,1]}));
    node.extra.insert("opaque".into(), json!(true));
    let tree = document(node.clone());
    let original = tree.to_json().unwrap();
    let snapshot = workspace.image_asset_snapshot(&project, &tree).unwrap();
    assert_eq!(*snapshot.load(&node).unwrap(), bytes);
    assert_eq!(tree.to_json().unwrap(), original);
    assert_eq!(
        node.string_attr("src"),
        Some(format!("/api/media/{}/content", asset.id).as_str())
    );
}

#[test]
fn opaque_external_missing_and_unsupported_refs_survive_canonical_save_and_snapshot() {
    let (workspace, project) = fixture();
    let content = json!({"type":"doc","attrs":{"schemaVersion":2,"opaque":null},"content":[{"type":"imageBlock","attrs":{"blockId":"external","src":"https://example.invalid/private.png","assetId":null,"vendor":{"keep":true}}},{"type":"imageBlock","attrs":{"blockId":"unknown","src":"asset://opaque-historical-key","assetId":"opaque-historical-key"}}]});
    let saved = workspace
        .create_rich_document(&project, "Opaque", content.clone())
        .unwrap();
    let tree = RichTree::from_json(content.clone()).unwrap();
    let snapshot = workspace.image_asset_snapshot(&project, &tree).unwrap();
    assert_eq!(snapshot.failures().len(), 2);
    for node in tree.blocks() {
        assert!(snapshot.load(node).is_err());
    }
    assert_eq!(
        workspace.document(&saved.id).unwrap().rich_json,
        Some(content)
    );
}

#[test]
fn image_bytes_survive_drafts_structural_undo_redo_and_rich_history() {
    let (workspace, project) = fixture();
    let bytes = picture(ImageFormat::Png, 2, 2);
    let asset = workspace
        .import_image(&project, "image.png", None, &bytes)
        .unwrap();
    let node = workspace
        .image_node(&project, &asset.id, "image", "Description", "Caption")
        .unwrap();
    let initial = RichTree::plain("p", "Writing").unwrap();
    let saved = workspace
        .create_rich_document(&project, "Doc", initial.to_json().unwrap())
        .unwrap();
    let mut editor = Editor::new(initial, Revision(0));
    let parent = editor.document().node_ref(&NodePath::root()).unwrap();
    let change = editor
        .apply(Transaction {
            base_revision: Revision(0),
            commands: vec![Command::InsertChildren {
                parent,
                index: 1,
                children: vec![node],
            }],
        })
        .unwrap();
    let draft = workspace
        .save_rich_draft(&saved.id, 1, None, editor.document().to_json().unwrap())
        .unwrap();
    let inserted = workspace
        .save_rich_editor_changes(&saved.id, 1, &[change.replay_proof()], draft.content)
        .unwrap();
    let undo = editor.undo(editor.revision()).unwrap().replay_proof();
    let removed = workspace
        .save_rich_editor_changes(
            &saved.id,
            inserted.revision,
            &[undo],
            editor.document().to_json().unwrap(),
        )
        .unwrap();
    assert_eq!(
        workspace.image_asset_bytes(&project, &asset.id).unwrap(),
        bytes
    );
    let redo = editor.redo(editor.revision()).unwrap().replay_proof();
    let restored = workspace
        .save_rich_editor_changes(
            &saved.id,
            removed.revision,
            &[redo],
            editor.document().to_json().unwrap(),
        )
        .unwrap();
    assert_eq!(restored.rich_json, inserted.rich_json);
    assert_eq!(
        workspace.image_asset_bytes(&project, &asset.id).unwrap(),
        bytes
    );
    assert_eq!(workspace.rich_revisions(&saved.id).unwrap().len(), 4);
}

#[test]
fn byte_corruption_and_metadata_changes_fail_instead_of_serving_substitute_images() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite");
    let workspace = Workspace::open(&path).unwrap();
    let project = workspace.create_project("Integrity").unwrap();
    let bytes = picture(ImageFormat::Png, 1, 1);
    let asset = workspace
        .import_image(&project.id, "image.png", None, &bytes)
        .unwrap();
    let node = workspace
        .image_node(&project.id, &asset.id, "image", "", "")
        .unwrap();
    let snapshot = workspace
        .image_asset_snapshot(&project.id, &document(node.clone()))
        .unwrap();
    let raw = rusqlite::Connection::open(&path).unwrap();
    let mut corrupted = bytes.clone();
    corrupted[0] ^= 1;
    raw.execute(
        "UPDATE project_assets SET data=?2 WHERE id=?1",
        rusqlite::params![asset.id, corrupted],
    )
    .unwrap();
    assert!(matches!(
        workspace.image_asset_bytes(&project.id, &asset.id),
        Err(StudioError::CorruptData(_))
    ));
    assert!(snapshot.load(&node).is_err());
    raw.execute(
        "UPDATE project_assets SET data=?2,original_name='Changed externally.png' WHERE id=?1",
        rusqlite::params![asset.id, bytes],
    )
    .unwrap();
    assert!(snapshot.load(&node).is_err());
}

#[test]
fn failed_asset_insert_is_atomic_and_never_mutates_the_manuscript() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite");
    let workspace = Workspace::open(&path).unwrap();
    let project = workspace.create_project("Failure").unwrap();
    let document = workspace
        .create_document(&project.id, "Doc", "Keep text")
        .unwrap();
    let raw = rusqlite::Connection::open(path).unwrap();
    raw.execute_batch("CREATE TRIGGER reject_image BEFORE INSERT ON project_assets BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
    assert!(
        workspace
            .import_image(
                &project.id,
                "image.png",
                None,
                &picture(ImageFormat::Png, 1, 1)
            )
            .is_err()
    );
    assert_eq!(workspace.document(&document.id).unwrap(), document);
    assert_eq!(
        raw.query_row("SELECT count(*) FROM project_assets", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn schema_four_migration_preserves_existing_rich_notes_drafts_and_history() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite");
    let (saved, annotation, draft) = {
        let workspace = Workspace::open(&path).unwrap();
        let project = workspace.create_project("Existing").unwrap();
        let tree = RichTree::plain("p", "Keep note").unwrap();
        let saved = workspace
            .create_rich_document(&project.id, "Doc", tree.to_json().unwrap())
            .unwrap();
        let anchor = workspace
            .capture_annotation_anchor(
                &saved.id,
                1,
                "p",
                craft_editor_model::ScalarOffset(0),
                craft_editor_model::ScalarOffset(4),
                "Keep",
            )
            .unwrap();
        let annotation = workspace
            .create_annotation(&saved.id, 1, anchor, "Remember")
            .unwrap();
        let draft = workspace
            .save_rich_draft(&saved.id, 1, None, tree.to_json().unwrap())
            .unwrap();
        (saved, annotation, draft)
    };
    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute_batch(
        "DROP TABLE project_annotation_projections; DROP TABLE project_quarantined_recovery; DROP TABLE project_canvas_drafts; DROP TABLE project_canvas_revisions; DROP TABLE project_canvases; DROP TABLE project_asset_metadata; DROP TABLE project_source_records; DROP TABLE project_nodes; DROP TABLE project_resource_aliases; DROP TABLE project_portability; DROP TABLE rich_draft_annotations; DROP TABLE project_assets; PRAGMA user_version=4;",
    )
    .unwrap();
    drop(raw);
    let workspace = Workspace::open(&path).unwrap();
    assert_eq!(workspace.document(&saved.id).unwrap(), saved);
    assert_eq!(workspace.annotation(&annotation.id).unwrap(), annotation);
    assert_eq!(workspace.rich_draft(&saved.id).unwrap(), Some(draft));
    assert_eq!(
        workspace.annotation_history(&annotation.id).unwrap().len(),
        1
    );
    assert_eq!(workspace.rich_revisions(&saved.id).unwrap().len(), 1);
    let raw = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        raw.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        7
    );
}

#[test]
fn generic_media_rows_preserve_nonimage_and_unknown_bytes_without_an_image_capacity_cut() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.sqlite");
    let workspace = Workspace::open(&path).unwrap();
    let project = workspace.create_project("Future import").unwrap();
    let bytes = vec![0; MAX_IMAGE_BYTES + 1];
    let id = uuid::Uuid::new_v4().to_string();
    let raw = rusqlite::Connection::open(path).unwrap();
    let opaque = json!({"unknown":[null,1,true]});
    raw.execute("INSERT INTO project_assets(id,project_id,media_type,original_name,sha256,byte_size,data,source_json) VALUES(?1,?2,'application/x-opaque-project-resource','Preserve original',?3,?4,?5,?6)",rusqlite::params![id,project.id,format!("{:x}",Sha256::digest(&bytes)),bytes.len() as i64,bytes,serde_json::to_string(&opaque).unwrap()]).unwrap();
    assert!(workspace.image_asset(&project.id, &id).is_err());
    workspace
        .create_rich_document(
            &project.id,
            "Doc",
            RichTree::plain("p", "Edit without dropping other media")
                .unwrap()
                .to_json()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        raw.query_row(
            "SELECT data FROM project_assets WHERE id=?1",
            [&id],
            |row| { row.get::<_, Vec<u8>>(0) }
        )
        .unwrap(),
        bytes
    );
    let preserved = raw
        .query_row(
            "SELECT source_json FROM project_assets WHERE id=?1",
            [id],
            |row| row.get::<_, String>(0),
        )
        .unwrap();
    assert_eq!(serde_json::from_str::<Value>(&preserved).unwrap(), opaque);
}

#[test]
fn authoring_node_validation_preserves_asset_and_checks_the_original_text_limits() {
    let (workspace, project) = fixture();
    let bytes = picture(ImageFormat::Png, 1, 1);
    let asset = workspace
        .import_image(&project, "C:\\private\\co\0ver.png", None, &bytes)
        .unwrap();
    assert_eq!(asset.original_name, "cover.png");
    assert!(
        workspace
            .image_node(&project, &asset.id, "", "", "")
            .is_err()
    );
    assert!(
        workspace
            .image_node(&project, &asset.id, "image", &"я".repeat(501), "")
            .is_err()
    );
    let node = workspace
        .image_node(&project, &asset.id, "image", "Description", "Caption")
        .unwrap();
    assert_eq!(node.string_attr("src"), Some(asset.source().as_str()));
    assert_eq!(
        node.attrs.as_ref().unwrap().get("width"),
        Some(&Value::Null)
    );
    assert!(workspace.list_documents(&project).unwrap().is_empty());
    assert_eq!(
        workspace.image_asset_bytes(&project, &asset.id).unwrap(),
        bytes
    );
}
