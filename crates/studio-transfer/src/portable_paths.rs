//! Portable names retain Unicode titles and use globally unique ASCII identity suffixes.
use std::collections::{HashMap, HashSet};

use sha2::{Digest, Sha256};

use super::{Result, TransferError};

#[derive(Default)]
pub(super) struct NameAllocator {
    identities: HashMap<String, String>,
    suffixes: HashSet<String>,
}
impl NameAllocator {
    pub(super) fn name(&mut self, identity: &str, title: &str, extension: &str) -> Result<String> {
        if let Some(name) = self.identities.get(identity) {
            return Ok(name.clone());
        }
        let digest = format!("{:x}", Sha256::digest(identity.as_bytes()));
        let mut length = 16;
        let mut collision_index = self.suffixes.len();
        let suffix = loop {
            let candidate = digest[..length].to_owned();
            if self.suffixes.insert(candidate.clone()) {
                break candidate;
            }
            if length < 64 {
                length += 2;
            } else {
                let candidate = format!("{digest}-{collision_index}");
                collision_index = collision_index.checked_add(1).ok_or_else(|| {
                    TransferError::Invalid("Portable identity suffix counter overflow".into())
                })?;
                if self.suffixes.insert(candidate.clone()) {
                    break candidate;
                }
            }
        };
        let mut base: String = title
            .trim()
            .chars()
            .map(|c| {
                if c.is_control() || "/\\<>:\"|?*".contains(c) {
                    '-'
                } else {
                    c
                }
            })
            .collect();
        base = base.trim_end_matches(['.', ' ']).to_owned();
        if base.is_empty() || base == "." || base == ".." {
            base = "untitled".into();
        }
        if base.starts_with('.') {
            base.insert(0, '_');
        }
        let reserved = base.split('.').next().unwrap_or("").to_ascii_uppercase();
        if matches!(reserved.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || ((reserved.starts_with("COM") || reserved.starts_with("LPT"))
                && reserved.len() == 4
                && matches!(reserved.as_bytes()[3], b'1'..=b'9'))
        {
            base.insert(0, '_');
        }
        if !extension.is_empty()
            && (!extension.starts_with('.')
                || !extension.bytes().skip(1).all(|b| b.is_ascii_alphanumeric()))
        {
            return Err(TransferError::Invalid(
                "Unsupported portable filename extension".into(),
            ));
        }
        let max = 240_usize
            .checked_sub(suffix.len() + 2 + extension.len())
            .ok_or_else(|| {
                TransferError::Invalid("Portable filename identity is too long".into())
            })?
            / 3;
        while base.len() > max {
            base.pop();
        }
        let name = format!("{base}--{suffix}{extension}");
        self.identities.insert(identity.to_owned(), name.clone());
        Ok(name)
    }
}

pub(super) fn relative_link(source: &str, target: &str) -> Result<String> {
    Ok(relative_file_path(source, target)?
        .split('/')
        .map(percent_encode)
        .collect::<Vec<_>>()
        .join("/"))
}

pub(super) fn relative_file_path(source: &str, target: &str) -> Result<String> {
    let from: Vec<&str> = source.split('/').collect();
    let to: Vec<&str> = target.split('/').collect();
    if from.is_empty() || to.is_empty() || !safe_relative(source) || !safe_relative(target) {
        return Err(TransferError::Invalid(
            "Cannot construct an unsafe relative export link".into(),
        ));
    }
    let from = &from[..from.len() - 1];
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut parts = vec!["..".to_owned(); from.len() - common];
    parts.extend(to[common..].iter().map(|part| (*part).to_owned()));
    Ok(parts.join("/"))
}
pub(super) fn percent_encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(*byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}
pub(super) fn safe_relative(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.contains(['\\', ':'])
        && !value.chars().any(char::is_control)
        && value.split('/').all(|part| {
            !part.is_empty() && part != "." && part != ".." && !part.ends_with(['.', ' '])
        })
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "Synthetic filename contracts fail immediately"
)]
mod tests {
    use super::*;

    #[test]
    fn unicode_normalization_case_reserved_names_and_dot_space_should_stay_distinct() {
        let mut names = NameAllocator::default();
        let titles = [
            "é",
            "e\u{301}",
            "É",
            "CON",
            "con.",
            "aux ",
            "..",
            ".hidden",
            "日本語 😀",
            "a/b\\c",
        ];
        let mut outputs = HashSet::new();
        for (index, title) in titles.iter().enumerate() {
            let name = names.name(&format!("node:{index}"), title, ".md").unwrap();
            assert!(safe_relative(&name), "{name}");
            assert!(outputs.insert(name.to_lowercase().trim_end_matches(['.', ' ']).to_owned()));
            assert!(name.len() <= 240);
        }
        assert_eq!(
            names.name("node:0", "renamed", ".md").unwrap(),
            names.name("node:0", "é", ".md").unwrap()
        );
        assert!(names.name("unsafe", "valid", ".a/b").is_err());
    }

    #[test]
    fn reading_links_encode_unicode_but_canvas_file_paths_use_real_unicode_names() {
        assert_eq!(
            relative_link("documents/one.md", "assets/é 😀.png").unwrap(),
            "../assets/%C3%A9%20%F0%9F%98%80.png"
        );
        assert_eq!(
            relative_file_path("canvases/one.canvas", "assets/é 😀.png").unwrap(),
            "../assets/é 😀.png"
        );
        assert!(relative_link("../unsafe", "assets/a").is_err());
    }
}
