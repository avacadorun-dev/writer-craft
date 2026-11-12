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
    let body_height = ui.available_height();
    let short = body_height < 520.0;
    // One local scroll includes the banner, so it cannot trap the list in a
    // short window. No horizontal scroll area or fixed screenshot geometry.
    egui::ScrollArea::vertical()
        .id_salt("home-body")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // set_max_width is a setter, not a cap: never expand a narrow
            // centered region beyond the actual pane left by shared chrome.
            let body_width = ui.available_width().min(1280.0);
            ui.vertical_centered(|ui| {
                ui.set_max_width(body_width);
                ui.with_layout(Layout::top_down(Align::Min), |ui| {
                    banner(
                        ui,
                        projects.is_empty(),
                        resume,
                        l,
                        t,
                        body_height,
                        &mut action,
                    );
                    ui.add_space(if short { 16.0 } else { 24.0 });
                    project_heading(ui, l, t, &mut action);
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
    body_height: f32,
    action: &mut Option<Action>,
) {
    let wide = ui.available_width() >= 660.0;
    let padding = if wide { 24 } else { 16 };
    let dark = u16::from(t.bg.r()) + u16::from(t.bg.g()) + u16::from(t.bg.b()) < 384;
    let field = if dark {
        t.accent_soft
    } else {
        t.editor
            .lerp_to_gamma(Color32::from_rgb(249, 240, 220), 0.45)
    };
    let mut hatch = None;
    let mut deferred_artwork = None;
    let frame = egui::Frame::new()
        .fill(field)
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(10)
        .inner_margin(padding)
        .show(ui, |ui| {
            // Reserve the decoration before native text/buttons so it can
            // never paint over them, even when the final height changes.
            hatch = Some((ui.painter().clone(), ui.painter().add(egui::Shape::Noop)));
            ui.set_width(ui.available_width());
            let width = ui.available_width();
            let image_width = if wide {
                (width * 0.50).min((body_height * 0.42).clamp(130.0, 320.0) * 1.65)
            } else {
                width.min((body_height * 0.28).clamp(90.0, 180.0) * 1.65)
            };
            let text = |ui: &mut egui::Ui, action: &mut Option<Action>| {
                ui.spacing_mut().item_spacing.y = 10.0;
                ui.scope(|ui| {
                    ui.set_max_width(ui.available_width().min(420.0));
                    ui.add(
                        egui::Label::new(
                            RichText::new(l.text(if resume.is_some() {
                                "Your story awaits"
                            } else if empty {
                                "Your first chapter starts here"
                            } else {
                                "Open project"
                            }))
                            .size(if wide { 40.0 } else { 32.0 })
                            .family(FontFamily::Name("HomeHeading".into()))
                            .color(t.text),
                        )
                        .wrap(),
                    );
                });
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
                let text_width = width - image_width - 24.0;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let column = ui.allocate_ui_with_layout(
                        Vec2::new(text_width, 0.0),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.set_width(text_width);
                            text(ui, action);
                        },
                    );
                    ui.add_space(24.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(
                            image_width,
                            column.response.rect.height().max(image_width / 1.65),
                        ),
                        Layout::bottom_up(Align::Min),
                        |ui| {
                            deferred_artwork =
                                illustration(ui, image_width, l, t, f32::from(padding), false);
                        },
                    );
                });
            } else {
                text(ui, action);
                ui.add_space(12.0);
                ui.with_layout(Layout::top_down(Align::Max), |ui| {
                    let _ = illustration(ui, image_width, l, t, f32::from(padding), true);
                });
            }
        });
    if let Some((painter, index)) = hatch {
        painter.set(
            index,
            engraved_field(frame.response.rect.shrink(12.0), t, dark),
        );
    }
    // The wide banner paints the illustration only after its final contour is known, so the
    // artwork meets the frame's right and bottom edge exactly instead of inheriting the
    // surrounding layout's rounding.
    if let Some((texture, size)) = deferred_artwork {
        let rect = Rect::from_min_size(frame.response.rect.max - Vec2::splat(1.0) - size, size);
        if ui.is_rect_visible(rect) {
            egui::Image::new((texture.id(), size))
                .corner_radius(egui::CornerRadius {
                    se: 10,
                    ..egui::CornerRadius::ZERO
                })
                .paint_at(ui, rect);
        }
        record(ui, "home.artwork", rect);
    }
    record(ui, "home.banner", frame.response.rect);
}

fn project_heading(ui: &mut egui::Ui, l: Language, t: Tokens, action: &mut Option<Action>) {
    let heading = RichText::new(l.text("All projects"))
        .size(24.0)
        .family(FontFamily::Name("HomeHeading".into()))
        .color(t.text);
    let measure = |text: &str, size| {
        ui.painter()
            .layout_no_wrap(text.into(), FontId::proportional(size), t.text)
            .size()
            .x
    };
    let action_width = measure(l.text("New project"), 14.0)
        + measure(l.text("Open .wstudio…"), 14.0)
        + ui.spacing().button_padding.x * 4.0
        + ui.spacing().item_spacing.x;
    let heading_width = ui
        .painter()
        .layout_no_wrap(
            l.text("All projects").into(),
            FontId::new(24.0, FontFamily::Name("HomeHeading".into())),
            t.text,
        )
        .size()
        .x;
    let wide = ui.available_width() >= heading_width + action_width + 40.0;
    let actions = |ui: &mut egui::Ui, action: &mut Option<Action>| {
        let buttons = [
            ("New project", "home.new", Action::New),
            ("Open .wstudio…", "home.import", Action::Import),
        ];
        for (label, key, intent) in buttons {
            if action_button(ui, l.text(label), false, t, key).clicked() {
                *action = Some(intent);
            }
        }
    };
    if wide {
        ui.horizontal(|ui| {
            ui.label(heading);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Right-align the group, retaining the visual New → Import
                // order for keyboard traversal inside it.
                ui.allocate_ui_with_layout(
                    Vec2::new(action_width, 40.0),
                    Layout::left_to_right(Align::Center),
                    |ui| actions(ui, action),
                );
            });
        });
    } else {
        ui.label(heading);
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| actions(ui, action));
    }
}

fn engraved_field(rect: Rect, t: Tokens, dark: bool) -> egui::Shape {
    let color = t.text.gamma_multiply(if dark { 0.045 } else { 0.035 });
    let mut lines = Vec::new();
    let mut offset = 7.0;
    while offset < rect.width() + rect.height() {
        let start = egui::pos2(
            (rect.left() + offset).min(rect.right()),
            rect.bottom() - (offset - rect.width()).max(0.0),
        );
        let end = egui::pos2(
            (rect.left() + offset - rect.height()).max(rect.left()),
            rect.top() + (rect.height() - offset).max(0.0),
        );
        lines.push(egui::Shape::line_segment(
            [start, end],
            Stroke::new(0.65, color),
        ));
        offset += 11.0;
    }
    egui::Shape::Vec(lines)
}

fn illustration(
    ui: &mut egui::Ui,
    width: f32,
    l: Language,
    t: Tokens,
    padding: f32,
    paint: bool,
) -> Option<(egui::TextureHandle, Vec2)> {
    match artwork::texture(ui.ctx(), t.accent) {
        Ok(texture) => {
            let size = Vec2::new(width, width * texture.size_vec2().y / texture.size_vec2().x);
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            // Text retains its inset, while the trimmed artwork meets the
            // banner's right and bottom contour, as in the selected concept.
            let rect = rect.translate(Vec2::splat(padding));
            if paint {
                if ui.is_rect_visible(rect) {
                    egui::Image::new((texture.id(), size))
                        .corner_radius(egui::CornerRadius {
                            se: 10,
                            ..egui::CornerRadius::ZERO
                        })
                        .paint_at(ui, rect);
                }
                record(ui, "home.artwork", rect);
            }
            Some((texture, size))
        }
        Err(error) => {
            ui.colored_label(t.warning, l.text("Home illustration could not be loaded."));
            ui.label(error);
            None
        }
    }
}

fn action_button(ui: &mut egui::Ui, label: &str, primary: bool, t: Tokens, key: &str) -> Response {
    let mut button = egui::Button::new(
        RichText::new(label)
            .size(if primary { 18.0 } else { 14.0 })
            .color(if primary { t.on_accent } else { t.text }),
    )
    .min_size(Vec2::new(0.0, 40.0))
    .wrap();
    if primary {
        button = button.fill(t.accent);
    } else if key == "home.new" {
        button = button.fill(t.panel).stroke(Stroke::new(1.0, t.accent));
    } else if key == "home.import" {
        button = button.frame(false);
    }
    let response = ui.add(button);
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.shrink(1.0),
            5.0,
            Stroke::new(2.0, t.accent),
            egui::StrokeKind::Inside,
        );
    }
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
                .size(18.0)
                .family(FontFamily::Name("HomeHeading".into()))
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
