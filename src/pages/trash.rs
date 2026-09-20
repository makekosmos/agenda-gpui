use super::*;

impl Agenda {
    pub(crate) fn trash_page(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let rows: Vec<FlatRow> = filter_idx(SmartList::Trash, &self.todos)
            .into_iter()
            .map(FlatRow::Task)
            .collect();
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(if rows.is_empty() {
                empty_state().into_any_element()
            } else {
                self.task_list(
                    "list-trash".to_string(),
                    "trash",
                    std::rc::Rc::new(rows),
                    false,
                    cx,
                )
            })
            .into_any_element()
    }
}
