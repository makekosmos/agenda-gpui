use super::*;

impl Agenda {
    // ==========================================================================
    // Calendar
    // ==========================================================================

    pub(crate) fn calendar_page(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let anchor = parse_key(&self.cal_anchor).unwrap_or_else(|| Local::now().date_naive());
        let (days, period): (Vec<String>, String) = match self.cal_mode {
            CalMode::Day => {
                let k = key_of(anchor);
                (vec![k.clone()], fmt_day_month_year(&k))
            }
            CalMode::Week => {
                let monday =
                    anchor - Duration::days(anchor.weekday().num_days_from_monday() as i64);
                let days: Vec<String> =
                    (0..7).map(|i| key_of(monday + Duration::days(i))).collect();
                (days.clone(), fmt_week_period(&days[0], &days[6]))
            }
        };
        let today = today_key();

        // header controls
        let seg = |app: &mut Self,
                   window: &mut Window,
                   cx: &mut Context<Self>,
                   mode: CalMode,
                   label: &str| {
            let hid = format!("cal-seg-{label}");
            let t = app.hover_t(window, &hid);
            let active = app.cal_mode == mode;
            let weak = cx.weak_entity();
            let key = SharedString::from(hid.clone());
            div()
                .id(SharedString::from(format!("el-{hid}")))
                .h_7()
                .px_3()
                .flex()
                .items_center()
                .rounded_md()
                .text_size(px(12.))
                .text_color(if active { c(FG()) } else { c(MUTED_FG()) })
                .bg(if active {
                    fg_mix(0.10)
                } else {
                    fg_mix(0.05 * t)
                })
                .child(label.to_string())
                .on_hover({
                    let weak = weak.clone();
                    move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                    }
                })
                .on_click(move |_: &ClickEvent, _, cx| {
                    let _ = weak.update(cx, |this, _| this.cal_mode = mode);
                })
        };
        let nav = |app: &mut Self,
                   window: &mut Window,
                   cx: &mut Context<Self>,
                   hid: &str,
                   label: &str,
                   delta: i64| {
            let t = app.hover_t(window, hid);
            let weak = cx.weak_entity();
            let key = SharedString::from(hid.to_string());
            div()
                .id(SharedString::from(format!("el-{hid}")))
                .h_7()
                .px_3()
                .flex()
                .items_center()
                .rounded_md()
                .text_size(px(12.))
                .text_color(mix(FG(), 0.65 + 0.35 * t, BG()))
                .bg(fg_mix(0.05 + 0.03 * t))
                .child(label.to_string())
                .on_hover({
                    let weak = weak.clone();
                    move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                    }
                })
                .on_click(move |_: &ClickEvent, _, cx| {
                    let _ = weak.update(cx, |this, _| {
                        if delta == 0 {
                            this.cal_anchor = today_key();
                        } else {
                            let a = parse_key(&this.cal_anchor)
                                .unwrap_or_else(|| Local::now().date_naive());
                            let step = if this.cal_mode == CalMode::Week {
                                delta * 7
                            } else {
                                delta
                            };
                            this.cal_anchor = key_of(a + Duration::days(step));
                        }
                    });
                })
        };

        let header = div()
            .flex_none()
            .flex()
            .items_center()
            .px_7()
            .pt_4()
            .pb_3()
            .child(
                div()
                    .flex_1()
                    .text_size(px(13.))
                    .text_color(c(MUTED_FG()))
                    .child(period),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(seg(self, window, cx, CalMode::Day, "День"))
                    .child(seg(self, window, cx, CalMode::Week, "Неделя")),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .ml_2()
                    .child(nav(self, window, cx, "cal-prev", "←", -1))
                    .child(nav(self, window, cx, "cal-today", "Сегодня", 0))
                    .child(nav(self, window, cx, "cal-next", "→", 1)),
            );

        // columns
        let mut grid = div()
            .flex_1()
            .min_h_0()
            .flex()
            .gap_3()
            .px_7()
            .pb_4()
            .overflow_hidden();
        for d in &days {
            grid = grid.child(self.cal_day_col(d, &today, window, cx));
        }

        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(header)
            .child(grid)
            .into_any_element()
    }
}
