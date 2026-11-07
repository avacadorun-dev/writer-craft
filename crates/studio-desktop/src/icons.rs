//! One outline icon system, painted directly without a symbol font.

use egui::{
    Color32, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, WidgetInfo,
    WidgetType, pos2, vec2,
};

/// Utility symbols used consistently across the desktop application.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Icon {
    Home,
    Book,
    Folder,
    Document,
    Plus,
    Search,
    Sidebar,
    Assistant,
    Mic,
    Settings,
    Import,
    Export,
    Save,
    Undo,
    Redo,
    Check,
    ChevronRight,
    More,
    Close,
    History,
    Board,
    Trash,
    ArrowRight,
    Shield,
    Sun,
    Moon,
}

/// Allocate a decorative icon in the current layout.
pub fn icon(ui: &mut Ui, kind: Icon, size: f32, color: Color32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    if ui.is_rect_visible(rect) {
        paint(ui.painter(), rect, kind, color);
    }
    response
}

/// Add a keyboard-focusable utility button with a tooltip and accessible name.
pub fn icon_button(ui: &mut Ui, kind: Icon, label: &str) -> Response {
    let response = ui.add(
        egui::Button::new("")
            .min_size(vec2(32.0, 30.0))
            .frame(false),
    );
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), label));
    if ui.is_rect_visible(response.rect) {
        let visuals = ui.style().interact(&response);
        if response.hovered() || response.has_focus() || response.is_pointer_button_down_on() {
            ui.painter()
                .rect_filled(response.rect, 6, visuals.weak_bg_fill);
        }
        if response.has_focus() {
            ui.painter().rect_stroke(
                response.rect,
                6,
                Stroke::new(1.0, ui.visuals().hyperlink_color),
                StrokeKind::Inside,
            );
        }
        let color = if ui.is_enabled() {
            visuals.fg_stroke.color
        } else {
            ui.visuals().disable(visuals.fg_stroke.color)
        };
        paint(
            ui.painter(),
            Rect::from_center_size(response.rect.center(), Vec2::splat(18.0)),
            kind,
            color,
        );
    }
    response.on_hover_text(label)
}

fn paint(painter: &Painter, rect: Rect, kind: Icon, color: Color32) {
    let scale = rect.width().min(rect.height()) / 20.0;
    let origin = rect.center() - vec2(10.0, 10.0) * scale;
    let point = |x: f32, y: f32| origin + vec2(x, y) * scale;
    let stroke = Stroke::new(1.6 * scale, color);
    let line = |points: &[[f32; 2]]| {
        painter.add(Shape::line(
            points.iter().map(|[x, y]| point(*x, *y)).collect(),
            stroke,
        ));
    };
    let rectangle = |x: f32, y: f32, w: f32, h: f32, radius: u8| {
        painter.rect_stroke(
            Rect::from_min_size(point(x, y), vec2(w, h) * scale),
            radius,
            stroke,
            StrokeKind::Middle,
        );
    };
    let circle = |x: f32, y: f32, radius: f32| {
        painter.circle_stroke(point(x, y), radius * scale, stroke);
    };
    match kind {
        Icon::Home => {
            line(&[[2.5, 8.5], [10.0, 2.5], [17.5, 8.5]]);
            line(&[
                [5.0, 7.5],
                [5.0, 17.0],
                [8.0, 17.0],
                [8.0, 11.5],
                [12.0, 11.5],
                [12.0, 17.0],
                [15.0, 17.0],
                [15.0, 7.5],
            ]);
        }
        Icon::Book => {
            line(&[
                [10.0, 5.0],
                [7.5, 3.5],
                [2.5, 3.5],
                [2.5, 15.5],
                [7.5, 15.5],
                [10.0, 17.0],
                [12.5, 15.5],
                [17.5, 15.5],
                [17.5, 3.5],
                [12.5, 3.5],
                [10.0, 5.0],
                [10.0, 17.0],
            ]);
        }
        Icon::Folder => line(&[
            [2.5, 6.0],
            [2.5, 4.0],
            [7.5, 4.0],
            [9.5, 6.0],
            [17.5, 6.0],
            [17.5, 16.5],
            [2.5, 16.5],
            [2.5, 6.0],
            [9.5, 6.0],
        ]),
        Icon::Document => {
            line(&[
                [11.5, 2.5],
                [4.5, 2.5],
                [4.5, 17.5],
                [15.5, 17.5],
                [15.5, 6.5],
                [11.5, 2.5],
                [11.5, 6.5],
                [15.5, 6.5],
            ]);
            line(&[[7.5, 10.0], [12.5, 10.0]]);
            line(&[[7.5, 13.0], [12.5, 13.0]]);
        }
        Icon::Plus => {
            line(&[[10.0, 3.5], [10.0, 16.5]]);
            line(&[[3.5, 10.0], [16.5, 10.0]]);
        }
        Icon::Search => {
            circle(8.5, 8.5, 5.5);
            line(&[[12.5, 12.5], [17.0, 17.0]]);
        }
        Icon::Sidebar => {
            rectangle(2.5, 3.5, 15.0, 13.0, 2);
            line(&[[7.0, 3.5], [7.0, 16.5]]);
            line(&[[4.5, 7.0], [5.0, 7.0]]);
            line(&[[4.5, 10.0], [5.0, 10.0]]);
        }
        Icon::Assistant => {
            line(&[
                [4.0, 3.5],
                [16.0, 3.5],
                [17.0, 4.5],
                [17.0, 13.0],
                [16.0, 14.0],
                [8.0, 14.0],
                [4.0, 17.0],
                [4.0, 14.0],
                [3.0, 13.0],
                [3.0, 4.5],
                [4.0, 3.5],
            ]);
            line(&[[6.0, 7.0], [14.0, 7.0]]);
            line(&[[6.0, 10.5], [11.5, 10.5]]);
        }
        Icon::Mic => {
            rectangle(7.0, 2.0, 6.0, 10.0, 3);
            line(&[
                [4.5, 8.5],
                [4.5, 11.0],
                [6.0, 13.5],
                [8.0, 14.5],
                [12.0, 14.5],
                [14.0, 13.5],
                [15.5, 11.0],
                [15.5, 8.5],
            ]);
            line(&[[10.0, 14.5], [10.0, 18.0]]);
            line(&[[7.0, 18.0], [13.0, 18.0]]);
        }
        Icon::Settings => {
            circle(10.0, 10.0, 5.5);
            circle(10.0, 10.0, 2.0);
            for step in 0..8 {
                let angle = step as f32 * std::f32::consts::TAU / 8.0;
                let direction = vec2(angle.cos(), angle.sin());
                painter.line_segment(
                    [
                        point(10.0, 10.0) + direction * (5.5 * scale),
                        point(10.0, 10.0) + direction * (8.0 * scale),
                    ],
                    stroke,
                );
            }
        }
        Icon::Import | Icon::Export => {
            line(&[[3.5, 12.5], [3.5, 17.0], [16.5, 17.0], [16.5, 12.5]]);
            line(&[[10.0, 2.5], [10.0, 12.0]]);
            if kind == Icon::Import {
                line(&[[6.5, 8.5], [10.0, 12.0], [13.5, 8.5]]);
            } else {
                line(&[[6.5, 6.0], [10.0, 2.5], [13.5, 6.0]]);
            }
        }
        Icon::Save => {
            line(&[
                [3.0, 2.5],
                [14.0, 2.5],
                [17.0, 5.5],
                [17.0, 17.5],
                [3.0, 17.5],
                [3.0, 2.5],
            ]);
            line(&[[6.5, 2.5], [6.5, 7.0], [13.0, 7.0], [13.0, 2.5]]);
            rectangle(6.5, 11.0, 7.0, 6.5, 0);
        }
        Icon::Undo | Icon::Redo => {
            let mirrored = |x: f32| if kind == Icon::Redo { 20.0 - x } else { x };
            let points = [
                [3.0, 8.0],
                [11.5, 8.0],
                [14.5, 9.0],
                [16.0, 11.5],
                [16.0, 14.0],
                [14.0, 16.5],
            ];
            painter.add(Shape::line(
                points
                    .iter()
                    .map(|[x, y]| point(mirrored(*x), *y))
                    .collect(),
                stroke,
            ));
            line(&[
                [mirrored(7.0), 4.0],
                [mirrored(3.0), 8.0],
                [mirrored(7.0), 12.0],
            ]);
        }
        Icon::Check => line(&[[3.5, 10.5], [8.0, 15.0], [16.5, 5.5]]),
        Icon::ChevronRight => line(&[[7.5, 4.5], [13.0, 10.0], [7.5, 15.5]]),
        Icon::More => {
            for x in [4.0, 10.0, 16.0] {
                painter.circle_filled(point(x, 10.0), 1.3 * scale, color);
            }
        }
        Icon::Close => {
            line(&[[5.0, 5.0], [15.0, 15.0]]);
            line(&[[15.0, 5.0], [5.0, 15.0]]);
        }
        Icon::History => {
            arc(painter, point(10.0, 10.0), 6.7 * scale, -2.5, 2.7, stroke);
            line(&[[2.5, 3.5], [2.5, 7.0], [6.0, 7.0]]);
            line(&[[10.0, 6.0], [10.0, 10.0], [13.0, 11.5]]);
        }
        Icon::Board => {
            rectangle(2.5, 3.5, 15.0, 13.0, 1);
            rectangle(5.0, 6.0, 3.5, 5.0, 0);
            rectangle(11.5, 6.0, 3.5, 8.0, 0);
        }
        Icon::Trash => {
            line(&[[3.0, 5.0], [17.0, 5.0]]);
            line(&[[7.0, 5.0], [7.0, 2.5], [13.0, 2.5], [13.0, 5.0]]);
            line(&[[5.0, 5.0], [6.0, 17.5], [14.0, 17.5], [15.0, 5.0]]);
            line(&[[8.5, 8.0], [8.5, 14.5]]);
            line(&[[11.5, 8.0], [11.5, 14.5]]);
        }
        Icon::ArrowRight => {
            line(&[[3.0, 10.0], [17.0, 10.0]]);
            line(&[[11.0, 4.0], [17.0, 10.0], [11.0, 16.0]]);
        }
        Icon::Shield => {
            line(&[
                [10.0, 2.0],
                [17.0, 5.0],
                [16.0, 12.0],
                [13.5, 15.5],
                [10.0, 18.0],
                [6.5, 15.5],
                [4.0, 12.0],
                [3.0, 5.0],
                [10.0, 2.0],
            ]);
            line(&[[6.5, 10.0], [9.0, 12.5], [13.5, 8.0]]);
        }
        Icon::Sun => {
            circle(10.0, 10.0, 3.5);
            for step in 0..8 {
                let angle = step as f32 * std::f32::consts::TAU / 8.0;
                let direction = vec2(angle.cos(), angle.sin());
                painter.line_segment(
                    [
                        point(10.0, 10.0) + direction * (6.0 * scale),
                        point(10.0, 10.0) + direction * (8.0 * scale),
                    ],
                    stroke,
                );
            }
        }
        Icon::Moon => {
            line(&[
                [13.0, 2.8],
                [9.0, 2.8],
                [5.5, 4.7],
                [3.5, 8.0],
                [3.2, 11.0],
                [4.5, 14.5],
                [7.5, 16.7],
                [11.0, 17.2],
                [14.5, 16.0],
                [17.0, 13.0],
                [13.0, 13.0],
                [10.0, 11.5],
                [8.5, 8.5],
                [9.0, 5.5],
                [11.0, 3.5],
                [13.0, 2.8],
            ]);
        }
    }
}

fn arc(painter: &Painter, center: Pos2, radius: f32, start: f32, end: f32, stroke: Stroke) {
    let points = (0..=24)
        .map(|step| {
            let angle = start + (end - start) * step as f32 / 24.0;
            pos2(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            )
        })
        .collect();
    painter.add(Shape::line(points, stroke));
}
