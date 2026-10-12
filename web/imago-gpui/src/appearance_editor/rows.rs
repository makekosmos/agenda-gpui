//! Control rows for the compact appearance editor (mode tiles,
//! theme list, accent swatches). Split from `mod.rs` for the 300-line gate.
use super::*;

impl AppearanceEditor {
    pub(super) fn mode_row(&self, cx: &mut Context<Self>) -> Div {
        let style = self.style;
        let modes = [
            ("system", "Системная"),
            ("light", "Светлая"),
            ("dark", "Тёмная"),
        ];
        let mut row = div().flex().gap_2();
        for (key, label) in modes {
            let active = self.settings.mode == key;
            row = row.child(
                div()
                    .id(SharedString::from(format!("appearance-mode-{key}")))
                    .px_3()
                    .py_1p5()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(if active {
                        c(style.accent)
                    } else {
                        rgba(style.border, 0.8)
                    })
                    .bg(if active {
                        rgba(style.accent, 0.12)
                    } else {
                        rgba(style.surface, 0.6)
                    })
                    .text_size(style.text_px(13.))
                    .text_color(c(style.foreground))
                    .cursor_pointer()
                    .role(Role::Button)
                    .aria_label(label)
                    .child(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.patch(json!({"mode": key}), cx);
                    })),
            );
        }
        row
    }

    pub(super) fn theme_rows(&self, cx: &mut Context<Self>) -> Div {
        let style = self.style;
        let mut col = div().flex().flex_col();
        for def in THEMES {
            let active = self.settings.dark_theme == def.key;
            let key = def.key;
            col = col.child(
                div()
                    .id(SharedString::from(format!("appearance-theme-{key}")))
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_3()
                    .py_2()
                    .rounded(px(8.))
                    .when(active, |d| d.bg(rgba(style.accent, 0.10)))
                    .cursor_pointer()
                    .role(Role::Button)
                    .aria_label(def.name)
                    .child(
                        div()
                            .w(px(14.))
                            .h(px(14.))
                            .rounded_full()
                            .border_1()
                            .border_color(c(def.dark.border))
                            .bg(c(def.dark.bg)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(style.text_px(13.))
                                    .text_color(c(style.foreground))
                                    .child(def.name),
                            )
                            .child(
                                div()
                                    .text_size(style.text_px(11.5))
                                    .text_color(rgba(style.muted, 0.8))
                                    .child(def.desc),
                            ),
                    )
                    .when(active, |d| {
                        d.child(
                            div()
                                .text_size(style.text_px(13.))
                                .text_color(c(style.accent))
                                .child("✓"),
                        )
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.patch(json!({"light_theme": key, "dark_theme": key}), cx);
                    })),
            );
        }
        col
    }

    pub(super) fn accent_row(&self, cx: &mut Context<Self>) -> Div {
        let style = self.style;
        let mut row = div().flex().gap_2().items_center();
        for (idx, swatch) in ACCENTS.iter().enumerate() {
            let color = *swatch;
            let active = match color {
                Some(hex) => {
                    self.settings.accent_source == "custom"
                        && self.settings.accent_color.as_deref() == Some(hex)
                }
                None => self.settings.accent_source == "theme",
            };
            row = row.child(
                div()
                    .id(SharedString::from(format!("appearance-accent-{idx}")))
                    .w(px(20.))
                    .h(px(20.))
                    .rounded_full()
                    .border_2()
                    .border_color(if active {
                        c(style.accent)
                    } else {
                        rgba(style.border, 0.6)
                    })
                    .when_some(color, |d, hex| {
                        d.bg(c(u32::from_str_radix(&hex[1..], 16).unwrap_or(0)))
                    })
                    .when_none(&color, |d| {
                        d.bg(rgba(style.muted, 0.3))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(10.))
                            .text_color(c(style.muted))
                            .child("A")
                    })
                    .cursor_pointer()
                    .role(Role::Button)
                    .aria_label(match color {
                        Some(_) => "Акцентный цвет",
                        None => "Акцент темы",
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.patch(
                            match color {
                                Some(hex) => {
                                    json!({"accent_source": "custom", "accent_color": hex})
                                }
                                None => json!({"accent_source": "theme"}),
                            },
                            cx,
                        );
                    })),
            );
        }
        row
    }
}
