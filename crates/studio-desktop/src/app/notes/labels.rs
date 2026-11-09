//! Project label presentation; definitions are changed only by explicit guarded commands.
use super::*;
use studio_core::{ProjectLabelCatalog, ProjectLabelColor};

#[derive(Default)]
pub(super) struct LabelUi {
    pub catalog: Option<ProjectLabelCatalog>,
    pub manager: bool,
    pub archived: bool,
    pub editor: Option<LabelEditor>,
}

pub(super) struct LabelEditor {
    pub catalog: ProjectLabelCatalog,
    pub original: Option<String>,
    pub name: String,
    pub color: ProjectLabelColor,
    pub target: Option<LabelTarget>,
    pub focus: bool,
    pub conflict: bool,
}

#[derive(Clone)]
pub(super) enum LabelTarget {
    Note(String),
    Composer(String),
}

pub(super) fn same_catalog(
    a: Option<&ProjectLabelCatalog>,
    b: Option<&ProjectLabelCatalog>,
) -> bool {
    a.zip(b)
        .is_some_and(|(a, b)| a.project_id() == b.project_id() && a.revision() == b.revision())
}

pub(super) fn assignments_available(source: &str, catalog: Option<&ProjectLabelCatalog>) -> bool {
    let tags = parsed_tags(source);
    tags.is_empty()
        || catalog.is_some_and(|catalog| {
            tags.iter().all(|tag| {
                catalog
                    .labels()
                    .binary_search_by(|label| label.name.cmp(tag))
                    .is_ok()
            })
        })
}

pub(super) fn color_name(color: ProjectLabelColor) -> &'static str {
    match color {
        ProjectLabelColor::Blue => "Blue",
        ProjectLabelColor::Violet => "Violet",
        ProjectLabelColor::Rose => "Rose",
        ProjectLabelColor::Orange => "Orange",
        ProjectLabelColor::Amber => "Amber",
        ProjectLabelColor::Gray => "Gray",
    }
}

fn luminance(color: egui::Color32) -> f32 {
    let linear = |value: u8| {
        let value = f32::from(value) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
}

fn mix(a: egui::Color32, b: egui::Color32, weight: f32) -> egui::Color32 {
    let channel = |a, b| (f32::from(a) * (1.0 - weight) + f32::from(b) * weight).round() as u8;
    egui::Color32::from_rgb(
        channel(a.r(), b.r()),
        channel(a.g(), b.g()),
        channel(a.b(), b.b()),
    )
}

/// Restrained hue anchors take the palette's neutral undertone and a readable underline contrast.
pub(super) fn color(choice: ProjectLabelColor, t: Tokens) -> egui::Color32 {
    let base = match choice {
        ProjectLabelColor::Blue => [85, 133, 181],
        ProjectLabelColor::Violet => [139, 112, 173],
        ProjectLabelColor::Rose => [178, 108, 130],
        ProjectLabelColor::Orange => [185, 127, 85],
        ProjectLabelColor::Amber => [166, 139, 69],
        ProjectLabelColor::Gray => [124, 132, 143],
    };
    let mut result = mix(
        egui::Color32::from_rgb(base[0], base[1], base[2]),
        t.muted,
        0.18,
    );
    let background = luminance(t.editor);
    for _ in 0..20 {
        let foreground = luminance(result);
        if (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05) >= 3.0 {
            break;
        }
        result = mix(result, t.text, 0.08);
    }
    result
}

pub(super) fn named_color(
    name: &str,
    catalog: Option<&ProjectLabelCatalog>,
    t: Tokens,
) -> egui::Color32 {
    let lookup = |labels: &[studio_core::ProjectLabel]| {
        labels
            .binary_search_by(|label| label.name.as_str().cmp(name))
            .ok()
            .map(|index| labels[index].color)
    };
    color(
        catalog
            .and_then(|catalog| {
                lookup(catalog.labels()).or_else(|| lookup(catalog.retired_labels()))
            })
            .unwrap_or(ProjectLabelColor::Blue),
        t,
    )
}

pub(super) fn primary(
    note: &PassageAnnotation,
    catalog: Option<&ProjectLabelCatalog>,
    t: Tokens,
) -> egui::Color32 {
    note.tags
        .first()
        .map_or(t.warning, |name| named_color(name, catalog, t))
}

pub(super) fn chip(
    ui: &mut egui::Ui,
    name: &str,
    tint: egui::Color32,
    t: Tokens,
) -> egui::Response {
    egui::Frame::new()
        .fill(tint.gamma_multiply(0.13))
        .stroke(Stroke::new(1.0, tint))
        .corner_radius(5)
        .inner_margin(egui::Margin::symmetric(6, 3))
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(RichText::new(name).size(12.0).color(t.text))
                    .selectable(false)
                    .truncate(),
            )
        })
        .inner
        .on_hover_text(name)
}

pub(super) fn preview_chips(
    ui: &mut egui::Ui,
    tags: &[String],
    catalog: Option<&ProjectLabelCatalog>,
    l: Language,
    t: Tokens,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        let mut shown = 0;
        for name in tags {
            let width = ui
                .painter()
                .layout_no_wrap(name.clone(), egui::FontId::proportional(12.0), t.text)
                .size()
                .x
                + 14.0;
            if shown > 0 && width + 42.0 > ui.available_width() {
                break;
            }
            let reserve = if shown + 1 < tags.len() { 42.0 } else { 0.0 };
            ui.allocate_ui_with_layout(
                egui::vec2(width.min((ui.available_width() - reserve).max(24.0)), 25.0),
                Layout::left_to_right(Align::Center),
                |ui| {
                    chip(ui, name, named_color(name, catalog, t), t);
                },
            );
            shown += 1;
        }
        if shown < tags.len() {
            let response = ui.button(format!("+{}", tags.len() - shown));
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, l.text("All labels"))
            });
            egui::Popup::menu(&response).show(|ui| {
                for name in tags {
                    chip(ui, name, named_color(name, catalog, t), t);
                }
            });
        }
    });
}

#[derive(Default)]
pub(super) struct AssignmentAction {
    pub create: bool,
    pub remove: Option<String>,
}

/// Assignment edits stay in the draft; saved-note removals are returned for one guarded command.
pub(super) fn assignment_picker(
    ui: &mut egui::Ui,
    source: &mut String,
    catalog: Option<&ProjectLabelCatalog>,
    l: Language,
    t: Tokens,
    read_only: bool,
    immediate_removal: bool,
) -> AssignmentAction {
    let mut action = AssignmentAction::default();
    let mut tags = parsed_tags(source);
    let original = tags.clone();
    let mut remove = None;
    let mut primary = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
        for (index, name) in tags.iter().enumerate() {
            let tint = named_color(name, catalog, t);
            egui::Frame::new()
                .fill(tint.gamma_multiply(0.13))
                .stroke(Stroke::new(1.0, tint))
                .corner_radius(5)
                .inner_margin(egui::Margin::symmetric(6, 2))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let response = ui.add(
                            egui::Button::new(RichText::new(name).size(13.0).color(t.text))
                                .frame(false)
                                .truncate(),
                        );
                        response.clone().on_hover_text(name);
                        // One chip-anchored popup serves pointer and keyboard activation.
                        // Rendering a second context menu with this response would reuse its ID.
                        if response.secondary_clicked() {
                            egui::Popup::open_id(
                                ui.ctx(),
                                egui::Popup::default_response_id(&response),
                            );
                        }
                        egui::Popup::menu(&response).show(|ui| {
                            if ui
                                .add_enabled(
                                    !read_only && index != 0,
                                    egui::Button::new(l.text("Use this color in text")),
                                )
                                .clicked()
                            {
                                primary = Some(index);
                                ui.close();
                            }
                        });
                        if ui
                            .add_enabled_ui(!read_only, |ui| {
                                icons::icon_button(
                                    ui,
                                    Icon::Close,
                                    l.text("Remove label from this passage"),
                                )
                            })
                            .inner
                            .clicked()
                        {
                            remove = Some(index);
                        }
                    });
                });
        }
    });
    if let Some(index) = remove {
        if immediate_removal {
            action.remove = Some(tags[index].clone());
        } else {
            tags.remove(index);
        }
    }
    if let Some(index) = primary
        && index < tags.len()
    {
        let name = tags.remove(index);
        tags.insert(0, name);
    }
    ui.add_enabled_ui(!read_only, |ui| {
        ui.menu_button(l.text("Add label…"), |ui| {
            egui::ScrollArea::vertical()
                .max_height(220.0)
                .show(ui, |ui| {
                    if let Some(catalog) = catalog {
                        for label in catalog.labels() {
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(12.0, 12.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().circle_filled(
                                    rect.center(),
                                    5.0,
                                    color(label.color, t),
                                );
                                let mut assigned = tags.contains(&label.name);
                                if ui
                                    .add_enabled(
                                        assigned || tags.len() < 32,
                                        egui::Checkbox::new(&mut assigned, &label.name),
                                    )
                                    .on_disabled_hover_text(
                                        l.text("Use at most 32 tags on one passage"),
                                    )
                                    .changed()
                                {
                                    if assigned {
                                        tags.push(label.name.clone());
                                    } else if immediate_removal {
                                        action.remove = Some(label.name.clone());
                                        ui.close();
                                    } else {
                                        tags.retain(|name| name != &label.name);
                                    }
                                }
                            });
                        }
                    }
                });
            ui.separator();
            if ui.button(l.text("Create label…")).clicked() {
                action.create = true;
                ui.close();
            }
        });
    });
    if tags.len() > 1 {
        ui.label(
            RichText::new(
                l.text("The first label sets the text color. Open a label to change the order."),
            )
            .size(11.0)
            .color(t.muted),
        );
    }
    if tags != original {
        *source = tags.join("\n");
    }
    action
}

impl StudioApp {
    fn acknowledge_label_edit(
        &mut self,
        before: &ProjectLabelCatalog,
        after: &ProjectLabelCatalog,
        target: Option<&LabelTarget>,
        all: bool,
    ) {
        for (id, draft) in &mut self.notes.drafts {
            if (all || matches!(target, Some(LabelTarget::Note(target)) if target == id))
                && same_catalog(draft.catalog.as_ref(), Some(before))
            {
                draft.catalog = Some(after.clone());
            }
        }
        for composer in self
            .notes
            .composer
            .iter_mut()
            .chain(self.notes.paused_composers.values_mut())
        {
            if (all
                || matches!(target, Some(LabelTarget::Composer(target)) if target == &composer.id))
                && same_catalog(composer.catalog.as_ref(), Some(before))
            {
                composer.catalog = Some(after.clone());
            }
        }
    }
    pub(super) fn cache_label_catalog(&mut self, catalog: ProjectLabelCatalog) -> AppResult<()> {
        self.notes.labels.catalog = Some(catalog);
        self.notes.epoch = self
            .notes
            .epoch
            .checked_add(1)
            .ok_or("label_presentation_epoch_overflow")?;
        Ok(())
    }

    pub(super) fn refresh_label_catalog(&mut self) -> AppResult<()> {
        let project = self
            .document
            .as_ref()
            .map(|document| document.project_id.as_str())
            .or_else(|| self.project.as_ref().map(|project| project.id.as_str()))
            .ok_or("missing_label_project")?;
        self.cache_label_catalog(self.workspace.project_labels(project)?)
    }

    pub(super) fn begin_label_creation(&mut self, target: Option<LabelTarget>) -> AppResult<()> {
        self.refresh_label_catalog()?;
        let catalog = self
            .notes
            .labels
            .catalog
            .clone()
            .ok_or("missing_label_catalog")?;
        self.notes.labels.editor = Some(LabelEditor {
            catalog,
            original: None,
            name: String::new(),
            color: ProjectLabelColor::Blue,
            target,
            focus: true,
            conflict: false,
        });
        Ok(())
    }

    pub(super) fn open_label_manager(&mut self) -> AppResult<()> {
        self.refresh_label_catalog()?;
        self.notes.labels.manager = true;
        Ok(())
    }

    fn attach_created_label(&mut self, target: LabelTarget, name: &str) -> AppResult<()> {
        let source = match target {
            LabelTarget::Note(id) => self.notes.drafts.get_mut(&id).map(|draft| &mut draft.tags),
            LabelTarget::Composer(id) => self
                .notes
                .composer
                .iter_mut()
                .chain(self.notes.paused_composers.values_mut())
                .find(|composer| composer.id == id)
                .map(|composer| &mut composer.tags),
        }
        .ok_or("The label was created. Add it to the passage again.")?;
        let mut tags = parsed_tags(source);
        if !tags.iter().any(|tag| tag == name) {
            if tags.len() >= 32 {
                return Err(
                    "The label was created, but this passage already has 32 labels.".into(),
                );
            }
            tags.push(name.into());
            *source = tags.join("\n");
        }
        Ok(())
    }

    /// Returns true while a label surface owns modal input; other note drafts stay in memory.
    pub(super) fn label_dialogs(&mut self, ctx: &egui::Context, t: Tokens) -> bool {
        let l = self.language;
        if let Some(mut editor) = self.notes.labels.editor.take() {
            let mut save = false;
            let mut cancel = false;
            let mut review = false;
            let response = components::modal(
                ctx,
                l,
                "project-label-editor",
                l.text(if editor.original.is_some() {
                    "Label color"
                } else {
                    "Create label"
                }),
                480.0,
                |ui| {
                    if editor.original.is_some() {
                        ui.label(RichText::new(&editor.name).strong());
                    } else {
                        ui.label(l.text("Label name"));
                        let field =
                            components::singleline(ui, &mut editor.name, l.text("Label name"));
                        if editor.focus {
                            field.request_focus();
                            editor.focus = false;
                        }
                    }
                    ui.add_space(12.0);
                    color_picker(ui, &mut editor.color, l, t);
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(
                            l.text("This color identifies the label throughout the project."),
                        )
                        .size(12.0)
                        .color(t.muted),
                    );
                    if editor.conflict {
                        review = ui.button(l.text("Review label changes")).clicked();
                    }
                    components::footer(ui, |ui| {
                        save = components::action(
                            ui,
                            l.text("Save"),
                            valid_tags(&[editor.name.clone()]),
                            Some(t),
                        )
                        .clicked();
                        cancel = components::action(ui, l.text("Cancel"), true, None).clicked();
                    });
                },
            );
            if review {
                match self.workspace.project_labels(editor.catalog.project_id()) {
                    Ok(catalog) => {
                        editor.catalog = catalog.clone();
                        editor.conflict = false;
                        let result = self.cache_label_catalog(catalog);
                        self.record_error(result);
                    }
                    Err(error) => self.record_error(Err(error.into())),
                }
            }
            if save {
                let changed = if let Some(name) = &editor.original {
                    self.workspace
                        .set_project_label_color(&editor.catalog, name, editor.color)
                } else {
                    self.workspace
                        .create_project_label(&editor.catalog, &editor.name, editor.color)
                };
                match changed {
                    Ok(catalog) => {
                        self.acknowledge_label_edit(
                            &editor.catalog,
                            &catalog,
                            editor.target.as_ref(),
                            editor.original.is_some(),
                        );
                        let result = self.cache_label_catalog(catalog).and_then(|_| {
                            if let Some(target) = editor.target {
                                self.attach_created_label(target, &editor.name)?;
                            }
                            Ok(())
                        });
                        self.record_error(result);
                    }
                    Err(error) => {
                        editor.conflict = true;
                        self.notes.labels.editor = Some(editor);
                        self.record_error(Err(error.into()));
                    }
                }
            } else if !cancel && !response.should_close() {
                self.notes.labels.editor = Some(editor);
            }
            return true;
        }
        if !self.notes.labels.manager {
            return false;
        }
        let Some(catalog) = self.notes.labels.catalog.clone() else {
            return false;
        };
        let mut create = false;
        let mut recolor = None;
        let mut rename = None;
        let mut remove = None;
        let mut restore = None;
        let mut close = false;
        let response = components::modal(
            ctx,
            l,
            "project-label-manager",
            l.text("Project labels"),
            540.0,
            |ui| {
                ui.horizontal(|ui| {
                    create =
                        components::action(ui, l.text("Create label…"), true, Some(t)).clicked();
                    ui.checkbox(&mut self.notes.labels.archived, l.text("Retired labels"));
                });
                ui.add_space(12.0);
                let entries = if self.notes.labels.archived {
                    catalog.retired_labels()
                } else {
                    catalog.labels()
                };
                egui::ScrollArea::vertical()
                    .max_height((ctx.content_rect().height() - 300.0).clamp(80.0, 360.0))
                    .show(ui, |ui| {
                        for label in entries {
                            ui.push_id(&label.name, |ui| {
                                ui.horizontal(|ui| {
                                    ui.allocate_ui_with_layout(
                                        egui::vec2((ui.available_width() - 120.0).max(40.0), 36.0),
                                        Layout::left_to_right(Align::Center),
                                        |ui| {
                                            chip(ui, &label.name, color(label.color, t), t);
                                        },
                                    );
                                    if self.notes.labels.archived {
                                        if ui.button(l.text("Restore label")).clicked() {
                                            restore = Some(label.name.clone());
                                        }
                                    } else {
                                        if icons::icon_button(
                                            ui,
                                            Icon::Palette,
                                            l.text("Label color"),
                                        )
                                        .clicked()
                                        {
                                            recolor = Some(label.clone());
                                        }
                                        let more = icons::icon_button(
                                            ui,
                                            Icon::More,
                                            l.text("Label actions"),
                                        );
                                        egui::Popup::menu(&more).show(|ui| {
                                            if ui.button(l.text("Rename tag…")).clicked() {
                                                rename = Some(label.name.clone());
                                                ui.close();
                                            }
                                            if ui.button(l.text("Remove tag…")).clicked() {
                                                remove = Some(label.name.clone());
                                                ui.close();
                                            }
                                        });
                                    }
                                })
                            });
                        }
                        if entries.is_empty() {
                            ui.label(l.text("No labels in this view."));
                        }
                    });
                components::footer(ui, |ui| {
                    close = components::action(ui, l.text("Done"), true, Some(t)).clicked();
                });
            },
        );
        if close || response.should_close() {
            self.notes.labels.manager = false;
        }
        if create {
            let result = self.begin_label_creation(None);
            self.record_error(result);
        }
        if let Some(label) = recolor {
            self.notes.labels.editor = Some(LabelEditor {
                catalog: catalog.clone(),
                original: Some(label.name.clone()),
                name: label.name,
                color: label.color,
                target: None,
                focus: false,
                conflict: false,
            });
        }
        if let Some(name) = restore {
            let result = self
                .workspace
                .restore_project_label(&catalog, &name)
                .map_err(Into::into)
                .and_then(|after| {
                    self.acknowledge_label_edit(&catalog, &after, None, true);
                    self.cache_label_catalog(after)
                });
            self.record_error(result);
        }
        let change = rename
            .map(|name| (name, false))
            .or_else(|| remove.map(|name| (name, true)));
        if let Some((name, remove)) = change {
            let result = (|| -> AppResult<()> {
                self.open_project_marks()?;
                let snapshot = self
                    .workspace
                    .project_tag_snapshot(catalog.project_id(), &name)?;
                let pending_drafts = self.pending_project_markup_drafts(catalog.project_id())?;
                if let Some(browser) = &mut self.notes.project_browser {
                    browser.tag_action = Some(TagAction {
                        replacement: name,
                        snapshot,
                        remove,
                        focus: true,
                        pending_drafts,
                    });
                }
                self.notes.labels.manager = false;
                Ok(())
            })();
            self.record_error(result);
        }
        true
    }
}

pub(super) fn color_picker(
    ui: &mut egui::Ui,
    selected: &mut ProjectLabelColor,
    l: Language,
    t: Tokens,
) {
    ui.horizontal_wrapped(|ui| {
        for choice in ProjectLabelColor::ALL {
            let active = *selected == choice;
            let tint = color(choice, t);
            let response = ui.add(
                egui::Button::new(RichText::new(l.text(color_name(choice))).color(t.text))
                    .min_size(egui::vec2(104.0, 34.0))
                    .fill(tint.gamma_multiply(0.13))
                    .stroke(Stroke::new(
                        if active { 2.5 } else { 1.0 },
                        if active { tint } else { t.border },
                    )),
            );
            response.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::WidgetType::SelectableLabel,
                    true,
                    active,
                    l.text(color_name(choice)),
                )
            });
            if response.clicked() {
                *selected = choice;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_colors_remain_distinct_readable_and_soft_in_every_palette_and_mode() {
        for palette in crate::theme::Palette::ALL {
            for appearance in [
                crate::theme::Appearance::Light,
                crate::theme::Appearance::Dark,
            ] {
                let tokens = crate::theme::resolve(
                    &crate::theme::ThemeSettings {
                        appearance,
                        palette,
                    },
                    false,
                );
                let mut seen = std::collections::HashSet::new();
                for choice in ProjectLabelColor::ALL {
                    let foreground = color(choice, tokens);
                    assert!(seen.insert([foreground.r(), foreground.g(), foreground.b()]));
                    let a = luminance(foreground);
                    let b = luminance(tokens.editor);
                    assert!(
                        (a.max(b) + 0.05) / (a.min(b) + 0.05) >= 3.0,
                        "{palette:?} {appearance:?} {choice:?}"
                    );
                    let chip = mix(tokens.panel, foreground, 0.13);
                    let a = luminance(tokens.text);
                    let b = luminance(chip);
                    assert!(
                        (a.max(b) + 0.05) / (a.min(b) + 0.05) >= 4.5,
                        "{palette:?} {appearance:?} {choice:?}"
                    );
                }
            }
        }
    }
}
