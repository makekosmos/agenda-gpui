use super::*;

impl Agenda {
    pub(crate) fn project_page(
        &mut self,
        id: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let pid = self
            .projects
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.id.as_str());
        let today = today_key();
        let items: Vec<usize> = self
            .todos
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                t.project_id.as_deref() == pid && !t.is_trashed && !is_archived(t, &today)
            })
            .map(|(i, _)| i)
            .collect();
        let storage = format!("agenda.project.{id}.view");
        let opts = self.board_opts(&storage);
        let items = sort_idx(&self.todos, items, opts.sort);
        let rows: Vec<FlatRow> = items.into_iter().map(FlatRow::Task).collect();
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(if rows.is_empty() {
                empty_state().into_any_element()
            } else {
                self.task_list(
                    "list-project".to_string(),
                    "project",
                    std::rc::Rc::new(rows),
                    true,
                    cx,
                )
            })
            .into_any_element()
    }
}
