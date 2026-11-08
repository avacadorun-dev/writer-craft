//! Shared protection for the live database, future sidecars and sibling preferences.
use std::path::{Path, PathBuf};

const PREFERENCES: [&str; 3] = ["appearance.json", "interface.json", "editor.json"];

fn sidecar(database: &Path, suffix: &str) -> PathBuf {
    let mut path = database.as_os_str().to_owned();
    path.push(suffix);
    path.into()
}
fn normalized_file(path: &Path) -> std::io::Result<PathBuf> {
    match std::fs::canonicalize(path) {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            Ok(
                std::fs::canonicalize(parent)?.join(path.file_name().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "missing_output_file_name",
                    )
                })?),
            )
        }
        Err(error) => Err(error),
    }
}

/// Whether an output path names live workspace data, including absent reserved sidecars.
///
/// Canonical paths, existing hardlinks/symlinks and conservative case-insensitive sibling names
/// are refused. User-driven exports must call this before opening or publishing output.
///
/// # Errors
/// Returns a filesystem error if path identity cannot safely be determined.
pub fn is_workspace_data_path(target: &Path, database: &Path) -> std::io::Result<bool> {
    let target_normalized = normalized_file(target)?;
    // SQLite may resolve a selected symlink before naming its own journals.
    // Reserve both the selected spelling and the real database's sidecars.
    let canonical = std::fs::canonicalize(database)?;
    let mut paths = Vec::new();
    for base in [database, canonical.as_path()] {
        paths.push(base.to_path_buf());
        for suffix in ["-wal", "-shm", "-journal"] {
            paths.push(sidecar(base, suffix));
        }
        paths.extend(PREFERENCES.map(|name| base.with_file_name(name)));
    }
    for protected in paths {
        let protected_normalized = normalized_file(&protected)?;
        let same_parent_name = target_normalized.parent() == protected_normalized.parent()
            && target_normalized
                .file_name()
                .zip(protected_normalized.file_name())
                .is_some_and(|(a, b)| {
                    a.to_string_lossy()
                        .eq_ignore_ascii_case(&b.to_string_lossy())
                });
        if target_normalized == protected_normalized || same_parent_name {
            return Ok(true);
        }
        if target.try_exists()?
            && protected.try_exists()?
            && same_file::is_same_file(target, &protected)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn validate_database_path(database: &Path) -> std::io::Result<()> {
    let reserved = |path: &Path| {
        path.file_name().is_some_and(|name| {
            PREFERENCES
                .iter()
                .any(|reserved| name.to_string_lossy().eq_ignore_ascii_case(reserved))
        })
    };
    let conflict = reserved(database)
        || (database.try_exists()? && reserved(&std::fs::canonicalize(database)?));
    let mut alias = false;
    if database.try_exists()? {
        for name in PREFERENCES {
            let preference = database.with_file_name(name);
            if preference.try_exists()? && same_file::is_same_file(database, &preference)? {
                alias = true;
                break;
            }
        }
    }
    if conflict || alias {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Select a different workspace filename; appearance.json, interface.json and editor.json are reserved for settings",
        ));
    }
    Ok(())
}
