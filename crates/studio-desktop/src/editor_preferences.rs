//! Versioned local reading preferences, independent of author content and appearance.
use std::{io, path::PathBuf};

use crate::preference_file::PreferenceFile;
use serde::{Deserialize, Serialize};

const VERSION: u32 = 1;
#[cfg(test)]
const MAX_FILE_BYTES: u64 = crate::preference_file::MAX_PREFERENCE_BYTES;

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
}

/// Typed reading preferences backed by the shared acknowledged JSON-file publisher.
pub(crate) struct EditorPreferenceStore {
    pub(crate) values: EditorPreferences,
    file: PreferenceFile,
    legacy_font_size: Option<f32>,
}
impl EditorPreferenceStore {
    /// A missing editor file migrates an acknowledged pre-existing interface size only.
    pub(crate) fn load(path: PathBuf, legacy_font_size: Option<f32>) -> io::Result<Self> {
        let file = PreferenceFile::load(path)?;
        let values = match file.decode::<Envelope>()? {
            Some(envelope) => {
                if envelope.version != VERSION {
                    return Err(invalid("editor_preferences_version_unsupported"));
                }
                envelope.preferences.validate()?;
                envelope.preferences
            }
            None => EditorPreferences::migrate_legacy_size(legacy_font_size),
        };
        Ok(Self {
            values,
            file,
            legacy_font_size,
        })
    }
    pub(crate) fn blocked(path: PathBuf, legacy_font_size: Option<f32>) -> Self {
        Self {
            values: EditorPreferences::migrate_legacy_size(legacy_font_size),
            file: PreferenceFile::blocked(path),
            legacy_font_size,
        }
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
    pub(crate) fn source_preserved(&self) -> bool {
        self.file.source_preserved()
    }
    pub(crate) fn has_pending_write(&self) -> bool {
        self.file.has_pending_write()
    }
    pub(crate) fn queue(&mut self) -> io::Result<()> {
        self.file.mark_pending();
        self.values.validate()?;
        self.file.queue(&Envelope {
            version: VERSION,
            preferences: self.values.clone(),
        })
    }
    pub(crate) fn save(&mut self) -> io::Result<()> {
        self.queue()?;
        self.file.publish_pending()
    }
    pub(crate) fn reload(&mut self) -> io::Result<()> {
        let fresh = Self::load(self.file.path().to_owned(), self.legacy_font_size)?;
        if !fresh.file.loaded_from_file() {
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
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
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
    fn failed_write_can_be_retried_without_changing_the_desired_values() -> TestResult {
        let directory = tempfile::tempdir()?;
        let parent = directory.path().join("preferences");
        std::fs::create_dir(&parent)?;
        let path = parent.join("editor.json");
        let mut store = EditorPreferenceStore::load(path.clone(), None)?;
        store.values.font_size = 21.0;
        std::fs::remove_dir(&parent)?;
        std::fs::write(&parent, b"blocked destination")?;
        assert!(store.save().is_err());
        assert!(store.has_pending_write());
        assert_eq!(store.values.font_size, 21.0);
        std::fs::remove_file(&parent)?;
        std::fs::create_dir(&parent)?;
        store.save()?;
        assert!(!store.has_pending_write());
        assert_eq!(
            EditorPreferenceStore::load(path, None)?.values,
            store.values
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
