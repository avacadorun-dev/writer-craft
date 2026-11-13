#![expect(
    clippy::unwrap_used,
    reason = "Synthetic format2 contracts fail immediately"
)]
//! Transport verification is separate from semantic import and sealed replay authority.
use std::io::{Cursor, Read};

use sha2::{Digest, Sha256};
use studio_core::{
    MAX_PORTABLE_MATERIALIZED_BYTES, MAX_PORTABLE_RECORD_BYTES, MAX_PORTABLE_ROOT_BYTES,
    MAX_PORTABLE_SHARD_RECORDS, MAX_PORTABLE_TRANSPORT_BYTES, PortableObjectReference,
    PortableProjectHeader, PortableProjectScope, PortableResourceBudget, PortableShardDescriptor,
    ProjectTransferCancellation, visit_portable_shard,
};

fn descriptor(bytes: &[u8], records: u64) -> PortableShardDescriptor {
    PortableShardDescriptor::new(
        "unknown.synthetic",
        PortableObjectReference::new(
            &format!("{:x}", Sha256::digest(bytes)),
            u64::try_from(bytes.len()).unwrap(),
        )
        .unwrap(),
        records,
    )
    .unwrap()
}
#[test]
fn small_root_should_keep_unknown_raw_fields_and_exact_numeric_lexemes() {
    let root = format!(
        r#"{{"format":"writercraft-portable-project","formatVersion":2,"scope":"complete-project","sourceOriginal":{{"sha256":"{}","byteSize":12,"sourceVendor":{{ "precise":1.234567890123456789e+100 }}}},"components":[{{"domain":"unknown.synthetic","sha256":"{}","byteSize":0,"recordCount":0,"futureShard":{{ "n":18446744073709551616000123 }}}}],"futureRoot":{{"$serde_json::private::Number":"opaque"}}}}"#,
        "a".repeat(64),
        "b".repeat(64)
    );
    let parsed = PortableProjectHeader::parse(root.as_bytes()).unwrap();
    assert_eq!(parsed.scope(), PortableProjectScope::CompleteProject);
    let encoded = String::from_utf8(parsed.to_bytes().unwrap()).unwrap();
    for raw in [
        r#"{ "precise":1.234567890123456789e+100 }"#,
        r#"{ "n":18446744073709551616000123 }"#,
        r#"{"$serde_json::private::Number":"opaque"}"#,
    ] {
        assert!(encoded.contains(raw), "{encoded}");
    }
    PortableProjectHeader::parse(encoded.as_bytes()).unwrap();
}
#[test]
fn selected_scope_should_never_embed_a_complete_original_source_or_accept_duplicate_keys() {
    let source = PortableObjectReference::new(&"a".repeat(64), 12).unwrap();
    assert!(
        PortableProjectHeader::new(
            PortableProjectScope::SelectedDocuments,
            Some(source),
            vec![]
        )
        .is_err()
    );
    let root=br#"{"format":"writercraft-portable-project","formatVersion":2,"scope":"selected-documents","components":[],"duplicate":1,"duplicate":2}"#;
    assert!(PortableProjectHeader::parse(root).is_err());
    assert!(PortableProjectHeader::parse(&vec![b' '; MAX_PORTABLE_ROOT_BYTES + 1]).is_err());
    assert!(PortableObjectReference::new(&"A".repeat(64), 1).is_err());
    let too_large_source =
        PortableObjectReference::new(&"a".repeat(64), 64 * 1024 * 1024 + 1).unwrap();
    assert!(
        PortableProjectHeader::new(
            PortableProjectScope::CompleteProject,
            Some(too_large_source),
            vec![]
        )
        .is_err()
    );
}
#[test]
fn metadata_shards_should_verify_every_record_before_visiting_and_keep_raw_record_framing() {
    let bytes = b" {\"id\":\"one\",\"precise\":1.234567890123456789e+100}\n{\"id\":\"two\"}\n";
    let mut records = Vec::new();
    visit_portable_shard(
        &descriptor(bytes, 2),
        &mut Cursor::new(bytes),
        &ProjectTransferCancellation::default(),
        &mut |record| {
            records.push(record.raw_json().to_owned());
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(
        records[0],
        " {\"id\":\"one\",\"precise\":1.234567890123456789e+100}"
    );
    assert_eq!(records[1], "{\"id\":\"two\"}");
    let last_without_newline = b"{\"id\":\"one\"}\n{\"id\":\"two\"}";
    let mut seen = Vec::new();
    visit_portable_shard(
        &descriptor(last_without_newline, 2),
        &mut Cursor::new(last_without_newline),
        &ProjectTransferCancellation::default(),
        &mut |record| {
            seen.push(record.raw_json().to_owned());
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(seen, vec!["{\"id\":\"one\"}", "{\"id\":\"two\"}"]);
}
#[test]
fn corrupt_count_hash_duplicate_key_bad_record_or_extra_bytes_should_visit_nothing() {
    let valid = b"{\"id\":\"one\"}\n{\"id\":\"two\"}\n";
    let duplicate = b"{\"id\":\"one\"}\n{\"id\":\"bad\",\"id\":\"duplicate\"}\n";
    let scalar = b"{\"id\":\"one\"}\n123\n";
    for (bytes, desc) in [
        (valid.as_slice(), descriptor(valid, 1)),
        (duplicate.as_slice(), descriptor(duplicate, 2)),
        (scalar.as_slice(), descriptor(scalar, 2)),
        (
            valid.as_slice(),
            PortableShardDescriptor::new(
                "unknown",
                PortableObjectReference::new(&"0".repeat(64), u64::try_from(valid.len()).unwrap())
                    .unwrap(),
                2,
            )
            .unwrap(),
        ),
        (
            valid.as_slice(),
            PortableShardDescriptor::new(
                "unknown",
                PortableObjectReference::new(
                    &format!("{:x}", Sha256::digest(valid)),
                    u64::try_from(valid.len() - 1).unwrap(),
                )
                .unwrap(),
                2,
            )
            .unwrap(),
        ),
    ] {
        let mut visited = 0;
        assert!(
            visit_portable_shard(
                &desc,
                &mut Cursor::new(bytes),
                &ProjectTransferCancellation::default(),
                &mut |_| {
                    visited += 1;
                    Ok(())
                }
            )
            .is_err()
        );
        assert_eq!(visited, 0);
    }
    let oversized = format!(
        "{{\"large\":\"{}\"}}\n",
        "x".repeat(MAX_PORTABLE_RECORD_BYTES)
    );
    let mut visited = 0;
    assert!(
        visit_portable_shard(
            &descriptor(oversized.as_bytes(), 1),
            &mut Cursor::new(oversized.as_bytes()),
            &ProjectTransferCancellation::default(),
            &mut |_| {
                visited += 1;
                Ok(())
            }
        )
        .is_err()
    );
    assert_eq!(visited, 0);
    assert!(
        PortableShardDescriptor::new(
            "unknown",
            PortableObjectReference::new(&"a".repeat(64), 0).unwrap(),
            MAX_PORTABLE_SHARD_RECORDS + 1
        )
        .is_err()
    );
}
struct CancelReader {
    reader: Cursor<Vec<u8>>,
    cancel: ProjectTransferCancellation,
}
impl Read for CancelReader {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        let count = self.reader.read(bytes)?;
        if count > 0 {
            self.cancel.cancel();
        }
        Ok(count)
    }
}
#[test]
fn shard_cancellation_should_expose_no_partially_verified_records() {
    let bytes = b"{\"id\":\"one\"}\n";
    let cancel = ProjectTransferCancellation::default();
    let mut reader = CancelReader {
        reader: Cursor::new(bytes.to_vec()),
        cancel: cancel.clone(),
    };
    let mut visited = 0;
    assert!(
        visit_portable_shard(&descriptor(bytes, 1), &mut reader, &cancel, &mut |_| {
            visited += 1;
            Ok(())
        })
        .is_err()
    );
    assert_eq!(visited, 0);
}
#[test]
fn shared_transport_objects_should_not_bypass_independent_materialization_or_record_guards() {
    let mut budget = PortableResourceBudget::default();
    let shared = 16 * 1024 * 1024;
    budget.account_unique_transport(shared).unwrap();
    for _ in 0..64 {
        budget.account_materialized(shared).unwrap();
    }
    assert_eq!(budget.transport_bytes(), shared);
    assert_eq!(budget.materialized_bytes(), MAX_PORTABLE_MATERIALIZED_BYTES);
    assert!(budget.account_materialized(shared).is_err());
    assert_eq!(budget.materialized_bytes(), MAX_PORTABLE_MATERIALIZED_BYTES);
    budget.account_records(1_000_000).unwrap();
    assert!(budget.account_records(1).is_err());
    assert_eq!(budget.record_count(), 1_000_000);
    let mut transport = PortableResourceBudget::default();
    transport
        .account_unique_transport(MAX_PORTABLE_TRANSPORT_BYTES)
        .unwrap();
    assert!(transport.account_unique_transport(1).is_err());
    assert_eq!(transport.transport_bytes(), MAX_PORTABLE_TRANSPORT_BYTES);
}
#[test]
fn root_should_count_declared_shards_and_refuse_repeated_domain_pages() {
    let shard = descriptor(b"", 0);
    assert!(
        PortableProjectHeader::new(
            PortableProjectScope::CompleteProject,
            None,
            vec![shard.clone(), shard]
        )
        .is_err()
    );
    let records = PortableShardDescriptor::new(
        "a",
        PortableObjectReference::new(&"a".repeat(64), 0).unwrap(),
        4096,
    )
    .unwrap();
    let components = (0..245)
        .map(|index| {
            PortableShardDescriptor::new(
                &format!("domain{index}"),
                records.object().clone(),
                records.record_count(),
            )
            .unwrap()
        })
        .collect();
    assert!(
        PortableProjectHeader::new(PortableProjectScope::CompleteProject, None, components)
            .is_err()
    );
}

#[test]
fn physical_members_and_encoded_container_should_be_counted_independently_of_shared_hashes() {
    let mut budget = PortableResourceBudget::default();
    budget.account_unique_transport(20).unwrap();
    budget.account_physical_member(20).unwrap();
    budget.account_physical_member(20).unwrap();
    assert_eq!(budget.transport_bytes(), 20);
    assert_eq!(budget.expanded_member_bytes(), 40);
    assert_eq!(budget.member_count(), 2);
    budget.record_encoded_container_size(100).unwrap();
    assert!(budget.record_encoded_container_size(101).is_err());
    assert_eq!(budget.encoded_container_bytes(), Some(100));
    assert!(
        budget
            .record_encoded_container_size(MAX_PORTABLE_TRANSPORT_BYTES + 1)
            .is_err()
    );
    let mut members = PortableResourceBudget::default();
    for _ in 0..65_536 {
        members.account_physical_member(0).unwrap();
    }
    assert!(members.account_physical_member(1).is_err());
    assert_eq!(members.member_count(), 65_536);
    assert_eq!(members.expanded_member_bytes(), 0);
}
