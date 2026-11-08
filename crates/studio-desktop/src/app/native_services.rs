//! Host-owned native services shared across rich sessions.
use super::*;
#[cfg(not(test))]
use std::sync::Arc;
use std::sync::OnceLock;

// The budget covers the combined canonical, HTML and plain representations, not
// just visible prose. It is a transport bound, not a renderer/table capacity.
#[cfg(not(test))]
const CLIPBOARD_BYTES: usize = 256 * 1024 * 1024;

#[cfg(not(test))]
pub(super) fn clipboard() -> craft_editor_egui::clipboard::ClipboardResult<
    Arc<dyn craft_editor_egui::clipboard::ClipboardProvider>,
> {
    Ok(Arc::new(
        craft_editor_egui::clipboard::NativeClipboard::new(CLIPBOARD_BYTES)?,
    ))
}

/// Acknowledged once per process, before layout; no ambient font/path lookup.
pub(super) fn manuscript_fonts() -> AppResult<()> {
    static REGISTERED: OnceLock<Result<(), String>> = OnceLock::new();
    let result = REGISTERED.get_or_init(|| {
        let registration = craft_editor_egui::fonts::register_font(
            include_bytes!("../../../../assets/fonts/NotoSansCJKjp-Regular.otf").to_vec(),
        )
        .map_err(|error| error.to_string())?;
        let sample = "中文写作、日本語の文章。漢字とひらがな、カタカナ。";
        let covered = registration.families.iter().any(|family| {
            craft_editor_egui::fonts::coverage(family, "Regular", sample)
                .is_ok_and(|coverage| coverage.exact_style && coverage.missing.is_empty())
        });
        if !covered {
            return Err("native_manuscript_font_coverage".into());
        }
        Ok(())
    });
    result
        .as_ref()
        .copied()
        .map_err(|error| error.clone().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_native_manuscript_font_covers_chinese_and_japanese_without_os_clipboard_access()
    -> AppResult<()> {
        manuscript_fonts()?;
        let coverage = craft_editor_egui::fonts::coverage(
            "Noto Sans CJK JP",
            "Regular",
            "中文写作 日本語の文章 漢字 ひらがな カタカナ",
        )?;
        assert!(coverage.exact_style && coverage.missing.is_empty());
        Ok(())
    }
}
