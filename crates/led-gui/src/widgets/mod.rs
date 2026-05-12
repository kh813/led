pub mod menu_bar;
pub mod editor_view;
pub mod tab_bar;
pub mod find_panel;
pub mod status_bar;
pub mod dialog;

use gpui::Rgba;
use led_core::theme::Color as LedColor;

pub fn led_color_to_gpui(color: LedColor) -> Rgba {
    match color {
        LedColor::Rgb(r, g, b) => {
            Rgba {
                r: r as f32 / 255.0,
                g: g as f32 / 255.0,
                b: b as f32 / 255.0,
                a: 1.0,
            }
        }
        LedColor::Ansi(i) => {
            let (r, g, b) = match i {
                0 => (0, 0, 0),
                1 => (170, 0, 0),
                2 => (0, 170, 0),
                3 => (170, 170, 0),
                4 => (0, 0, 170),
                5 => (170, 0, 170),
                6 => (0, 170, 170),
                7 => (170, 170, 170),
                8 => (85, 85, 85),
                9 => (255, 85, 85),
                10 => (85, 255, 85),
                11 => (255, 255, 85),
                12 => (85, 85, 255),
                13 => (255, 85, 255),
                14 => (85, 255, 255),
                15 => (255, 255, 255),
                _ => (128, 128, 128),
            };
            Rgba {
                r: r as f32 / 255.0,
                g: g as f32 / 255.0,
                b: b as f32 / 255.0,
                a: 1.0,
            }
        }
    }
}
