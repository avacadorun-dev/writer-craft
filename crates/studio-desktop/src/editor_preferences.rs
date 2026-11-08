//! Versioned local reading preferences, independent of author content and appearance.
use std::{
    collections::BTreeMap,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

const VERSION: u32 = 1;
const MAX_FILE_BYTES: u64 = 64 * 1024;

/// Source-backed native manuscript families. Names are stable font identities, not translations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ManuscriptFont {
    #[default]
    Sofia,
    Inter,
    SourceSerif,
}
impl ManuscriptFont {
    pub(crate) const ALL: [Self; 3] = [Self::Sofia, Self::Inter, Self::SourceSerif];
    pub(crate) fn family(self) -> &'static str {
        match self {
            Self::Sofia => "Sofia Sans",
            Self::Inter => "Inter",
            Self::SourceSerif => "Source Serif 4",
        }
    }
}

/// User intent for the formatting row; constrained windows may still use safe overflow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ToolbarMode {
    #[default]
    Auto,
    Expanded,
    Compact,
}
impl ToolbarMode {
    pub(crate) const ALL: [Self; 3] = [Self::Auto, Self::Expanded, Self::Compact];
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Expanded => "Expanded",
            Self::Compact => "Compact",
        }
    }
}

/// Reading presentation and existing editor behavior; never serialized into a manuscript.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct EditorPreferences {
    pub(crate) font_family: ManuscriptFont,
    /// Normal choices are 17/19/21; pre-existing native 16–28 values remain valid.
    pub(crate) font_size: f32,
    pub(crate) line_height: f32,
    pub(crate) paragraph_spacing: f32,
    pub(crate) list_item_spacing: f32,
    pub(crate) manuscript_width: u16,
    pub(crate) toolbar_mode: ToolbarMode,
    pub(crate) spellcheck: bool,
    pub(crate) word_goal: u32,
    pub(crate) auto_play_source: bool,
    #[serde(flatten)]
    extensions: BTreeMap<String, Value>,
}
impl Default for EditorPreferences {
    fn default() -> Self {
        Self {
            font_family: ManuscriptFont::Sofia,
            font_size: 19.0,
            line_height: 1.75,
            paragraph_spacing: 0.9,
            list_item_spacing: 0.25,
            manuscript_width: 70,
            toolbar_mode: ToolbarMode::Auto,
            spellcheck: true,
            word_goal: 0,
            auto_play_source: false,
            extensions: BTreeMap::new(),
        }
    }
}
impl EditorPreferences {
    fn validate(&self) -> io::Result<()> {
        if !self.font_size.is_finite()
            || !(16.0..=28.0).contains(&self.font_size)
            || ![1.5, 1.75, 2.0].contains(&self.line_height)
            || ![0.65, 0.9, 1.1].contains(&self.paragraph_spacing)
            || ![0.0, 0.25, 0.5].contains(&self.list_item_spacing)
            || ![60, 70, 80].contains(&self.manuscript_width)
            || self.word_goal > 1_000_000
        {
            return Err(invalid("editor_preferences_value_out_of_range"));
        }
        Ok(())
    }

    fn migrate_legacy_size(size: Option<f32>) -> Self {
        let mut preferences = Self::default();
        if let Some(size) = size.filter(|size| size.is_finite() && (16.0..=28.0).contains(size)) {
            preferences.font_size = size;
        }
        preferences
    }
}

#[derive(Serialize, Deserialize)]
struct Envelope {
    version: u32,
    #[serde(default)]
    preferences: EditorPreferences,
    #[serde(flatten)]
    extensions: BTreeMap<String, Value>,
}

enum SavedFile {
    Missing,
    Exact(Vec<u8>),
    /// Decode/version/I/O failures cannot grant permission to replace the preserved file.
    Blocked,
}

/// A loaded preference file and its replacement guard. An invalid file stays on disk.
pub(crate) struct EditorPreferenceStore {
    pub(crate) values: EditorPreferences,
    path: PathBuf,
    saved: SavedFile,
    extensions: BTreeMap<String, Value>,
}
impl EditorPreferenceStore {
    /// A missing editor file migrates an existing native font size only when the host
    /// actually found the old interface file. First launch uses original-editor defaults.
    pub(crate) fn load(path: PathBuf, legacy_font_size: Option<f32>) -> io::Result<Self> {
        let original = read_bounded(&path)?;
        let Some(bytes) = original else {
            return Ok(Self {
                values: EditorPreferences::migrate_legacy_size(legacy_font_size),
                path,
                saved: SavedFile::Missing,
                extensions: BTreeMap::new(),
            });
        };
        let envelope: Envelope = serde_json::from_slice(&bytes).map_err(invalid)?;
        if envelope.version != VERSION {
            return Err(invalid("editor_preferences_version_unsupported"));
        }
        envelope.preferences.validate()?;
        Ok(Self {
            values: envelope.preferences,
            path,
            saved: SavedFile::Exact(bytes),
            extensions: envelope.extensions,
        })
    }

    /// Use defaults for the current process after reporting a load failure. This store
    /// deliberately refuses persistence until the original file is repaired/reloaded.
    pub(crate) fn blocked(path: PathBuf, legacy_font_size: Option<f32>) -> Self {
        Self {
            values: EditorPreferences::migrate_legacy_size(legacy_font_size),
            path,
            saved: SavedFile::Blocked,
            extensions: BTreeMap::new(),
        }
    }

    pub(crate) fn writable(&self) -> bool {
        !matches!(self.saved, SavedFile::Blocked)
    }

    /// Replace only the same acknowledged valid file. Failed writes retain the exact
    /// in-memory choices, allowing an explicit retry without changing a manuscript.
    pub(crate) fn save(&mut self) -> io::Result<()> {
        self.values.validate()?;
        if !self.writable() {
            return Err(invalid(
                "editor_preferences_preserved_file_requires_recovery",
            ));
        }
        let current = read_bounded(&self.path)?;
        let matches = match (&self.saved, current.as_deref()) {
            (SavedFile::Missing, None) => true,
            (SavedFile::Exact(expected), Some(actual)) => expected.as_slice() == actual,
            _ => false,
        };
        if !matches {
            self.saved = SavedFile::Blocked;
            return Err(invalid("editor_preferences_changed_externally"));
        }
        let bytes = serde_json::to_vec_pretty(&Envelope {
            version: VERSION,
            preferences: self.values.clone(),
            extensions: self.extensions.clone(),
        })
        .map_err(invalid)?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err(invalid("editor_preferences_file_too_large"));
        }
        let parent = self
            .path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        file.write_all(&bytes)?;
        file.as_file().sync_all()?;
        file.persist(&self.path).map_err(|error| error.error)?;
        self.saved = SavedFile::Exact(bytes);
        Ok(())
    }
}

fn invalid(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}
fn read_bounded(path: &Path) -> io::Result<Option<Vec<u8>>> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(invalid("editor_preferences_file_too_large"));
    }
    Ok(Some(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn first_launch_defaults_and_old_native_custom_size_are_distinct() -> TestResult {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("editor.json");
        let first = EditorPreferenceStore::load(path.clone(), None)?;
        assert_eq!(first.values, EditorPreferences::default());
        let migrated = EditorPreferenceStore::load(path, Some(24.0))?;
        assert_eq!(migrated.values.font_size, 24.0);
        assert_eq!(migrated.values.line_height, 1.75);
        Ok(())
    }

    #[test]
    fn all_editor_choices_persist_without_touching_appearance_or_interface() -> TestResult {
        let directory = tempfile::tempdir()?;
        let appearance = directory.path().join("appearance.json");
        let interface = directory.path().join("interface.json");
        std::fs::write(&appearance, b"keep appearance")?;
        std::fs::write(&interface, b"keep interface")?;
        let path = directory.path().join("editor.json");
        let mut store = EditorPreferenceStore::load(path.clone(), None)?;
        store.values.font_family = ManuscriptFont::Inter;
        store.values.font_size = 17.0;
        store.values.line_height = 2.0;
        store.values.paragraph_spacing = 1.1;
        store.values.list_item_spacing = 0.5;
        store.values.manuscript_width = 80;
        store.values.toolbar_mode = ToolbarMode::Compact;
        store.values.spellcheck = false;
        store.values.word_goal = 12_500;
        store.values.auto_play_source = true;
        store.save()?;
        assert_eq!(
            EditorPreferenceStore::load(path, None)?.values,
            store.values
        );
        assert_eq!(std::fs::read(appearance)?, b"keep appearance");
        assert_eq!(std::fs::read(interface)?, b"keep interface");
        Ok(())
    }

    #[test]
    fn invalid_future_or_external_preferences_are_never_overwritten() -> TestResult {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("editor.json");
        for bytes in [
            b"{invalid".as_slice(),
            br#"{"version":2,"preferences":{"font_size":24}}"#,
        ] {
            std::fs::write(&path, bytes)?;
            assert!(EditorPreferenceStore::load(path.clone(), None).is_err());
            let mut blocked = EditorPreferenceStore::blocked(path.clone(), None);
            blocked.values.font_size = 17.0;
            assert!(blocked.save().is_err());
            assert_eq!(std::fs::read(&path)?, bytes);
        }
        std::fs::remove_file(&path)?;
        let mut store = EditorPreferenceStore::load(path.clone(), None)?;
        store.save()?;
        std::fs::write(&path, b"external settings change")?;
        store.values.font_size = 21.0;
        assert!(store.save().is_err());
        assert!(!store.writable());
        assert_eq!(std::fs::read(path)?, b"external settings change");
        Ok(())
    }

    #[test]
    fn unknown_current_version_fields_survive_a_known_preference_change() -> TestResult {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("editor.json");
        std::fs::write(
            &path,
            br#"{"version":1,"vendor":{"retain":true},"preferences":{"vendor_option":[1,2,3]}}"#,
        )?;
        let mut store = EditorPreferenceStore::load(path.clone(), None)?;
        store.values.font_size = 21.0;
        store.save()?;
        let saved: Value = serde_json::from_slice(&std::fs::read(path)?)?;
        assert_eq!(saved["vendor"]["retain"], true);
        assert_eq!(
            saved["preferences"]["vendor_option"],
            serde_json::json!([1, 2, 3])
        );
        Ok(())
    }

    #[test]
    fn oversized_and_invalid_numeric_preferences_are_preserved() -> TestResult {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("editor.json");
        std::fs::write(&path, vec![b' '; MAX_FILE_BYTES as usize + 1])?;
        assert!(EditorPreferenceStore::load(path.clone(), None).is_err());
        assert_eq!(std::fs::metadata(&path)?.len(), MAX_FILE_BYTES + 1);
        let mut store = EditorPreferenceStore::blocked(path, None);
        store.values.line_height = f32::NAN;
        assert!(store.save().is_err());
        Ok(())
    }
}
