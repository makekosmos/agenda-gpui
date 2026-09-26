use super::*;

impl Agenda {
    /// `.task-prop` chip button: h28 px8 r6 gap6 fs13, hover fg6%.
    /// `labels` is `(a11y name, visible label)` — the name carries the
    /// property plus its current value ("Статус: В работе"); the label is
    /// only the text drawn inside the chip.
    pub(crate) fn prop_chip(
        &mut self,
        hid: &str,
        kind: DropKind,
        todo_id: &str,
        labels: (impl Into<SharedString>, &str),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let (name, label) = labels;
        let t = self.hover_t(window, hid);
        let weak = cx.weak_entity();
        let key = SharedString::from(hid.to_string());
        let tid = todo_id.to_string();
        div()
            .id(SharedString::from(format!("chip-{hid}")))
            .h_7()
            .px_2()
            .flex()
            .items_center()
            .gap_1p5()
            .rounded_md()
            .text_size(px(13.))
            .text_color(c(FG()))
            .bg(fg_mix(0.06 * t))
            .when(!label.is_empty(), |el| el.child(label.to_string()))
            .on_hover({
                let weak = weak.clone();
                move |hovered, _, cx| {
                    let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                }
            })
            .on_click(move |ev: &ClickEvent, _, cx| {
                let pos = ev.position();
                let tid = tid.clone();
                let _ = weak.update(cx, |this, _| {
                    this.dropdown = Some(DropState {
                        kind,
                        x: pos.x.into(),
                        y: pos.y.into(),
                        todo_id: Some(tid),
                    });
                });
            })
            .a11y_button(name)
    }

    pub(crate) fn recur_chip(
        &mut self,
        hid: &str,
        todo_id: &str,
        label: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let t = self.hover_t(window, hid);
        let weak = cx.weak_entity();
        let key = SharedString::from(hid.to_string());
        let tid = todo_id.to_string();
        let a11y_name = format!("Повторение: {label}");
        div()
            .id(SharedString::from(format!("chip-{hid}")))
            .h_7()
            .px_2()
            .flex()
            .items_center()
            .gap_1p5()
            .rounded_md()
            .text_size(px(13.))
            .text_color(c(FG()))
            .bg(fg_mix(0.06 * t))
            .when(!label.is_empty(), |el| el.child(label.to_string()))
            .on_hover({
                let weak = weak.clone();
                move |hovered, _, cx| {
                    let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                }
            })
            .on_click(move |_: &ClickEvent, _, cx| {
                let tid = tid.clone();
                let _ = weak.update(cx, |this, _| {
                    this.recur_open = !this.recur_open;
                    if this.recur_open {
                        if let Some(t) = this.todos.iter().find(|t| t.id == tid) {
                            if let Some(r) = &t.recurrence {
                                this.recur_freq = r.frequency;
                                this.recur_interval = r.interval;
                                this.recur_type = r.recurrence_type;
                                this.recur_days = r.days_of_week.clone();
                            } else {
                                this.recur_freq = 1;
                                this.recur_interval = 1;
                                this.recur_type = 0;
                                this.recur_days = vec![];
                            }
                        }
                    }
                });
            })
            .a11y_button(a11y_name)
    }

    /// Quick-entry chips row: date pill, billable toggle, project picker.
    pub(crate) fn qe_chips(&mut self, window: &mut Window, cx: &mut Context<Self>) -> gpui::Div {
        let weak = cx.weak_entity();

        // chips row
        let mut chips = div()
            .h(px(52.))
            .flex_none()
            .flex()
            .items_center()
            .gap_2()
            .px_3();

        // date chip
        let date_label = self
            .qe_date
            .as_ref()
            .map(|d| fmt_day_month(d))
            .unwrap_or_else(|| "Срок".to_string());
        let has_date = self.qe_date.is_some();
        let mut date_chip = div()
            .id("qe-date")
            .h_7()
            .pl_2()
            .pr_2()
            .flex()
            .items_center()
            .gap_1p5()
            .rounded_full()
            .text_size(px(12.))
            .when(has_date, |el| {
                el.bg(c(QE_CHIP_BG())).text_color(c(QE_CHIP_FG()))
            })
            .when(!has_date, |el| {
                el.bg(panel_mix(0.10)).text_color(c(MUTED_FG()))
            })
            .child(icon(
                "icons/calendar-02.svg",
                13.,
                if has_date {
                    c(QE_CHIP_FG())
                } else {
                    c(MUTED_FG())
                },
            ))
            .child(date_label.clone());
        if has_date {
            date_chip = date_chip.child(
                div()
                    .id("qe-date-clear")
                    .w_3p5()
                    .h_3p5()
                    .grid()
                    .items_center()
                    .justify_center()
                    .child(icon("icons/status-x.svg", 9., c(QE_CHIP_FG())))
                    .on_click({
                        let weak = weak.clone();
                        move |_: &ClickEvent, _, cx| {
                            let _ = weak.update(cx, |this, _| {
                                this.qe_date = None;
                                this.qe_date_touched = true;
                            });
                        }
                    })
                    .a11y_button("Убрать срок"),
            );
        }
        let qe_date_t = self.hover_t(window, "qe-date");
        let _ = qe_date_t;
        chips = chips.child(
            date_chip
                .on_click({
                    let weak = weak.clone();
                    move |ev: &ClickEvent, _, cx| {
                        let pos = ev.position();
                        let _ = weak.update(cx, |this, _| {
                            this.dropdown = Some(DropState {
                                kind: DropKind::QeDate,
                                x: pos.x.into(),
                                y: pos.y.into(),
                                todo_id: None,
                            });
                        });
                    }
                })
                .a11y_button(format!("Срок: {date_label}")),
        );

        // billable pill (white when on)
        let bill_t = self.hover_t(window, "qe-bill");
        let _ = bill_t;
        chips = chips.child(
            div()
                .id("qe-bill")
                .h_7()
                .px_3()
                .flex()
                .items_center()
                .gap_1p5()
                .rounded_full()
                .text_size(px(12.))
                .when(self.qe_billable, |el| {
                    el.bg(c(ACCENT())).text_color(c(ACCENT_FG()))
                })
                .when(!self.qe_billable, |el| {
                    el.bg(panel_mix(0.10)).text_color(c(MUTED_FG()))
                })
                .child(icon(
                    "icons/dollar.svg",
                    13.,
                    if self.qe_billable {
                        c(ACCENT_FG())
                    } else {
                        c(MUTED_FG())
                    },
                ))
                .child(if self.qe_billable {
                    "Оплачиваемая"
                } else {
                    "Без оплаты"
                })
                .on_click({
                    let weak = weak.clone();
                    move |_: &ClickEvent, _, cx| {
                        let _ = weak.update(cx, |this, _| {
                            this.qe_billable = !this.qe_billable;
                        });
                    }
                })
                .a11y_switch(
                    if self.qe_billable {
                        "Оплачиваемая"
                    } else {
                        "Без оплаты"
                    },
                    self.qe_billable,
                ),
        );

        // project pill (right-aligned, white)
        let proj_label = self
            .qe_project
            .as_ref()
            .and_then(|pid| self.projects.iter().find(|p| p.id == pid.as_str()))
            .map(|p| p.title.clone())
            .unwrap_or_else(|| "Входящие".to_string());
        let proj_name = format!("Проект: {proj_label}");
        chips = chips.child(div().flex_1()).child(
            div()
                .id("qe-proj")
                .h_7()
                .px_3()
                .flex()
                .items_center()
                .gap_1p5()
                .rounded_full()
                .bg(c(SECONDARY()))
                .text_color(c(FG()))
                .text_size(px(12.))
                .child(icon("icons/folder.svg", 13., c(MUTED_FG())))
                .child(proj_label)
                .on_click({
                    let weak = weak.clone();
                    move |ev: &ClickEvent, _, cx| {
                        let pos = ev.position();
                        let _ = weak.update(cx, |this, _| {
                            this.dropdown = Some(DropState {
                                kind: DropKind::QeProject,
                                x: pos.x.into(),
                                y: pos.y.into(),
                                todo_id: None,
                            });
                        });
                    }
                })
                .a11y_button(proj_name),
        );

        chips
    }
}
