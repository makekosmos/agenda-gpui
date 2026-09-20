use super::*;

impl Agenda {
    pub(crate) fn settings_page(
        &mut self,
        fuel: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut col = div()
            .id("settings")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll("settings"))
            .flex()
            .flex_col()
            .px_7()
            .pt_4()
            .gap_3()
            .max_w(px(720.));
        if fuel {
            col = col.child(
                div()
                    .text_size(px(15.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c(FG()))
                    .child("Мыслетопливо"),
            );
            col = col.child(
                div()
                    .text_size(px(13.))
                    .text_color(c(MUTED_FG()))
                    .child(
                        "Мыслетопливо оценивает стоимость задач в процентах. Оценивайте задачи от 1 до 10 на странице задачи.",
                    ),
            );
        } else {
            // --- Mode ---
            col = col.child(
                div()
                    .text_size(px(13.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c(FG()))
                    .child("Режим"),
            );
            let mut row = div().flex().gap_2();
            for (i, l) in ["Светлая", "Тёмная", "Системная"].iter().enumerate()
            {
                let active = self.theme_sel == i as u8;
                let hid = format!("mode-{i}");
                let t = self.hover_t(window, &hid);
                let weak = cx.weak_entity();
                let key = SharedString::from(hid.clone());
                row = row.child(
                    div()
                        .id(SharedString::from(format!("el-{hid}")))
                        .h_8()
                        .px_3()
                        .flex()
                        .items_center()
                        .rounded_md()
                        .text_size(px(13.))
                        .text_color(if active { c(ACCENT_FG()) } else { c(FG()) })
                        .bg(if active {
                            c(ACCENT())
                        } else {
                            fg_mix(0.05 + 0.03 * t)
                        })
                        .child(*l)
                        .on_hover(move |hovered, _, cx| {
                            let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                        })
                        .on_click({
                            let weak = cx.weak_entity();
                            move |_: &ClickEvent, _, cx| {
                                let _ = weak.update(cx, |this, _| this.theme_sel = i as u8);
                            }
                        }),
                );
            }
            col = col.child(row);

            // --- Named themes (zeron palette list) ---
            col = col.child(
                div()
                    .pt_3()
                    .text_size(px(13.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c(FG()))
                    .child("Тема"),
            );
            for (i, def) in crate::palettes::THEMES.iter().enumerate() {
                let active = self.theme_idx == i;
                let p = if is_dark() { &def.dark } else { &def.light };
                let hid = format!("theme-{i}");
                let t = self.hover_t(window, &hid);
                let weak = cx.weak_entity();
                let key = SharedString::from(hid.clone());
                let mut dots = div().flex().gap_1().items_center();
                for sw in [p.accent, p.card, p.fg] {
                    dots = dots.child(
                        div()
                            .size_3()
                            .rounded_full()
                            .bg(c(sw))
                            .border_1()
                            .border_color(border_mix(0.6)),
                    );
                }
                col = col.child(
                    div()
                        .id(SharedString::from(format!("el-{hid}")))
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .border_1()
                        .border_color(if active {
                            rgba(ACCENT(), 0.5)
                        } else {
                            rgba(FG(), 0.08 + 0.05 * t)
                        })
                        .bg(if active {
                            mix(ACCENT(), 0.08, BG())
                        } else {
                            fg_mix(0.02 + 0.02 * t)
                        })
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_0p5()
                                .child(div().text_size(px(13.)).text_color(c(FG())).child(def.name))
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(c(MUTED_FG()))
                                        .child(def.desc),
                                ),
                        )
                        .child(dots)
                        .on_hover(move |hovered, _, cx| {
                            let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                        })
                        .on_click({
                            let weak = cx.weak_entity();
                            move |_: &ClickEvent, _, cx| {
                                let _ = weak.update(cx, |this, _| this.theme_idx = i);
                            }
                        }),
                );
            }

            // --- Sidebar material ---
            col = col.child(
                div()
                    .pt_3()
                    .text_size(px(13.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c(FG()))
                    .child("Сайдбар"),
            );
            let material_labels: [&str; 3] = if cfg!(target_os = "windows") {
                ["Монолит", "Акрилик", "Мика"]
            } else if cfg!(target_os = "macos") {
                ["Монолит", "Акрилик", "Вибранси"]
            } else {
                ["Монолит", "Акрилик", "Материал"]
            };
            let mut row = div().flex().gap_2();
            for (i, l) in material_labels.iter().enumerate() {
                let active = self.sb_material == i as u8;
                let hid = format!("material-{i}");
                let t = self.hover_t(window, &hid);
                let weak = cx.weak_entity();
                let key = SharedString::from(hid.clone());
                row = row.child(
                    div()
                        .id(SharedString::from(format!("el-{hid}")))
                        .h_8()
                        .px_3()
                        .flex()
                        .items_center()
                        .rounded_md()
                        .text_size(px(13.))
                        .text_color(if active { c(ACCENT_FG()) } else { c(FG()) })
                        .bg(if active {
                            c(ACCENT())
                        } else {
                            fg_mix(0.05 + 0.03 * t)
                        })
                        .child(*l)
                        .on_hover(move |hovered, _, cx| {
                            let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                        })
                        .on_click({
                            let weak = cx.weak_entity();
                            move |_: &ClickEvent, _, cx| {
                                let _ = weak.update(cx, |this, _| this.sb_material = i as u8);
                            }
                        }),
                );
            }
            col = col.child(row);
        }
        col.into_any_element()
    }

    pub(crate) fn about_page(&mut self) -> AnyElement {
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .items_center()
            .pt_16()
            .gap_2()
            .child(
                div()
                    .text_size(px(20.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c(FG()))
                    .child("Agenda"),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(c(MUTED_FG()))
                    .child(concat!(
                        "Версия ",
                        env!("CARGO_PKG_VERSION"),
                        " (GPUI clone)"
                    )),
            )
            .into_any_element()
    }
}
