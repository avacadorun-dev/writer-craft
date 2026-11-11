use super::*;
use egui::{Rect, Response, Sense};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Action {
    New,
    Import,
    Sample,
    Open(String),
}

pub(super) fn show(
    ui: &mut egui::Ui,
    projects: &[Project],
    resume: Option<&Project>,
    l: Language,
    t: Tokens,
) -> Option<Action> {
    let mut action = None;
    let short = ui.available_height() < 520.0;
    // One local scroll includes the banner, so it cannot trap the list in a
    // short window. No horizontal scroll area or fixed screenshot geometry.
    egui::ScrollArea::vertical()
        .id_salt("home-body")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.set_max_width(1280.0);
                ui.with_layout(Layout::top_down(Align::Min), |ui| {
                    banner(ui, projects.is_empty(), resume, l, t, short, &mut action);
                    ui.add_space(if short { 16.0 } else { 24.0 });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(l.text("All projects"))
                                .size(23.0)
                                .strong()
                                .color(t.text),
                        );
                        ui.add_space(16.0);
                        if action_button(ui, l.text("New project"), false, t, "home.new").clicked()
                        {
                            action = Some(Action::New);
                        }
                        if action_button(ui, l.text("Open .wstudio…"), false, t, "home.import")
                            .clicked()
                        {
                            action = Some(Action::Import);
                        }
                    });
                    ui.add_space(12.0);
                    for project in projects {
                        ui.push_id(&project.id, |ui| {
                            if project_row(ui, project, l, t).clicked() {
                                action = Some(Action::Open(project.id.clone()));
                            }
                        });
                    }
                });
            });
        });
    action
}

fn banner(
    ui: &mut egui::Ui,
    empty: bool,
    resume: Option<&Project>,
    l: Language,
    t: Tokens,
    short: bool,
    action: &mut Option<Action>,
) {
    let wide = ui.available_width() >= 660.0;
    let padding = if wide { 24 } else { 16 };
    let frame = egui::Frame::new()
        .fill(t.accent_soft)
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(10)
        .inner_margin(padding)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let width = ui.available_width();
            let image_width = if wide {
                (width * 0.47).min(if short { 330.0 } else { 440.0 })
            } else {
                width.min(if short { 230.0 } else { 290.0 })
            };
            let text = |ui: &mut egui::Ui, action: &mut Option<Action>| {
                ui.spacing_mut().item_spacing.y = 10.0;
                ui.add(
                    egui::Label::new(
                        RichText::new(l.text(if resume.is_some() {
                            "Continue writing"
                        } else if empty {
                            "Your first chapter starts here"
                        } else {
                            "All projects"
                        }))
                        .size(if wide { 30.0 } else { 26.0 })
                        .strong()
                        .color(t.text),
                    )
                    .wrap(),
                );
                if let Some(project) = resume {
                    ui.add(
                        egui::Label::new(RichText::new(&project.title).size(16.0).color(t.text))
                            .truncate(),
                    )
                    .on_hover_text(&project.title);
                }
                ui.add_space(6.0);
                let label = if resume.is_some() {
                    "Continue writing"
                } else {
                    "New project"
                };
                if action_button(ui, l.text(label), true, t, "home.primary").clicked() {
                    *action = Some(
                        resume.map_or(Action::New, |project| Action::Open(project.id.clone())),
                    );
                }
                if empty
                    && action_button(ui, l.text("Try a sample"), false, t, "home.sample").clicked()
                {
                    *action = Some(Action::Sample);
                }
            };
            if wide {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.allocate_ui_with_layout(
                        Vec2::new(width - image_width - 24.0, 0.0),
                        Layout::top_down(Align::Min),
                        |ui| text(ui, action),
                    );
                    ui.add_space(24.0);
                    illustration(ui, image_width, l, t);
                });
            } else {
                text(ui, action);
                ui.add_space(12.0);
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    illustration(ui, image_width, l, t)
                });
            }
        });
    record(ui, "home.banner", frame.response.rect);
}

fn illustration(ui: &mut egui::Ui, width: f32, l: Language, t: Tokens) {
    match artwork::texture(ui.ctx(), t.accent) {
        Ok(texture) => {
            let size = Vec2::new(width, width * texture.size_vec2().y / texture.size_vec2().x);
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            if ui.is_rect_visible(rect) {
                ui.painter().image(
                    texture.id(),
                    rect,
                    Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            record(ui, "home.artwork", rect);
        }
        Err(error) => {
            ui.colored_label(t.warning, l.text("Home illustration could not be loaded."));
            ui.label(error);
        }
    }
}

fn action_button(ui: &mut egui::Ui, label: &str, primary: bool, t: Tokens, key: &str) -> Response {
    let mut button = egui::Button::new(RichText::new(label).size(14.0).color(if primary {
        t.on_accent
    } else {
        t.text
    }))
    .min_size(Vec2::new(0.0, 40.0))
    .wrap();
    if primary {
        button = button.fill(t.accent);
    }
    let response = ui.add(button);
    if response.gained_focus() {
        response.scroll_to_me(None);
    }
    #[cfg(test)]
    ui.ctx()
        .data_mut(|data| data.insert_temp(egui::Id::new(("home-test-control", key)), response.id));
    record(ui, key, response.rect);
    response
}

fn project_row(ui: &mut egui::Ui, project: &Project, l: Language, t: Tokens) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 86.0), Sense::click());
    if response.gained_focus() {
        response.scroll_to_me(None);
    }
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("home-test-row", project.id.as_str())),
            response.id,
        )
    });
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled(),
            format!("{}: {}", l.text("Open project"), project.title),
        )
    });
    let fill = if response.hovered() || response.has_focus() {
        t.surface
    } else {
        t.panel
    };
    ui.painter().rect_filled(rect, 6.0, fill);
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.shrink(1.0),
            6.0,
            Stroke::new(2.0, t.accent),
            egui::StrokeKind::Inside,
        );
    }
    let inner = rect.shrink2(Vec2::new(16.0, 12.0));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(Layout::left_to_right(Align::Center)),
    );
    child.set_clip_rect(inner.intersect(ui.clip_rect()));
    let tile = Rect::from_min_size(inner.min, Vec2::splat(54.0));
    child.painter().rect_filled(tile, 6.0, t.accent_soft);
    let mut icon_ui = child.new_child(
        egui::UiBuilder::new()
            .max_rect(tile)
            .layout(Layout::centered_and_justified(egui::Direction::LeftToRight)),
    );
    icons::icon(&mut icon_ui, Icon::Book, 25.0, t.accent);
    let title_rect = Rect::from_min_max(
        egui::pos2(tile.right() + 16.0, inner.top()),
        egui::pos2(inner.right() - 38.0, inner.bottom()),
    );
    let mut labels = child.new_child(
        egui::UiBuilder::new()
            .max_rect(title_rect)
            .layout(Layout::top_down(Align::Min)),
    );
    labels.set_clip_rect(title_rect.intersect(ui.clip_rect()));
    labels.set_width(title_rect.width().max(0.0));
    labels.add(
        egui::Label::new(
            RichText::new(&project.title)
                .size(17.0)
                .strong()
                .color(t.text),
        )
        .truncate(),
    );
    labels.add(
        egui::Label::new(
            RichText::new(l.text("Local project"))
                .size(12.0)
                .color(t.muted),
        )
        .truncate(),
    );
    let mut arrow = child.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_max(
                egui::pos2(inner.right() - 28.0, inner.top()),
                inner.max,
            ))
            .layout(Layout::centered_and_justified(egui::Direction::LeftToRight)),
    );
    icons::icon(&mut arrow, Icon::ArrowRight, 22.0, t.text);
    record(ui, &format!("home.project.{}", project.id), rect);
    ui.add_space(8.0);
    response.on_hover_text(&project.title)
}

fn record(ui: &egui::Ui, key: &str, rect: Rect) {
    #[cfg(feature = "editor-preview")]
    super::super::capture_connected_writing::control(ui.ctx(), key, rect);
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.get_temp_mut_or_default::<std::collections::BTreeMap<String, Rect>>(egui::Id::new(
            "home-test-geometry",
        ))
        .insert(key.into(), rect)
    });
    #[cfg(not(any(test, feature = "editor-preview")))]
    let _ = (ui, key, rect);
}
