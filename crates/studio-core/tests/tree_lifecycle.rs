//! Focused hierarchy contracts for the structured-writing task; native acceptance is separate.
use std::collections::BTreeMap;
use std::io::{Cursor, Read};

use serde_json::json;
use studio_core::{
    ImportAssetSource, PortableObjectReference, PortableObjectSource, PreparedPortableProject,
    ProjectAttachment, ProjectImportWorker, ProjectPortableExportWorker,
    ProjectTransferCancellation, ProjectTreeNode, StudioError, ValidatedProjectImport,
    VerifiedPortableProjectStream, Workspace,
};

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn plain_desktop_tree_creation_keeps_its_new_document_writable() -> TestResult {
    let workspace = Workspace::memory()?;
    let project = workspace.create_project("Plain writing")?;
    let node = workspace.create_project_tree_node_with_format(
        &project.id,
        1,
        None,
        "document",
        "Scene",
        studio_core::InitialManuscript::Plain,
    )?;
    let document = workspace.document(node.document_id.as_deref().ok_or("missing document")?)?;
    assert!(document.rich_json.is_none());
    let saved = workspace.save_document(
        &document.id,
        document.revision,
        "An ordinary writable scene.",
    )?;
    assert_eq!(saved.body, "An ordinary writable scene.");
    Ok(())
}

fn node(workspace: &Workspace, project: &str, id: &str) -> studio_core::Result<ProjectTreeNode> {
    workspace
        .initialize_project_tree(project)?
        .into_iter()
        .find(|node| node.logical_id == id)
        .ok_or_else(|| StudioError::NotFound {
            kind: "test node",
            id: id.into(),
        })
}

#[test]
fn native_tree_crud_preserves_writing_and_rejects_stale_cyclic_and_foreign_placement() -> TestResult
{
    let workspace = Workspace::memory()?;
    let project = workspace.create_project("Synthetic manuscript")?;
    let document =
        workspace.create_document(&project.id, "Old flat scene", "A promise. 😀 A promise.")?;
    let saved = workspace.save_document(
        &document.id,
        document.revision,
        "A promise. 😀 A changed promise.",
    )?;
    let history = workspace.revisions(&saved.id)?;
    let nodes = workspace.initialize_project_tree(&project.id)?;
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].document_id.as_deref(), Some(saved.id.as_str()));
    assert_eq!(workspace.document(&saved.id)?, saved);
    let structure = workspace.project_structure(&project.id)?;
    assert!(structure.is_content_bearing("note"));
    assert!(!structure.is_content_bearing("folder"));
    let chapter = workspace.create_project_tree_node(
        &project.id,
        structure.revision,
        None,
        "folder",
        "Chapter one",
    )?;
    let other = workspace.create_project_tree_node(
        &project.id,
        structure.revision,
        None,
        "folder",
        "Chapter two",
    )?;
    let moved = workspace.move_project_tree_node(
        &project.id,
        &nodes[0],
        Some(&chapter.logical_id),
        None,
    )?;
    assert_eq!(moved.metadata["parentId"], chapter.logical_id);
    let renamed = workspace.rename_project_tree_node(&project.id, &moved, "A kept promise")?;
    assert_eq!(workspace.document(&saved.id)?.title, "A kept promise");
    assert_eq!(workspace.document(&saved.id)?.body, saved.body);
    assert_eq!(workspace.revisions(&saved.id)?, history);
    assert!(matches!(
        workspace.rename_project_tree_node(&project.id, &moved, "Stale"),
        Err(StudioError::ProjectNodeChanged { .. })
    ));

    let child_folder = workspace.create_project_tree_node(
        &project.id,
        structure.revision,
        Some(&chapter.logical_id),
        "folder",
        "Nested",
    )?;
    let before = workspace.initialize_project_tree(&project.id)?;
    assert!(
        workspace
            .move_project_tree_node(&project.id, &chapter, Some(&child_folder.logical_id), None)
            .is_err()
    );
    assert!(
        workspace
            .move_project_tree_node(&project.id, &renamed, Some("foreign-parent"), None)
            .is_err()
    );
    assert!(
        workspace
            .move_project_tree_node(
                &project.id,
                &renamed,
                Some(&other.logical_id),
                Some(&child_folder.logical_id)
            )
            .is_err()
    );
    assert_eq!(workspace.initialize_project_tree(&project.id)?, before);

    let second = workspace.create_project_tree_node(
        &project.id,
        structure.revision,
        Some(&chapter.logical_id),
        "document",
        "Second scene",
    )?;
    let second = workspace.move_project_tree_node(
        &project.id,
        &second,
        Some(&chapter.logical_id),
        Some(&renamed.logical_id),
    )?;
    let first = node(&workspace, &project.id, &renamed.logical_id)?;
    assert!(second.metadata["position"].as_f64() < first.metadata["position"].as_f64());
    let renamed_again = workspace.rename_project_tree_node(&project.id, &first, "Renamed away")?;
    let _restored_title =
        workspace.rename_project_tree_node(&project.id, &renamed_again, "A kept promise")?;
    assert!(matches!(
        workspace.rename_project_tree_node(&project.id, &first, "Same-title stale capture"),
        Err(StudioError::ProjectNodeChanged { .. })
    ));
    Ok(())
}

#[test]
fn branch_restore_preserves_independent_trash_and_requires_the_saved_parent() -> TestResult {
    let workspace = Workspace::memory()?;
    let project = workspace.create_project("Synthetic recovery")?;
    let folder = workspace.create_project_tree_node(&project.id, 1, None, "folder", "Draft")?;
    let keep = workspace.create_project_tree_node(
        &project.id,
        1,
        Some(&folder.logical_id),
        "document",
        "Keep",
    )?;
    let removed = workspace.create_project_tree_node(
        &project.id,
        1,
        Some(&folder.logical_id),
        "note",
        "Already removed",
    )?;
    let removed = workspace.trash_project_tree_node(&project.id, &removed)?;
    let keep_id = keep.document_id.as_deref().ok_or("missing document")?;
    let baseline = workspace.document(keep_id)?;
    let history = workspace.revisions(keep_id)?;
    let trash = workspace.trash_project_tree_node(&project.id, &folder)?;
    let trashed_keep = node(&workspace, &project.id, &keep.logical_id)?;
    assert!(trashed_keep.metadata["trashedAt"].is_string());
    assert_eq!(node(&workspace, &project.id, &removed.logical_id)?, removed);
    assert!(
        workspace
            .restore_project_tree_node(&project.id, &trashed_keep)
            .is_err()
    );
    assert!(
        workspace
            .create_project_tree_node(&project.id, 1, Some(&folder.logical_id), "note", "Refused")
            .is_err()
    );
    let restored = workspace.restore_project_tree_node(&project.id, &trash)?;
    assert!(restored.metadata["trashedAt"].is_null());
    assert!(node(&workspace, &project.id, &keep.logical_id)?.metadata["trashedAt"].is_null());
    assert_eq!(node(&workspace, &project.id, &removed.logical_id)?, removed);
    assert_eq!(workspace.document(keep_id)?, baseline);
    assert_eq!(workspace.revisions(keep_id)?, history);
    assert!(matches!(
        workspace.restore_project_tree_node(&project.id, &trash),
        Err(StudioError::ProjectNodeChanged { .. })
    ));
    workspace.restore_project_tree_node(&project.id, &removed)?;
    Ok(())
}

struct NoAssets;
impl ImportAssetSource for NoAssets {
    fn open<'a>(&'a mut self, _: &ProjectAttachment) -> std::io::Result<Box<dyn Read + 'a>> {
        Err(std::io::Error::other(
            "No assets in this synthetic hierarchy",
        ))
    }
}

struct Objects(BTreeMap<String, Vec<u8>>);
impl PortableObjectSource for Objects {
    fn verify_transport(&mut self) -> std::io::Result<()> {
        Ok(())
    }
    fn visit_objects(
        &mut self,
        visitor: &mut dyn FnMut(PortableObjectReference) -> studio_core::Result<()>,
    ) -> studio_core::Result<()> {
        for (hash, bytes) in &self.0 {
            visitor(PortableObjectReference::new(hash, bytes.len() as u64)?)?;
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

fn objects(prepared: &PreparedPortableProject) -> studio_core::Result<Objects> {
    let mut objects = BTreeMap::new();
    prepared.visit_objects(&mut |object| {
        let mut bytes = Vec::new();
        prepared.copy_object(object, &mut bytes, &ProjectTransferCancellation::default())?;
        objects.insert(object.sha256().to_owned(), bytes);
        Ok(())
    })?;
    Ok(Objects(objects))
}

#[test]
fn native_hierarchy_survives_both_exports_with_order_history_and_recoverable_trash() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("source.sqlite");
    let workspace = Workspace::open(&path)?;
    let project = workspace.create_project("Synthetic transfer")?;
    let folder =
        workspace.create_project_tree_node(&project.id, 1, None, "folder", "Manuscript")?;
    let scene = workspace.create_project_tree_node(
        &project.id,
        1,
        Some(&folder.logical_id),
        "document",
        "Scene 😀",
    )?;
    let notes = workspace.create_project_tree_node(&project.id, 1, None, "folder", "Materials")?;
    let profile = workspace.create_project_tree_node(
        &project.id,
        1,
        Some(&notes.logical_id),
        "note",
        "Motivation",
    )?;
    let profile_document = workspace.document(profile.document_id.as_deref().ok_or("profile")?)?;
    let profile_content = craft_editor_model::Document::plain(
        "motivation",
        "The promise conflicts with the public duty.",
    )?
    .to_json()?;
    workspace.save_rich_document(
        &profile_document.id,
        profile_document.revision,
        profile_content,
    )?;
    let folder = workspace.rename_project_tree_node(&project.id, &folder, "Draft chapters")?;
    let _trashed = workspace.trash_project_tree_node(&project.id, &scene)?;
    let before = workspace.initialize_project_tree(&project.id)?;

    let snapshot = workspace.project_export_snapshot(&project.id)?;
    let validated = ValidatedProjectImport::parse(snapshot.manifest_bytes())?;
    let mut legacy = ProjectImportWorker::memory()?;
    let legacy_project = legacy.import(
        &validated,
        &mut NoAssets,
        &ProjectTransferCancellation::default(),
    )?;
    assert_eq!(
        legacy
            .workspace()
            .project_structure(&legacy_project.project_id)?
            .schema,
        workspace.project_structure(&project.id)?.schema
    );
    let restored_folder = node(
        legacy.workspace(),
        &legacy_project.project_id,
        &folder.logical_id,
    )?;
    assert_eq!(restored_folder.metadata["title"], "Draft chapters");
    let restored_scene = node(
        legacy.workspace(),
        &legacy_project.project_id,
        &scene.logical_id,
    )?;
    assert!(restored_scene.metadata["trashedAt"].is_string());
    legacy
        .workspace()
        .restore_project_tree_node(&legacy_project.project_id, &restored_scene)?;

    let prepared = ProjectPortableExportWorker::open(&path)?
        .prepare(&project.id, &ProjectTransferCancellation::default())?;
    let verified = VerifiedPortableProjectStream::preflight(
        prepared.root_bytes(),
        &mut objects(&prepared)?,
        &ProjectTransferCancellation::default(),
    )?;
    let mut target = ProjectImportWorker::memory()?;
    let restored = target.restore_stream(&verified, &ProjectTransferCancellation::default())?;
    let after = target
        .workspace()
        .initialize_project_tree(&restored.project_id)?;
    assert_eq!(before.len(), after.len());
    for previous in &before {
        let current = after
            .iter()
            .find(|node| node.logical_id == previous.logical_id)
            .ok_or("missing restored node")?;
        assert_eq!(previous.metadata, current.metadata);
        if let (Some(old), Some(new)) = (&previous.document_id, &current.document_id) {
            assert_ne!(old, new);
            assert_eq!(
                workspace.document(old)?.rich_json,
                target.workspace().document(new)?.rich_json
            );
            assert_eq!(
                workspace.revisions(old)?.len(),
                target.workspace().revisions(new)?.len()
            );
        }
    }
    Ok(())
}

#[test]
fn imported_custom_fields_and_original_bytes_survive_native_lifecycle() -> TestResult {
    let seed = Workspace::memory()?;
    let seed_project = seed.create_project("Synthetic custom")?;
    let folder =
        seed.create_project_tree_node(&seed_project.id, 1, None, "folder", "Custom folder")?;
    seed.create_project_tree_node(
        &seed_project.id,
        1,
        Some(&folder.logical_id),
        "document",
        "Original",
    )?;
    let mut manifest = seed
        .project_export_snapshot(&seed_project.id)?
        .manifest()
        .clone();
    manifest["project"]["kind"] = json!("custom");
    manifest["project"]["structure"]["vendor"] = json!({"keep":[1,null,"😀"]});
    let original_folder = manifest["nodes"]
        .as_array_mut()
        .ok_or("nodes")?
        .iter_mut()
        .find(|node| node["id"] == folder.logical_id)
        .ok_or("folder")?;
    original_folder["vendor"] = json!({"keep":"source metadata"});
    let original = serde_json::to_vec_pretty(&manifest)?;
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("imported.sqlite");
    let mut importer = ProjectImportWorker::open(&path)?;
    let imported = importer.import(
        &ValidatedProjectImport::parse(&original)?,
        &mut NoAssets,
        &ProjectTransferCancellation::default(),
    )?;
    let workspace = importer.workspace();
    let folder = node(workspace, &imported.project_id, &folder.logical_id)?;
    let renamed = workspace.rename_project_tree_node(
        &imported.project_id,
        &folder,
        "Renamed custom folder",
    )?;
    let trashed = workspace.trash_project_tree_node(&imported.project_id, &renamed)?;
    let restored = workspace.restore_project_tree_node(&imported.project_id, &trashed)?;
    assert_eq!(
        restored.metadata["vendor"],
        json!({"keep":"source metadata"})
    );
    assert_eq!(
        workspace.project_structure(&imported.project_id)?.schema["vendor"],
        json!({"keep":[1,null,"😀"]})
    );
    let prepared = ProjectPortableExportWorker::open(&path)?.prepare(
        &imported.project_id,
        &ProjectTransferCancellation::default(),
    )?;
    let captured = objects(&prepared)?;
    let header = studio_core::PortableProjectHeader::parse(prepared.root_bytes())?;
    let original_ref = header.source_original().ok_or("original baseline")?;
    assert_eq!(captured.0.get(original_ref.sha256()), Some(&original));
    Ok(())
}

#[test]
fn imported_ancestor_only_trash_blocks_hidden_descendant_mutations_until_restore() -> TestResult {
    let seed = Workspace::memory()?;
    let seed_project = seed.create_project("Legacy branch trash")?;
    let root = seed.create_project_tree_node(&seed_project.id, 1, None, "folder", "Draft")?;
    let nested = seed.create_project_tree_node(
        &seed_project.id,
        1,
        Some(&root.logical_id),
        "folder",
        "Chapter",
    )?;
    let scene = seed.create_project_tree_node_with_format(
        &seed_project.id,
        1,
        Some(&nested.logical_id),
        "document",
        "Saved scene",
        studio_core::InitialManuscript::Plain,
    )?;
    seed.save_document(
        scene
            .document_id
            .as_deref()
            .ok_or("missing seed document")?,
        1,
        "The old draft remains recoverable.",
    )?;
    let mut manifest = seed
        .project_export_snapshot(&seed_project.id)?
        .manifest()
        .clone();
    let nodes = manifest["nodes"].as_array_mut().ok_or("nodes")?;
    nodes
        .iter_mut()
        .find(|node| node["id"] == root.logical_id)
        .ok_or("root")?["trashedAt"] = json!("2026-11-09T01:00:00Z");
    nodes.push(json!({
        "id":"empty-scene", "projectId":seed_project.id,
        "parentId":nested.logical_id, "kind":"document", "title":"Unwritten scene",
        "status":"draft", "position":2000, "currentRevision":0,
        "createdAt":"2026-11-09T00:00:00Z", "updatedAt":"2026-11-09T00:00:00Z"
    }));
    let mut importer = ProjectImportWorker::memory()?;
    let imported = importer.import(
        &ValidatedProjectImport::parse(&serde_json::to_vec(&manifest)?)?,
        &mut NoAssets,
        &ProjectTransferCancellation::default(),
    )?;
    let workspace = importer.workspace();
    let project = &imported.project_id;
    let before = workspace.initialize_project_tree(project)?;
    let scene = node(workspace, project, &scene.logical_id)?;
    let empty = node(workspace, project, "empty-scene")?;
    let document = workspace.document(scene.document_id.as_deref().ok_or("missing scene")?)?;
    let revisions = workspace.revisions(&document.id)?;
    assert!(scene.metadata["trashedAt"].is_null());
    assert!(empty.metadata["trashedAt"].is_null());

    assert!(matches!(
        workspace.initialize_project_tree_node_with_format(
            project,
            &empty,
            studio_core::InitialManuscript::Plain
        ),
        Err(StudioError::Validation(_))
    ));
    assert!(matches!(
        workspace.rename_project_tree_node(project, &scene, "Hidden rename"),
        Err(StudioError::Validation(_))
    ));
    assert!(matches!(
        workspace.move_project_tree_node(project, &scene, None, None),
        Err(StudioError::Validation(_))
    ));
    assert!(matches!(
        workspace.trash_project_tree_node(project, &scene),
        Err(StudioError::Validation(_))
    ));
    assert_eq!(workspace.initialize_project_tree(project)?, before);
    assert_eq!(workspace.document(&document.id)?, document);
    assert_eq!(workspace.revisions(&document.id)?, revisions);

    workspace.restore_project_tree_node(project, &node(workspace, project, &root.logical_id)?)?;
    let renamed = workspace.rename_project_tree_node(project, &scene, "Recovered scene")?;
    assert_eq!(renamed.metadata["title"], "Recovered scene");
    let initialized = workspace.initialize_project_tree_node_with_format(
        project,
        &empty,
        studio_core::InitialManuscript::Plain,
    )?;
    assert!(initialized.rich_json.is_none());
    assert_eq!(workspace.document(&document.id)?.body, document.body);
    assert_eq!(workspace.revisions(&document.id)?, revisions);
    Ok(())
}
