//! The approved lion writer, shared with the repository and packaged application identity.
use egui::{Color32, Id, Rect, Response, Sense, TextureHandle, Ui, Vec2, WidgetInfo, WidgetType};

const RENDITIONS: [(u16, &[u8]); 3] = [
    (
        32,
        include_bytes!("../../../assets/brand/writercraft-mark-32.png"),
    ),
    (
        64,
        include_bytes!("../../../assets/brand/writercraft-mark-64.png"),
    ),
    (
        128,
        include_bytes!("../../../assets/brand/writercraft-mark-128.png"),
    ),
];

fn texture_id(pixels: u16) -> Id {
    Id::new(("writercraft-lion-brand", pixels))
}

/// Decode once at startup; malformed bundled artwork fails before the workspace opens.
pub fn initialize(ctx: &egui::Context) -> Result<(), image::ImageError> {
    for (resolution, bytes) in RENDITIONS {
        let pixels = image::load_from_memory(bytes)?.into_rgba8();
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [pixels.width() as usize, pixels.height() as usize],
            pixels.as_raw(),
        );
        let texture = ctx.load_texture("WriterCraft lion", image, egui::TextureOptions::LINEAR);
        ctx.data_mut(|data| data.insert_temp(texture_id(resolution), texture));
    }
    Ok(())
}

/// Paint the full rounded tile without app-icon safe-area padding or theme tinting.
pub fn mark(ui: &mut Ui, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Image, ui.is_enabled(), "WriterCraft"));
    // Use native downsampled artwork near its actual physical size, including Retina.
    let physical_size = size * ui.ctx().pixels_per_point();
    let resolution = if physical_size <= 32.0 {
        32
    } else if physical_size <= 64.0 {
        64
    } else {
        128
    };
    if ui.is_rect_visible(rect)
        && let Some(texture) = ui
            .ctx()
            .data(|data| data.get_temp::<TextureHandle>(texture_id(resolution)))
    {
        ui.painter().image(
            texture.id(),
            rect,
            Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    response
}
