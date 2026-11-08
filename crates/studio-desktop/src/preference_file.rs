//! Bounded preference-file acknowledgement, raw extension retention and atomic publication.
use std::{
    collections::{BTreeMap, HashSet},
    fmt,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use serde::{
    Serialize,
    de::{Deserialize, DeserializeOwned, Deserializer, MapAccess, SeqAccess, Visitor},
};
use serde_json::value::RawValue;

pub(crate) const MAX_PREFERENCE_BYTES: u64 = 64 * 1024;

#[derive(Clone, PartialEq, Eq)]
enum AcknowledgedFile {
    Missing,
    Exact(Vec<u8>),
    /// Invalid or unreadable input never grants authority to replace it with defaults.
    Blocked,
}
#[derive(Clone)]
struct DesiredDocument {
    raw: Box<RawValue>,
    bytes: Vec<u8>,
}
struct PreparedWrite {
    destination: PathBuf,
    acknowledged: AcknowledgedFile,
    file: tempfile::NamedTempFile,
    desired: DesiredDocument,
}

/// One JSON file's desired state and last acknowledged source. This is not global filesystem CAS.
pub(crate) struct PreferenceFile {
    path: PathBuf,
    acknowledged: AcknowledgedFile,
    original: Option<Box<RawValue>>,
    desired: Option<DesiredDocument>,
    pending_write: bool,
    conflict: bool,
}
impl PreferenceFile {
    pub(crate) fn load(path: PathBuf) -> io::Result<Self> {
        let bytes = read_bounded(&path)?;
        let original = bytes.as_deref().map(parse_object).transpose()?;
        Ok(Self {
            path,
            acknowledged: bytes.map_or(AcknowledgedFile::Missing, AcknowledgedFile::Exact),
            original,
            desired: None,
            pending_write: false,
            conflict: false,
        })
    }
    pub(crate) fn blocked(path: PathBuf) -> Self {
        Self {
            path,
            acknowledged: AcknowledgedFile::Blocked,
            original: None,
            desired: None,
            pending_write: false,
            conflict: false,
        }
    }
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
    pub(crate) fn loaded_from_file(&self) -> bool {
        matches!(self.acknowledged, AcknowledgedFile::Exact(_))
    }
    pub(crate) fn decode<T: DeserializeOwned>(&self) -> io::Result<Option<T>> {
        self.original
            .as_ref()
            .map(|raw| serde_json::from_str(raw.get()).map_err(invalid))
            .transpose()
    }
    /// Flat legacy files have no version field; an explicit future version remains protected.
    fn validate_flat_version(&self) -> io::Result<()> {
        if let Some(raw) = &self.original {
            let fields: BTreeMap<String, Box<RawValue>> =
                serde_json::from_str(raw.get()).map_err(invalid)?;
            if let Some(version) = fields.get("version")
                && serde_json::from_str::<u32>(version.get()).map_err(invalid)? != 1
            {
                return Err(invalid("preference_file_version_unsupported"));
            }
        }
        Ok(())
    }
    /// Automatic saves stop on observed conflicts. Explicit retry can recheck the old acknowledgement.
    pub(crate) fn writable(&self) -> bool {
        self.retryable() && !self.conflict
    }
    pub(crate) fn retryable(&self) -> bool {
        !matches!(self.acknowledged, AcknowledgedFile::Blocked)
    }
    pub(crate) fn needs_attention(&self) -> bool {
        !self.retryable() || self.conflict || self.pending_write
    }
    pub(crate) fn has_pending_write(&self) -> bool {
        self.pending_write
    }
    pub(crate) fn source_preserved(&self) -> bool {
        !self.retryable() || self.conflict
    }

    pub(crate) fn mark_pending(&mut self) {
        self.pending_write = true;
        self.desired = None;
    }

    /// Queue only known typed fields, retaining all untouched raw JSON envelopes.
    pub(crate) fn queue<T: Serialize>(&mut self, values: &T) -> io::Result<()> {
        self.mark_pending();
        let patch = serde_json::value::to_raw_value(values).map_err(invalid)?;
        let raw = merge_known(self.original.as_deref(), &patch)?;
        let bytes = match (&self.acknowledged, self.original.as_ref()) {
            (AcknowledgedFile::Exact(bytes), Some(original)) if original.get() == raw.get() => {
                bytes.clone()
            }
            (AcknowledgedFile::Exact(bytes), _) => preserve_framing(bytes, raw.get().as_bytes()),
            _ => raw.get().as_bytes().to_vec(),
        };
        if bytes.len() as u64 > MAX_PREFERENCE_BYTES {
            return Err(invalid("preference_file_too_large"));
        }
        self.desired = Some(DesiredDocument { raw, bytes });
        Ok(())
    }
    pub(crate) fn publish_pending(&mut self) -> io::Result<()> {
        if let Some(prepared) = self.prepare_publication()? {
            self.publish_prepared(prepared)?;
        }
        Ok(())
    }
    fn verify_acknowledgement(&mut self) -> io::Result<()> {
        if !self.retryable() {
            return Err(invalid("preference_file_preserved_requires_reload"));
        }
        let current = match read_bounded(&self.path) {
            Ok(current) => current,
            Err(error) => {
                if error.kind() == io::ErrorKind::InvalidData {
                    self.conflict = true;
                }
                return Err(error);
            }
        };
        let matches = match (&self.acknowledged, current.as_deref()) {
            (AcknowledgedFile::Missing, None) => true,
            (AcknowledgedFile::Exact(expected), Some(actual)) => expected.as_slice() == actual,
            _ => false,
        };
        if !matches {
            self.conflict = true;
            return Err(invalid("preference_file_changed_externally"));
        }
        Ok(())
    }
    fn prepare_publication(&mut self) -> io::Result<Option<PreparedWrite>> {
        self.verify_acknowledgement()?;
        let Some(desired) = &self.desired else {
            return if self.pending_write {
                Err(invalid("preference_values_could_not_be_prepared"))
            } else {
                Ok(None)
            };
        };
        if let AcknowledgedFile::Exact(bytes) = &self.acknowledged
            && *bytes == desired.bytes
        {
            self.desired = None;
            self.pending_write = false;
            self.conflict = false;
            return Ok(None);
        }
        let parent = self
            .path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        file.write_all(&desired.bytes)?;
        file.as_file().sync_all()?;
        Ok(Some(PreparedWrite {
            destination: self.path.clone(),
            acknowledged: self.acknowledged.clone(),
            file,
            desired: desired.clone(),
        }))
    }
    fn publish_prepared(&mut self, prepared: PreparedWrite) -> io::Result<()> {
        if prepared.destination != self.path
            || prepared.acknowledged != self.acknowledged
            || self
                .desired
                .as_ref()
                .is_none_or(|desired| desired.bytes != prepared.desired.bytes)
        {
            return Err(invalid(
                "preference_publication_no_longer_matches_desired_state",
            ));
        }
        // Stage/flush can take time. Refuse a change observed at the last publication boundary.
        // Rename is atomic publication, not compare-and-swap against noncooperating writers.
        self.verify_acknowledgement()?;
        let result = match self.acknowledged {
            AcknowledgedFile::Missing => prepared.file.persist_noclobber(&self.path),
            AcknowledgedFile::Exact(_) => prepared.file.persist(&self.path),
            AcknowledgedFile::Blocked => {
                return Err(invalid("preference_file_preserved_requires_reload"));
            }
        };
        if let Err(error) = result {
            if error.error.kind() == io::ErrorKind::AlreadyExists {
                self.conflict = true;
            }
            return Err(error.error);
        }
        self.acknowledged = AcknowledgedFile::Exact(prepared.desired.bytes);
        self.original = Some(prepared.desired.raw);
        self.desired = None;
        self.pending_write = false;
        self.conflict = false;
        Ok(())
    }
}

/// Typed validation for the two existing flat preference formats.
pub(crate) trait FlatPreferences: Serialize + DeserializeOwned + Default {
    fn normalize(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub(crate) struct FlatPreferenceStore<T: FlatPreferences> {
    pub(crate) values: T,
    file: PreferenceFile,
}
impl<T: FlatPreferences> FlatPreferenceStore<T> {
    pub(crate) fn load(path: PathBuf) -> io::Result<Self> {
        let file = PreferenceFile::load(path)?;
        file.validate_flat_version()?;
        let mut values: T = file.decode()?.unwrap_or_default();
        T::normalize(&mut values)?;
        Ok(Self { values, file })
    }
    pub(crate) fn blocked(path: PathBuf) -> Self {
        Self {
            values: T::default(),
            file: PreferenceFile::blocked(path),
        }
    }
    pub(crate) fn loaded_from_file(&self) -> bool {
        self.file.loaded_from_file()
    }
    pub(crate) fn writable(&self) -> bool {
        self.file.writable()
    }
    pub(crate) fn retryable(&self) -> bool {
        self.file.retryable()
    }
    pub(crate) fn needs_attention(&self) -> bool {
        self.file.needs_attention()
    }
    pub(crate) fn has_pending_write(&self) -> bool {
        self.file.has_pending_write()
    }
    pub(crate) fn source_preserved(&self) -> bool {
        self.file.source_preserved()
    }
    pub(crate) fn queue(&mut self) -> io::Result<()> {
        self.file.mark_pending();
        self.values.normalize()?;
        self.file.queue(&self.values)
    }
    pub(crate) fn save(&mut self) -> io::Result<()> {
        self.queue()?;
        self.file.publish_pending()
    }
    pub(crate) fn reload(&mut self) -> io::Result<()> {
        let fresh = Self::load(self.file.path().to_owned())?;
        if !fresh.loaded_from_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "no_saved_preferences_to_reload",
            ));
        }
        *self = fresh;
        Ok(())
    }
}

fn invalid(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}
fn read_bounded(path: &Path) -> io::Result<Option<Vec<u8>>> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if !metadata.file_type().is_file() {
        return Err(invalid("preference_file_is_not_a_regular_file"));
    }
    if metadata.len() > MAX_PREFERENCE_BYTES {
        return Err(invalid("preference_file_too_large"));
    }
    let file = std::fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.take(MAX_PREFERENCE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_PREFERENCE_BYTES {
        return Err(invalid("preference_file_too_large"));
    }
    Ok(Some(bytes))
}
fn parse_object(bytes: &[u8]) -> io::Result<Box<RawValue>> {
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    UniqueJson::deserialize(&mut decoder).map_err(invalid)?;
    decoder.end().map_err(invalid)?;
    let _: BTreeMap<String, Box<RawValue>> = serde_json::from_slice(bytes).map_err(invalid)?;
    serde_json::from_slice(bytes).map_err(invalid)
}
fn merge_known(original: Option<&RawValue>, patch: &RawValue) -> io::Result<Box<RawValue>> {
    if patch.get().starts_with('{') {
        let mut retained: BTreeMap<String, Box<RawValue>> = original
            .filter(|raw| raw.get().starts_with('{'))
            .map(|raw| serde_json::from_str(raw.get()))
            .transpose()
            .map_err(invalid)?
            .unwrap_or_default();
        let fields: BTreeMap<String, Box<RawValue>> =
            serde_json::from_str(patch.get()).map_err(invalid)?;
        let mut changed = original.is_none_or(|raw| !raw.get().starts_with('{'));
        for (key, value) in fields {
            let merged = merge_known(retained.get(&key).map(AsRef::as_ref), &value)?;
            changed |= retained
                .get(&key)
                .is_none_or(|previous| previous.get() != merged.get());
            retained.insert(key, merged);
        }
        if !changed && let Some(original) = original {
            return Ok(original.to_owned());
        }
        let encoded = serde_json::to_string(&retained).map_err(invalid)?;
        return RawValue::from_string(encoded).map_err(invalid);
    }
    if let Some(original) = original {
        let previous: serde_json::Value = serde_json::from_str(original.get()).map_err(invalid)?;
        let next: serde_json::Value = serde_json::from_str(patch.get()).map_err(invalid)?;
        let same_known_number = previous
            .as_number()
            .zip(next.as_number())
            .is_some_and(|(a, b)| {
                a.as_f64()
                    .zip(b.as_f64())
                    .is_some_and(|(a, b)| a.is_finite() && b.is_finite() && a == b)
            });
        if previous == next || same_known_number {
            return Ok(original.to_owned());
        }
    }
    Ok(patch.to_owned())
}
fn preserve_framing(original: &[u8], content: &[u8]) -> Vec<u8> {
    let is_space = |byte: &u8| matches!(byte, b' ' | b'\t' | b'\n' | b'\r');
    let start = original.iter().take_while(|byte| is_space(byte)).count();
    let end = original.len()
        - original
            .iter()
            .rev()
            .take_while(|byte| is_space(byte))
            .count();
    let mut bytes = original[..start].to_vec();
    bytes.extend_from_slice(content);
    bytes.extend_from_slice(&original[end..]);
    bytes
}

// Inspect keys without materializing unknown values through floating point or app settings.
struct UniqueJson;
impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        decoder.deserialize_any(UniqueVisitor)
    }
}
struct UniqueVisitor;
impl<'de> Visitor<'de> for UniqueVisitor {
    type Value = UniqueJson;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON without duplicate keys")
    }
    fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<Self::Value, E> {
        Ok(UniqueJson)
    }
    fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<Self::Value, E> {
        Ok(UniqueJson)
    }
    fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<Self::Value, E> {
        Ok(UniqueJson)
    }
    fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<Self::Value, E> {
        Ok(UniqueJson)
    }
    fn visit_str<E: serde::de::Error>(self, _: &str) -> Result<Self::Value, E> {
        Ok(UniqueJson)
    }
    fn visit_string<E: serde::de::Error>(self, _: String) -> Result<Self::Value, E> {
        Ok(UniqueJson)
    }
    fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        Ok(UniqueJson)
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        Ok(UniqueJson)
    }
    fn visit_some<D: Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        UniqueJson::deserialize(decoder)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        while sequence.next_element::<UniqueJson>()?.is_some() {}
        Ok(UniqueJson)
    }
    fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Self::Value, A::Error> {
        let mut keys = HashSet::new();
        while let Some(key) = object.next_key::<String>()? {
            if !keys.insert(key) {
                return Err(serde::de::Error::custom("duplicate JSON object key"));
            }
            object.next_value::<UniqueJson>()?;
        }
        Ok(UniqueJson)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        editor_preferences::EditorPreferenceStore,
        i18n::{InterfacePreferences, Language},
        theme::{Appearance, ThemeSettings},
    };
    use serde_json::json;
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn an_unchanged_valid_file_keeps_exact_framing_and_unknown_values() -> TestResult {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("interface.json");
        let original = b" \n {\"language\":\"en\",\"font_size\":21,\"future\":{\"n\":1.234567890123456789e+123}}\t\n";
        std::fs::write(&path, original)?;
        let mut store = FlatPreferenceStore::<InterfacePreferences>::load(path.clone())?;
        store.save()?;
        assert_eq!(std::fs::read(path)?, original);
        assert!(!store.needs_attention());
        Ok(())
    }

    #[test]
    fn known_edits_preserve_raw_unknown_objects_at_every_settings_level() -> TestResult {
        let directory = tempfile::tempdir()?;
        let opaque = r#"{ "huge": 1.234567890123456789e+99999, "$serde_json::private::Number": "literal", "items":[null,{"n":1234567890123456789012345678901234567890}] }"#;
        let appearance = directory.path().join("appearance.json");
        let interface = directory.path().join("interface.json");
        let editor = directory.path().join("editor.json");
        std::fs::write(
            &appearance,
            format!(r#"{{"appearance":"dark","palette":"solid-craft-blue","vendor":{opaque}}}"#),
        )?;
        std::fs::write(
            &interface,
            format!(r#"{{"language":"en","font_size":24,"vendor":{opaque}}}"#),
        )?;
        std::fs::write(
            &editor,
            format!(
                r#"{{"version":1,"vendor":{opaque},"preferences":{{"font_size":19,"future":{opaque}}}}}"#
            ),
        )?;
        let mut theme = FlatPreferenceStore::<ThemeSettings>::load(appearance.clone())?;
        theme.values.appearance = Appearance::Light;
        theme.save()?;
        let mut language = FlatPreferenceStore::<InterfacePreferences>::load(interface.clone())?;
        language.values.language = Language::Japanese;
        language.save()?;
        let mut reading = EditorPreferenceStore::load(editor.clone(), None)?;
        reading.values.font_size = 21.0;
        reading.save()?;
        for path in [&appearance, &interface, &editor] {
            let actual = std::fs::read_to_string(path)?;
            assert!(
                actual.contains(opaque),
                "Untouched raw extension changed in {path:?}"
            );
        }
        assert_eq!(std::fs::read_to_string(editor)?.matches(opaque).count(), 2);
        Ok(())
    }

    #[test]
    fn duplicate_keys_and_oversized_or_nonobject_files_never_become_acknowledged() -> TestResult {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("appearance.json");
        let invalids = [
            br#"{"appearance":"dark","appearance":"light"}"#.to_vec(),
            br#"{"appearance":"dark","vendor":{"a":1,"a":2}}"#.to_vec(),
            b"[]".to_vec(),
            b"{invalid".to_vec(),
            vec![b' '; MAX_PREFERENCE_BYTES as usize + 1],
        ];
        for original in invalids {
            std::fs::write(&path, &original)?;
            assert!(FlatPreferenceStore::<ThemeSettings>::load(path.clone()).is_err());
            let mut blocked = FlatPreferenceStore::<ThemeSettings>::blocked(path.clone());
            blocked.values.appearance = Appearance::Light;
            blocked.queue()?;
            assert!(blocked.has_pending_write());
            assert!(blocked.save().is_err());
            assert_eq!(std::fs::read(&path)?, original);
        }
        Ok(())
    }

    #[test]
    fn explicit_future_flat_versions_and_unknown_known_values_are_preserved() -> TestResult {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("appearance.json");
        for original in [
            br#"{"version":2,"appearance":"light"}"#.as_slice(),
            br#"{"appearance":"future-mode"}"#,
        ] {
            std::fs::write(&path, original)?;
            assert!(FlatPreferenceStore::<ThemeSettings>::load(path.clone()).is_err());
            let mut blocked = FlatPreferenceStore::<ThemeSettings>::blocked(path.clone());
            blocked.values.appearance = Appearance::System;
            assert!(blocked.save().is_err());
            assert!(blocked.has_pending_write());
            assert_eq!(std::fs::read(&path)?, original);
        }
        Ok(())
    }

    #[test]
    fn write_failure_retains_each_groups_unchanged_choices_for_retry() -> TestResult {
        let directory = tempfile::tempdir()?;
        let parent = directory.path().join("preferences");
        std::fs::create_dir(&parent)?;
        let mut theme = FlatPreferenceStore::<ThemeSettings>::load(parent.join("appearance.json"))?;
        let mut language =
            FlatPreferenceStore::<InterfacePreferences>::load(parent.join("interface.json"))?;
        let mut reading = EditorPreferenceStore::load(parent.join("editor.json"), None)?;
        theme.values.appearance = Appearance::Light;
        language.values.language = Language::French;
        reading.values.font_size = 17.0;
        std::fs::remove_dir(&parent)?;
        std::fs::write(&parent, b"synthetic blocked destination")?;
        assert!(theme.save().is_err());
        assert!(language.save().is_err());
        assert!(reading.save().is_err());
        assert!(
            theme.has_pending_write()
                && language.has_pending_write()
                && reading.has_pending_write()
        );
        std::fs::remove_file(&parent)?;
        std::fs::create_dir(&parent)?;
        theme.save()?;
        assert!(!theme.has_pending_write());
        assert!(language.has_pending_write() && reading.has_pending_write());
        language.save()?;
        reading.save()?;
        assert_eq!(
            FlatPreferenceStore::<ThemeSettings>::load(parent.join("appearance.json"))?.values,
            theme.values
        );
        assert_eq!(
            FlatPreferenceStore::<InterfacePreferences>::load(parent.join("interface.json"))?
                .values,
            language.values
        );
        assert_eq!(
            EditorPreferenceStore::load(parent.join("editor.json"), None)?.values,
            reading.values
        );
        Ok(())
    }

    #[test]
    fn replacement_after_staging_preserves_the_newer_file_and_pending_desire() -> TestResult {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("appearance.json");
        let original = br#"{"appearance":"dark"}"#;
        let newer = br#"{"appearance":"system","external":{"keep":true}}"#;
        std::fs::write(&path, original)?;
        let mut file = PreferenceFile::load(path.clone())?;
        file.queue(&json!({"appearance":"light"}))?;
        let prepared = file.prepare_publication()?.ok_or("missing staged file")?;
        std::fs::write(&path, newer)?;
        assert!(file.publish_prepared(prepared).is_err());
        assert_eq!(std::fs::read(&path)?, newer);
        assert!(file.has_pending_write());
        assert!(!file.writable());
        assert!(file.retryable());
        std::fs::write(&path, original)?;
        file.publish_pending()?;
        assert!(!file.needs_attention());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&std::fs::read(path)?)?["appearance"],
            "light"
        );
        Ok(())
    }

    #[test]
    fn creation_or_deletion_after_staging_never_silently_overwrites_or_recreates() -> TestResult {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("interface.json");
        let mut missing = PreferenceFile::load(path.clone())?;
        missing.queue(&json!({"language":"ja"}))?;
        let prepared = missing
            .prepare_publication()?
            .ok_or("missing new-file stage")?;
        let appeared = br#"{"language":"de","external":true}"#;
        std::fs::write(&path, appeared)?;
        assert!(missing.publish_prepared(prepared).is_err());
        assert_eq!(std::fs::read(&path)?, appeared);
        let mut loaded = PreferenceFile::load(path.clone())?;
        loaded.queue(&json!({"language":"fr"}))?;
        let prepared = loaded
            .prepare_publication()?
            .ok_or("missing existing-file stage")?;
        std::fs::remove_file(&path)?;
        assert!(loaded.publish_prepared(prepared).is_err());
        assert!(!path.exists());
        assert!(loaded.has_pending_write());
        Ok(())
    }

    #[test]
    fn encoding_growth_failure_is_visible_and_does_not_publish_an_old_intent() -> TestResult {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("appearance.json");
        std::fs::write(&path, br#"{"appearance":"dark"}"#)?;
        let mut file = PreferenceFile::load(path.clone())?;
        file.queue(&json!({"appearance":"light"}))?;
        let oversized =
            json!({"appearance":"system","known": "x".repeat(MAX_PREFERENCE_BYTES as usize)});
        assert!(file.queue(&oversized).is_err());
        assert!(file.has_pending_write());
        assert!(file.publish_pending().is_err());
        assert_eq!(std::fs::read(path)?, br#"{"appearance":"dark"}"#);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn sqlite_symlink_and_hardlink_aliases_are_preserved_without_a_preference_write() -> TestResult
    {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("workspace.sqlite");
        let workspace = studio_core::Workspace::open(&database)?;
        let project = workspace.create_project("Synthetic project")?;
        let document =
            workspace.create_document(&project.id, "Synthetic document", "Keep exact text")?;
        for (name, symbolic) in [("symlink.json", true), ("hardlink.json", false)] {
            let path = directory.path().join(name);
            if symbolic {
                std::os::unix::fs::symlink(&database, &path)?;
            } else {
                std::fs::hard_link(&database, &path)?;
            }
            assert!(FlatPreferenceStore::<ThemeSettings>::load(path.clone()).is_err());
            let mut blocked = FlatPreferenceStore::<ThemeSettings>::blocked(path);
            blocked.values.appearance = Appearance::Light;
            assert!(blocked.save().is_err());
            assert_eq!(workspace.document(&document.id)?, document);
        }
        Ok(())
    }
}
