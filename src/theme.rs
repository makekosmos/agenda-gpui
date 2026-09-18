// Exact color/dimension tokens extracted from Vue CSS (global.css + imago theme).
use gpui::{Hsla, rgb};

pub const BG: u32 = 0xffffff;
pub const FG: u32 = 0x0a0a0a;
pub const MUTED_FG: u32 = 0x737373;
pub const BORDER: u32 = 0xe5e5e5;
pub const ACCENT: u32 = 0x275ff3;
pub const ACCENT_FG: u32 = 0xfafafa;
pub const PRIMARY: u32 = 0x171717;
pub const SECONDARY: u32 = 0xf5f5f5;
pub const SURFACE: u32 = 0xf5f5f5;
pub const DESTRUCTIVE: u32 = 0xe7000b;
pub const WARN: u32 = 0xe1a035;
pub const SUCCESS: u32 = 0x00a63e;
pub const SIDEBAR_DIVIDER: u32 = 0x313131;
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
        _ => MUTED_FG,
    }
}

/// foreground over background mixes (light theme: over white).
pub fn fg_mix(a: f32) -> Hsla {
    mix(FG, a, BG)
}
pub fn border_mix(a: f32) -> Hsla {
    mix(BORDER, a, BG)
}
pub fn muted_fg_mix(a: f32) -> Hsla {
    mix(MUTED_FG, a, BG)
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
