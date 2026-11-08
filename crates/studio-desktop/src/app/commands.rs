//! Native commands component.
use super::*;

impl StudioApp {
    pub(super) fn toolbar(&mut self, ui: &mut egui::Ui, t: Tokens) {
        let l = self.language;
        let ctx = ui.ctx().clone();
        let (show_nav, show_inspector) = self.effective_panes(&ctx);
        let nav_width = self.navigator_width(&ctx);
        let inspector_width =
            egui::containers::panel::PanelState::load(&ctx, egui::Id::new("inspector"))
                .map_or(320.0, |state| state.outer_rect.width().clamp(280.0, 460.0));
        egui::Panel::top("commands")
            .exact_size(48.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(t.panel))
            .show(ui, |ui| {
                let rect = ui.max_rect();
                let identity_width = if self.library || show_nav {
                    nav_width
                } else {
                    248.0
                };
                let boundary = rect.left() + identity_width;
                let identity =
                    egui::Rect::from_min_max(rect.min, egui::pos2(boundary, rect.bottom()));
                let commands = egui::Rect::from_min_max(egui::pos2(boundary, rect.top()), rect.max);
                let drag = ui.interact(rect, egui::Id::new("titlebar-drag"), egui::Sense::drag());
                if drag.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(identity.shrink2(egui::vec2(12.0, 5.0)))
                        .layout(Layout::left_to_right(Align::Center)),
                    |ui| {
                        ui.set_clip_rect(identity);
                        if cfg!(target_os = "macos") {
                            ui.add_space(76.0);
                        }
                        crate::brand::mark(ui, 28.0, t.accent, t.text);
                        ui.add(
                            egui::Label::new(RichText::new("WriterCraft").size(17.0).strong())
                                .truncate(),
                        )
                        .on_hover_text("WriterCraft");
                    },
                );
                ui.painter()
                    .vline(boundary, rect.y_range(), egui::Stroke::new(1.0, t.border));
                if show_inspector && !self.library {
                    ui.painter().vline(
                        rect.right() - inspector_width,
                        rect.y_range(),
                        egui::Stroke::new(1.0, t.border),
                    );
                }
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(commands.shrink2(egui::vec2(12.0, 5.0)))
                        .layout(Layout::left_to_right(Align::Center)),
                    |ui| {
                        ui.set_clip_rect(commands);
                        if icons::icon_button(ui, Icon::Home, l.text("Project library")).clicked() {
                            let result = self.save_explicit();
                            if result.is_ok() {
                                self.library = true;
                            }
                            self.record_error(result);
                        }
                        ui.menu_button(l.text("File"), |ui| {
                            if ui.button(l.text("New project…     ⌘N")).clicked() {
                                self.project_dialog = true;
                                ui.close();
                            }
                            if ui.button(l.text("Open .wstudio…     ⌘O")).clicked() {
                                self.choose_archive();
                                ui.close();
                            }
                            if ui.button(l.text("Export text…")).clicked() {
                                self.export_text();
                                ui.close();
                            }
                        });
                        if icons::icon_button(ui, Icon::Save, l.text("Save · ⌘S")).clicked() {
                            let result = self.save_explicit();
                            self.record_error(result);
                        }
                        if !self.library
                            && let Some(project) = &self.project
                        {
                            let right_zone = if show_inspector {
                                inspector_width
                            } else {
                                128.0
                            };
                            let width = (ui.available_width() - right_zone).max(0.0);
                            ui.add_sized(
                                [width, 30.0],
                                egui::Label::new(
                                    RichText::new(&project.title).size(14.0).color(t.muted),
                                )
                                .truncate(),
                            )
                            .on_hover_text(&project.title);
                        } else {
                            ui.add_space((ui.available_width() - 128.0).max(0.0));
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if icons::icon_button(ui, Icon::Settings, l.text("Settings · ⌘,"))
                                .clicked()
                            {
                                self.settings_open = true;
                            }
                            if !self.library {
                                if icons::icon_button(
                                    ui,
                                    Icon::Assistant,
                                    l.text("Review panel · ⌘⇧A"),
                                )
                                .clicked()
                                {
                                    let (_, visible) = self.effective_panes(ui.ctx());
                                    self.inspector_visible = !visible;
                                    self.prefer_inspector = true;
                                    self.focus_mode = false;
                                }
                                if icons::icon_button(
                                    ui,
                                    Icon::Sidebar,
                                    l.text("Project structure"),
                                )
                                .clicked()
                                {
                                    let (visible, _) = self.effective_panes(ui.ctx());
                                    self.navigator = !visible;
                                    self.prefer_inspector = false;
                                    self.focus_mode = false;
                                }
                            }
                        });
                    },
                );
            });
    }

    fn document_statistics(&self) -> studio_core::DocumentStatistics {
        #[cfg(feature = "editor-preview")]
        if let Some(rich) = &self.rich {
            return rich.statistics();
        }
        studio_core::document_statistics(&self.body)
    }

    pub(super) fn status_bar(&mut self, ui: &mut egui::Ui, t: Tokens) {
        let l = self.language;
        egui::Panel::bottom("status")
            .exact_size(28.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(t.panel)
                    .inner_margin(egui::Margin::symmetric(16, 4)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if crate::editor_preview_requested() {
                            l.text("Editor Preview")
                        } else {
                            env!("CARGO_PKG_VERSION")
                        })
                        .size(11.0)
                        .color(t.muted),
                    );
                    ui.separator();
                    icons::icon(ui, Icon::Shield, 13.0, t.good);
                    ui.label(
                        RichText::new(l.text("On this computer"))
                            .size(12.0)
                            .color(t.muted),
                    );
                    ui.separator();
                    #[cfg(feature = "editor-preview")]
                    let unsaved_notes = self.notes.has_unsaved_drafts();
                    #[cfg(not(feature = "editor-preview"))]
                    let unsaved_notes = false;
                    let status = if unsaved_notes && self.status == "All changes saved" {
                        l.text("Manuscript saved · unsaved note edits")
                    } else {
                        l.text(&self.status)
                    };
                    ui.label(RichText::new(status).size(12.0).color(
                        if self.error.is_some() || unsaved_notes {
                            t.warning
                        } else {
                            t.muted
                        },
                    ));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if let Some(document) = &self.document
                            && !self.library
                        {
                            let statistics = self.document_statistics();
                            let label = l.format(
                                "Words: {count}   ·   Revision {revision}",
                                &[
                                    ("count", statistics.words.to_string()),
                                    ("revision", document.revision.to_string()),
                                ],
                            );
                            let response = ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(label).size(12.0).color(t.muted),
                                    )
                                    .frame(false),
                                )
                                .on_hover_text(l.text("Document statistics"));
                            egui::Popup::menu(&response).show(|ui| {
                                ui.set_min_width(240.0);
                                ui.label(RichText::new(l.text("Document statistics")).strong());
                                ui.add_space(8.0);
                                egui::Grid::new("document-statistics")
                                    .num_columns(2)
                                    .spacing([24.0, 12.0])
                                    .show(ui, |ui| {
                                        for (msgid, value) in [
                                            ("Words", statistics.words.to_string()),
                                            ("Characters", statistics.characters.to_string()),
                                            (
                                                "Reading time",
                                                l.format(
                                                    "≈ {minutes} min read",
                                                    &[(
                                                        "minutes",
                                                        statistics.reading_minutes.to_string(),
                                                    )],
                                                ),
                                            ),
                                        ] {
                                            ui.label(l.text(msgid));
                                            ui.label(value);
                                            ui.end_row();
                                        }
                                    });
                                let goal = self.editor_preferences.values.word_goal;
                                if goal > 0 {
                                    ui.add_space(12.0);
                                    ui.label(RichText::new(l.text("Word goal")).strong());
                                    ui.add(
                                        egui::ProgressBar::new(
                                            (statistics.words as f32 / goal as f32).clamp(0.0, 1.0),
                                        )
                                        .text(l.format(
                                            "{words} / {goal}",
                                            &[
                                                ("words", statistics.words.to_string()),
                                                ("goal", goal.to_string()),
                                            ],
                                        )),
                                    );
                                }
                            });
                        }
                    });
                });
            });
    }

    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.daily_controls.title_has_focus(ctx) {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
                let result = self.save_explicit();
                self.record_error(result);
            }
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            #[cfg(feature = "editor-preview")]
            let result = self
                .save_focused_note(ctx)
                .unwrap_or_else(|| self.save_explicit());
            #[cfg(not(feature = "editor-preview"))]
            let result = self.save_explicit();
            self.record_error(result);
            if self.modal_open() {
                return;
            }
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F)) {
            self.open_document_find();
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::N)) {
            self.project_dialog = true;
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O)) {
            self.choose_archive();
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Comma)) {
            self.settings_open = true;
            return;
        }
        if ctx.input_mut(|i| {
            i.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::A,
            )
        }) {
            let (_, visible) = self.effective_panes(ctx);
            self.inspector_visible = !visible;
            self.prefer_inspector = true;
            self.focus_mode = false;
        }
        if ctx.input_mut(|i| {
            i.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::F,
            )
        }) {
            self.focus_mode = !self.focus_mode;
        }
    }
}
