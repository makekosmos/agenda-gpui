use super::*;

impl Agenda {
    // ==========================================================================
    // Statistics heatmap (84 days, 12×7)
    // ==========================================================================

    pub(crate) fn statistics_page(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let metric_labels = ["Выполнено", "Значимость", "Мыслетопливо"];
        let mut tabs = div().flex().gap_2();
        for (i, l) in metric_labels.iter().enumerate() {
            let hid = format!("stat-tab-{i}");
            let t = self.hover_t(window, &hid);
            let active = self.stat_metric == i as u8;
            let weak = cx.weak_entity();
            let key = SharedString::from(hid.clone());
            tabs = tabs.child(
                div()
                    .id(SharedString::from(format!("el-{hid}")))
                    .h_8()
                    .px_3()
                    .flex()
                    .items_center()
                    .rounded_md()
                    .text_size(px(13.))
                    .font_weight(gpui::FontWeight::MEDIUM)
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
                            let _ = weak.update(cx, |this, _| this.stat_metric = i as u8);
                        }
                    }),
            );
        }

        // values: last 84 days ending today (12 weeks × 7)
        let today = Local::now().date_naive();
        let start = today - Duration::days(83);
        let mut vals = [0f64; 84];
        let mut max_v = 0f64;
        for t in &self.todos {
            if let Some(d) = date_only(&t.completed_at).and_then(|k| parse_key(&k)) {
                let idx = (d - start).num_days();
                if (0..84).contains(&idx) {
                    let v = match self.stat_metric {
                        1 => t.significance.unwrap_or(0) as f64,
                        2 => t.fuel_cost.unwrap_or(0) as f64,
                        _ => 1.0,
                    };
                    vals[idx as usize] += v;
                    if vals[idx as usize] > max_v {
                        max_v = vals[idx as usize];
                    }
                }
            }
        }

        // grid-flow-col grid-rows-7: 12 week columns, 7 cells each, stretch to fill.
        // Emit column-major (week = 7 consecutive days) so ordering matches Vue.
        // Available content width = viewport - sidebar - px_8 padding, capped at max-w-4xl
        // inner (832px). w_full collapses inside the scroll container, so size explicitly.
        let grid_w = (window.viewport_size().width - px(240.0 + 64.0)).min(px(832.));
        let mut grid = div().flex().gap_1().w(grid_w).mt_4();
        for col_i in 0..12usize {
            let mut week = div().flex_1().min_w_0().flex().flex_col().gap_1();
            for row in 0..7usize {
                let v = vals[col_i * 7 + row];
                let t = (v / max_v.max(1.0)).min(1.0) as f32;
                let cell_bg = if v <= 0.0 {
                    c(SECONDARY())
                } else {
                    lerp(SECONDARY(), ACCENT(), 0.35 + 0.65 * t)
                };
                week = week.child(
                    div()
                        .w_full()
                        .h_8()
                        .rounded_md()
                        .border_1()
                        .border_color(c(BORDER()))
                        .bg(cell_bg)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(10.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(if v > 0.0 {
                            lerp(FG(), ACCENT_FG(), 0.35 + 0.65 * t)
                        } else {
                            rgba(0, 0.0)
                        })
                        .child(if v > 0.0 {
                            format!("{}", v as i64)
                        } else {
                            String::new()
                        }),
                );
            }
            grid = grid.child(week);
        }

        let desc = div()
            .mt_2()
            .text_size(px(12.))
            .text_color(c(MUTED_FG()))
            .child(format!(
                "{}: светлее — меньше, насыщеннее — больше. Ячейка показывает точное значение; серый цвет означает неполные или недоступные данные.",
                metric_labels[self.stat_metric as usize]
            ));

        let legend = div()
            .flex()
            .items_center()
            .gap_1()
            .mt_2()
            .text_size(px(12.))
            .text_color(c(MUTED_FG()))
            .child("Меньше")
            .children((0..5).map(|i| {
                div()
                    .w_3()
                    .h_3()
                    .rounded(px(2.))
                    .bg(lerp(SECONDARY(), ACCENT(), i as f32 / 4.0))
            }))
            .child("Больше");

        div()
            .flex_1()
            .min_h_0()
            .id("stats-scroll")
            .overflow_y_scroll()
            .track_scroll(&self.scroll("stats"))
            .child(
                div()
                    .w_full()
                    .max_w(px(896.))
                    .mx_auto()
                    .pt_4()
                    .px_8()
                    .flex()
                    .flex_col()
                    .child(tabs)
                    .child(grid)
                    .child(desc)
                    .child(legend),
            )
            .into_any_element()
    }
}
