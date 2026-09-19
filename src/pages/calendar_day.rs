use super::*;

impl Agenda {
    /// One calendar day column (.calendar-day): dashed event cards + task
    /// cards; `is_today` switches the border to accent.
    pub(crate) fn cal_day_col(
        &mut self,
        d: &str,
        today: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let day_events: Vec<CalEvent> = self
            .events
            .iter()
            .filter(|e| date_only(&Some(e.starts_at.clone())).as_deref() == Some(d))
            .cloned()
            .collect();
        let day_todos: Vec<Todo> = self
            .todos
            .iter()
            .filter(|t| !t.is_trashed && task_date(t).0.as_deref() == Some(d))
            .cloned()
            .collect();
        let is_today = d == today;

        // .calendar-day: min-h-40 rounded-lg border bg-secondary/20 p-3; today → border-accent
        let mut head = div()
            .text_size(px(13.))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(c(FG()))
            .flex()
            .gap_1()
            .child(fmt_cal_day_label(d));
        if is_today {
            head = head.child(
                div()
                    .text_size(px(11.))
                    .text_color(c(ACCENT()))
                    .child("Сегодня"),
            );
        }
        let mut col = div()
            .flex_1()
            .min_w_0()
            .min_h(px(160.))
            .rounded_lg()
            .border_1()
            .border_color(if is_today { c(ACCENT()) } else { c(BORDER()) })
            .bg(rgba(SECONDARY(), 0.2))
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .child(head);
        if day_events.is_empty() && day_todos.is_empty() {
            col = col.child(
                div()
                    .text_size(px(12.))
                    .text_color(c(MUTED_FG()))
                    .child("Нет задач"),
            );
        }
        for e in &day_events {
            let mut card = div()
                .rounded_md()
                .border_1()
                .border_dashed()
                .border_color(c(BORDER()))
                .bg(c(CARD()))
                .p_2()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(c(ACCENT()))
                        .child(format!(
                            "{}–{}",
                            fmt_time(&e.starts_at),
                            e.ends_at.as_deref().map(fmt_time).unwrap_or_default()
                        )),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(c(FG()))
                        .child(e.title),
                );
            let loc = e.location.unwrap_or("PseudoCalendar");
            card = card.child(
                div()
                    .text_size(px(12.))
                    .text_color(c(MUTED_FG()))
                    .child(format!("{} · PseudoCalendar", loc)),
            );
            col = col.child(card);
        }
        for t in &day_todos {
            let st = task_status(t);
            let st_label = match st {
                Status::Done => "Готово",
                Status::Started => "В работе",
                Status::Canceled => "Отменено",
                Status::Deferred => "Потом",
                _ => "К выполнению",
            };
            let weak = cx.weak_entity();
            let tid = t.id.to_string();
            let hid = format!("cal-t-{}", t.id);
            let ht = self.hover_t(window, &hid);
            let key = SharedString::from(hid.clone());
            col = col.child(
                div()
                    .id(SharedString::from(format!("el-{hid}")))
                    .rounded_md()
                    .border_1()
                    .border_color(lerp(BORDER(), ACCENT(), ht * 0.5))
                    .bg(c(CARD()))
                    .p_2()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(c(FG()))
                            .child(t.title.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(c(MUTED_FG()))
                            .child(format!("{} · Agenda", st_label)),
                    )
                    .on_hover(move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                    })
                    .on_click({
                        let weak = cx.weak_entity();
                        let tid = tid.clone();
                        move |_: &ClickEvent, _, cx| {
                            let _ =
                                weak.update(cx, |this, _| this.navigate(Route::Task(tid.clone())));
                        }
                    }),
            );
        }
        col
    }
}
