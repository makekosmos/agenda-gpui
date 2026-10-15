use super::*;

pub(super) struct Property {
    pub(super) id: &'static str,
    pub(super) icon_path: &'static str,
    pub(super) label: &'static str,
    pub(super) value: String,
    pub(super) has_value: bool,
    pub(super) kind: DropKind,
    pub(super) clearable: bool,
    pub(super) todo_id: Option<String>,
}

impl Agenda {
    /// Date property label: someday, a picked day, or the muted "Пусто".
    pub(super) fn qe_date_label(&self) -> String {
        if self.qe_week {
            "Эта неделя".to_string()
        } else if self.qe_someday {
            "Потом".to_string()
        } else {
            let next_mon = next_monday_key(&today_key());
            self.qe_date
                .as_ref()
                .map(|d| {
                    if Some(d) == next_mon.as_ref() {
                        "Следующая неделя".to_string()
                    } else {
                        fmt_day_month(d)
                    }
                })
                .unwrap_or_else(|| "Пусто".to_string())
        }
    }

    /// Notion-style property row on a two-column grid: the label cell (icon +
    /// name, fixed 112px column) and the value cell are separate hover/click
    /// zones — hovering the property name highlights only the name, hovering
    /// the result highlights only the value.
    pub(super) fn qe_prop_row(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        property: Property,
    ) -> gpui::Stateful<gpui::Div> {
        let Property {
            id,
            icon_path,
            label,
            value,
            has_value,
            kind,
            clearable,
            todo_id,
        } = property;
        let value_id = format!("{id}-value");
        let t_value = self.hover_t(window, &value_id);
        let weak = cx.weak_entity();
        let weak_v = weak.clone();
        let open_v = move |ev: &ClickEvent, _: &mut Window, cx: &mut gpui::App| {
            let pos = ev.position();
            let _ = weak_v.update(cx, |this, _| {
                this.dropdown = Some(DropState {
                    kind,
                    x: pos.x.into(),
                    y: pos.y.into(),
                    todo_id: todo_id.clone(),
                });
            });
        };
        let value_str = value.clone();
        div()
            .id(id)
            .h(px(30.))
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .w(px(112.))
                    .flex_none()
                    .h_full()
                    .pl_1p5()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .child(icon(icon_path, 14., c(MUTED_FG())))
                    .child(
                        div()
                            .text_size(crate::theme::text_px(13.))
                            .text_color(c(MUTED_FG()))
                            .child(label),
                    ),
            )
            .child(
                div()
                    .id(SharedString::from(value_id.clone()))
                    .debug_selector(|| id.to_string())
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .px_2()
                    .rounded(px(5.))
                    .bg(rgba(FG(), 0.04 + 0.05 * t_value))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .text_size(crate::theme::text_px(13.))
                    .text_color(if has_value { c(FG()) } else { c(MUTED_FG()) })
                    .child(value_str)
                    .child(div().flex_1())
                    .when(clearable && has_value, |el| {
                        el.child(
                            div()
                                .id("qe-date-clear")
                                .debug_selector(|| "qe-date-clear".to_string())
                                .w_3p5()
                                .h_3p5()
                                .grid()
                                .items_center()
                                .justify_center()
                                .child(icon("icons/status-x.svg", 9., c(MUTED_FG())))
                                .on_click({
                                    let weak = weak.clone();
                                    move |_: &ClickEvent, _, cx| {
                                        // The cell's own on_click opens the
                                        // picker — clearing must not also
                                        // open it.
                                        cx.stop_propagation();
                                        let _ = weak.update(cx, |this, _| {
                                            this.qe_date = None;
                                            this.qe_someday = false;
                                            this.qe_week = false;
                                            this.qe_date_touched = true;
                                        });
                                    }
                                })
                                .a11y_button("Убрать срок"),
                        )
                    })
                    .on_hover({
                        let weak = weak.clone();
                        move |hovered, _, cx| {
                            let _ = weak.update(cx, |this, _| this.set_hover(&value_id, *hovered));
                        }
                    })
                    .on_click(open_v),
            )
            .a11y_button(format!("{label}: {value}", value = value))
    }
}
