//! Categorized native preferences; incomplete parity sections never expose fake controls.
use super::*;
use crate::editor_preferences::{ManuscriptFont, ToolbarMode};

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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PreferenceGroup {
    Appearance,
    Interface,
    Editor,
}
impl PreferenceGroup {
    const ALL: [Self; 3] = [Self::Appearance, Self::Interface, Self::Editor];
    fn label(self) -> &'static str {
        match self {
            Self::Appearance => "Appearance",
            Self::Interface => "Interface",
            Self::Editor => "Editor",
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PreferenceSaveOutcome {
    Acknowledged,
    SessionOnly,
}
#[derive(Default)]
struct PreferenceActions {
    retry: bool,
    reload: Option<PreferenceGroup>,
}
impl StudioApp {
    pub(super) fn preferences_need_attention(&self) -> bool {
        PreferenceGroup::ALL
            .into_iter()
            .any(|group| self.preference_needs_attention(group))
    }
    fn preference_needs_attention(&self, group: PreferenceGroup) -> bool {
        match group {
            PreferenceGroup::Appearance => self.appearance_preferences.needs_attention(),
            PreferenceGroup::Interface => self.interface_preferences.needs_attention(),
            PreferenceGroup::Editor => self.editor_preferences.needs_attention(),
        }
    }
    fn preference_pending(&self, group: PreferenceGroup) -> bool {
        match group {
            PreferenceGroup::Appearance => self.appearance_preferences.has_pending_write(),
            PreferenceGroup::Interface => self.interface_preferences.has_pending_write(),
            PreferenceGroup::Editor => self.editor_preferences.has_pending_write(),
        }
    }
    fn preference_retryable(&self, group: PreferenceGroup) -> bool {
        match group {
            PreferenceGroup::Appearance => self.appearance_preferences.retryable(),
            PreferenceGroup::Interface => self.interface_preferences.retryable(),
            PreferenceGroup::Editor => self.editor_preferences.retryable(),
        }
    }
    fn preference_source_preserved(&self, group: PreferenceGroup) -> bool {
        match group {
            PreferenceGroup::Appearance => self.appearance_preferences.source_preserved(),
            PreferenceGroup::Interface => self.interface_preferences.source_preserved(),
            PreferenceGroup::Editor => self.editor_preferences.source_preserved(),
        }
    }
    fn preference_attention_summary(&self) -> String {
        let groups = PreferenceGroup::ALL
            .into_iter()
            .filter(|group| self.preference_needs_attention(*group))
            .map(|group| self.language.text(group.label()))
            .collect::<Vec<_>>()
            .join(", ");
        self.language
            .format("Settings need attention: {groups}", &[("groups", groups)])
    }
    fn request_preference_save(
        &mut self,
        group: PreferenceGroup,
    ) -> std::io::Result<PreferenceSaveOutcome> {
        match group {
            PreferenceGroup::Appearance => {
                self.appearance_preferences.values = self.settings;
                self.appearance_preferences.queue()?;
                if self.appearance_preferences.writable() {
                    self.appearance_preferences.save()?;
                }
            }
            PreferenceGroup::Interface => {
                // This size is retained only for the old interface-to-editor migration.
                self.interface_preferences.values.language = self.language;
                self.interface_preferences.queue()?;
                if self.interface_preferences.writable() {
                    self.interface_preferences.save()?;
                }
            }
            PreferenceGroup::Editor => {
                self.editor_preferences.queue()?;
                if self.editor_preferences.writable() {
                    self.editor_preferences.save()?;
                }
            }
        }
        Ok(if self.preference_pending(group) {
            PreferenceSaveOutcome::SessionOnly
        } else {
            PreferenceSaveOutcome::Acknowledged
        })
    }
    fn retry_preference_save(&mut self, group: PreferenceGroup) -> std::io::Result<()> {
        match group {
            PreferenceGroup::Appearance => self.appearance_preferences.save(),
            PreferenceGroup::Interface => self.interface_preferences.save(),
            PreferenceGroup::Editor => self.editor_preferences.save(),
        }
    }
    fn reload_preference_group(&mut self, group: PreferenceGroup) -> std::io::Result<()> {
        match group {
            PreferenceGroup::Appearance => {
                self.appearance_preferences.reload()?;
                self.settings = self.appearance_preferences.values;
            }
            PreferenceGroup::Interface => {
                self.interface_preferences.reload()?;
                self.language = self.interface_preferences.values.language;
            }
            PreferenceGroup::Editor => {
                self.editor_preferences.reload()?;
                self.font_size = self.editor_preferences.values.font_size;
            }
        }
        Ok(())
    }
    fn report_preference_errors(
        &mut self,
        msgid: &'static str,
        failures: Vec<(PreferenceGroup, std::io::Error)>,
    ) {
        if !failures.is_empty() {
            self.error = Some(UiError {
                msgid,
                details: failures
                    .into_iter()
                    .map(|(group, error)| format!("{}: {error}", self.language.text(group.label())))
                    .collect::<Vec<_>>()
                    .join("\n"),
            });
            self.settings_open = true;
        }
    }
    fn preference_recovery(
        &mut self,
        ui: &mut egui::Ui,
        t: Tokens,
        group: PreferenceGroup,
        actions: &mut PreferenceActions,
    ) {
        if !self.preference_needs_attention(group) {
            return;
        }
        let l = self.language;
        let message = if self.preference_source_preserved(group) {
            "The settings file was preserved. Repair it or reload valid saved settings before saving."
        } else {
            "These settings are not saved. Your choices still apply to this session."
        };
        ui.label(RichText::new(l.text(message)).size(13.0).color(t.warning));
        ui.label(RichText::new(l.text("Reloading replaces only this group's session choices. Other settings and your document stay unchanged.")).size(13.0).color(t.muted));
        let response = components::action(ui, l.text("Reload saved settings"), true, None);
        let accessible = l.format(
            "Reload saved settings for {group}",
            &[("group", l.text(group.label()).into())],
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &accessible)
        });
        if response.on_hover_text(accessible).clicked() {
            actions.reload = Some(group);
        }
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);
    }
    pub(super) fn save_status_msgid(&self, unsaved_notes: bool) -> &str {
        #[cfg(feature = "editor-preview")]
        if let Some(canvas) = &self.canvas
            && canvas.durability_status() != "All changes saved"
        {
            return canvas.durability_status();
        }
        if self.status == "All changes saved" && self.preferences_need_attention() {
            if unsaved_notes {
                "Manuscript saved · unsaved notes and session-only settings"
            } else {
                "Manuscript saved · session-only settings"
            }
        } else if self.status == "All changes saved" && unsaved_notes {
            "Manuscript saved · unsaved note edits"
        } else {
            &self.status
        }
    }
}

impl StudioApp {
    fn settings_content(&mut self, ui: &mut egui::Ui, t: Tokens, actions: &mut PreferenceActions) {
        let l = self.language;
        let group = match self.settings_section {
            Section::General => Some(PreferenceGroup::Interface),
            Section::Appearance => Some(PreferenceGroup::Appearance),
            Section::Editor => Some(PreferenceGroup::Editor),
            _ => None,
        };
        if let Some(group) = group {
            self.preference_recovery(ui, t, group, actions);
        }
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
            Section::Editor => self.editor_settings(ui, t),
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

    fn editor_settings(&mut self, ui: &mut egui::Ui, t: Tokens) {
        let l = self.language;
        #[cfg(feature = "editor-preview")]
        let spelling_unavailable = !cfg!(target_os = "macos")
            || self.rich.as_ref().is_some_and(|rich| {
                rich.proofing.availability() == proofing::Availability::Unavailable
            });
        ui.scope(|ui| {
            ui.spacing_mut().interact_size.y = 40.0;
            ui.spacing_mut().item_spacing.y = 16.0;
            let preferences = &mut self.editor_preferences.values;
            ui.label(RichText::new(l.text("Typography")).strong());
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                ui.label(l.text("Manuscript font"));
                egui::ComboBox::from_id_salt("manuscript-font-family")
                    .selected_text(preferences.font_family.family())
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for family in ManuscriptFont::ALL {
                            ui.selectable_value(
                                &mut preferences.font_family,
                                family,
                                family.family(),
                            );
                        }
                    });
            });
            if ![17.0, 19.0, 21.0].contains(&preferences.font_size) {
                ui.label(format!("{} {}", preferences.font_size, l.text("px")));
            }
            preference_choices(
                ui,
                l.text("Manuscript text size"),
                &mut preferences.font_size,
                &[(17.0, "17 px"), (19.0, "19 px"), (21.0, "21 px")],
            );
            preference_choices(
                ui,
                l.text("Line spacing"),
                &mut preferences.line_height,
                &[(1.5, "1.5"), (1.75, "1.75"), (2.0, "2")],
            );
            preference_choices(
                ui,
                l.text("Paragraph spacing"),
                &mut preferences.paragraph_spacing,
                &[(0.65, "0.65 em"), (0.9, "0.9 em"), (1.1, "1.1 em")],
            );
            preference_choices(
                ui,
                l.text("List item spacing"),
                &mut preferences.list_item_spacing,
                &[(0.0, "0 em"), (0.25, "0.25 em"), (0.5, "0.5 em")],
            );
            ui.add_space(8.0);
            ui.separator();
            preference_choices(
                ui,
                l.text("Manuscript width"),
                &mut preferences.manuscript_width,
                &[(60, "60 ch"), (70, "70 ch"), (80, "80 ch")],
            );
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                ui.label(l.text("Formatting toolbar"));
                let width = (ui.available_width() - 16.0) / 3.0;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for mode in ToolbarMode::ALL {
                        if ui
                            .add_sized(
                                [width, 40.0],
                                egui::Button::new(l.text(mode.label()))
                                    .selected(preferences.toolbar_mode == mode),
                            )
                            .clicked()
                        {
                            preferences.toolbar_mode = mode;
                        }
                    }
                });
            });
            #[cfg(feature = "editor-preview")]
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                ui.add_enabled_ui(cfg!(target_os = "macos"), |ui| {
                    ui.checkbox(&mut preferences.spellcheck, l.text("Spellcheck"));
                });
                ui.label(
                    RichText::new(l.text(if spelling_unavailable && preferences.spellcheck {
                        "Spellcheck is unavailable on this system."
                    } else {
                        "Uses your system spelling languages. No automatic corrections."
                    }))
                    .size(13.0)
                    .color(
                        if spelling_unavailable && preferences.spellcheck {
                            t.warning
                        } else {
                            t.muted
                        },
                    ),
                );
            });
            ui.add_space(8.0);
            ui.separator();
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                ui.label(l.text("Word goal"));
                ui.add_sized(
                    [ui.available_width(), 40.0],
                    egui::DragValue::new(&mut preferences.word_goal)
                        .range(0..=1_000_000)
                        .speed(100),
                );
                ui.label(
                    RichText::new(l.text("0 disables the word goal."))
                        .size(13.0)
                        .color(t.muted),
                );
            });
        });
        self.font_size = self.editor_preferences.values.font_size;
    }

    pub(super) fn settings_dialog(&mut self, ctx: &egui::Context, t: Tokens) {
        let l = self.language;
        let previous = self.settings;
        let previous_editor = self.editor_preferences.values.clone();
        let previous_language = self.language;
        let mut actions = PreferenceActions::default();
        let notice = self.preferences_need_attention();
        let footer_height = if notice { 112.0 } else { 56.0 };
        let response = modal(
            ctx,
            l,
            "settings",
            l.text("Settings · WriterCraft"),
            840.0,
            |ui| {
                let height = (ctx.content_rect().height() - 220.0).clamp(300.0, 410.0)
                    - (footer_height - 56.0);
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
                        egui::ScrollArea::vertical()
                            .id_salt("settings-navigation")
                            .max_height(sidebar.height() - 24.0)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
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
                            });
                    },
                );
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(content.shrink(24.0))
                        .layout(Layout::top_down(Align::Min)),
                    |ui| {
                        ui.set_clip_rect(content);
                        ui.spacing_mut().item_spacing = Vec2::new(8.0, 12.0);
                        ui.spacing_mut().interact_size.y = 40.0;
                        ui.horizontal(|ui| {
                            icons::icon(ui, self.settings_section.icon(), 22.0, t.accent);
                            ui.heading(l.text(self.settings_section.label()));
                        });
                        ui.add_space(12.0);
                        egui::ScrollArea::vertical()
                            .id_salt(("settings-content", self.settings_section as u8))
                            .max_height(height - 100.0)
                            .auto_shrink([false, false])
                            .show(ui, |ui| self.settings_content(ui, t, &mut actions));
                    },
                );
                ui.add_space(12.0);
                ui.separator();
                let footer = ui
                    .allocate_exact_size(
                        Vec2::new(ui.available_width(), footer_height),
                        egui::Sense::hover(),
                    )
                    .0;
                let pending = self.preferences_need_attention();
                let retryable = PreferenceGroup::ALL.into_iter().any(|group| {
                    self.preference_pending(group) && self.preference_retryable(group)
                });
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(footer.shrink2(egui::vec2(24.0, 8.0)))
                        .layout(Layout::top_down(Align::Min)),
                    |ui| {
                        if pending {
                            let summary = self.preference_attention_summary();
                            ui.add_sized(
                                [ui.available_width(), 40.0],
                                egui::Label::new(
                                    RichText::new(summary).size(13.0).color(t.warning),
                                )
                                .wrap()
                                .halign(Align::Min),
                            );
                            ui.add_space(8.0);
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let label = if pending {
                                "Continue for this session"
                            } else {
                                "Done"
                            };
                            if components::action(ui, l.text(label), true, Some(t)).clicked() {
                                self.settings_open = false;
                            }
                            if pending
                                && components::action(ui, l.text("Retry save"), retryable, None)
                                    .clicked()
                            {
                                actions.retry = true;
                            }
                        });
                    },
                );
            },
        );
        let mut reload_failure = Vec::new();
        if let Some(group) = actions.reload
            && let Err(error) = self.reload_preference_group(group)
        {
            reload_failure.push((group, error));
        }
        let mut save_failures = Vec::new();
        let changes = [
            (PreferenceGroup::Appearance, self.settings != previous),
            (
                PreferenceGroup::Interface,
                self.language != previous_language,
            ),
            (
                PreferenceGroup::Editor,
                self.editor_preferences.values != previous_editor,
            ),
        ];
        for (group, changed) in changes {
            if changed
                && actions.reload != Some(group)
                && let Err(error) = self.request_preference_save(group)
            {
                save_failures.push((group, error));
            }
        }
        if actions.retry {
            for group in PreferenceGroup::ALL {
                if self.preference_pending(group)
                    && self.preference_retryable(group)
                    && let Err(error) = self.retry_preference_save(group)
                {
                    save_failures.push((group, error));
                }
            }
        }
        if self.settings != previous {
            theme::apply(ctx, &self.settings);
        }
        self.font_size = self.editor_preferences.values.font_size;
        self.report_preference_errors(
            "Settings could not be saved. Your choices still apply to this session.",
            save_failures,
        );
        self.report_preference_errors("Saved settings could not be reloaded. Your session choices and the file are unchanged.", reload_failure);
        if response.should_close() {
            self.settings_open = false;
        }
    }
}

/// One label/control inset and minimum target geometry for each reading preference.
fn preference_choices<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut T,
    choices: &[(T, &str)],
) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 8.0;
        ui.label(label);
        let width = (ui.available_width() - 8.0 * (choices.len().saturating_sub(1)) as f32)
            / choices.len().max(1) as f32;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            for &(choice, text) in choices {
                if ui
                    .add_sized(
                        [width, 40.0],
                        egui::Button::new(text).selected(*value == choice),
                    )
                    .clicked()
                {
                    *value = choice;
                }
            }
        });
    });
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

    fn fixture(directory: &std::path::Path) -> AppResult<(StudioApp, egui::Context)> {
        let context = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(context.clone());
        let app = StudioApp::new(&creation, directory.join("settings.sqlite"), false, None)?;
        Ok((app, context))
    }

    #[test]
    fn all_invalid_groups_stay_protected_while_session_choices_remain_usable() -> AppResult<()> {
        let directory = tempfile::tempdir()?;
        let originals = [
            ("appearance.json", r#"{"appearance":"future-mode"}"#),
            ("interface.json", "{invalid"),
            ("editor.json", r#"{"version":999,"preferences":{}}"#),
        ];
        for (name, original) in originals {
            std::fs::write(directory.path().join(name), original)?;
        }
        let (mut app, _) = fixture(directory.path())?;
        let details = &app
            .error
            .as_ref()
            .ok_or("missing settings diagnostic")?
            .details;
        for (name, _) in originals {
            assert!(details.contains(name));
        }
        app.settings.appearance = Appearance::Light;
        app.language = Language::English;
        app.editor_preferences.values.font_size = 21.0;
        for group in PreferenceGroup::ALL {
            assert_eq!(
                app.request_preference_save(group)?,
                PreferenceSaveOutcome::SessionOnly
            );
            assert!(app.preference_pending(group));
            assert!(!app.preference_retryable(group));
        }
        app.settings_open = false;
        app.error = None;
        for (name, original) in originals {
            assert_eq!(
                std::fs::read_to_string(directory.path().join(name))?,
                original
            );
        }
        let summary = app.preference_attention_summary();
        for group in PreferenceGroup::ALL {
            assert!(summary.contains(group.label()));
        }
        assert_eq!(app.settings.appearance, Appearance::Light);
        assert_eq!(app.editor_preferences.values.font_size, 21.0);
        assert_eq!(
            app.save_status_msgid(false),
            "Manuscript saved · session-only settings"
        );
        assert_eq!(
            app.save_status_msgid(true),
            "Manuscript saved · unsaved notes and session-only settings"
        );
        Ok(())
    }

    #[test]
    fn partial_preference_failure_remains_retriable_after_manuscript_save() -> AppResult<()> {
        let directory = tempfile::tempdir()?;
        let (mut app, _) = fixture(directory.path())?;
        let project = app.workspace.create_project("Synthetic settings project")?;
        app.workspace
            .create_document(&project.id, "Synthetic chapter", "Original")?;
        app.open_project(project)?;
        #[cfg(feature = "editor-preview")]
        {
            app.rich = None;
        }
        let appearance = directory.path().join("appearance.json");
        let external = r#"{"appearance":"dark","external":true}"#;
        std::fs::write(&appearance, external)?;
        app.settings.appearance = Appearance::Light;
        assert!(
            app.request_preference_save(PreferenceGroup::Appearance)
                .is_err()
        );
        app.language = Language::Japanese;
        assert_eq!(
            app.request_preference_save(PreferenceGroup::Interface)?,
            PreferenceSaveOutcome::Acknowledged
        );
        app.editor_preferences.values.font_size = 21.0;
        assert_eq!(
            app.request_preference_save(PreferenceGroup::Editor)?,
            PreferenceSaveOutcome::Acknowledged
        );
        assert!(app.preference_pending(PreferenceGroup::Appearance));
        assert!(!app.preference_pending(PreferenceGroup::Interface));
        assert!(!app.preference_pending(PreferenceGroup::Editor));
        app.body = "The manuscript has its own save acknowledgement".into();
        app.dirty = Some(Instant::now());
        app.save()?;
        assert!(app.dirty.is_none());
        assert_eq!(
            app.save_status_msgid(false),
            "Manuscript saved · session-only settings"
        );
        assert!(
            app.retry_preference_save(PreferenceGroup::Appearance)
                .is_err()
        );
        assert_eq!(std::fs::read_to_string(&appearance)?, external);
        // Repair the external conflict back to the originally acknowledged absence.
        std::fs::remove_file(&appearance)?;
        app.retry_preference_save(PreferenceGroup::Appearance)?;
        assert!(!app.preferences_need_attention());
        assert_eq!(app.save_status_msgid(false), "All changes saved");
        assert_eq!(
            app.save_status_msgid(true),
            "Manuscript saved · unsaved note edits"
        );
        let (reopened, _) = fixture(directory.path())?;
        assert_eq!(reopened.settings.appearance, Appearance::Light);
        assert_eq!(reopened.language, Language::Japanese);
        assert_eq!(reopened.editor_preferences.values.font_size, 21.0);
        Ok(())
    }

    #[test]
    fn reload_is_fresh_validated_and_scoped_to_its_named_group() -> AppResult<()> {
        let directory = tempfile::tempdir()?;
        let appearance = directory.path().join("appearance.json");
        let interface = directory.path().join("interface.json");
        let editor = directory.path().join("editor.json");
        std::fs::write(&appearance, r#"{"appearance":"future-mode"}"#)?;
        std::fs::write(&interface, "{invalid")?;
        let (mut app, _) = fixture(directory.path())?;
        app.settings.appearance = Appearance::Light;
        app.language = Language::French;
        app.editor_preferences.values.font_size = 21.0;
        app.request_preference_save(PreferenceGroup::Appearance)?;
        app.request_preference_save(PreferenceGroup::Interface)?;
        app.request_preference_save(PreferenceGroup::Editor)?;
        let saved_editor = std::fs::read(&editor)?;
        app.body = "Unsaved manuscript choices stay exact".into();
        app.dirty = Some(Instant::now());
        let dirty = app.dirty;
        assert!(
            app.reload_preference_group(PreferenceGroup::Appearance)
                .is_err()
        );
        assert_eq!(app.settings.appearance, Appearance::Light);
        assert!(app.preference_pending(PreferenceGroup::Appearance));
        let repaired = r#"{"appearance":"dark","palette":"dusty-lavender","vendor":{"keep":true}}"#;
        std::fs::write(&appearance, repaired)?;
        app.reload_preference_group(PreferenceGroup::Appearance)?;
        assert_eq!(app.settings.appearance, Appearance::Dark);
        assert_eq!(app.settings.palette, Palette::DustyLavender);
        assert!(!app.preference_needs_attention(PreferenceGroup::Appearance));
        assert_eq!(app.language, Language::French);
        assert!(app.preference_pending(PreferenceGroup::Interface));
        assert_eq!(app.editor_preferences.values.font_size, 21.0);
        assert_eq!(std::fs::read(&editor)?, saved_editor);
        assert_eq!(std::fs::read_to_string(&interface)?, "{invalid");
        assert_eq!(std::fs::read_to_string(&appearance)?, repaired);
        assert_eq!(app.body, "Unsaved manuscript choices stay exact");
        assert_eq!(app.dirty, dirty);
        Ok(())
    }

    #[test]
    fn reload_cannot_reset_pending_choices_when_the_saved_file_is_missing() -> AppResult<()> {
        let directory = tempfile::tempdir()?;
        let (mut app, _) = fixture(directory.path())?;
        app.settings.appearance = Appearance::Light;
        app.language = Language::Japanese;
        app.editor_preferences.values.font_size = 21.0;
        // A directory blocks publication but is never a valid saved settings file.
        for (group, name) in [
            (PreferenceGroup::Appearance, "appearance.json"),
            (PreferenceGroup::Interface, "interface.json"),
            (PreferenceGroup::Editor, "editor.json"),
        ] {
            let path = directory.path().join(name);
            std::fs::create_dir(&path)?;
            assert!(app.request_preference_save(group).is_err());
            std::fs::remove_dir(&path)?;
            assert!(app.reload_preference_group(group).is_err());
            assert!(app.preference_pending(group));
            assert!(!path.exists());
        }
        assert_eq!(app.settings.appearance, Appearance::Light);
        assert_eq!(app.language, Language::Japanese);
        assert_eq!(app.editor_preferences.values.font_size, 21.0);
        Ok(())
    }

    #[test]
    fn editor_size_is_authoritative_and_language_save_preserves_legacy_migration_size()
    -> AppResult<()> {
        let directory = tempfile::tempdir()?;
        let interface = directory.path().join("interface.json");
        let legacy =
            r#"{"language":"en","font_size":24,"vendor":{"exact":123456789012345678901234567890}}"#;
        std::fs::write(&interface, legacy)?;
        let (mut app, _) = fixture(directory.path())?;
        assert_eq!(app.font_size, 24.0);
        app.editor_preferences.values.font_size = 17.0;
        app.request_preference_save(PreferenceGroup::Editor)?;
        assert_eq!(std::fs::read_to_string(&interface)?, legacy);
        app.language = Language::German;
        app.request_preference_save(PreferenceGroup::Interface)?;
        let actual = std::fs::read_to_string(&interface)?;
        assert!(actual.contains("\"font_size\":24"));
        assert!(actual.contains("123456789012345678901234567890"));
        app.reload_preference_group(PreferenceGroup::Editor)?;
        assert_eq!(app.font_size, 17.0);
        assert_eq!(std::fs::read_to_string(&interface)?, actual);
        let (reopened, _) = fixture(directory.path())?;
        assert_eq!(reopened.font_size, 17.0);
        assert_eq!(reopened.language, Language::German);
        Ok(())
    }

    #[test]
    fn recovery_actions_have_bounded_40_point_geometry_in_every_catalog() -> AppResult<()> {
        for language in Language::ALL {
            for size in [Vec2::new(1000.0, 680.0), Vec2::new(1440.0, 940.0)] {
                let directory = tempfile::tempdir()?;
                for name in ["appearance.json", "interface.json", "editor.json"] {
                    std::fs::write(directory.path().join(name), "{invalid")?;
                }
                let (mut app, context) = fixture(directory.path())?;
                context.enable_accesskit();
                app.language = language;
                app.settings_open = true;
                app.error = None;
                app.settings_section = Section::Appearance;
                app.settings.appearance = Appearance::Light;
                app.request_preference_save(PreferenceGroup::Appearance)?;
                let summary = app.preference_attention_summary();
                let reload = language.format(
                    "Reload saved settings for {group}",
                    &[("group", language.text("Appearance").into())],
                );
                let labels = [
                    language.text("Continue for this session"),
                    language.text("Retry save"),
                    reload.as_str(),
                    summary.as_str(),
                ];
                let mut bounds = std::collections::BTreeMap::new();
                for _ in 0..3 {
                    let mut output = context.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                            ..Default::default()
                        },
                        |_| app.settings_dialog(&context, theme::tokens(&context)),
                    );
                    let update = output
                        .platform_output
                        .accesskit_update
                        .take()
                        .ok_or("missing accessibility update")?;
                    // Each pass emits a fresh tree. Do not retain earlier positioning frames.
                    bounds.clear();
                    for (_, node) in update.nodes {
                        // egui exposes static label text as its accessible value; commands use labels.
                        let text = match node.role() {
                            egui::accesskit::Role::Label => node.value(),
                            egui::accesskit::Role::Button => node.label(),
                            _ => None,
                        };
                        if let Some(text) = text
                            && labels.contains(&text)
                            && let Some(rect) = node.bounds()
                        {
                            bounds.insert(text.to_owned(), rect);
                        }
                    }
                    output.textures_delta.clear();
                }
                for label in labels {
                    let rect = bounds
                        .get(label)
                        .ok_or_else(|| format!("{} {size:?}: missing {label}", language.code()))?;
                    if label != labels[3] {
                        assert!(
                            rect.height() >= 39.5,
                            "{} {label}: {rect:?}",
                            language.code()
                        );
                    }
                    assert!(
                        rect.x0 >= 0.0
                            && rect.y0 >= 0.0
                            && rect.x1 <= f64::from(size.x)
                            && rect.y1 <= f64::from(size.y),
                        "{} {label}: {rect:?} outside {size:?}",
                        language.code()
                    );
                }
                let done = bounds.get(labels[0]).ok_or("missing continue bounds")?;
                let retry = bounds.get(labels[1]).ok_or("missing retry bounds")?;
                let summary = bounds.get(labels[3]).ok_or("missing summary bounds")?;
                assert!(retry.x1 <= done.x0, "footer actions overlap");
                assert!(summary.y1 <= done.y0, "summary overlaps footer actions");
                assert!(app.settings_open);
                assert!(app.preferences_need_attention());
                for name in ["appearance.json", "interface.json", "editor.json"] {
                    assert_eq!(
                        std::fs::read_to_string(directory.path().join(name))?,
                        "{invalid"
                    );
                }
            }
        }
        Ok(())
    }
}
