//! Shared desktop field and modal geometry.
//!
//! These helpers constrain chrome, never manuscript typography or canonical content.
use super::*;

pub(super) const FIELD_HEIGHT: f32 = 40.0;
const MODAL_INSET: i8 = 24;

/// A full-width, comfortably sized single-line field with native edit semantics.
pub(super) fn singleline(ui: &mut egui::Ui, text: &mut String, hint: &str) -> egui::Response {
    ui.add(
        egui::TextEdit::singleline(text)
            .hint_text(hint)
            .desired_width(f32::INFINITY)
            .min_size(Vec2::new(0.0, FIELD_HEIGHT))
            .margin(egui::Margin::symmetric(12, 8))
            .vertical_align(Align::Center),
    )
}

/// A source editor for contextual block dialogs; the host owns scrolling bounds.
#[cfg(feature = "editor-preview")]
pub(super) fn source(
    ui: &mut egui::Ui,
    text: &mut String,
    rows: usize,
    syntax_language: &str,
    language: Language,
) -> egui::text_edit::TextEditOutput {
    let mut diagnostic = None;
    let mut layouter = |ui: &egui::Ui, buffer: &dyn egui::TextBuffer, wrap_width: f32| {
        let source = buffer.as_str();
        let foreground = ui.visuals().text_color();
        let background = ui.visuals().extreme_bg_color;
        let font = egui::TextStyle::Monospace.resolve(ui.style());
        let id = ui.make_persistent_id("source-highlight");
        let cached = ui
            .ctx()
            .data(|data| data.get_temp::<std::sync::Arc<SourceHighlight>>(id));
        let cached = cached
            .filter(|cached| {
                cached.source == source
                    && cached.language == syntax_language
                    && cached.foreground == foreground
                    && cached.background == background
                    && cached.font == font
            })
            .unwrap_or_else(|| {
                let cached = std::sync::Arc::new(source_highlight(
                    source,
                    syntax_language,
                    foreground,
                    background,
                    font,
                ));
                ui.ctx()
                    .data_mut(|data| data.insert_temp(id, cached.clone()));
                cached
            });
        diagnostic = cached.diagnostic.clone();
        let mut job = cached.job.clone();
        job.wrap.max_width = wrap_width;
        ui.fonts_mut(|fonts| fonts.layout_job(job))
    };
    let output = egui::TextEdit::multiline(text)
        .code_editor()
        .desired_width(f32::INFINITY)
        .desired_rows(rows.max(3))
        .margin(egui::Margin::same(12))
        .layouter(&mut layouter)
        .show(ui);
    if let Some(error) = diagnostic {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            language.text("Syntax highlighting is unavailable. The source is preserved."),
        );
        ui.label(error);
    }
    output
}

#[cfg(feature = "editor-preview")]
#[derive(Clone)]
struct SourceHighlight {
    source: String,
    language: String,
    foreground: Color32,
    background: Color32,
    font: FontId,
    job: egui::text::LayoutJob,
    diagnostic: Option<String>,
}

#[cfg(feature = "editor-preview")]
fn source_highlight(
    source: &str,
    language: &str,
    foreground: Color32,
    background: Color32,
    font: FontId,
) -> SourceHighlight {
    use craft_editor_egui::syntax::{self, Coverage};
    let palette = syntax::Palette::for_surface(
        [foreground.r(), foreground.g(), foreground.b()],
        [background.r(), background.g(), background.b()],
    );
    let mut job = egui::text::LayoutJob::default();
    let highlighted = syntax::highlight(source, language);
    let mut diagnostic = None;
    match highlighted {
        Ok(highlighted) => {
            if matches!(
                highlighted.coverage,
                Coverage::GrammarBundleUnavailable | Coverage::MissingRequiredGrammar { .. }
            ) {
                diagnostic = Some("syntax_grammar_unavailable".into());
            }
            for token in highlighted.tokens {
                match token.text(source) {
                    Ok(text) => {
                        let [r, g, b] = palette.color(token.kind);
                        job.append(
                            text,
                            0.0,
                            egui::TextFormat {
                                font_id: font.clone(),
                                color: Color32::from_rgb(r, g, b),
                                ..Default::default()
                            },
                        );
                    }
                    Err(error) => {
                        diagnostic = Some(error.to_string());
                        break;
                    }
                }
            }
        }
        Err(error) => diagnostic = Some(error.to_string()),
    }
    // An incomplete token partition cannot change, truncate or hide editable source.
    if job.text != source {
        job = egui::text::LayoutJob::simple(source.into(), font.clone(), foreground, f32::INFINITY);
        diagnostic.get_or_insert_with(|| "syntax_token_partition".into());
    }
    SourceHighlight {
        source: source.into(),
        language: language.into(),
        foreground,
        background,
        font,
        job,
        diagnostic,
    }
}

/// A standard-height dialog action, with an optional semantic primary appearance.
pub(super) fn action(
    ui: &mut egui::Ui,
    label: &str,
    enabled: bool,
    primary: Option<Tokens>,
) -> egui::Response {
    let button = egui::Button::new(match primary {
        Some(tokens) => RichText::new(label).color(tokens.on_accent).strong(),
        None => RichText::new(label),
    })
    .min_size(Vec2::new(112.0, FIELD_HEIGHT));
    ui.add_enabled(
        enabled,
        match primary {
            Some(tokens) => button.fill(tokens.accent),
            None => button,
        },
    )
}

/// A separated action row sharing the body and header's horizontal inset.
/// Add the primary action first; actions flow from the trailing edge.
pub(super) fn footer(ui: &mut egui::Ui, actions: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(16.0);
    ui.separator();
    ui.add_space(8.0);
    ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), FIELD_HEIGHT),
        Layout::right_to_left(Align::Center),
        actions,
    );
}

/// A centered modal with one header/body/footer inset grid.
pub(super) fn modal(
    ctx: &egui::Context,
    language: Language,
    id: &str,
    title: &str,
    width: f32,
    body: impl FnOnce(&mut egui::Ui),
) -> egui::ModalResponse<()> {
    let frame =
        egui::Frame::popup(&ctx.global_style()).inner_margin(egui::Margin::same(MODAL_INSET));
    egui::Modal::new(egui::Id::new(id))
        .frame(frame)
        .show(ctx, |ui| {
            let available =
                (ctx.content_rect().width() - 64.0 - 2.0 * f32::from(MODAL_INSET)).max(0.0);
            let width = width.min(available);
            ui.set_width(width);
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
            let header = ui
                .allocate_exact_size(Vec2::new(width, 44.0), egui::Sense::hover())
                .0;
            let title_rect =
                egui::Rect::from_min_max(header.min, header.max - egui::vec2(48.0, 0.0));
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(title_rect)
                    .layout(Layout::left_to_right(Align::Center)),
                |ui| {
                    ui.set_clip_rect(title_rect);
                    ui.add(egui::Label::new(RichText::new(title).size(20.0).strong()).truncate());
                },
            );
            let close_rect = egui::Rect::from_center_size(
                egui::pos2(header.right() - 18.0, header.center().y),
                Vec2::splat(36.0),
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(close_rect)
                    .layout(Layout::left_to_right(Align::Center)),
                |ui| {
                    if icons::icon_button(ui, Icon::Close, language.text("Close")).clicked() {
                        ui.close();
                    }
                },
            );
            ui.separator();
            ui.add_space(16.0);
            body(ui);
        })
}

#[cfg(all(test, feature = "editor-preview"))]
mod source_tests {
    use super::*;

    #[test]
    fn source_highlighting_preserves_exact_unicode_and_line_endings_in_both_modes() {
        let source = "// 中文 日本語 α\r\nlet value = \"text\";\n";
        for (foreground, background) in [
            (Color32::WHITE, Color32::BLACK),
            (Color32::BLACK, Color32::WHITE),
        ] {
            let highlighted = source_highlight(
                source,
                "rust",
                foreground,
                background,
                FontId::monospace(13.0),
            );
            assert_eq!(highlighted.job.text, source);
            assert!(highlighted.diagnostic.is_none());
            assert!(
                highlighted
                    .job
                    .sections
                    .iter()
                    .any(|section| section.format.color != foreground)
            );
        }
    }

    #[test]
    fn unknown_imported_source_language_stays_plain_and_verbatim() {
        let source = "opaque <source> α\n";
        let highlighted = source_highlight(
            source,
            "future-language",
            Color32::WHITE,
            Color32::BLACK,
            FontId::monospace(13.0),
        );
        assert_eq!(highlighted.job.text, source);
        assert!(highlighted.diagnostic.is_none());
        assert_eq!(highlighted.language, "future-language");
    }
}
