//! Official Lucide utility geometry rendered as native antialiased vectors.
//! Source SVGs, pinned revision and license notices live in assets/icons/lucide.
use egui::{
    Color32, Painter, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, WidgetInfo,
    WidgetType, vec2,
};

/// Utility icons from the pinned official Lucide subset.
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
    Palette,
    Keyboard,
    Plug,
    TextCursor,
    Bold,
    Italic,
    Underline,
    Strike,
    Code,
    Quote,
    BulletList,
    OrderedList,
    Indent,
    Outdent,
    Table,
    Formula,
    Image,
    Diagram,
    Info,
    Idea,
    Warning,
    Todo,
    Link,
    Minus,
    Focus,
    Copy,
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
            Rect::from_center_size(response.rect.center(), Vec2::splat(20.0)),
            kind,
            color,
        );
    }
    response.on_hover_text(label)
}

/// A labelled native action with a fixed 20 px glyph and 8 px icon/text gap.
/// The leading space is a layout measurement, never font-dependent padding characters.
pub fn labeled_button(ui: &mut Ui, kind: Icon, label: &str, selected: bool) -> Response {
    let foreground = if selected {
        ui.visuals().selection.stroke.color
    } else {
        ui.visuals().text_color()
    };
    let foreground = if ui.is_enabled() {
        foreground
    } else {
        ui.visuals().disable(foreground)
    };
    let mut job = egui::text::LayoutJob::default();
    job.append(
        label,
        28.0,
        egui::TextFormat {
            font_id: egui::TextStyle::Button.resolve(ui.style()),
            color: foreground,
            ..Default::default()
        },
    );
    let response = ui.add(
        egui::Button::new(job)
            .selected(selected)
            .frame_when_inactive(selected)
            .min_size(vec2(0.0, 36.0))
            .wrap_mode(egui::TextWrapMode::Extend),
    );
    response
        .widget_info(|| WidgetInfo::selected(WidgetType::Button, ui.is_enabled(), selected, label));
    if ui.is_rect_visible(response.rect) {
        let center = egui::pos2(
            response.rect.left() + ui.spacing().button_padding.x + 10.0,
            response.rect.center().y,
        );
        paint(
            ui.painter(),
            Rect::from_center_size(center, Vec2::splat(20.0)),
            kind,
            foreground,
        );
    }
    response
}

struct IconGeometry {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Retained to identify the upstream source in validation"
        )
    )]
    name: &'static str,
    vertices: &'static [[f32; 4]],
    triangles: &'static [u16],
    contour_edges: &'static [[u16; 2]],
}

include!("../../../assets/icons/lucide/native_geometry.rs");

pub(crate) fn paint(painter: &Painter, rect: Rect, kind: Icon, color: Color32) {
    paint_oriented(painter, rect, kind, color, false);
}

/// The same official Lucide chevron, rotated clockwise for an expanded tree row.
pub(crate) fn paint_disclosure(painter: &Painter, rect: Rect, expanded: bool, color: Color32) {
    paint_oriented(painter, rect, Icon::ChevronRight, color, expanded);
}

fn paint_oriented(painter: &Painter, rect: Rect, kind: Icon, color: Color32, quarter_turn: bool) {
    let source = geometry(kind);
    let scale = rect.width().min(rect.height()) / 24.0;
    if !scale.is_finite() || scale <= 0.0 {
        return;
    }
    let orient = |value: Vec2| {
        if quarter_turn {
            vec2(-value.y, value.x)
        } else {
            value
        }
    };
    let position = |x, y| rect.center() + orient(vec2(x - 12.0, y - 12.0)) * scale;
    let half_fringe = 0.5 / painter.ctx().pixels_per_point();
    let mut mesh = egui::Mesh::default();
    mesh.vertices.reserve(source.vertices.len() * 2);
    for [x, y, nx, ny] in source.vertices {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: position(*x, *y) - orient(vec2(*nx, *ny)) * half_fringe,
            uv: egui::epaint::WHITE_UV,
            color,
        });
    }
    let outer_offset = mesh.vertices.len() as u32;
    for [x, y, nx, ny] in source.vertices {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: position(*x, *y) + orient(vec2(*nx, *ny)) * half_fringe,
            uv: egui::epaint::WHITE_UV,
            color: Color32::TRANSPARENT,
        });
    }
    mesh.indices
        .extend(source.triangles.iter().copied().map(u32::from));
    for [a, b] in source.contour_edges {
        let (a, b) = (u32::from(*a), u32::from(*b));
        mesh.indices.extend([
            a,
            b,
            outer_offset + b,
            a,
            outer_offset + b,
            outer_offset + a,
        ]);
    }
    painter.add(Shape::mesh(mesh));
}

#[cfg(test)]
mod tests {
    use super::*;
    const ALL: [Icon; 52] = [
        Icon::Home,
        Icon::Book,
        Icon::Folder,
        Icon::Document,
        Icon::Plus,
        Icon::Search,
        Icon::Sidebar,
        Icon::Assistant,
        Icon::Mic,
        Icon::Settings,
        Icon::Import,
        Icon::Export,
        Icon::Save,
        Icon::Undo,
        Icon::Redo,
        Icon::Check,
        Icon::ChevronRight,
        Icon::More,
        Icon::Close,
        Icon::History,
        Icon::Board,
        Icon::Trash,
        Icon::ArrowRight,
        Icon::Shield,
        Icon::Sun,
        Icon::Moon,
        Icon::Palette,
        Icon::Keyboard,
        Icon::Plug,
        Icon::TextCursor,
        Icon::Bold,
        Icon::Italic,
        Icon::Underline,
        Icon::Strike,
        Icon::Code,
        Icon::Quote,
        Icon::BulletList,
        Icon::OrderedList,
        Icon::Indent,
        Icon::Outdent,
        Icon::Table,
        Icon::Formula,
        Icon::Image,
        Icon::Diagram,
        Icon::Info,
        Icon::Idea,
        Icon::Warning,
        Icon::Todo,
        Icon::Link,
        Icon::Minus,
        Icon::Focus,
        Icon::Copy,
    ];
    #[test]
    fn every_utility_has_a_pinned_official_lucide_source() -> Result<(), Box<dyn std::error::Error>>
    {
        let directory =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons/lucide");
        let notice = std::fs::read_to_string(directory.join("NOTICE.md"))?;
        for kind in ALL {
            let source = geometry(kind);
            let svg = std::fs::read_to_string(directory.join(format!("{}.svg", source.name)))?;
            assert!(svg.contains("viewBox=\"0 0 24 24\""));
            assert!(notice.contains(source.name));
        }
        assert!(notice.contains("53fab88ff9d1b48f4704b5b5b560d6b4d560bf4c"));
        Ok(())
    }
    #[test]
    fn native_lucide_geometry_has_finite_coordinates_and_valid_triangle_edges() {
        for kind in ALL {
            let source = geometry(kind);
            assert!(!source.vertices.is_empty());
            assert_eq!(source.triangles.len() % 3, 0);
            assert!(!source.contour_edges.is_empty());
            for vertex in source.vertices {
                assert!(vertex.iter().all(|value| value.is_finite()));
                assert!((-1.0..=25.0).contains(&vertex[0]));
                assert!((-1.0..=25.0).contains(&vertex[1]));
            }
            assert!(
                source
                    .triangles
                    .iter()
                    .all(|index| usize::from(*index) < source.vertices.len())
            );
            assert!(
                source
                    .contour_edges
                    .iter()
                    .flatten()
                    .all(|index| usize::from(*index) < source.vertices.len())
            );
        }
    }
}
