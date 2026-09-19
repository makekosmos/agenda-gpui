// Runtime color palette, zeron themes (zeronsh/chat src/themes.css),
// oklch→sRGB converted by `cargo run --bin gen-themes` into src/palettes.rs.
// Sidebar bg is a step darker than the app bg.
#![allow(non_snake_case)]

use crate::palettes::THEMES;
use gpui::{rgb, Hsla};
use std::sync::atomic::{AtomicU8, Ordering};

/// Resolved color set for one theme+mode (see src/palettes.rs).
pub struct Palette {
    pub bg: u32,
    pub fg: u32,
    pub muted_fg: u32,
    pub border: u32,
    pub accent: u32,
    pub accent_fg: u32,
    pub accent_dim: u32,
    pub secondary: u32,
    pub card: u32,
    pub popover: u32,
    pub sidebar_bg: u32,
    pub sidebar_divider: u32,
    pub destructive: u32,
    pub warn: u32,
    pub success: u32,
    pub qe_chip_bg: u32,
    pub qe_chip_fg: u32,
}

/// Selected theme index into palettes::THEMES.
static THEME_IDX: AtomicU8 = AtomicU8::new(0);
/// Resolved mode: 0 = light, 1 = dark. "System" is resolved at render time.
static MODE: AtomicU8 = AtomicU8::new(1);

pub fn set_theme(idx: usize) {
    THEME_IDX.store(idx.min(THEMES.len() - 1) as u8, Ordering::Relaxed);
}
pub fn set_mode(dark: bool) {
    MODE.store(dark as u8, Ordering::Relaxed);
}
pub fn is_dark() -> bool {
    MODE.load(Ordering::Relaxed) != 0
}

pub fn pal() -> &'static Palette {
    let def = &THEMES[THEME_IDX.load(Ordering::Relaxed) as usize];
    if is_dark() {
        &def.dark
    } else {
        &def.light
    }
}

pub fn BG() -> u32 {
    pal().bg
}
pub fn FG() -> u32 {
    pal().fg
}
pub fn MUTED_FG() -> u32 {
    pal().muted_fg
}
pub fn BORDER() -> u32 {
    pal().border
}
pub fn ACCENT() -> u32 {
    pal().accent
}
pub fn ACCENT_FG() -> u32 {
    pal().accent_fg
}
pub fn ACCENT_DIM() -> u32 {
    pal().accent_dim
}
pub fn SECONDARY() -> u32 {
    pal().secondary
}
pub fn CARD() -> u32 {
    pal().card
}
pub fn POPOVER() -> u32 {
    pal().popover
}
pub fn SIDEBAR_BG() -> u32 {
    pal().sidebar_bg
}
pub fn DESTRUCTIVE() -> u32 {
    pal().destructive
}
pub fn WARN() -> u32 {
    pal().warn
}
#[allow(dead_code)]
pub fn SUCCESS() -> u32 {
    pal().success
}
pub fn SIDEBAR_DIVIDER() -> u32 {
    pal().sidebar_divider
}
pub fn QE_CHIP_BG() -> u32 {
    pal().qe_chip_bg
}
pub fn QE_CHIP_FG() -> u32 {
    pal().qe_chip_fg
}
pub const TAG_GRAY: u32 = 0x7a7a7a;

pub fn c(hex: u32) -> Hsla {
    rgb(hex).into()
}

/// Mix `top` over `bottom` with alpha a (color-mix(in srgb, top a%, bottom)).
pub fn mix(top: u32, a: f32, bottom: u32) -> Hsla {
    let tr = ((top >> 16) & 0xff) as f32;
    let tg = ((top >> 8) & 0xff) as f32;
    let tb = (top & 0xff) as f32;
    let br = ((bottom >> 16) & 0xff) as f32;
    let bg = ((bottom >> 8) & 0xff) as f32;
    let bb = (bottom & 0xff) as f32;
    let r = (tr * a + br * (1.0 - a)).round() as u32;
    let g = (tg * a + bg * (1.0 - a)).round() as u32;
    let b = (tb * a + bb * (1.0 - a)).round() as u32;
    c((r << 16) | (g << 8) | b)
}

/// Color with alpha.
pub fn rgba(hex: u32, a: f32) -> Hsla {
    let mut col: Hsla = rgb(hex).into();
    col.a = a;
    col
}

/// lerp two opaque colors, t ∈ [0,1].
pub fn lerp(a: u32, b: u32, t: f32) -> Hsla {
    let (r1, g1, b1) = ((a >> 16) & 0xff, (a >> 8) & 0xff, a & 0xff);
    let (r2, g2, b2) = ((b >> 16) & 0xff, (b >> 8) & 0xff, b & 0xff);
    let r = (r1 as f32 + (r2 as f32 - r1 as f32) * t).round() as u32;
    let g = (g1 as f32 + (g2 as f32 - g1 as f32) * t).round() as u32;
    let bb = (b1 as f32 + (b2 as f32 - b1 as f32) * t).round() as u32;
    c((r << 16) | (g << 8) | bb)
}

pub fn tag_color(name: &str) -> u32 {
    match name {
        "red" => 0xe66f64,
        "orange" => 0xd9975c,
        "yellow" => 0xccae59,
        "green" => 0x3db07c,
        "blue" => 0x548bd0,
        "purple" => 0xa074d0,
        "pink" => 0xd981ac,
        "gray" | "grey" => TAG_GRAY,
        _ => MUTED_FG(),
    }
}

/// foreground over background mixes (light theme: over white).
pub fn fg_mix(a: f32) -> Hsla {
    mix(FG(), a, BG())
}
pub fn border_mix(a: f32) -> Hsla {
    mix(BORDER(), a, BG())
}
pub fn muted_fg_mix(a: f32) -> Hsla {
    mix(MUTED_FG(), a, BG())
}

/// cubic-bezier(x1,y1,x2,y2) easing via bisection on x.
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let sample = |t: f32| -> (f32, f32) {
        let u = 1.0 - t;
        (
            3.0 * u * u * t * x1 + 3.0 * u * t * t * x2 + t * t * t,
            3.0 * u * u * t * y1 + 3.0 * u * t * t * y2 + t * t * t,
        )
    };
    let mut lo = 0.0_f32;
    let mut hi = 1.0_f32;
    let mut t = x;
    for _ in 0..32 {
        let (sx, _) = sample(t);
        if (sx - x).abs() < 1e-5 {
            break;
        }
        if sx < x {
            lo = t;
        } else {
            hi = t;
        }
        t = (lo + hi) / 2.0;
    }
    sample(t).1
}

/// Sidebar & emphasized easing: cubic-bezier(0.22, 1, 0.36, 1).
pub fn ease_emphasized(x: f32) -> f32 {
    cubic_bezier(0.22, 1.0, 0.36, 1.0, x)
}

/// Standard ease for 120ms hover transitions (CSS `ease`).
pub fn ease_standard(x: f32) -> f32 {
    cubic_bezier(0.25, 0.1, 0.25, 1.0, x)
}

/// Sliding hover-highlight move: cubic-bezier(0.23, 1, 0.32, 1).
pub fn ease_out_quint(x: f32) -> f32 {
    cubic_bezier(0.23, 1.0, 0.32, 1.0, x)
}
