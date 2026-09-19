use super::*;

impl Agenda {
    // ==========================================================================
    // Recurring / Settings / About
    // ==========================================================================

    pub(crate) fn recurring_page(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let items: Vec<Todo> = self
            .todos
            .iter()
            .filter(|t| t.recurrence.is_some() && !t.is_trashed && !is_archived(t, &today_key()))
            .cloned()
            .collect();
        let mut list = div()
            .id("list-recurring")
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap_3()
            .px_7()
            .pt_4()
            .overflow_y_scroll()
            .track_scroll(&self.scroll("recurring"));
        if items.is_empty() {
            list = list.child(empty_state());
        }
        for t in &items {
            let hid = format!("rec-{}", t.id);
            let ht = self.hover_t(window, &hid);
            let weak = cx.weak_entity();
            let tid = t.id.to_string();
            let key = SharedString::from(hid.clone());
            list = list.child(
                div()
                    .id(SharedString::from(format!("el-{hid}")))
                    .rounded_lg()
                    .border_1()
                    .border_color(lerp(BORDER(), ACCENT(), ht * 0.4))
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .bg(fg_mix(0.02 * ht))
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(c(FG()))
                            .child(t.title.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(c(MUTED_FG()))
                            .child(describe_recurrence(&t.recurrence)),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(c(MUTED_FG()))
                            .child(format!(
                                "Следующий срок: {}",
                                task_date(t).0.unwrap_or_default()
                            )),
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
        list.into_any_element()
    }
}
