#![expect(
    clippy::unwrap_used,
    reason = "Synthetic structure contracts fail immediately"
)]
//! Exact imported-schema authoring, atomic refusal and immutable original preservation.
use std::collections::BTreeMap;
use std::io::{Cursor, Read};

use serde_json::{Value, json};
use studio_core::{
    ImportAssetSource, PortableObjectReference, PortableObjectSource, PortableProjectHeader,
    PreparedPortableProject, ProjectAttachment, ProjectImportWorker, ProjectPortableExportWorker,
    ProjectTransferCancellation, ProjectTreeNode, StudioError, ValidatedProjectImport,
    VerifiedPortableProjectStream, Workspace,
};
struct NoAssets;
impl ImportAssetSource for NoAssets {
    fn open<'a>(&'a mut self, _: &ProjectAttachment) -> std::io::Result<Box<dyn Read + 'a>> {
        Err(std::io::Error::other(
            "This synthetic structure has no attachments",
        ))
    }
}
fn source(trashed: bool) -> Vec<u8> {
    let workspace = Workspace::memory().unwrap();
    let project = workspace.create_project("Custom 作者").unwrap();
    workspace
        .create_document(&project.id, "Unchanged seed", "Keep this text")
        .unwrap();
    let mut manifest = workspace
        .project_export_snapshot(&project.id)
        .unwrap()
        .manifest()
        .clone();
    manifest["project"]["structure"]["vendor"] = json!({"keep":[true,null,"custom"]});
    let types = manifest["project"]["structure"]["nodeTypes"]
        .as_array_mut()
        .unwrap();
    types.push(json!({"key":"section-custom","label":"Section","pluralLabel":"Sections","icon":"folder","contentBearing":false,"canBeRoot":true,"allowedChildren":["chapter-custom"]}));
    types.push(json!({"key":"chapter-custom","label":"Chapter","pluralLabel":"Chapters","icon":"file","contentBearing":true,"canBeRoot":false,"allowedChildren":[]}));
    let nodes = manifest["nodes"].as_array_mut().unwrap();
    nodes.push(json!({"id":"section","projectId":project.id,"kind":"section-custom","title":"Imported section","status":"idea","position":1000,"currentRevision":0,"createdAt":"2026-11-09T00:00:00Z","updatedAt":"2026-11-09T00:00:00Z","trashedAt":trashed.then_some("2026-11-09T01:00:00Z"),"vendor":{"keep":[1,null]}}));
    nodes.push(json!({"id":"empty","projectId":project.id,"parentId":"section","kind":"chapter-custom","title":"Empty imported chapter","status":"idea","position":1000,"currentRevision":0,"createdAt":"2026-11-09T00:00:00Z","updatedAt":"2026-11-09T00:00:00Z","vendor":{"keep":[1,null,"😀"]}}));
    serde_json::to_vec_pretty(&manifest).unwrap()
}
fn import(database: &std::path::Path, trashed: bool) -> (ProjectImportWorker, String, Vec<u8>) {
    let source = source(trashed);
    let validated = ValidatedProjectImport::parse(&source).unwrap();
    let mut worker = ProjectImportWorker::open(database).unwrap();
    let project = worker
        .import(
            &validated,
            &mut NoAssets,
            &ProjectTransferCancellation::default(),
        )
        .unwrap();
    (worker, project.project_id, source)
}
fn node(workspace: &Workspace, project: &str, logical: &str) -> ProjectTreeNode {
    workspace
        .project_tree(project)
        .unwrap()
        .into_iter()
        .find(|node| node.logical_id == logical)
        .unwrap()
}
#[test]
fn create_custom_section_and_child_should_follow_saved_schema_without_touching_other_text() {
    let directory = tempfile::tempdir().unwrap();
    let (worker, project, _) = import(&directory.path().join("create.sqlite"), false);
    let workspace = worker.workspace();
    let before = workspace.list_documents(&project).unwrap();
    let structure = workspace
        .imported_project_structure(&project)
        .unwrap()
        .unwrap();
    assert_eq!(
        structure.schema["vendor"],
        json!({"keep":[true,null,"custom"]})
    );
    let section = workspace
        .create_project_tree_node(
            &project,
            structure.revision,
            None,
            "section-custom",
            "  New section  ",
        )
        .unwrap();
    assert!(section.document_id.is_none());
    assert_eq!(section.metadata["currentRevision"], 0);
    assert_eq!(section.metadata["title"], "New section");
    let child = workspace
        .create_project_tree_node(
            &project,
            structure.revision,
            Some(&section.logical_id),
            "chapter-custom",
            "Chapter 日本語",
        )
        .unwrap();
    assert_eq!(child.metadata["parentId"], section.logical_id);
    assert_eq!(child.metadata["kind"], "chapter-custom");
    let document = workspace
        .document(child.document_id.as_deref().unwrap())
        .unwrap();
    assert_eq!(document.revision, 1);
    assert_eq!(document.body, "");
    assert!(document.rich_json.is_some());
    assert_eq!(workspace.revisions(&document.id).unwrap().len(), 1);
    assert_eq!(workspace.rich_revisions(&document.id).unwrap().len(), 1);
    assert_eq!(workspace.document(&before[0].id).unwrap(), before[0]);
    assert_eq!(
        workspace
            .resolve_project_resource(&project, "node", &child.logical_id)
            .unwrap(),
        document.id
    );
    assert_eq!(
        workspace
            .imported_project_structure(&project)
            .unwrap()
            .unwrap(),
        structure
    );
    ValidatedProjectImport::parse(
        workspace
            .project_export_snapshot(&project)
            .unwrap()
            .manifest_bytes(),
    )
    .unwrap();
}
#[test]
fn invalid_or_stale_placement_should_not_partially_insert_nodes_or_text() {
    let directory = tempfile::tempdir().unwrap();
    let (worker, project, _) = import(&directory.path().join("refuse.sqlite"), false);
    let workspace = worker.workspace();
    let revision = workspace
        .imported_project_structure(&project)
        .unwrap()
        .unwrap()
        .revision;
    let before_nodes = workspace.project_tree(&project).unwrap();
    let before_docs = workspace.list_documents(&project).unwrap();
    assert!(matches!(
        workspace.create_project_tree_node(
            &project,
            revision + 1,
            None,
            "section-custom",
            "Wrong revision"
        ),
        Err(StudioError::ProjectStructureConflict { .. })
    ));
    for (parent, kind, title) in [
        (None, "chapter-custom", "Wrong root"),
        (Some("section"), "section-custom", "Wrong child"),
        (
            Some("not-in-this-project"),
            "chapter-custom",
            "Foreign parent",
        ),
        (None, "unknown", "Unknown"),
        (None, "section-custom", "   "),
    ] {
        assert!(
            workspace
                .create_project_tree_node(&project, revision, parent, kind, title)
                .is_err()
        );
        assert_eq!(workspace.project_tree(&project).unwrap(), before_nodes);
        assert_eq!(workspace.list_documents(&project).unwrap(), before_docs);
    }
}
#[test]
fn trashed_or_contentless_nodes_should_not_acquire_text_or_children() {
    let directory = tempfile::tempdir().unwrap();
    let (worker, project, _) = import(&directory.path().join("trashed.sqlite"), true);
    let workspace = worker.workspace();
    let section = node(workspace, &project, "section");
    let revision = workspace
        .imported_project_structure(&project)
        .unwrap()
        .unwrap()
        .revision;
    let before = workspace.project_tree(&project).unwrap();
    assert!(
        workspace
            .create_project_tree_node(
                &project,
                revision,
                Some("section"),
                "chapter-custom",
                "Refused"
            )
            .is_err()
    );
    assert!(
        workspace
            .initialize_project_tree_node(&project, &section)
            .is_err()
    );
    assert_eq!(workspace.project_tree(&project).unwrap(), before);
    let (other, other_project, _) = import(&directory.path().join("active.sqlite"), false);
    let workspace = other.workspace();
    let section = node(workspace, &other_project, "section");
    assert!(
        workspace
            .initialize_project_tree_node(&other_project, &section)
            .is_err()
    );
    assert_eq!(workspace.list_documents(&other_project).unwrap().len(), 1);
}
#[test]
fn first_content_should_reuse_reserved_identity_and_refuse_repeated_or_stale_initialization() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("first.sqlite");
    let (worker, project, _) = import(&path, false);
    let workspace = worker.workspace();
    let empty = node(workspace, &project, "empty");
    let reserved = workspace
        .resolve_project_resource(&project, "document", "empty")
        .unwrap();
    let mut stale = empty.clone();
    stale.metadata["title"] = json!("Forged old title");
    assert!(matches!(
        workspace.initialize_project_tree_node(&project, &stale),
        Err(StudioError::ProjectNodeChanged { .. })
    ));
    let document = workspace
        .initialize_project_tree_node(&project, &empty)
        .unwrap();
    assert_eq!(document.id, reserved);
    assert_eq!(document.project_id, project);
    assert_eq!(document.revision, 1);
    assert_eq!(document.body, "");
    let current = node(workspace, &project, "empty");
    assert_eq!(current.document_id.as_deref(), Some(document.id.as_str()));
    assert_eq!(current.metadata["vendor"], empty.metadata["vendor"]);
    assert_eq!(current.metadata["parentId"], empty.metadata["parentId"]);
    let other = Workspace::open(&path).unwrap();
    assert!(matches!(
        other.initialize_project_tree_node(&project, &empty),
        Err(StudioError::ProjectNodeChanged { .. })
    ));
    assert_eq!(other.project_tree(&project).unwrap().len(), 3);
    assert_eq!(other.revisions(&document.id).unwrap().len(), 1);
    assert_eq!(other.rich_revisions(&document.id).unwrap().len(), 1);
    assert_eq!(other.list_documents(&project).unwrap().len(), 2);
}
struct Objects(BTreeMap<String, Vec<u8>>);
impl PortableObjectSource for Objects {
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
fn objects(prepared: &PreparedPortableProject) -> Objects {
    let mut objects = BTreeMap::new();
    prepared
        .visit_objects(&mut |object| {
            let mut bytes = Vec::new();
            prepared.copy_object(object, &mut bytes, &ProjectTransferCancellation::default())?;
            objects.insert(object.sha256().to_owned(), bytes);
            Ok(())
        })
        .unwrap();
    Objects(objects)
}
#[test]
fn canonical_restore_should_keep_created_nodes_first_content_and_exact_original_baseline() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("roundtrip.sqlite");
    let (worker, project, original) = import(&path, false);
    let workspace = worker.workspace();
    let revision = workspace
        .imported_project_structure(&project)
        .unwrap()
        .unwrap()
        .revision;
    let folder = workspace
        .create_project_tree_node(&project, revision, None, "section-custom", "New folder")
        .unwrap();
    let child = workspace
        .create_project_tree_node(
            &project,
            revision,
            Some(&folder.logical_id),
            "chapter-custom",
            "New child",
        )
        .unwrap();
    let initialized = workspace
        .initialize_project_tree_node(&project, &node(workspace, &project, "empty"))
        .unwrap();
    let prepared = ProjectPortableExportWorker::open(&path)
        .unwrap()
        .prepare(&project, &ProjectTransferCancellation::default())
        .unwrap();
    let header = PortableProjectHeader::parse(prepared.root_bytes()).unwrap();
    let mut captured = objects(&prepared);
    let source = header.source_original().unwrap();
    assert_eq!(captured.0.get(source.sha256()).unwrap(), &original);
    let verified = VerifiedPortableProjectStream::preflight(
        prepared.root_bytes(),
        &mut captured,
        &ProjectTransferCancellation::default(),
    )
    .unwrap();
    let mut target = ProjectImportWorker::memory().unwrap();
    let restored = target
        .restore_stream(&verified, &ProjectTransferCancellation::default())
        .unwrap();
    assert_ne!(restored.project_id, project);
    let nodes = target
        .workspace()
        .project_tree(&restored.project_id)
        .unwrap();
    assert_eq!(nodes.len(), 5);
    let restored_child = nodes
        .iter()
        .find(|node| node.logical_id == child.logical_id)
        .unwrap();
    assert_eq!(restored_child.metadata["parentId"], folder.logical_id);
    assert_ne!(restored_child.document_id, child.document_id);
    let restored_empty = nodes
        .iter()
        .find(|node| node.logical_id == "empty")
        .unwrap();
    let document = target
        .workspace()
        .document(restored_empty.document_id.as_deref().unwrap())
        .unwrap();
    assert_ne!(document.id, initialized.id);
    assert_eq!(document.rich_json, initialized.rich_json);
    assert_eq!(document.revision, initialized.revision);
    assert_eq!(
        restored_empty.metadata["vendor"],
        json!({"keep":[1,null,"😀"]})
    );
}
