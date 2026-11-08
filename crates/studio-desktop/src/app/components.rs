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
) -> egui::text_edit::TextEditOutput {
    egui::TextEdit::multiline(text)
        .code_editor()
        .desired_width(f32::INFINITY)
        .desired_rows(rows.max(3))
        .margin(egui::Margin::same(12))
        .show(ui)
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
    let frame = egui::Frame::popup(&ctx.style()).inner_margin(egui::Margin::same(MODAL_INSET));
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
