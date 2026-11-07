//! Complete interface catalogs and appearance-independent language preferences.
//!
//! Catalog lookup never translates manuscript text, project titles or imported data.

use std::io::{self, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

const MISSING_MESSAGE: &str = "⟦missing translation⟧";

/// Interface languages, kept in the same order as every translation row.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum Language {
    /// Russian.
    #[default]
    #[serde(rename = "ru")]
    Russian,
    /// English.
    #[serde(rename = "en")]
    English,
    /// German.
    #[serde(rename = "de")]
    German,
    /// French.
    #[serde(rename = "fr")]
    French,
    /// Spanish.
    #[serde(rename = "es")]
    Spanish,
    /// Portuguese.
    #[serde(rename = "pt")]
    Portuguese,
    /// Simplified Chinese.
    #[serde(rename = "zh")]
    Chinese,
    /// Japanese.
    #[serde(rename = "ja")]
    Japanese,
}

impl Language {
    /// Every supported language, in catalog column order.
    pub const ALL: [Self; 8] = [
        Self::Russian, Self::English, Self::German, Self::French,
        Self::Spanish, Self::Portuguese, Self::Chinese, Self::Japanese,
    ];

    /// Stable persisted language tag.
    pub const fn code(self) -> &'static str {
        match self {
            Self::Russian => "ru", Self::English => "en", Self::German => "de",
            Self::French => "fr", Self::Spanish => "es", Self::Portuguese => "pt",
            Self::Chinese => "zh", Self::Japanese => "ja",
        }
    }

    /// A language's own name, so its setting remains discoverable in every interface.
    pub const fn native_name(self) -> &'static str {
        match self {
            Self::Russian => "Русский", Self::English => "English", Self::German => "Deutsch",
            Self::French => "Français", Self::Spanish => "Español", Self::Portuguese => "Português",
            Self::Chinese => "简体中文", Self::Japanese => "日本語",
        }
    }

    /// Resolve a stable message ID without falling back to Russian.
    pub fn text(self, msgid: &str) -> &'static str {
        CATALOG.iter().find(|(key, _)| *key == msgid)
            .map(|(_, translations)| translations[self as usize])
            .unwrap_or(MISSING_MESSAGE)
    }

    /// Interpolate named placeholders once, leaving user-provided text untouched.
    /// Unknown parameters remain visible as `{name}` instead of disappearing.
    pub fn format(self, msgid: &str, parameters: &[(&str, String)]) -> String {
        let mut remaining = self.text(msgid);
        let mut output = String::with_capacity(remaining.len());
        while let Some(start) = remaining.find('{') {
            output.push_str(&remaining[..start]);
            let after = &remaining[start + 1..];
            let Some(end) = after.find('}') else {
                output.push_str(&remaining[start..]);
                return output;
            };
            let key = &after[..end];
            if let Some((_, value)) = parameters.iter().find(|(name, _)| *name == key) {
                output.push_str(value);
            } else {
                output.push_str(&remaining[start..start + end + 2]);
            }
            remaining = &after[end + 1..];
        }
        output.push_str(remaining);
        output
    }
}

/// Preferences stored separately from the existing appearance/palette file.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InterfacePreferences {
    /// Language of interface labels; never a manuscript translation preference.
    pub language: Language,
    /// Manuscript font size in logical pixels.
    pub font_size: f32,
}

impl Default for InterfacePreferences {
    fn default() -> Self {
        Self { language: Language::Russian, font_size: 21.0 }
    }
}

impl InterfacePreferences {
    /// Load preferences, treating a missing file as a first launch.
    ///
    /// # Errors
    /// Returns an I/O error for unreadable or invalid JSON preferences, preserving that file.
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error),
        };
        let preferences: Self = serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        Ok(preferences.sanitized())
    }

    /// Atomically persist language and font size without touching appearance/palette settings.
    ///
    /// # Errors
    /// Returns an I/O error if staging, flushing or replacing the preferences file fails.
    pub fn save(self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        let parent = path.parent().filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;
        let bytes = serde_json::to_vec_pretty(&self.sanitized()).map_err(io::Error::other)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        file.write_all(&bytes)?;
        file.as_file().sync_all()?;
        file.persist(path).map_err(|error| error.error)?;
        Ok(())
    }

    fn sanitized(mut self) -> Self {
        self.font_size = if self.font_size.is_finite() { self.font_size.clamp(16.0, 28.0) } else { 21.0 };
        self
    }
}

// Each row is msgid + Russian, English, German, French, Spanish, Portuguese, Chinese, Japanese.
const CATALOG: &[(&str, [&str; 8])] = &[];
