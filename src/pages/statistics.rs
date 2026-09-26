use super::*;
use gpui_component::tooltip::Tooltip;

mod clipboard;
mod share;

impl Agenda {
    pub(crate) fn statistics_page(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let today = Local::now().date_naive();

        // Per-day completed counts for all history.
        let mut days = std::collections::BTreeMap::<NaiveDate, i64>::new();
        for t in &self.todos {
            if let Some(d) = date_only(&t.completed_at).and_then(|k| parse_key(&k)) {
                *days.entry(d).or_insert(0) += 1;
            }
        }
        let total_done: i64 = days.values().sum();
        let last7: i64 = days
            .iter()
            .filter(|(d, _)| **d > today - Duration::days(7))
            .map(|(_, c)| c)
            .sum();
        let last30: i64 = days
            .iter()
            .filter(|(d, _)| **d > today - Duration::days(30))
            .map(|(_, c)| c)
            .sum();

        // Streaks: current streak may start yesterday if today is still empty.
        let mut cur_streak = 0i64;
        let mut d = today;
        if !days.contains_key(&d) {
            d -= Duration::days(1);
        }
        while days.contains_key(&d) {
            cur_streak += 1;
            d -= Duration::days(1);
        }
        let mut best_streak = 0i64;
        let mut run = 0i64;
        let mut prev: Option<NaiveDate> = None;
        for day in days.keys() {
            run = if prev == Some(*day - Duration::days(1)) {
                run + 1
            } else {
                1
            };
            best_streak = best_streak.max(run);
            prev = Some(*day);
        }

        // Summary strip (bordered card, one row, thin separators).
        let stat_items = [
            (format!("{total_done}"), "Всего выполнено"),
            (format!("{last7}"), "За 7 дней"),
            (format!("{last30}"), "За 30 дней"),
            (format!("{cur_streak}"), "Текущая серия"),
            (format!("{best_streak}"), "Лучшая серия"),
        ];
        let mut card = div()
            .flex()
            .items_stretch()
            .rounded_lg()
            .border_1()
            .border_color(c(BORDER()))
            .py_3();
        let last_i = stat_items.len() - 1;
        for (i, (value, label)) in stat_items.into_iter().enumerate() {
            card = card.child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_0p5()
                    .child(
                        div()
                            .text_size(px(16.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(c(FG()))
                            .child(value),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(c(MUTED_FG()))
                            .child(label),
                    ),
            );
            if i != last_i {
                card = card.child(div().w(px(1.)).my_1().bg(fg_mix(0.08)));
            }
        }

        // GitHub-style contribution heatmap: columns = calendar weeks
        // (Mon–Sun), rows = weekday. The last column is the current week and
        // the first one starts on the Monday `weeks-1` weeks ago. Width adapts
        // to the viewport (~a year on a wide window).
        const CELL: f32 = 12.0;
        const GAP: f32 = 3.0;
        let grid_w = (window.viewport_size().width - px(240.0 + 64.0)).min(px(832.));
        let weeks = ((grid_w / px(CELL + GAP)) as usize).clamp(4, 53);

        let dow = today.weekday().num_days_from_monday() as i64;
        let last_monday = today - Duration::days(dow);
        let start = last_monday - Duration::days(7 * (weeks as i64 - 1));
        let total = weeks * 7;

        let mut vals = vec![0f64; total];
        let mut max_v = 0f64;
        for (day, count) in &days {
            let idx = (*day - start).num_days();
            if (0..total as i64).contains(&idx) {
                vals[idx as usize] = *count as f64;
                max_v = max_v.max(*count as f64);
            }
        }

        let cell_bg = |v: f64| {
            if v <= 0.0 {
                fg_mix(0.06)
            } else {
                rgba(
                    ACCENT(),
                    0.25 + 0.75 * ((v / max_v.max(1.0)).min(1.0) as f32),
                )
            }
        };

        // Month captions under the week containing the 1st of a month
        // (3-letter). A label is written only when the month fits: its 1st
        // day must lie inside the grid and two columns must remain so the
        // text does not overflow the right edge.
        let mut months = div().flex().gap(px(GAP)).h(px(16.)).mt_3().flex_none();
        for w in 0..weeks {
            let monday = start + Duration::days(7 * w as i64);
            let sunday = monday + Duration::days(6);
            // The 1st of a month falls inside this week.
            let first = if monday.day() == 1 {
                Some(monday)
            } else if sunday.month() != monday.month() {
                NaiveDate::from_ymd_opt(sunday.year(), sunday.month(), 1)
            } else {
                None
            };
            let label = match first {
                Some(f) if f >= start && w + 2 <= weeks => MONTH_SHORT[(f.month() - 1) as usize]
                    .chars()
                    .take(3)
                    .collect::<String>(),
                _ => String::new(),
            };
            months = months.child(
                div()
                    .w(px(CELL))
                    .flex_none()
                    .text_size(px(10.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(c(MUTED_FG()))
                    .whitespace_nowrap()
                    .child(label),
            );
        }

        let mut cols = div().flex().gap(px(GAP)).flex_none();
        for w in 0..weeks {
            let mut col = div().flex().flex_col().gap(px(GAP));
            for row in 0..7usize {
                let idx = w * 7 + row;
                let date = start + Duration::days(idx as i64);
                if date > today {
                    break; // the grid ends at today, no future cells
                }
                let v = vals[idx];
                let caption = format!(
                    "{} задач {} {}",
                    v as i64,
                    date.day(),
                    MONTH_LONG[(date.month() - 1) as usize]
                );
                col = col.child(
                    div()
                        .id(SharedString::from(format!("heat-{idx}")))
                        .size(px(CELL))
                        .flex_none()
                        .rounded(px(2.5))
                        .bg(cell_bg(v))
                        .tooltip(move |window, cx| Tooltip::new(caption.clone()).build(window, cx)),
                );
            }
            cols = cols.child(col);
        }

        let grid = div().mt_3().flex().gap_0().child(cols);

        // "Поделиться" button, top-right like the reference header.
        let share_t = self.hover_t(window, "stat-share");
        let weak = cx.weak_entity();
        let share_btn = div()
            .id("el-stat-share")
            .size(px(32.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .bg(fg_mix(0.05 + 0.05 * share_t))
            .child(icon("icons/share.svg", 16., rgba(FG(), 0.9)))
            .on_hover(move |hovered, _, cx| {
                let _ = weak.update(cx, |this, _| this.set_hover("stat-share", *hovered));
            })
            .on_click({
                let weak = cx.weak_entity();
                move |_: &ClickEvent, _, cx| {
                    let _ = weak.update(cx, |this, _| this.share_open = true);
                }
            })
            .a11y_button("Поделиться");

        let share_overlay = self.statistics_share_overlay(
            today,
            &days,
            cell_bg,
            [total_done, last7, cur_streak, best_streak],
            cx,
        );

        // Profile header: empty avatar placeholder + stub name (wired up later).
        let profile = div()
            .mt_2()
            .flex()
            .flex_col()
            .items_center()
            .child(
                div()
                    .size(px(72.))
                    .rounded_full()
                    .border_1()
                    .border_color(fg_mix(0.12))
                    .bg(fg_mix(0.06)),
            )
            .child(
                div()
                    .mt_3()
                    .text_size(px(20.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c(FG()))
                    .child("Имя Фамилия"),
            );

        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .relative()
            .child(
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
                            .child(div().flex().justify_end().child(share_btn))
                            .child(profile)
                            .child(div().mt_4().child(card))
                            .child(
                                div()
                                    .mt_6()
                                    .text_size(px(15.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("Активность"),
                            )
                            .child(grid)
                            .child(months)
                            .child(div().pb_8()),
                    ),
            )
            .when_some(share_overlay, |el, overlay| el.child(overlay))
            .into_any_element()
    }
}
