//! Complete canonical label state crosses both formats without changing the source manifest.
use std::collections::BTreeMap;
use std::io::{Cursor, Read};

use craft_editor_model::{Document as RichTree, ScalarOffset};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use studio_core::{
    ImportAssetSource, PortableObjectReference, PortableObjectSource, PreparedPortableProject,
    ProjectAttachment, ProjectImportWorker, ProjectLabelColor, ProjectPortableExportWorker,
    ProjectTransferCancellation, ValidatedPortableProjectImport, ValidatedProjectImport,
    VerifiedPortableProjectStream, Workspace,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

struct NoAssets;
impl ImportAssetSource for NoAssets {
    fn open<'a>(&'a mut self, _: &ProjectAttachment) -> std::io::Result<Box<dyn Read + 'a>> {
        Err(std::io::Error::other(
            "No assets in this authored label fixture",
        ))
    }
}

#[derive(Clone)]
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
        Ok(Box::new(Cursor::new(
            self.0
                .get(object.sha256())
                .ok_or_else(|| std::io::Error::other("Missing fixture object"))?
                .as_slice(),
        )))
    }
}

fn objects(prepared: &PreparedPortableProject) -> studio_core::Result<Objects> {
    let mut output = BTreeMap::new();
    prepared.visit_objects(&mut |reference| {
        let mut bytes = Vec::new();
        prepared.copy_object(
            reference,
            &mut bytes,
            &ProjectTransferCancellation::default(),
        )?;
        output.insert(reference.sha256().into(), bytes);
        Ok(())
    })?;
    Ok(Objects(output))
}

fn fixture(path: &std::path::Path) -> TestResult<(ProjectImportWorker, String, Vec<u8>)> {
    let seed = Workspace::memory()?;
    let project = seed.create_project("The Last Light")?;
    let document = seed.create_rich_document(
        &project.id,
        "Night watch",
        RichTree::plain("p", "The light remains.")?.to_json()?,
    )?;
    let mut manifest = seed
        .project_export_snapshot(&project.id)?
        .manifest()
        .clone();
    manifest["vendor"] = json!({"keep":"original manifest"});
    let original = serde_json::to_vec_pretty(&manifest)?;
    let mut worker = ProjectImportWorker::open(path)?;
    let imported = worker.import(
        &ValidatedProjectImport::parse(&original)?,
        &mut NoAssets,
        &ProjectTransferCancellation::default(),
    )?;
    let workspace = worker.workspace();
    let project = imported.project_id;
    let document = workspace.resolve_project_resource(&project, "document", &document.id)?;
    let mut labels = workspace.project_labels(&project)?;
    for (name, color) in [
        ("Asya", ProjectLabelColor::Rose),
        ("Unused archive", ProjectLabelColor::Gray),
        ("Retired question", ProjectLabelColor::Orange),
    ] {
        labels = workspace.create_project_label(&labels, name, color)?;
    }
    workspace.remove_project_tag(&workspace.project_tag_snapshot(&project, "Retired question")?)?;
    let anchor = workspace.capture_annotation_anchor(
        &document,
        1,
        "p",
        ScalarOffset(4),
        ScalarOffset(9),
        "light",
    )?;
    workspace.create_tagged_annotation(
        &document,
        1,
        anchor,
        "Why does she keep the log?",
        &["Asya".into()],
    )?;
    let raw = Connection::open(path)?;
    let mut catalog: Value = serde_json::from_str(&raw.query_row(
        "SELECT raw_json FROM project_label_catalog WHERE project_id=?1",
        [&project],
        |row| row.get::<_, String>(0),
    )?)?;
    catalog["vendor"] = json!({"retain":"catalog extension"});
    // This valid extension is larger than a metadata record: it belongs in a bounded object.
    catalog["labels"][0]["vendor"] =
        json!({"retain":"x".repeat(studio_core::MAX_PORTABLE_RECORD_BYTES+1)});
    raw.execute(
        "UPDATE project_label_catalog SET raw_json=?1 WHERE project_id=?2",
        params![catalog.to_string(), project],
    )?;
    Ok((worker, project, original))
}

fn raw_catalog(workspace: &Workspace, project: &str) -> studio_core::Result<Value> {
    Ok(workspace.project_export_snapshot(project)?.native_state()["projectLabels"].clone())
}

#[test]
fn complete_v1_and_v2_keep_unused_retired_colors_extensions_and_original_source_bytes() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("source.sqlite");
    let (source, project, original) = fixture(&path)?;
    let catalog = raw_catalog(source.workspace(), &project)?;
    let snapshot = source.workspace().project_export_snapshot(&project)?;
    assert!(snapshot.requires_canonical_companion());
    let companion = snapshot.canonical_manifest_bytes()?;
    let mut legacy_target = ProjectImportWorker::memory()?;
    let restored = legacy_target.restore_portable(
        &ValidatedPortableProjectImport::parse_v1(&companion)?,
        &mut NoAssets,
        &ProjectTransferCancellation::default(),
    )?;
    assert_eq!(
        raw_catalog(legacy_target.workspace(), &restored.project_id)?,
        catalog
    );
    let notes = legacy_target
        .workspace()
        .project_annotations(&restored.project_id, true)?;
    assert_eq!(notes[0].annotation.tags, ["Asya"]);
    assert_eq!(notes[0].annotation.anchor.exact_text, "light");
    assert_eq!(
        legacy_target
            .workspace()
            .annotation_history(&notes[0].annotation.id)?
            .len(),
        1
    );
    let mut exporter = ProjectPortableExportWorker::open(&path)?;
    let prepared = exporter.prepare(&project, &ProjectTransferCancellation::default())?;
    let stream = VerifiedPortableProjectStream::preflight(
        prepared.root_bytes(),
        &mut objects(&prepared)?,
        &ProjectTransferCancellation::default(),
    )?;
    let target_path = directory.path().join("target.sqlite");
    let mut target = ProjectImportWorker::open(&target_path)?;
    let restored = target.restore_stream(&stream, &ProjectTransferCancellation::default())?;
    assert_eq!(
        raw_catalog(target.workspace(), &restored.project_id)?,
        catalog
    );
    assert_eq!(
        target
            .workspace()
            .project_labels(&restored.project_id)?
            .retired_labels()[0]
            .color,
        ProjectLabelColor::Orange
    );
    for (path, project) in [(&path, &project), (&target_path, &restored.project_id)] {
        let raw = Connection::open(path)?;
        let saved: String = raw.query_row(
            "SELECT raw_manifest FROM project_portability WHERE project_id=?1",
            [project],
            |row| row.get(0),
        )?;
        assert_eq!(saved.as_bytes(), original);
    }
    drop(target);
    let reopened = Workspace::open(&target_path)?;
    assert_eq!(raw_catalog(&reopened, &restored.project_id)?, catalog);

    // An older companion has no project-level colors; read its exact note names as Blue.
    let mut old: Value = serde_json::from_slice(&companion)?;
    old["nativeState"]
        .as_object_mut()
        .ok_or("state")?
        .remove("projectLabels");
    let restored = legacy_target.restore_portable(
        &ValidatedPortableProjectImport::parse_v1(&serde_json::to_vec(&old)?)?,
        &mut NoAssets,
        &ProjectTransferCancellation::default(),
    )?;
    let labels = legacy_target
        .workspace()
        .project_labels(&restored.project_id)?;
    assert_eq!(labels.labels().len(), 1);
    assert_eq!(labels.labels()[0].name, "Asya");
    assert_eq!(labels.labels()[0].color, ProjectLabelColor::Blue);
    Ok(())
}

fn project_record(root: &Value, objects: &Objects) -> TestResult<Value> {
    let component = root["components"]
        .as_array()
        .ok_or("components")?
        .iter()
        .find(|component| component["domain"] == "project")
        .ok_or("project component")?;
    Ok(serde_json::from_slice(
        objects
            .0
            .get(component["sha256"].as_str().ok_or("sha")?)
            .ok_or("project shard")?,
    )?)
}

fn replace_project_record(root: &mut Value, objects: &mut Objects, record: &Value) -> TestResult {
    let component = root["components"]
        .as_array_mut()
        .ok_or("components")?
        .iter_mut()
        .find(|component| component["domain"] == "project")
        .ok_or("project component")?;
    let old = component["sha256"].as_str().ok_or("sha")?.to_owned();
    let mut bytes = serde_json::to_vec(record)?;
    bytes.push(b'\n');
    let sha = format!("{:x}", Sha256::digest(&bytes));
    component["sha256"] = json!(sha);
    component["byteSize"] = json!(bytes.len());
    objects.0.remove(&old);
    objects.0.insert(sha, bytes);
    Ok(())
}

#[test]
fn bad_catalog_objects_and_semantics_refuse_before_a_partial_project_is_committed() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("source.sqlite");
    let (_source, project, _) = fixture(&path)?;
    let prepared = ProjectPortableExportWorker::open(&path)?
        .prepare(&project, &ProjectTransferCancellation::default())?;
    let original = objects(&prepared)?;
    let root: Value = serde_json::from_slice(prepared.root_bytes())?;
    let record = project_record(&root, &original)?;
    let catalog_sha = record["labelCatalog"]["sha256"]
        .as_str()
        .ok_or("catalog sha")?;
    let mut missing = original.clone();
    missing.0.remove(catalog_sha);
    assert!(
        VerifiedPortableProjectStream::preflight(
            prepared.root_bytes(),
            &mut missing,
            &ProjectTransferCancellation::default()
        )
        .is_err()
    );
    let mut corrupt = original.clone();
    corrupt.0.get_mut(catalog_sha).ok_or("catalog object")?[0] = b'!';
    assert!(
        VerifiedPortableProjectStream::preflight(
            prepared.root_bytes(),
            &mut corrupt,
            &ProjectTransferCancellation::default()
        )
        .is_err()
    );
    let mut oversized = record.clone();
    oversized["labelCatalog"]["byteSize"] = json!(studio_core::MAX_PROJECT_MANIFEST_BYTES + 1);
    let mut changed_root = root.clone();
    let mut changed_objects = original.clone();
    replace_project_record(&mut changed_root, &mut changed_objects, &oversized)?;
    assert!(
        VerifiedPortableProjectStream::preflight(
            &serde_json::to_vec(&changed_root)?,
            &mut changed_objects,
            &ProjectTransferCancellation::default()
        )
        .is_err()
    );

    let mut target = ProjectImportWorker::memory()?;
    for omit_assignments in [false, true] {
        let mut catalog: Value =
            serde_json::from_slice(original.0.get(catalog_sha).ok_or("catalog object")?)?;
        if omit_assignments {
            catalog["labels"] = json!([]);
        } else {
            catalog["labels"][0]["color"] = json!("unsupported-color");
        }
        let bytes = serde_json::to_vec(&catalog)?;
        let sha = format!("{:x}", Sha256::digest(&bytes));
        let mut record = record.clone();
        record["labelCatalog"] = json!({"sha256":sha,"byteSize":bytes.len()});
        let mut objects = original.clone();
        objects.0.remove(catalog_sha);
        objects.0.insert(sha, bytes);
        let mut root = root.clone();
        replace_project_record(&mut root, &mut objects, &record)?;
        let verified = VerifiedPortableProjectStream::preflight(
            &serde_json::to_vec(&root)?,
            &mut objects,
            &ProjectTransferCancellation::default(),
        )?;
        assert!(
            target
                .restore_stream(&verified, &ProjectTransferCancellation::default())
                .is_err()
        );
        assert!(target.workspace().list_projects()?.is_empty());
    }
    Ok(())
}
