#![expect(
    clippy::unwrap_used,
    reason = "Synthetic pinned transport contracts fail immediately"
)]
//! Transport integrity precedes semantic import; external bytes cannot become replay authority.
use std::collections::BTreeMap;
use std::io::{Cursor, Read};

use craft_editor_model::Document as RichTree;
use sha2::{Digest, Sha256};
use studio_core::{
    PortableObjectReference, PortableObjectSource, PortableProjectHeader, PortableProjectScope,
    PortableShardDescriptor, ProjectPortableExportWorker, ProjectTransferCancellation,
    VerifiedPortableProjectStream, Workspace,
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
fn fixture() -> (Vec<u8>, Objects) {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite");
    let workspace = Workspace::open(&database).unwrap();
    let project = workspace.create_project("Preflight").unwrap();
    workspace
        .create_rich_document(
            &project.id,
            "Text",
            RichTree::plain("p", "Native 😀")
                .unwrap()
                .to_json()
                .unwrap(),
        )
        .unwrap();
    let mut worker = ProjectPortableExportWorker::open(&database).unwrap();
    let prepared = worker
        .prepare(&project.id, &ProjectTransferCancellation::default())
        .unwrap();
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
    (prepared.root_bytes().to_vec(), Objects(objects))
}
#[test]
fn all_objects_and_records_should_be_verified_before_returning_a_private_pinned_snapshot() {
    let (root, mut objects) = fixture();
    let verified = VerifiedPortableProjectStream::preflight(
        &root,
        &mut objects,
        &ProjectTransferCancellation::default(),
    )
    .unwrap();
    objects.0.clear(); // The original transport is no longer needed after successful preflight.
    assert_eq!(verified.root_bytes(), root);
    assert_eq!(verified.domain_record_count("project").unwrap(), 1);
    assert_eq!(verified.domain_record_count("nodes").unwrap(), 1);
    assert_eq!(
        verified.domain_record_count("documentRevisions").unwrap(),
        1
    );
    assert_eq!(verified.record_count(), 3);
    assert_eq!(verified.domain_record_count("missing").unwrap(), 0);
}
#[test]
fn changed_object_bytes_and_missing_known_references_should_refuse_preflight() {
    let (root, mut objects) = fixture();
    let bytes = objects
        .0
        .values_mut()
        .find(|bytes| !bytes.is_empty())
        .unwrap();
    bytes[0] ^= 1;
    assert!(
        VerifiedPortableProjectStream::preflight(
            &root,
            &mut objects,
            &ProjectTransferCancellation::default()
        )
        .is_err()
    );
    let raw = format!(
        "{{\"logicalDocumentId\":\"doc\",\"body\":{{\"sha256\":\"{}\",\"byteSize\":3}}}}\n",
        "f".repeat(64)
    );
    let sha = format!("{:x}", Sha256::digest(raw.as_bytes()));
    let shard = PortableShardDescriptor::new(
        "documentRevisions",
        PortableObjectReference::new(&sha, raw.len() as u64).unwrap(),
        1,
    )
    .unwrap();
    let root = PortableProjectHeader::new(PortableProjectScope::CompleteProject, None, vec![shard])
        .unwrap()
        .to_bytes()
        .unwrap();
    let mut objects = Objects(BTreeMap::from([(sha, raw.into_bytes())]));
    assert!(
        VerifiedPortableProjectStream::preflight(
            &root,
            &mut objects,
            &ProjectTransferCancellation::default()
        )
        .is_err()
    );
}
struct CancelSource {
    objects: Objects,
    cancel: ProjectTransferCancellation,
}
struct CancelReader<'a> {
    bytes: Cursor<&'a [u8]>,
    cancel: ProjectTransferCancellation,
}
impl Read for CancelReader<'_> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        let count = self.bytes.read(output)?;
        if count > 0 {
            self.cancel.cancel();
        }
        Ok(count)
    }
}
impl PortableObjectSource for CancelSource {
    fn verify_transport(&mut self) -> std::io::Result<()> {
        Ok(())
    }
    fn visit_objects(
        &mut self,
        visitor: &mut dyn FnMut(PortableObjectReference) -> studio_core::Result<()>,
    ) -> studio_core::Result<()> {
        self.objects.visit_objects(visitor)
    }
    fn open<'a>(
        &'a mut self,
        object: &PortableObjectReference,
    ) -> std::io::Result<Box<dyn Read + 'a>> {
        let bytes = self
            .objects
            .0
            .get(object.sha256())
            .ok_or_else(|| std::io::Error::other("Missing synthetic object"))?;
        Ok(Box::new(CancelReader {
            bytes: Cursor::new(bytes),
            cancel: self.cancel.clone(),
        }))
    }
}
#[test]
fn cancellation_during_object_copy_should_expose_no_partly_verified_snapshot() {
    let (root, objects) = fixture();
    let cancel = ProjectTransferCancellation::default();
    let mut source = CancelSource {
        objects,
        cancel: cancel.clone(),
    };
    assert!(VerifiedPortableProjectStream::preflight(&root, &mut source, &cancel).is_err());
}
