//! Palette adaptation is restricted to the illustration's blue robe pigment.
use egui::{Color32, Context, Id, TextureHandle};

const SCRIBE: &[u8] = include_bytes!("../../../../../assets/home/scribe.png");

#[derive(Clone)]
struct Cached {
    accent: Color32,
    texture: Result<TextureHandle, String>,
}

pub(super) fn texture(ctx: &Context, accent: Color32) -> Result<TextureHandle, String> {
    let id = Id::new("home-scribe-artwork");
    if let Some(cached) = ctx.data(|data| data.get_temp::<Cached>(id))
        && cached.accent == accent
    {
        return cached.texture;
    }
    let texture = decode(accent)
        .map(|image| ctx.load_texture("Home scribe", image, egui::TextureOptions::LINEAR));
    ctx.data_mut(|data| {
        data.insert_temp(
            id,
            Cached {
                accent,
                texture: texture.clone(),
            },
        )
    });
    texture
}

fn decode(accent: Color32) -> Result<egui::ColorImage, String> {
    let mut pixels = image::load_from_memory(SCRIBE)
        .map_err(|error| error.to_string())?
        .into_rgba8();
    for pixel in pixels.pixels_mut() {
        recolor(&mut pixel.0, accent);
    }
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [pixels.width() as usize, pixels.height() as usize],
        pixels.as_raw(),
    ))
}

fn recolor(pixel: &mut [u8; 4], accent: Color32) {
    let [r, g, b, alpha] = *pixel;
    // The supplied cutout uses blue only in fabric. Paper/skin, black strokes
    // and transparent silhouette are deliberately excluded from the mask.
    if alpha != 0 && u16::from(b) > u16::from(r) + 20 && u16::from(b) > u16::from(g) + 15 {
        let intensity = f32::from(b) / 190.0;
        for (channel, target) in pixel[..3]
            .iter_mut()
            .zip([accent.r(), accent.g(), accent.b()])
        {
            *channel = (f32::from(target) * intensity).min(255.0) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_changes_fabric_but_preserves_neutral_ink_paper_and_alpha() {
        let accent = Color32::from_rgb(180, 120, 70);
        let mut fabric = [45, 60, 170, 183];
        recolor(&mut fabric, accent);
        assert!(fabric[0] > fabric[2]);
        assert_eq!(fabric[3], 183);
        for original in [[242, 222, 180, 255], [12, 12, 12, 255], [0, 0, 180, 0]] {
            let mut pixel = original;
            recolor(&mut pixel, accent);
            assert_eq!(pixel, original);
        }
    }

    #[test]
    fn bundled_scribe_is_a_transparent_cutout_with_blue_fabric() -> Result<(), image::ImageError> {
        let pixels = image::load_from_memory(SCRIBE)?.into_rgba8();
        assert!(pixels.pixels().any(|pixel| pixel[3] == 0));
        assert!(
            pixels
                .pixels()
                .any(|pixel| pixel[3] > 200 && u16::from(pixel[2]) > u16::from(pixel[0]) + 20)
        );
        Ok(())
    }
}
