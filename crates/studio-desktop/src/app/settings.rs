//! Categorized native preferences; incomplete parity sections never expose fake controls.
use super::*;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum Section {
    General,
    Appearance,
    Editor,
    Assistant,
    Mcp,
    Voice,
    Files,
    Keyboard,
}
impl Section {
    pub(super) const ALL: [Self; 8] = [
        Self::General,
        Self::Appearance,
        Self::Editor,
        Self::Assistant,
        Self::Mcp,
        Self::Voice,
        Self::Files,
        Self::Keyboard,
    ];
    fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Appearance => "Appearance",
            Self::Editor => "Editor",
            Self::Assistant => "AI & providers",
            Self::Mcp => "Agents & MCP",
            Self::Voice => "Voice",
            Self::Files => "Files & recovery",
            Self::Keyboard => "Keyboard shortcuts",
        }
    }
    fn icon(self) -> Icon {
        match self {
            Self::General => Icon::Settings,
            Self::Appearance => Icon::Palette,
            Self::Editor => Icon::TextCursor,
            Self::Assistant => Icon::Assistant,
            Self::Mcp => Icon::Plug,
            Self::Voice => Icon::Mic,
            Self::Files => Icon::Folder,
            Self::Keyboard => Icon::Keyboard,
        }
    }
    fn description(self) -> Option<&'static str> {
        match self {
            Self::Assistant => Some("Provider connections, models and assistant behavior."),
            Self::Mcp => Some("Agent connections and MCP server settings."),
            Self::Voice => Some("Microphone, transcription language and voice commands."),
            Self::Files => Some("Project storage, backups, recovery and audio retention."),
            _ => None,
        }
    }
}
impl StudioApp {
    fn settings_content(&mut self, ui: &mut egui::Ui, t: Tokens) {
        let l = self.language;
        match self.settings_section {
            Section::General => {
                ui.label(RichText::new(l.text("Interface language")).strong());
                egui::ComboBox::from_id_salt("interface-language")
                    .selected_text(self.language.native_name())
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        for language in Language::ALL {
                            ui.selectable_value(
                                &mut self.language,
                                language,
                                language.native_name(),
                            );
                        }
                    });
                ui.add_space(8.0);
                ui.label(
                    RichText::new(l.text(
                        "Language changes only the interface; your document text stays the same.",
                    ))
                    .size(13.0)
                    .color(t.muted),
                );
            }
            Section::Appearance => {
                let available = ui.available_width();
                let columns = preview_columns(available);
                let card_width = (available - 12.0 * (columns - 1) as f32) / columns as f32;
                let system_dark =
                    ui.ctx().system_theme().unwrap_or(egui::Theme::Dark) == egui::Theme::Dark;
                let current_palette = self.settings.palette;
                ui.label(RichText::new(l.text("Mode")).strong());
                for row in Appearance::ALL.chunks(columns) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        for &appearance in row {
                            let resolve = |mode| {
                                theme::resolve(
                                    &ThemeSettings {
                                        appearance: mode,
                                        palette: current_palette,
                                    },
                                    system_dark,
                                )
                            };
                            let preview = if appearance == Appearance::System {
                                ThemePreview::Split(
                                    resolve(Appearance::Light),
                                    resolve(Appearance::Dark),
                                )
                            } else {
                                ThemePreview::Single(resolve(appearance))
                            };
                            if preview_card(
                                ui,
                                l.text(appearance.label()),
                                self.settings.appearance == appearance,
                                card_width,
                                preview,
                                t,
                            )
                            .clicked()
                            {
                                self.settings.appearance = appearance;
                            }
                        }
                    });
                }
                ui.add_space(12.0);
                ui.label(RichText::new(l.text("Palette")).strong());
                for row in Palette::ALL.chunks(columns) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        for &palette in row {
                            let preview = theme::resolve(
                                &ThemeSettings {
                                    appearance: self.settings.appearance,
                                    palette,
                                },
                                system_dark,
                            );
                            if preview_card(
                                ui,
                                l.text(palette.label()),
                                self.settings.palette == palette,
                                card_width,
                                ThemePreview::Single(preview),
                                t,
                            )
                            .clicked()
                            {
                                self.settings.palette = palette;
                            }
                        }
                    });
                }
            }
            Section::Editor => {
                ui.label(RichText::new(l.text("Manuscript text size")).strong());
                ui.add(
                    egui::Slider::new(&mut self.font_size, 16.0..=28.0)
                        .suffix(format!(" {}", l.text("px"))),
                );
            }
            Section::Files => self.storage_settings(ui, t),
            Section::Keyboard => {
                ui.label(RichText::new(l.text("⌘S — save     ⌘N — new project     ⌘O — open archive\n⌘⇧A — review panel     ⌘⇧F — focus mode     ⌘, — settings")).size(14.0).color(t.text));
            }
            section => {
                if let Some(description) = section.description() {
                    ui.label(l.text(description));
                }
                ui.add_space(12.0);
                ui.label(
                    RichText::new(l.text("This section is not available in this alpha."))
                        .size(13.0)
                        .color(t.muted),
                );
            }
        }
    }

    pub(super) fn settings_dialog(&mut self, ctx: &egui::Context, t: Tokens) {
        let l = self.language;
        let previous = self.settings;
        let previous_interface = InterfacePreferences {
            language: self.language,
            font_size: self.font_size,
        };
        let response = modal(
            ctx,
            l,
            "settings",
            l.text("Settings · WriterCraft"),
            840.0,
            |ui| {
                let height = (ctx.content_rect().height() - 220.0).clamp(300.0, 410.0);
                let rect = ui
                    .allocate_exact_size(
                        Vec2::new(ui.available_width(), height),
                        egui::Sense::hover(),
                    )
                    .0;
                let split = rect.left() + 196.0;
                let sidebar = egui::Rect::from_min_max(rect.min, egui::pos2(split, rect.bottom()));
                let content =
                    egui::Rect::from_min_max(egui::pos2(split + 1.0, rect.top()), rect.max);
                ui.painter().rect_filled(sidebar, 0, t.surface);
                ui.painter()
                    .vline(split, rect.y_range(), Stroke::new(1.0, t.border));
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(sidebar.shrink2(egui::vec2(8.0, 12.0)))
                        .layout(Layout::top_down(Align::Min)),
                    |ui| {
                        ui.set_clip_rect(sidebar);
                        ui.spacing_mut().item_spacing.y = 4.0;
                        for section in Section::ALL {
                            let group = match section {
                                Section::General => Some("Workspace"),
                                Section::Assistant => Some("Connections"),
                                Section::Files => Some("Application"),
                                _ => None,
                            };
                            if let Some(group) = group {
                                if section != Section::General {
                                    ui.add_space(8.0);
                                }
                                let (heading, heading_response) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), 18.0),
                                    egui::Sense::hover(),
                                );
                                heading_response.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::Label,
                                        ui.is_enabled(),
                                        l.text(group),
                                    )
                                });
                                ui.painter().text(
                                    egui::pos2(heading.left() + 12.0, heading.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    l.text(group).to_uppercase(),
                                    FontId::proportional(10.0),
                                    t.muted,
                                );
                            }
                            let selected = self.settings_section == section;
                            let response = ui.add_sized(
                                [ui.available_width(), 34.0],
                                egui::Button::new("").frame(false),
                            );
                            response.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::SelectableLabel,
                                    ui.is_enabled(),
                                    l.text(section.label()),
                                )
                            });
                            let row = response.rect;
                            let foreground = if selected { t.accent } else { t.text };
                            if selected {
                                ui.painter().rect_filled(row, 6, t.accent_soft);
                            }
                            icons::paint(
                                ui.painter(),
                                egui::Rect::from_center_size(
                                    egui::pos2(row.left() + 18.0, row.center().y),
                                    Vec2::splat(18.0),
                                ),
                                section.icon(),
                                foreground,
                            );
                            let text_rect = egui::Rect::from_min_max(
                                row.min + egui::vec2(36.0, 0.0),
                                row.max - egui::vec2(8.0, 0.0),
                            );
                            let mut job = egui::text::LayoutJob::simple(
                                l.text(section.label()).to_owned(),
                                FontId::proportional(14.0),
                                foreground,
                                text_rect.width(),
                            );
                            job.wrap.max_rows = 1;
                            let galley = ui.painter().layout_job(job);
                            ui.painter().with_clip_rect(text_rect).galley(
                                egui::pos2(
                                    text_rect.left(),
                                    text_rect.center().y - galley.size().y / 2.0,
                                ),
                                galley,
                                foreground,
                            );
                            if response.on_hover_text(l.text(section.label())).clicked() {
                                self.settings_section = section;
                            }
                        }
                    },
                );
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(content.shrink(24.0))
                        .layout(Layout::top_down(Align::Min)),
                    |ui| {
                        ui.set_clip_rect(content);
                        ui.spacing_mut().item_spacing = Vec2::new(8.0, 12.0);
                        ui.spacing_mut().interact_size.y = 36.0;
                        ui.horizontal(|ui| {
                            icons::icon(ui, self.settings_section.icon(), 22.0, t.accent);
                            ui.heading(l.text(self.settings_section.label()));
                        });
                        ui.add_space(12.0);
                        egui::ScrollArea::vertical()
                            .id_salt(("settings-content", self.settings_section as u8))
                            .max_height(height - 100.0)
                            .auto_shrink([false, false])
                            .show(ui, |ui| self.settings_content(ui, t));
                    },
                );
                ui.add_space(12.0);
                ui.separator();
                let footer = ui
                    .allocate_exact_size(
                        Vec2::new(ui.available_width(), 56.0),
                        egui::Sense::hover(),
                    )
                    .0;
                let button_rect = egui::Rect::from_center_size(
                    egui::pos2(footer.right() - 72.0, footer.center().y),
                    Vec2::new(96.0, 36.0),
                );
                if ui
                    .put(
                        button_rect,
                        egui::Button::new(RichText::new(l.text("Done")).color(t.on_accent))
                            .fill(t.accent),
                    )
                    .clicked()
                {
                    self.settings_open = false;
                }
            },
        );
        if self.settings != previous {
            theme::apply(ctx, &self.settings);
            let result = serde_json::to_vec_pretty(&self.settings)
                .map_err(Into::into)
                .and_then(|bytes| {
                    atomic_write(&self.database.with_file_name("appearance.json"), &bytes)
                });
            self.record_error(result);
        }
        let preferences = InterfacePreferences {
            language: self.language,
            font_size: self.font_size,
        };
        if preferences != previous_interface
            && let Err(error) = preferences.save(self.database.with_file_name("interface.json"))
        {
            self.error = Some(UiError {
                msgid: "Interface settings could not be saved.",
                details: error.to_string(),
            });
            self.status = "Attention needed".into();
        }
        if response.should_close() {
            self.settings_open = false;
        }
    }
}

#[derive(Clone, Copy)]
enum ThemePreview {
    Single(Tokens),
    Split(Tokens, Tokens),
}

fn preview_columns(width: f32) -> usize {
    if width >= 492.0 {
        3
    } else if width >= 304.0 {
        2
    } else {
        1
    }
}

fn preview_card(
    ui: &mut egui::Ui,
    label: &str,
    selected: bool,
    width: f32,
    preview: ThemePreview,
    t: Tokens,
) -> egui::Response {
    let response = ui.add_sized(
        [width, 108.0],
        egui::Button::new("")
            .min_size(Vec2::new(width, 108.0))
            .selected(selected)
            .frame(false),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::RadioButton,
            ui.is_enabled(),
            selected,
            label,
        )
    });
    if ui.is_rect_visible(response.rect) {
        let card = response.rect.shrink(1.0);
        let painter = ui.painter().with_clip_rect(card);
        let emphasized = selected || response.has_focus();
        let border = if emphasized {
            t.accent
        } else if response.hovered() {
            t.muted
        } else {
            t.border
        };
        painter.rect_filled(card, 7, if selected { t.accent_soft } else { t.surface });
        painter.rect_stroke(
            card,
            7,
            Stroke::new(if emphasized { 1.8 } else { 1.0 }, border),
            egui::StrokeKind::Inside,
        );
        let thumbnail = egui::Rect::from_min_size(
            card.min + egui::vec2(8.0, 8.0),
            Vec2::new((card.width() - 16.0).max(1.0), 56.0),
        );
        match preview {
            ThemePreview::Single(colors) => miniature_window(&painter, thumbnail, colors),
            ThemePreview::Split(light, dark) => {
                let left = egui::Rect::from_min_max(
                    thumbnail.min,
                    egui::pos2(thumbnail.center().x, thumbnail.bottom()),
                );
                let right = egui::Rect::from_min_max(
                    egui::pos2(thumbnail.center().x, thumbnail.top()),
                    thumbnail.max,
                );
                miniature_window(&painter.with_clip_rect(left), thumbnail, light);
                miniature_window(&painter.with_clip_rect(right), thumbnail, dark);
                painter.vline(
                    thumbnail.center().x,
                    thumbnail.y_range(),
                    Stroke::new(0.8, t.border),
                );
            }
        }
        let label_rect = egui::Rect::from_min_max(
            egui::pos2(card.left() + 7.0, thumbnail.bottom() + 5.0),
            card.max - egui::vec2(7.0, 3.0),
        );
        let mut job = egui::text::LayoutJob::simple(
            label.to_owned(),
            FontId::proportional(14.0),
            t.text,
            label_rect.width(),
        );
        job.halign = Align::Center;
        job.wrap.max_rows = 2;
        let galley = painter.layout_job(job);
        painter.with_clip_rect(label_rect).galley(
            egui::pos2(
                label_rect.center().x,
                label_rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            t.text,
        );
        if selected {
            let center = egui::pos2(card.right() - 14.0, card.top() + 14.0);
            painter.circle_filled(center, 6.5, t.accent);
            painter.add(egui::Shape::line(
                vec![
                    center + egui::vec2(-3.0, 0.0),
                    center + egui::vec2(-0.8, 2.2),
                    center + egui::vec2(3.2, -2.5),
                ],
                Stroke::new(1.3, t.on_accent),
            ));
        }
    }
    response.on_hover_text(label)
}

fn miniature_window(painter: &egui::Painter, rect: egui::Rect, t: Tokens) {
    let painter = painter.with_clip_rect(rect);
    painter.rect_filled(rect, 4, t.editor);
    let top = rect.top() + 11.0;
    let nav_edge = rect.left() + rect.width() * 0.22;
    let review_edge = rect.right() - rect.width() * 0.20;
    let toolbar = egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), top));
    painter.rect_filled(toolbar, 0, t.bg);
    for offset in [5.0, 9.0, 13.0] {
        painter.circle_filled(
            egui::pos2(rect.left() + offset, rect.top() + 5.5),
            1.1,
            t.muted.gamma_multiply(0.6),
        );
    }
    painter.line_segment(
        [
            egui::pos2(rect.left() + 20.0, rect.top() + 5.5),
            egui::pos2(rect.left() + 47.0, rect.top() + 5.5),
        ],
        Stroke::new(1.4, t.muted.gamma_multiply(0.55)),
    );
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(rect.left(), top),
            egui::pos2(nav_edge, rect.bottom()),
        ),
        0,
        t.panel,
    );
    painter.rect_filled(
        egui::Rect::from_min_max(egui::pos2(review_edge, top), rect.max),
        0,
        t.panel,
    );
    painter.vline(nav_edge, top..=rect.bottom(), Stroke::new(0.6, t.border));
    painter.vline(review_edge, top..=rect.bottom(), Stroke::new(0.6, t.border));
    painter.hline(rect.x_range(), top, Stroke::new(0.6, t.border));
    let selected = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 3.0, top + 6.0),
        Vec2::new((nav_edge - rect.left() - 6.0).max(1.0), 6.0),
    );
    painter.rect_filled(selected, 1, t.accent_soft);
    painter.line_segment(
        [
            egui::pos2(selected.left() + 3.0, selected.center().y),
            egui::pos2(selected.right() - 3.0, selected.center().y),
        ],
        Stroke::new(1.0, t.accent),
    );
    for offset in [19.0, 27.0, 35.0] {
        painter.line_segment(
            [
                egui::pos2(rect.left() + 6.0, top + offset),
                egui::pos2(nav_edge - 5.0, top + offset),
            ],
            Stroke::new(1.2, t.muted.gamma_multiply(0.42)),
        );
    }
    let writing_left = nav_edge + 7.0;
    let writing_width = (review_edge - writing_left - 7.0).max(1.0);
    painter.line_segment(
        [
            egui::pos2(writing_left, top + 9.0),
            egui::pos2(writing_left + writing_width * 0.59, top + 9.0),
        ],
        Stroke::new(2.0, t.text.gamma_multiply(0.80)),
    );
    for (offset, ratio) in [
        (17.0, 1.0),
        (22.0, 0.94),
        (27.0, 0.82),
        (35.0, 0.97),
        (40.0, 0.69),
    ] {
        painter.line_segment(
            [
                egui::pos2(writing_left, top + offset),
                egui::pos2(writing_left + writing_width * ratio, top + offset),
            ],
            Stroke::new(1.1, t.muted.gamma_multiply(0.62)),
        );
    }
    painter.line_segment(
        [
            egui::pos2(review_edge + 5.0, top + 8.0),
            egui::pos2(rect.right() - 5.0, top + 8.0),
        ],
        Stroke::new(1.7, t.accent),
    );
    for offset in [17.0, 23.0, 29.0] {
        painter.line_segment(
            [
                egui::pos2(review_edge + 5.0, top + offset),
                egui::pos2(rect.right() - 5.0, top + offset),
            ],
            Stroke::new(1.0, t.muted.gamma_multiply(0.45)),
        );
    }
    painter.rect_stroke(
        rect,
        4,
        Stroke::new(0.8, t.border),
        egui::StrokeKind::Inside,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_settings_section_and_scope_hint_is_localized() {
        for section in Section::ALL {
            for language in Language::ALL {
                assert_ne!(language.text(section.label()), "⟦missing translation⟧");
                if let Some(description) = section.description() {
                    assert_ne!(language.text(description), "⟦missing translation⟧");
                }
            }
        }
    }
}
