use super::*;

impl Agenda {
    pub(crate) fn project_page(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let pid: Option<&'static str> = self.projects.iter().find(|p| p.id == id).map(|p| p.id);
        let items: Vec<Todo> = self
            .todos
            .iter()
            .filter(|t| t.project_id == pid && !t.is_trashed && !is_archived(t, &today_key()))
            .cloned()
            .collect();
        let storage = format!("agenda.project.{id}.view");
        let opts = self.board_opts(&storage);
        let items = sorted(&items, opts.sort);
        let mut list = div()
            .id("list-project")
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .track_scroll(&self.scroll("project"));
        if items.is_empty() {
            list = list.child(empty_state());
        }
        for t in &items {
            list = list.child(self.task_row(t, true, window, cx));
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
