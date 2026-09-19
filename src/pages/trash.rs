use super::*;

impl Agenda {
    pub(crate) fn trash_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let items = filter_todos(SmartList::Trash, &self.todos);
        let mut list = div()
            .id("list-trash")
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .track_scroll(&self.scroll("trash"));
        if items.is_empty() {
            list = list.child(empty_state());
        }
        for t in &items {
            list = list.child(self.task_row(t, false, window, cx));
        }
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(list)
            .into_any_element()
    }
}
