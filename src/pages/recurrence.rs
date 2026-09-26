use super::*;

impl Agenda {
    /// Inline recurrence editor (dropdown + weekday toggles + apply/clear).
    pub(crate) fn render_recur_editor(
        &mut self,
        t: &Todo,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let freq_label = ["День", "Неделя", "Месяц", "Год"][self.recur_freq as usize];
        let type_label = if self.recur_type == 1 {
            "после выполнения"
        } else {
            "по расписанию"
        };
        let day_names = ["пн", "вт", "ср", "чт", "пт", "сб", "вс"];
        let tid = t.id.to_string();

        let chip_btn = |app: &mut Self,
                        window: &mut Window,
                        cx: &mut Context<Self>,
                        hid: &str,
                        label: String,
                        kind: DropKind| {
            let t = app.hover_t(window, hid);
            let weak = cx.weak_entity();
            let key = SharedString::from(hid.to_string());
            div()
                .id(SharedString::from(format!("rc-{hid}")))
                .h_7()
                .px_2()
                .flex()
                .items_center()
                .rounded_md()
                .text_size(px(12.))
                .text_color(c(FG()))
                .bg(fg_mix(0.05 + 0.02 * t))
                .border_1()
                .border_color(c(BORDER()))
                .child(label.clone())
                .on_hover({
                    let weak = weak.clone();
                    move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                    }
                })
                .on_click(move |ev: &ClickEvent, _, cx| {
                    let pos = ev.position();
                    let _ = weak.update(cx, |this, _| {
                        this.dropdown = Some(DropState {
                            kind,
                            x: pos.x.into(),
                            y: pos.y.into(),
                            todo_id: None,
                        });
                    });
                })
                .a11y_button(label)
        };

        let mut row = div().flex().items_center().gap_2().flex_wrap();
        row = row.child(chip_btn(
            self,
            window,
            cx,
            "rc-freq",
            freq_label.to_string(),
            DropKind::RecurFreq,
        ));
        row = row.child(chip_btn(
            self,
            window,
            cx,
            "rc-type",
            type_label.to_string(),
            DropKind::RecurType,
        ));
        if self.recur_freq == 1 {
            for (i, name) in day_names.iter().enumerate() {
                let d = (i + 1) as u8;
                let sel = self.recur_days.contains(&d);
                let hid = format!("rc-day-{i}");
                let ht = self.hover_t(window, &hid);
                let weak = cx.weak_entity();
                let key = SharedString::from(hid.clone());
                row = row.child(
                    div()
                        .id(SharedString::from(format!("el-{hid}")))
                        .h_7()
                        .w_8()
                        .grid()
                        .items_center()
                        .justify_center()
                        .rounded_md()
                        .text_size(px(12.))
                        .text_color(if sel { c(ACCENT_FG()) } else { c(FG()) })
                        .bg(if sel {
                            c(ACCENT())
                        } else {
                            fg_mix(0.04 + 0.03 * ht)
                        })
                        .child(*name)
                        .on_hover(move |hovered, _, cx| {
                            let _ = weak.update(cx, |this, _| this.set_hover(&key, *hovered));
                        })
                        .on_click({
                            let weak = cx.weak_entity();
                            move |_: &ClickEvent, _, cx| {
                                let _ = weak.update(cx, |this, _| {
                                    if this.recur_days.contains(&d) {
                                        this.recur_days.retain(|x| *x != d);
                                    } else {
                                        this.recur_days.push(d);
                                        this.recur_days.sort();
                                    }
                                });
                            }
                        })
                        .a11y_checkbox(*name, sel),
                );
            }
        }
        // apply + clear
        let apply = {
            let hid = "rc-apply";
            let t2 = self.hover_t(window, hid);
            let weak = cx.weak_entity();
            let tid2 = tid.clone();
            div()
                .id("rc-apply-btn")
                .h_7()
                .px_3()
                .flex()
                .items_center()
                .rounded_md()
                .text_size(px(12.))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(c(ACCENT_FG()))
                .bg(lerp(ACCENT(), ACCENT_DIM(), t2 * 0.5))
                .child("Применить")
                .on_hover({
                    let weak = weak.clone();
                    move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| this.set_hover(hid, *hovered));
                    }
                })
                .on_click(move |_: &ClickEvent, _, cx| {
                    let tid2 = tid2.clone();
                    let _ = weak.update(cx, |this, _| {
                        let rule = RecurrenceRule {
                            frequency: this.recur_freq,
                            interval: this.recur_interval.max(1),
                            recurrence_type: this.recur_type,
                            days_of_week: this.recur_days.clone(),
                        };
                        this.update_todo(&tid2, |t| t.recurrence = Some(rule));
                        this.recur_open = false;
                    });
                })
                .a11y_button("Применить")
        };
        let clear = {
            let hid = "rc-clear";
            let t2 = self.hover_t(window, hid);
            let weak = cx.weak_entity();
            let tid2 = tid.clone();
            div()
                .id("rc-clear-btn")
                .h_7()
                .px_3()
                .flex()
                .items_center()
                .rounded_md()
                .text_size(px(12.))
                .text_color(mix(FG(), 0.6 + 0.4 * t2, BG()))
                .bg(fg_mix(0.06 * t2))
                .child("Убрать")
                .on_hover({
                    let weak = weak.clone();
                    move |hovered, _, cx| {
                        let _ = weak.update(cx, |this, _| this.set_hover(hid, *hovered));
                    }
                })
                .on_click(move |_: &ClickEvent, _, cx| {
                    let tid2 = tid2.clone();
                    let _ = weak.update(cx, |this, _| {
                        this.update_todo(&tid2, |t| t.recurrence = None);
                        this.recur_open = false;
                    });
                })
                .a11y_button("Убрать")
        };
        row = row.child(apply).child(clear);

        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .rounded_lg()
            .border_1()
            .border_color(c(BORDER()))
            .bg(c(SECONDARY()))
            .child(row)
    }
}
