use super::*;

impl Agenda {
    /// The two wrap-rows of property chips on the task page plus the inline
    /// recurrence editor card (mounted while `recur_open`).
    pub(crate) fn task_props(
        &mut self,
        t: &Todo,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let status = task_status(t);
        let (date_val, _) = task_date(t);
        let today = today_key();
        let overdue = date_val.as_deref().is_some_and(|d| d < today.as_str())
            && status != Status::Done
            && status != Status::Canceled;

        // ---- prop chips
        let status_label = match status {
            Status::Inbox => "Входящие",
            Status::Todo => "Сделать",
            Status::Started => "В работе",
            Status::Deferred => "Потом",
            Status::Done => "Готово",
            Status::Canceled => "Отменено",
        };
        let priority_label =
            ["Без приоритета", "Низкий", "Средний", "Высокий"][t.priority as usize];

        let mut props = div().flex().flex_wrap().items_center().gap_1();
        props = props.child(
            self.prop_chip(
                "tp-status",
                DropKind::Status,
                id,
                (format!("Статус: {status_label}"), status_label),
                window,
                cx,
            )
            .child(status_ring(status)),
        );
        props = props.child(
            self.prop_chip(
                "tp-prio",
                DropKind::Priority,
                id,
                (format!("Приоритет: {priority_label}"), priority_label),
                window,
                cx,
            )
            .child(priority_bars(t.priority)),
        );

        // date chip
        let date_label = date_val
            .as_ref()
            .map(|d| fmt_day_month(d))
            .unwrap_or_else(|| "Срок".to_string());
        let date_name = format!("Срок: {date_label}");
        let mut date_chip = self
            .prop_chip("tp-date", DropKind::Date, id, (date_name, ""), window, cx)
            .child(icon("icons/calendar-02.svg", 14., c(MUTED_FG())))
            .child(
                div()
                    .text_color(if overdue { c(DESTRUCTIVE()) } else { c(FG()) })
                    .child(date_label),
            );
        if date_val.is_some() {
            date_chip = date_chip.child(
                div()
                    .id("tp-date-clear")
                    .w_3p5()
                    .h_3p5()
                    .grid()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .child(icon("icons/status-x.svg", 9., c(MUTED_FG())))
                    .on_click({
                        let weak = cx.weak_entity();
                        let id2 = id.to_string();
                        move |_: &ClickEvent, _, cx| {
                            let _ = weak.update(cx, |this, _| {
                                this.run_menu_action(MenuAction::SetDate(id2.clone(), None));
                            });
                        }
                    })
                    .a11y_button("Убрать срок"),
            );
        }
        props = props.child(date_chip);

        // project chip
        let project_label = t
            .project_id
            .as_deref()
            .and_then(|pid| self.projects.iter().find(|p| p.id == pid))
            .map(|p| p.title.clone())
            .unwrap_or_else(|| "Без проекта".to_string());
        props = props.child(
            self.prop_chip(
                "tp-proj",
                DropKind::Project,
                id,
                (format!("Проект: {project_label}"), &project_label),
                window,
                cx,
            )
            .child(icon("icons/folder.svg", 14., c(MUTED_FG()))),
        );

        // tag pills + "Метки" add chip
        for tag_id in &t.tag_ids {
            if let Some(tag) = self.tag(tag_id) {
                let dot = tag_color(&tag.color);
                props = props.child(
                    div()
                        .h(px(22.))
                        .pl_2()
                        .pr_1()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .rounded_full()
                        .bg(fg_mix(0.06))
                        .text_size(px(12.))
                        .text_color(c(FG()))
                        .child(div().w(px(7.)).h(px(7.)).rounded_full().bg(c(dot)))
                        .child(tag.title.clone())
                        .child(
                            div()
                                .id(SharedString::from(format!("tagx-{}", tag.id)))
                                .w_3p5()
                                .h_3p5()
                                .grid()
                                .items_center()
                                .justify_center()
                                .rounded_full()
                                .child(icon("icons/status-x.svg", 9., c(MUTED_FG())))
                                .on_click({
                                    let weak = cx.weak_entity();
                                    let id2 = id.to_string();
                                    let tagid = tag.id.to_string();
                                    move |_: &ClickEvent, _, cx| {
                                        let _ = weak.update(cx, |this, _| {
                                            this.run_menu_action(MenuAction::RemoveTag(
                                                id2.clone(),
                                                tagid.clone(),
                                            ));
                                        });
                                    }
                                })
                                .a11y_button(format!("Убрать метку «{}»", tag.title)),
                        ),
                );
            }
        }
        props = props.child(
            self.prop_chip(
                "tp-tags",
                DropKind::Tags,
                id,
                ("Метки", "Метки"),
                window,
                cx,
            )
            .child(icon("icons/tag.svg", 14., c(MUTED_FG()))),
        );

        // recurrence chip
        let recur_label = if t.recurrence.is_some() {
            describe_recurrence(&t.recurrence)
        } else {
            "Повторение".to_string()
        };
        props = props.child(
            self.recur_chip("tp-recur", id, &recur_label, window, cx)
                .child(icon("icons/repeat.svg", 14., c(MUTED_FG()))),
        );

        // second row: significance + billable + fuel
        let mut props2 = div().flex().flex_wrap().items_center().gap_1();
        let sig_label = t
            .significance
            .map(|s| format!("{}/10", s))
            .unwrap_or_else(|| "Не оценено".to_string());
        props2 = props2.child(
            self.prop_chip(
                "tp-sig",
                DropKind::Significance,
                id,
                (format!("Значимость: {sig_label}"), &sig_label),
                window,
                cx,
            )
            .child(icon("icons/star.svg", 14., c(MUTED_FG()))),
        );
        let bill_label = if t.billable {
            format!("Оплачиваемая ${}", t.price.unwrap_or(0.0))
        } else {
            "Без оплаты".to_string()
        };
        props2 = props2.child(
            self.prop_chip(
                "tp-bill",
                DropKind::Billable,
                id,
                (format!("Оплата: {bill_label}"), &bill_label),
                window,
                cx,
            )
            .child(icon("icons/dollar.svg", 14., c(MUTED_FG()))),
        );
        if let Some(f) = t.fuel_cost {
            props2 = props2.child(
                div()
                    .h_7()
                    .px_2()
                    .flex()
                    .items_center()
                    .text_size(px(13.))
                    .text_color(c(MUTED_FG()))
                    .child(format!("~{}%", f)),
            );
        }

        // recurrence editor card
        let recur_editor = if self.recur_open {
            self.render_recur_editor(t, window, cx).into_any_element()
        } else {
            div().into_any_element()
        };

        vec![
            props.into_any_element(),
            props2.into_any_element(),
            recur_editor,
        ]
    }
}
