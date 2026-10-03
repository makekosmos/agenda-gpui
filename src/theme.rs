// Re-exported from imago-gpui (KOS-132): the shared Mundus GPUI visual core
// was seeded from this file, so the API is a strict superset — `pal()`,
// `BG()`/`FG()`/…, `c`/`rgba`/`mix`/`lerp`, tag colors and easings are
// byte-identical. `imago_gpui::theme::apply` additionally installs the
// palette as the gpui-component theme.
#![allow(non_snake_case)] // Keep Imago's uppercase palette accessor API.
pub use imago_gpui::theme::*;

use std::sync::atomic::{AtomicU32, Ordering};

const NO_ACCENT: u32 = u32::MAX;
static ACCENT_OVERRIDE: AtomicU32 = AtomicU32::new(NO_ACCENT);
static FONT_SIZE: AtomicU32 = AtomicU32::new(13.0f32.to_bits());

/// Agenda-local facade: Imago's static ThemeDef palettes cannot be mutated.
/// All Agenda chrome calls these functions, including its sidebar controls.
pub fn set_accent_override(color: Option<u32>) {
    ACCENT_OVERRIDE.store(color.unwrap_or(NO_ACCENT), Ordering::Relaxed);
}

pub fn accent_override_active() -> bool {
    ACCENT_OVERRIDE.load(Ordering::Relaxed) != NO_ACCENT
}

pub fn ACCENT() -> u32 {
    let color = ACCENT_OVERRIDE.load(Ordering::Relaxed);
    if color == NO_ACCENT {
        imago_gpui::theme::ACCENT()
    } else {
        color
    }
}

pub fn ACCENT_FG() -> u32 {
    let color = ACCENT_OVERRIDE.load(Ordering::Relaxed);
    if color == NO_ACCENT {
        return imago_gpui::theme::ACCENT_FG();
    }
    let r = ((color >> 16) & 255) as f32 / 255.0;
    let g = ((color >> 8) & 255) as f32 / 255.0;
    let b = (color & 255) as f32 / 255.0;
    let linear = |v: f32| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    if 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b) > 0.179 {
        0x000000
    } else {
        0xffffff
    }
}

pub fn ACCENT_DIM() -> u32 {
    if ACCENT_OVERRIDE.load(Ordering::Relaxed) == NO_ACCENT {
        imago_gpui::theme::ACCENT_DIM()
    } else {
        let a = ACCENT();
        let bg = BG();
        let channel = |shift| {
            ((((a >> shift) & 255u32) * 7u32 + ((bg >> shift) & 255u32) * 3u32) / 10u32) << shift
        };
        channel(16) | channel(8) | channel(0)
    }
}

pub fn text_px(size: f32) -> gpui::Pixels {
    gpui::px(size * f32::from_bits(FONT_SIZE.load(Ordering::Relaxed)) / 13.0)
}

pub fn set_font_size(size: f32) {
    FONT_SIZE.store(size.to_bits(), Ordering::Relaxed);
}

/// Imago installs the immutable palette; patch its resolved component accent
/// tokens afterwards so Inputs, buttons, links, selection and sidebar agree.
pub fn apply_component_accent(cx: &mut gpui::App) {
    if ACCENT_OVERRIDE.load(Ordering::Relaxed) == NO_ACCENT {
        return;
    }
    let accent = c(ACCENT());
    let fg = c(ACCENT_FG());
    let hover = mix(ACCENT(), 0.85, ACCENT_FG());
    let active = mix(ACCENT(), 0.72, ACCENT_FG());
    let colors = &mut gpui_component::theme::Theme::global_mut(cx).colors;
    colors.accent = accent;
    colors.accent_foreground = fg;
    colors.primary = accent;
    colors.primary_foreground = fg;
    colors.primary_hover = hover;
    colors.primary_active = active;
    colors.button_primary = accent;
    colors.button_primary_foreground = fg;
    colors.button_primary_hover = hover;
    colors.button_primary_active = active;
    colors.button_info = rgba(ACCENT(), 0.15);
    colors.button_info_foreground = accent;
    colors.info = accent;
    colors.info_foreground = fg;
    colors.link = accent;
    colors.link_hover = hover;
    colors.link_active = active;
    colors.ring = accent;
    colors.selection = rgba(ACCENT(), 0.30);
    colors.sidebar_primary = accent;
    colors.sidebar_primary_foreground = fg;
    colors.drag_border = accent;
    colors.drop_target = rgba(ACCENT(), 0.20);
}
