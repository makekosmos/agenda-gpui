use super::*;

impl Agenda {
    // ==========================================================================
    // Task page (Linear-like): bar, title input, prop chips, notes.
    // ==========================================================================

    pub(crate) fn task_page(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        // Reset per-task inputs when navigating between tasks.
        if self.input_task.as_deref() != Some(id) {
            self.input_task = Some(id.to_string());
            self.inputs.remove("task-title");
            self.notes_input = None;
        }
        let todo = self.todos.iter().find(|t| t.id == id).cloned();
        let Some(t) = todo else {
            return div().flex_1().child(empty_state()).into_any_element();
        };

        let title_state = self.input_state(window, cx, "task-title", "Название задачи");
        if title_state.read(cx).value().is_empty() && !t.title.is_empty() {
            let tv = t.title.clone();
            title_state.update(cx, |s, cx| s.set_value(tv, window, cx));
        }
        let notes_state = self.notes_state(window, cx, "Добавить описание...");
        if notes_state.read(cx).value().is_empty() {
            if let Some(n) = &t.notes {
                let nv = n.clone();
                notes_state.update(cx, |s, cx| s.set_value(nv, window, cx));
            }
        }

        // ---- bar: more button
        let bar = div()
            .flex()
            .items_center()
            .justify_end()
            .gap_1()
            .min_h_7()
            .child({
                let hid = "tp-more";
                let t2 = self.hover_t(window, hid);
                let weak = cx.weak_entity();
                let id2 = id.to_string();
                div()
                    .id("tp-more-btn")
                    .w_7()
                    .h_7()
                    .grid()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .bg(fg_mix(0.06 * t2))
                    .child(icon(
                        "icons/more-h.svg",
                        16.,
                        mix(MUTED_FG(), 0.6 + 0.4 * t2, BG()),
                    ))
                    .on_hover({
                        let weak = weak.clone();
                        move |hovered, _, cx| {
                            let _ = weak.update(cx, |this, _| this.set_hover(hid, *hovered));
                        }
                    })
                    .on_click(move |ev: &ClickEvent, _, cx| {
                        let pos = ev.position();
                        let id2 = id2.clone();
                        let _ = weak.update(cx, |this, _| {
                            this.menu = Some(CtxMenu {
                                x: pos.x.into(),
                                y: pos.y.into(),
                                items: vec![
                                    (
                                        "Вернуть в работу".into(),
                                        false,
                                        MenuAction::CompleteTodo(id2.clone()),
                                    ),
                                    ("Удалить".into(), true, MenuAction::TrashTodo(id2)),
                                ],
                            });
                        });
                    })
            });

        let column = div()
            .w_full()
            .max_w(px(720.))
            .mx_auto()
            .pt_5()
            .pb_16()
            .px_8()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(bar)
            .child(
                Input::new(&title_state)
                    .text_size(px(22.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c(FG()))
                    .p_0()
                    .appearance(false)
                    .bordered(false)
                    .h(px(30.)),
            )
            .children(self.task_props(&t, id, window, cx))
            .child(
                Textarea::new(&notes_state)
                    .text_size(px(13.))
                    .text_color(c(FG()))
                    .p_0()
                    .appearance(false)
                    .bordered(false)
                    .h(px(120.)),
            );

        div()
            .id("task-page")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll("task"))
            .flex()
            .flex_col()
            .child(column)
            .into_any_element()
    }
}
