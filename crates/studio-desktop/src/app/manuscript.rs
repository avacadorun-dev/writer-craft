//! Native manuscript component.
use super::*;

impl StudioApp {
    pub(super) fn editor(&mut self, ui: &mut egui::Ui, t: Tokens) {
        let l = self.language;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(t.editor).inner_margin(0))
            .show(ui, |ui| {
                let Some(document) = self.document.clone() else {
                    ui.centered_and_justified(|ui| {
                        if ui.button(l.text("Add your first document")).clicked() {
                            self.document_dialog = true;
                        }
                    });
                    return;
                };
                egui::Panel::top("document-heading")
                    .exact_size(56.0)
                    .resizable(false)
                    .frame(
                        egui::Frame::new()
                            .fill(t.editor)
                            .inner_margin(egui::Margin::symmetric(24, 8)),
                    )
                    .show(ui, |ui| {
                        ui.horizontal_centered(|ui| {
                            let focus_label = l.text(if self.focus_mode {
                                "Exit focus"
                            } else {
                                "Focus"
                            });
                            let focus_width = ui
                                .painter()
                                .layout_no_wrap(
                                    focus_label.into(),
                                    FontId::proportional(14.0),
                                    t.text,
                                )
                                .size()
                                .x
                                + 52.0;
                            let read_only = !self.can_edit_manuscript();
                            let badge_width = if read_only {
                                ui.painter()
                                    .layout_no_wrap(
                                        l.text("Read only").into(),
                                        FontId::proportional(11.0),
                                        t.warning,
                                    )
                                    .size()
                                    .x
                                    + 16.0
                            } else {
                                0.0
                            };
                            let title_width =
                                (ui.available_width() - focus_width - badge_width - 128.0)
                                    .max(60.0);
                            ui.allocate_ui_with_layout(
                                egui::vec2(title_width, 40.0),
                                Layout::left_to_right(Align::Center),
                                |ui| {
                                    ui.add_enabled_ui(!self.is_recovery_read_only(), |ui| {
                                        self.document_title(ui, &document, t);
                                    });
                                },
                            );
                            if read_only {
                                ui.label(
                                    RichText::new(l.text("Read only"))
                                        .size(11.0)
                                        .color(t.warning),
                                )
                                .on_hover_text(l.text(
                                    if self.is_recovery_read_only() {
                                        "Recovery preview · read only"
                                    } else {
                                        "Original format · read only"
                                    },
                                ));
                            }
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if icons::icon_button(ui, Icon::Search, l.text("Find in document"))
                                    .clicked()
                                {
                                    self.open_document_find();
                                }
                                if icons::icon_button(ui, Icon::Export, l.text("Export text"))
                                    .clicked()
                                {
                                    self.export_text();
                                }
                                #[cfg(feature = "editor-preview")]
                                if self.rich.is_some()
                                    && icons::icon_button(ui, Icon::Assistant, l.text("Notes"))
                                        .clicked()
                                {
                                    self.inspector = Inspector::Notes;
                                    self.inspector_visible = true;
                                    self.prefer_inspector = true;
                                    self.focus_mode = false;
                                }
                                if icons::labeled_button(
                                    ui,
                                    Icon::Focus,
                                    focus_label,
                                    self.focus_mode,
                                )
                                .on_hover_text(l.text(if self.focus_mode {
                                    "Exit focus"
                                } else {
                                    "Hide side panels · ⌘⇧F"
                                }))
                                .clicked()
                                {
                                    self.focus_mode = !self.focus_mode;
                                }
                            });
                        });
                    });
                #[cfg(feature = "editor-preview")]
                if self.rich.is_some() {
                    self.rich_editor(ui, t);
                    return;
                }
                egui::ScrollArea::vertical()
                    .id_salt((&document.id, "manuscript"))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let available = ui.available_width();
                        let margin = if available < 520.0 { 20.0 } else { 36.0 };
                        let width = (available - margin * 2.0).clamp(160.0, 710.0);
                        let offset = ((available - width) * 0.5).max(0.0);
                        ui.horizontal(|ui| {
                            ui.add_space(offset);
                            ui.vertical(|ui| {
                                ui.set_width(width);
                                ui.add_space(28.0);
                                let editor_id = egui::Id::new(("manuscript-body", &document.id));
                                let keyboard_menu_requested = ui
                                    .memory(|memory| memory.has_focus(editor_id))
                                    && ui.input_mut(|input| {
                                        input.consume_key(egui::Modifiers::SHIFT, egui::Key::F10)
                                    });
                                let previous_selection =
                                    egui::TextEdit::load_state(ui.ctx(), editor_id)
                                        .and_then(|state| state.cursor.char_range())
                                        .filter(|range| !range.is_empty());
                                let mut output = egui::TextEdit::multiline(&mut self.body)
                                    .id(egui::Id::new(("manuscript-body", &document.id)))
                                    .font(FontId::new(
                                        self.font_size,
                                        FontFamily::Name("Manuscript".into()),
                                    ))
                                    .text_color(t.text)
                                    .frame(egui::Frame::NONE)
                                    .margin(0)
                                    .desired_width(width)
                                    .desired_rows(24)
                                    .interactive(
                                        !document.read_only && document.rich_json.is_none(),
                                    )
                                    .hint_text(l.text("Start writing…"))
                                    .show(ui);
                                if self.focus_manuscript {
                                    output.response.request_focus();
                                    self.focus_manuscript = false;
                                }
                                // egui moves its cursor for any mouse button. Preserve the author's
                                // existing range before opening a secondary-click context menu.
                                let secondary_press = output.response.hovered()
                                    && ui.input(|input| {
                                        input.pointer.button_pressed(egui::PointerButton::Secondary)
                                    });
                                if secondary_press && let Some(range) = previous_selection {
                                    output.cursor_range = Some(range);
                                    output.state.cursor.set_char_range(Some(range));
                                    output.state.clone().store(ui.ctx(), editor_id);
                                    output.response.request_focus();
                                }
                                if output.response.changed() {
                                    self.dirty = Some(Instant::now());
                                    self.status = "Saving…".into();
                                    let result = self
                                        .workspace
                                        .save_draft(&document.id, document.revision, &self.body)
                                        .map_err(Into::into);
                                    self.autosave_paused = result.is_err();
                                    self.record_error(result);
                                }
                                if let Some(range) =
                                    output.cursor_range.filter(|range| !range.is_empty())
                                {
                                    let caret_cursor = range.primary;
                                    let range = range.as_sorted_char_range();
                                    let selected: String = self
                                        .body
                                        .chars()
                                        .skip(usize::from(range.start))
                                        .take(usize::from(range.end) - usize::from(range.start))
                                        .collect();
                                    let keyboard_menu = keyboard_menu_requested;
                                    let popup_id = egui::Id::new(("selection-menu", &document.id));
                                    if keyboard_menu {
                                        egui::Popup::open_id(ui.ctx(), popup_id);
                                    }
                                    let mut popup =
                                        egui::Popup::context_menu(&output.response).id(popup_id);
                                    if keyboard_menu {
                                        let caret = output.galley.pos_from_cursor(caret_cursor);
                                        popup = popup.at_position(
                                            output.galley_pos + caret.left_bottom().to_vec2(),
                                        );
                                    }
                                    popup.show(|ui| {
                                        if ui.button(l.text("Copy selection")).clicked() {
                                            ui.ctx().copy_text(selected.clone());
                                            ui.close();
                                        }
                                        if !document.read_only
                                            && ui
                                                .button(l.text("Prepare an edit from selection"))
                                                .clicked()
                                        {
                                            let result = self.save();
                                            if result.is_ok()
                                                && let Some(current) = &self.document
                                            {
                                                self.expected.clone_from(&selected);
                                                self.focus_replacement = true;
                                                self.frozen_selection = Some(FrozenSelection {
                                                    document_id: current.id.clone(),
                                                    base_revision: current.revision,
                                                    body: self.body.clone(),
                                                    anchor: TextAnchor {
                                                        start: usize::from(range.start),
                                                        end: usize::from(range.end),
                                                        expected_text: selected.clone(),
                                                    },
                                                });
                                                self.inspector = Inspector::Review;
                                                self.inspector_visible = true;
                                                self.prefer_inspector = true;
                                                self.focus_mode = false;
                                                self.status =
                                                    "Selected excerpt sent to the review panel"
                                                        .into();
                                            }
                                            self.record_error(result);
                                            ui.close();
                                        }
                                    });
                                }
                                ui.add_space(80.0);
                            });
                        });
                    });
            });
    }
}
