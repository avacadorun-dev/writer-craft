//! Semantic colors and independent appearance/palette settings for the desktop UI.

use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Vec2};
use serde::{Deserialize, Serialize};

/// How the application chooses light or dark surfaces.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Appearance {
    System,
    Light,
    #[default]
    Dark,
}

impl Appearance {
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    pub const fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }
}

/// Supported palettes; the original green Quiet and Olive palettes are excluded.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Palette {
    #[default]
    SolidCraftBlue,
    CedarAndCopper,
    DriedRoseAndWine,
    FogGraphite,
    LinenClay,
    DustyLavender,
    PorcelainInk,
}

impl Palette {
    pub const ALL: [Self; 7] = [
        Self::SolidCraftBlue,
        Self::CedarAndCopper,
        Self::DriedRoseAndWine,
        Self::FogGraphite,
        Self::LinenClay,
        Self::DustyLavender,
        Self::PorcelainInk,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::SolidCraftBlue => "Graphite and blue",
            Self::CedarAndCopper => "Cedar and copper",
            Self::DriedRoseAndWine => "Dried rose and wine",
            Self::FogGraphite => "Fog and graphite",
            Self::LinenClay => "Linen and clay",
            Self::DustyLavender => "Dusty lavender",
            Self::PorcelainInk => "Porcelain and ink",
        }
    }

    pub const fn id(self) -> &'static str {
        match self {
            Self::SolidCraftBlue => "solid-craft-blue",
            Self::CedarAndCopper => "cedar-and-copper",
            Self::DriedRoseAndWine => "dried-rose-and-wine",
            Self::FogGraphite => "fog-graphite",
            Self::LinenClay => "linen-clay",
            Self::DustyLavender => "dusty-lavender",
            Self::PorcelainInk => "porcelain-ink",
        }
    }
}

/// Serializable preferences; appearance and palette are independent choices.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeSettings {
    pub appearance: Appearance,
    pub palette: Palette,
}

/// Resolved semantic colors. Use `on_accent` on a solid accent button.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tokens {
    pub bg: Color32,
    pub panel: Color32,
    pub editor: Color32,
    pub surface: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub border: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub good: Color32,
    pub warning: Color32,
    pub on_accent: Color32,
}

const fn color(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// Resolve settings without a renderer, allowing deterministic previews and tests.
pub fn resolve(settings: &ThemeSettings, system_dark: bool) -> Tokens {
    let dark = match settings.appearance {
        Appearance::System => system_dark,
        Appearance::Light => false,
        Appearance::Dark => true,
    };
    let values = match (settings.palette, dark) {
        (Palette::SolidCraftBlue, false) => [
            0xf0f2f6, 0xf7f8fa, 0xfdfdfd, 0xffffff, 0x252c38, 0x5d697a, 0xd4dce7, 0x285fa8,
            0xdfe9f7, 0x35674e, 0x78531f,
        ],
        (Palette::SolidCraftBlue, true) => [
            0x191c21, 0x21252b, 0x1c1f24, 0x2b313a, 0xe8ecf3, 0x9da8b9, 0x373f4b, 0x6da4ff,
            0x263d60, 0x7acaa6, 0xebbe70,
        ],
        (Palette::CedarAndCopper, false) => [
            0xf5eee7, 0xfaf5ef, 0xfffcf7, 0xffffff, 0x352d28, 0x74675e, 0xe3d4c4, 0x8e4e2a,
            0xf0ddd0, 0x4d6545, 0x79501b,
        ],
        (Palette::CedarAndCopper, true) => [
            0x1d1a17, 0x25211c, 0x2d2721, 0x373027, 0xf2e9dc, 0xb8aea2, 0x4e4233, 0xd19266,
            0x563a2b, 0xadc29b, 0xe4b873,
        ],
        (Palette::DriedRoseAndWine, false) => [
            0xf5eff1, 0xfbf6f8, 0xfffcfd, 0xffffff, 0x382f34, 0x75666d, 0xe4d6dd, 0x7a2940,
            0xf0dde4, 0x4c6652, 0x76521e,
        ],
        (Palette::DriedRoseAndWine, true) => [
            0x1e191d, 0x262025, 0x2e262d, 0x382f37, 0xf1e8e4, 0xb9acb1, 0x50414d, 0xb38a97,
            0x5d2c3d, 0xa7c1b3, 0xdeb676,
        ],
        (Palette::FogGraphite, false) => [
            0xeef1f4, 0xf5f7f9, 0xfcfdfe, 0xffffff, 0x252a30, 0x5e6c7d, 0xd5dde6, 0x596a80,
            0xe2e7ed, 0x456653, 0x78541f,
        ],
        (Palette::FogGraphite, true) => [
            0x171b20, 0x1e242b, 0x252c34, 0x2d3540, 0xece9e2, 0xacb2b8, 0x404c5a, 0x9aa8bb,
            0x323a46, 0xa8c3b5, 0xdcb873,
        ],
        (Palette::LinenClay, false) => [
            0xf2ede6, 0xf8f3ec, 0xfffbf5, 0xfffdfa, 0x3d332b, 0x746458, 0xe0d4c5, 0x795c48,
            0xeee1d6, 0x526241, 0x77501b,
        ],
        (Palette::LinenClay, true) => [
            0x1e1b18, 0x26221e, 0x2e2923, 0x383129, 0xf3ebdd, 0xc8bbae, 0x4e4337, 0xd0a17d,
            0x47372c, 0xbdc79b, 0xe5b76f,
        ],
        (Palette::DustyLavender, false) => [
            0xf2f0f5, 0xf8f6fa, 0xfefcff, 0xffffff, 0x302c36, 0x696274, 0xded6e8, 0x6d5c89,
            0xe8e2ef, 0x4b6553, 0x76531f,
        ],
        (Palette::DustyLavender, true) => [
            0x1b1921, 0x23202b, 0x2b2734, 0x342f3e, 0xece9e5, 0xb0b3bc, 0x4a4058, 0xb3a6c8,
            0x393445, 0xa9c3b7, 0xdbb878,
        ],
        (Palette::PorcelainInk, false) => [
            0xf3f2ef, 0xf9f8f5, 0xfffefa, 0xffffff, 0x2f2d31, 0x6f665f, 0xdfdcd4, 0x5d6b7e,
            0xe4e7eb, 0x496552, 0x79521c,
        ],
        (Palette::PorcelainInk, true) => [
            0x191b1f, 0x21242a, 0x282c33, 0x31363e, 0xf1ece3, 0xbdb4ab, 0x454d58, 0xa9b8ca,
            0x343a45, 0xa8c2b5, 0xe1b875,
        ],
    };
    let [
        bg,
        panel,
        editor,
        surface,
        text,
        muted,
        border,
        accent,
        accent_soft,
        good,
        warning,
    ] = values.map(color);
    let backgrounds = [bg, panel, editor, surface, accent_soft];
    let muted = readable(muted, text, &backgrounds);
    let on_accent = if contrast(color(0x171a20), accent) >= contrast(color(0xfafafa), accent) {
        color(0x171a20)
    } else {
        color(0xfafafa)
    };
    Tokens {
        bg,
        panel,
        editor,
        surface,
        text,
        muted,
        border,
        accent,
        accent_soft,
        good,
        warning,
        on_accent,
    }
}

/// Apply both mode variants, returning the currently active semantic palette.
pub fn apply(ctx: &egui::Context, settings: &ThemeSettings) -> Tokens {
    ctx.set_theme(match settings.appearance {
        Appearance::System => egui::ThemePreference::System,
        Appearance::Light => egui::ThemePreference::Light,
        Appearance::Dark => egui::ThemePreference::Dark,
    });
    for (appearance, theme) in [
        (Appearance::Light, egui::Theme::Light),
        (Appearance::Dark, egui::Theme::Dark),
    ] {
        let t = resolve(
            &ThemeSettings {
                appearance,
                ..*settings
            },
            false,
        );
        let mut style = (*ctx.style_of(theme)).clone();
        style
            .text_styles
            .insert(TextStyle::Body, FontId::new(14.5, FontFamily::Proportional));
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(14.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Small,
            FontId::new(12.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(21.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Monospace,
            FontId::new(13.0, FontFamily::Monospace),
        );
        style.spacing.item_spacing = Vec2::new(8.0, 8.0);
        style.spacing.button_padding = Vec2::new(10.0, 5.0);
        style.spacing.interact_size = Vec2::new(32.0, 30.0);
        style.spacing.icon_width = 17.0;
        style.spacing.icon_spacing = 7.0;
        style.spacing.indent = 16.0;
        style.spacing.tooltip_width = 300.0;
        style.spacing.scroll = egui::style::ScrollStyle::thin();
        style.animation_time = 0.12;
        style.visuals = if theme == egui::Theme::Dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        let v = &mut style.visuals;
        v.override_text_color = Some(t.text);
        v.weak_text_color = Some(t.muted);
        v.panel_fill = t.panel;
        v.window_fill = t.panel;
        v.extreme_bg_color = t.bg;
        v.faint_bg_color = t.surface;
        v.text_edit_bg_color = Some(t.bg);
        v.code_bg_color = t.bg;
        v.hyperlink_color = t.accent;
        v.warn_fg_color = t.warning;
        v.error_fg_color = if theme == egui::Theme::Dark {
            color(0xf49191)
        } else {
            color(0x9d3434)
        };
        v.selection.bg_fill = t.accent_soft;
        v.selection.stroke = Stroke::new(1.0, t.text);
        v.window_stroke = Stroke::new(1.0, t.border);
        v.window_corner_radius = CornerRadius::same(8);
        v.menu_corner_radius = CornerRadius::same(6);
        v.collapsing_header_frame = false;
        for widget in [
            &mut v.widgets.noninteractive,
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            widget.corner_radius = CornerRadius::same(6);
            widget.expansion = 0.0;
            widget.fg_stroke = Stroke::new(1.0, t.text);
            widget.bg_stroke = Stroke::new(1.0, t.border);
        }
        v.widgets.noninteractive.bg_fill = t.panel;
        v.widgets.noninteractive.weak_bg_fill = t.panel;
        v.widgets.inactive.bg_fill = t.surface;
        v.widgets.inactive.weak_bg_fill = t.surface;
        v.widgets.hovered.bg_fill = t.accent_soft;
        v.widgets.hovered.weak_bg_fill = t.accent_soft;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, t.accent);
        v.widgets.active.bg_fill = t.accent_soft;
        v.widgets.active.weak_bg_fill = t.accent_soft;
        v.widgets.active.bg_stroke = Stroke::new(1.0, t.accent);
        v.widgets.open.bg_fill = t.surface;
        v.widgets.open.weak_bg_fill = t.surface;
        v.text_cursor.stroke = Stroke::new(1.5, t.accent);
        ctx.set_style_of(theme, style);
    }
    let active = resolve(
        settings,
        ctx.system_theme().unwrap_or(egui::Theme::Dark) == egui::Theme::Dark,
    );
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("writercraft.theme.settings"), *settings));
    active
}

/// Resolve the applied settings against the current OS mode, including live system changes.
pub fn tokens(ctx: &egui::Context) -> Tokens {
    let settings = ctx
        .data(|data| data.get_temp::<ThemeSettings>(egui::Id::new("writercraft.theme.settings")))
        .unwrap_or_default();
    resolve(
        &settings,
        ctx.system_theme().unwrap_or(egui::Theme::Dark) == egui::Theme::Dark,
    )
}

fn readable(source: Color32, target: Color32, backgrounds: &[Color32]) -> Color32 {
    for step in 0..=100 {
        let candidate = mix(source, target, step as f32 / 100.0);
        if backgrounds.iter().all(|bg| contrast(candidate, *bg) >= 4.5) {
            return candidate;
        }
    }
    target
}

fn mix(a: Color32, b: Color32, factor: f32) -> Color32 {
    let component =
        |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * factor).round() as u8;
    Color32::from_rgb(
        component(a.r(), b.r()),
        component(a.g(), b.g()),
        component(a.b(), b.b()),
    )
}

fn luminance(color: Color32) -> f64 {
    let channel = |c: u8| {
        let v = f64::from(c) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
}

fn contrast(a: Color32, b: Color32) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prose_and_secondary_labels_remain_readable_in_every_palette() {
        for palette in Palette::ALL {
            for appearance in [Appearance::Light, Appearance::Dark] {
                let t = resolve(
                    &ThemeSettings {
                        appearance,
                        palette,
                    },
                    false,
                );
                for background in [t.bg, t.panel, t.editor, t.surface, t.accent_soft] {
                    for foreground in [t.text, t.muted] {
                        assert!(
                            contrast(foreground, background) >= 4.5,
                            "{palette:?}/{appearance:?}: contrast {}",
                            contrast(foreground, background)
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn filled_accent_commands_have_readable_foregrounds_in_every_palette() {
        for palette in Palette::ALL {
            for appearance in [Appearance::Light, Appearance::Dark] {
                let t = resolve(
                    &ThemeSettings {
                        appearance,
                        palette,
                    },
                    false,
                );
                assert!(
                    contrast(t.on_accent, t.accent) >= 4.5,
                    "{palette:?}/{appearance:?}"
                );
            }
        }
    }

    #[test]
    fn system_appearance_tracks_both_system_modes() {
        for palette in Palette::ALL {
            for system_dark in [false, true] {
                let system = resolve(
                    &ThemeSettings {
                        appearance: Appearance::System,
                        palette,
                    },
                    system_dark,
                );
                let explicit = resolve(
                    &ThemeSettings {
                        appearance: if system_dark {
                            Appearance::Dark
                        } else {
                            Appearance::Light
                        },
                        palette,
                    },
                    false,
                );
                assert_eq!(system, explicit);
            }
        }
    }

    #[test]
    fn settings_roundtrip_without_exposing_excluded_green_palettes() -> Result<(), serde_json::Error>
    {
        for palette in Palette::ALL {
            let original = ThemeSettings {
                appearance: Appearance::System,
                palette,
            };
            let bytes = serde_json::to_vec(&original)?;
            assert_eq!(serde_json::from_slice::<ThemeSettings>(&bytes)?, original);
        }
        assert!(serde_json::from_str::<Palette>("\"quiet\"").is_err());
        assert!(serde_json::from_str::<Palette>("\"olive-ink\"").is_err());
        Ok(())
    }
}
