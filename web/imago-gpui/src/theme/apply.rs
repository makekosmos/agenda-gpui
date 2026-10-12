// gpui-component theme wiring — split out of theme.rs to stay under the
// 300-line source gate. Same code as imago-gpui's theme.rs second half.
use super::*;
use gpui_component::theme::{Theme, ThemeConfig, ThemeConfigColors, ThemeMode};
use std::rc::Rc;

// gpui-component theme wiring
// ---------------------------------------------------------------------------

/// Install the selected theme's light+dark palettes as the app's
/// gpui-component themes and apply the active mode. Call once after
/// `gpui_component::init(cx)`, after any `set_theme`/`set_mode` calls.
pub fn apply(cx: &mut App) {
    let def = &THEMES[THEME_IDX.load(Ordering::Relaxed) as usize];
    let theme = Theme::global_mut(cx);
    theme.light_theme = Rc::new(config(&def.light, ThemeMode::Light));
    theme.dark_theme = Rc::new(config(&def.dark, ThemeMode::Dark));
    let mode = if is_dark() {
        ThemeMode::Dark
    } else {
        ThemeMode::Light
    };
    Theme::change(mode, None, cx);
}

fn config(p: &Palette, mode: ThemeMode) -> ThemeConfig {
    ThemeConfig {
        name: if mode.is_dark() {
            "Imago Dark".into()
        } else {
            "Imago Light".into()
        },
        mode,
        // Root uses this as the rem unit, including spacing and control sizes.
        // Agenda uses a 16px rem with explicit 13px body/sidebar text.
        font_size: Some(16.0),
        font_family: Some("Inter".into()),
        mono_font_family: None,
        mono_font_size: None,
        radius: Some(8),
        radius_lg: Some(10),
        shadow: Some(true),
        colors: colors(p),
        ..Default::default()
    }
}

fn hex(v: u32) -> Option<SharedString> {
    Some(format!("#{v:06x}").into())
}

fn hexa(v: u32, a: f32) -> Option<SharedString> {
    let byte = (a.clamp(0.0, 1.0) * 255.0).round() as u32;
    Some(format!("#{v:06x}{byte:02x}").into())
}

fn mix_hex(fg: u32, w: f32, bg: u32) -> Option<SharedString> {
    hex(mix_u32(fg, w, bg))
}

/// Palette → gpui-component `ThemeConfigColors`.
///
/// Hover/active shades derive from the palette: accent controls fade toward
/// `accent_fg` (lightens dark accents, darkens light ones); neutral fills
/// are `fg` alpha steps — the same `rgba(FG, …)` recipe agenda chrome uses.
fn colors(p: &Palette) -> ThemeConfigColors {
    let fg = p.fg;
    let bg = p.bg;
    let white = 0xffffff;
    let black = 0x000000;

    let mut cc = ThemeConfigColors::default();
    cc.accent = hex(p.accent);
    cc.accent_foreground = hex(p.accent_fg);
    cc.accordion = hex(p.card);
    cc.background = hex(bg);
    cc.border = hex(p.border);

    // Default button: neutral foreground-tinted fill (agenda rgba(FG, ·)).
    cc.button = hexa(fg, 0.06);
    cc.button_hover = hexa(fg, 0.10);
    cc.button_active = hexa(fg, 0.16);
    cc.button_foreground = hex(fg);

    // Primary = accent fill.
    cc.button_primary = hex(p.accent);
    cc.button_primary_foreground = hex(p.accent_fg);
    cc.button_primary_hover = mix_hex(p.accent, 0.85, p.accent_fg);
    cc.button_primary_active = mix_hex(p.accent, 0.72, p.accent_fg);

    // Secondary = `secondary` surface with fg-tinted hover.
    cc.button_secondary = hex(p.secondary);
    cc.button_secondary_foreground = hex(fg);
    cc.button_secondary_hover = mix_hex(fg, 0.10, p.secondary);
    cc.button_secondary_active = mix_hex(fg, 0.16, p.secondary);

    // Danger/success/info/warning buttons: tinted-outline styles.
    cc.button_danger = hexa(p.destructive, 0.10);
    cc.button_danger_foreground = hex(p.destructive);
    cc.button_danger_hover = hexa(p.destructive, 0.20);
    cc.button_danger_active = hexa(p.destructive, 0.35);
    cc.button_success = hexa(p.success, 0.20);
    cc.button_success_foreground = hex(p.success);
    cc.button_success_hover = hexa(p.success, 0.28);
    cc.button_success_active = hexa(p.success, 0.38);
    cc.button_info = hexa(p.accent, 0.15);
    cc.button_info_foreground = hex(p.accent);
    cc.button_info_hover = hexa(p.accent, 0.25);
    cc.button_info_active = hexa(p.accent, 0.35);
    cc.button_warning = hexa(p.warn, 0.15);
    cc.button_warning_foreground = hex(p.warn);
    cc.button_warning_hover = hexa(p.warn, 0.25);
    cc.button_warning_active = hexa(p.warn, 0.35);

    cc.caret = hex(fg);
    cc.chart_bullish = hex(p.success);
    cc.chart_bearish = hex(p.destructive);

    cc.danger = hex(p.destructive);
    cc.danger_foreground = hex(white);
    cc.danger_hover = mix_hex(p.destructive, 0.88, white);
    cc.danger_active = mix_hex(p.destructive, 0.80, black);
    cc.info = hex(p.accent);
    cc.info_foreground = hex(p.accent_fg);
    cc.info_hover = mix_hex(p.accent, 0.88, white);
    cc.info_active = mix_hex(p.accent, 0.80, black);
    cc.success = hex(p.success);
    cc.success_foreground = hex(white);
    cc.success_hover = mix_hex(p.success, 0.88, white);
    cc.success_active = mix_hex(p.success, 0.80, black);
    cc.warning = hex(p.warn);
    cc.warning_foreground = hex(black);
    cc.warning_hover = mix_hex(p.warn, 0.88, white);
    cc.warning_active = mix_hex(p.warn, 0.80, black);

    cc.description_list_label = hex(p.card);
    cc.description_list_label_foreground = hex(p.muted_fg);
    cc.drag_border = hex(p.accent);
    cc.drop_target = hexa(p.accent, 0.20);
    cc.foreground = hex(fg);

    cc.group_box = hex(p.card);
    cc.group_box_foreground = hex(fg);
    cc.group_box_title_foreground = hex(p.muted_fg);

    cc.input = hex(p.border);
    cc.link = hex(p.accent);
    cc.link_hover = mix_hex(p.accent, 0.85, p.accent_fg);
    cc.link_active = mix_hex(p.accent, 0.72, p.accent_fg);

    cc.list = hex(bg);
    cc.list_hover = hexa(fg, 0.05);
    cc.list_active = hexa(fg, 0.12);
    cc.list_active_border = hex(p.border);
    cc.list_even = hex(bg);
    cc.list_head = mix_hex(fg, 0.06, bg);

    cc.muted = hex(p.card);
    cc.muted_foreground = hex(p.muted_fg);
    cc.popover = hex(p.popover);
    cc.popover_foreground = hex(fg);
    cc.primary = hex(p.accent);
    cc.primary_foreground = hex(p.accent_fg);
    cc.primary_hover = mix_hex(p.accent, 0.85, p.accent_fg);
    cc.primary_active = mix_hex(p.accent, 0.72, p.accent_fg);
    cc.progress_bar = hex(p.accent);
    cc.ring = hex(p.accent);

    cc.scrollbar = hexa(0, 0.0);
    cc.scrollbar_thumb = hexa(fg, 0.18);
    cc.scrollbar_thumb_hover = hexa(fg, 0.32);

    cc.secondary = hex(p.secondary);
    cc.secondary_foreground = hex(fg);
    cc.secondary_hover = mix_hex(fg, 0.10, p.secondary);
    cc.secondary_active = mix_hex(fg, 0.16, p.secondary);
    cc.selection = hexa(p.accent, 0.30);

    cc.sidebar = hex(p.sidebar_bg);
    cc.sidebar_foreground = hex(fg);
    cc.sidebar_border = hex(p.sidebar_divider);
    cc.sidebar_primary = hex(p.accent);
    cc.sidebar_primary_foreground = hex(p.accent_fg);
    cc.sidebar_accent = hexa(fg, 0.12);
    cc.sidebar_accent_foreground = hex(fg);

    cc.skeleton = hexa(fg, 0.10);
    cc.slider_bar = hex(p.secondary);
    cc.slider_thumb = hex(fg);
    cc.switch = hexa(fg, 0.18);
    cc.switch_thumb = hex(fg);

    cc.tab = hexa(0, 0.0);
    cc.tab_foreground = hex(p.muted_fg);
    cc.tab_active = hex(p.card);
    cc.tab_active_foreground = hex(fg);
    cc.tab_bar = hex(bg);
    cc.tab_bar_segmented = hex(p.secondary);

    cc.table = hex(p.card);
    cc.table_row_border = hex(p.sidebar_divider);
    cc.table_hover = hexa(fg, 0.05);
    cc.table_active = hexa(fg, 0.12);
    cc.table_active_border = hex(p.border);
    cc.table_even = hex(p.card);
    cc.table_head = mix_hex(fg, 0.06, p.card);
    cc.table_foot = hex(p.card);
    cc.table_foot_foreground = hex(p.muted_fg);

    cc.title_bar = hex(bg);
    cc.title_bar_border = hex(p.border);
    cc.status_bar = hex(p.sidebar_bg);
    cc.status_bar_border = hex(p.sidebar_divider);

    cc.overlay = hexa(black, 0.45);
    cc.window_border = hex(p.sidebar_divider);
    cc
}
