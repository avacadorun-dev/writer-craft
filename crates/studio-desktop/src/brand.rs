//! WriterCraft's reading-monk mark, preserving the original Writers Studio crest.

use egui::{Color32, Pos2, Response, Sense, Shape, Stroke, Ui, Vec2, WidgetInfo, WidgetType, vec2};

/// Paint the brand at toolbar sizes without relying on fonts or raster assets.
pub fn mark(ui: &mut Ui, size: f32, accent: Color32, ink: Color32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Image, ui.is_enabled(), "WriterCraft"));
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let scale = size / 40.0;
    let point = |x: f32, y: f32| rect.min + vec2(x, y) * scale;
    let points = |coordinates: &[[f32; 2]]| -> Vec<Pos2> {
        coordinates.iter().map(|[x, y]| point(*x, *y)).collect()
    };
    let paper = Color32::from_rgb(255, 248, 232);
    let blue_ink = Color32::from_rgb(31, 55, 86);
    let outline = Stroke::new((0.9 * scale).max(0.6), blue_ink);
    let painter = ui.painter();

    painter.add(Shape::convex_polygon(
        points(&[
            [20.0, 1.5],
            [26.0, 2.4],
            [31.0, 5.5],
            [34.4, 10.0],
            [35.5, 16.0],
            [35.5, 26.2],
            [34.0, 29.6],
            [30.5, 32.9],
            [25.5, 35.8],
            [20.0, 38.1],
            [14.5, 35.8],
            [9.5, 32.9],
            [6.0, 29.6],
            [4.5, 26.2],
            [4.5, 16.0],
            [5.6, 10.0],
            [9.0, 5.5],
            [14.0, 2.4],
        ]),
        Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 24),
        Stroke::new((0.8 * scale).max(0.5), ink.gamma_multiply(0.3)),
    ));
    painter.add(Shape::convex_polygon(
        points(&[
            [10.0, 30.0],
            [11.2, 22.0],
            [12.7, 15.0],
            [15.0, 9.5],
            [18.0, 6.5],
            [20.0, 6.0],
            [22.0, 6.5],
            [25.0, 9.5],
            [27.3, 15.0],
            [28.8, 22.0],
            [30.0, 30.0],
        ]),
        accent,
        outline,
    ));
    painter.add(Shape::convex_polygon(
        points(&[
            [20.0, 10.0],
            [16.7, 12.4],
            [15.1, 16.0],
            [15.8, 19.6],
            [18.0, 22.4],
            [20.0, 23.5],
            [22.0, 22.4],
            [24.2, 19.6],
            [24.9, 16.0],
            [23.3, 12.4],
        ]),
        blue_ink,
        Stroke::NONE,
    ));
    painter.add(Shape::convex_polygon(
        points(&[
            [20.0, 12.2],
            [17.7, 14.2],
            [16.8, 16.8],
            [17.5, 19.4],
            [20.0, 21.8],
            [22.5, 19.4],
            [23.2, 16.8],
            [22.3, 14.2],
        ]),
        paper,
        Stroke::NONE,
    ));
    let fold = Stroke::new((0.8 * scale).max(0.55), blue_ink.gamma_multiply(0.6));
    painter.line_segment([point(15.7, 21.4), point(13.5, 27.0)], fold);
    painter.line_segment([point(24.3, 21.4), point(26.5, 27.0)], fold);

    for page in [
        [[7.0, 25.0], [20.0, 27.0], [20.0, 35.0], [7.5, 33.0]],
        [[20.0, 27.0], [33.0, 25.0], [32.5, 33.0], [20.0, 35.0]],
    ] {
        painter.add(Shape::convex_polygon(points(&page), paper, outline));
    }
    if size >= 24.0 {
        let page_rule = Stroke::new((0.55 * scale).max(0.5), blue_ink.gamma_multiply(0.65));
        painter.line_segment([point(10.2, 28.3), point(17.2, 29.5)], page_rule);
        painter.line_segment([point(29.8, 28.3), point(22.8, 29.5)], page_rule);
        painter.line_segment([point(10.4, 30.6), point(17.2, 31.8)], page_rule);
        painter.line_segment([point(29.6, 30.6), point(22.8, 31.8)], page_rule);
    }
    response
}
