use super::*;
use gpui::{App, Div, Entity};
use gpui_component::input::TextareaState;

impl Agenda {
    pub(crate) fn project_page(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let pid = self
            .projects
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.id.as_str());
        let today = today_key();
        // `pid == None` (a stale/unknown route id) must match nothing —
        // comparing against `project_id == None` listed every unprojected
        // task under the ghost page.
        let items: Vec<usize> = self
            .todos
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                pid.is_some_and(|pid| t.project_id.as_deref() == Some(pid))
                    && !t.is_trashed
                    && !is_archived(t, &today)
            })
            .map(|(i, _)| i)
            .collect();
        let storage = format!("agenda.project.{id}.view");
        let opts = self.board_opts(&storage);
        let items = sort_idx(&self.todos, items, opts.sort);
        let rows: Vec<FlatRow> = items.into_iter().map(FlatRow::Task).collect();
        let heading = pid.is_some().then(|| {
            let state = self.project_title_state(id, window, cx);
            project_heading(&state, cx)
        });
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .children(heading)
            .child(if rows.is_empty() {
                empty_state(
                    "icons/folder-open.svg",
                    "Проект пуст",
                    "Добавь задачу в этот проект",
                )
                .into_any_element()
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

/// Editable page heading — same type scale as the task panel title.
fn project_heading(state: &Entity<TextareaState>, cx: &App) -> Div {
    div().px_4().pt_4().pb_2().flex_none().child(
        crate::text_field::text_field("project-title", "Название проекта", state.clone(), cx)
            .w_full()
            .flex()
            .flex_col()
            .child(
                Textarea::new(state)
                    .role(None)
                    .w_full()
                    .flex_none()
                    .text_size(crate::theme::text_px(24.))
                    .line_height(crate::theme::text_px(32.))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(c(FG()))
                    .p_0()
                    .appearance(false)
                    .bordered(false),
            ),
    )
}
