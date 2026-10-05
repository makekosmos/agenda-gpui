//! Project page heading, rename and delete.
use super::*;
use crate::store::Command;

impl Agenda {
    /// The project page heading — one editor reused across projects and
    /// reseeded when the open project changes.
    pub(crate) fn project_title_state(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<TextareaState> {
        let state = match &self.project_title {
            Some(state) => state.clone(),
            None => {
                let state = cx.new(|cx| {
                    TextareaState::new(window, cx)
                        .placeholder("Название проекта")
                        .auto_grow(1, 4)
                        .submit_on_enter(true)
                });
                self._subs.push(cx.subscribe_in(
                    &state,
                    window,
                    |this, _, ev: &gpui_component::input::InputEvent, window, cx| {
                        this.on_project_title_event(ev, window, cx);
                    },
                ));
                self.project_title = Some(state.clone());
                state
            }
        };
        // Reseed on project switch, and whenever the stored title moved on
        // while the heading is not being edited (reload, external rename).
        let stored = self
            .project(id)
            .map(|p| p.title.clone())
            .unwrap_or_default();
        let editing = gpui::Focusable::focus_handle(state.read(cx), cx).is_focused(window);
        if self.project_title_for.as_deref() != Some(id)
            || (!editing && state.read(cx).value().as_ref() != stored.as_str())
        {
            self.project_title_for = Some(id.to_string());
            state.update(cx, |s, cx| s.set_value(stored, window, cx));
        }
        if self.project_title_focus {
            self.project_title_focus = false;
            state.update(cx, |s, cx| {
                s.focus(window, cx);
                s.select_all(window, cx);
            });
        }
        state
    }

    fn on_project_title_event(
        &mut self,
        ev: &gpui_component::input::InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use gpui_component::input::InputEvent;
        match ev {
            InputEvent::PressEnter { .. } => {
                self.commit_project_title(window, cx);
                self.root_focus.focus(window, cx);
            }
            InputEvent::Blur => self.commit_project_title(window, cx),
            _ => {}
        }
    }

    /// Saves the heading as the project name; an emptied heading falls back
    /// to the stored name instead of leaving a nameless project.
    fn commit_project_title(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(state), Some(id)) = (self.project_title.clone(), self.project_title_for.clone())
        else {
            return;
        };
        let Some(stored) = self.project(&id).map(|p| p.title.clone()) else {
            return;
        };
        let title = state.read(cx).value().trim().to_string();
        if title.is_empty() || title == stored || !self.rename_project(&id, &title) {
            state.update(cx, |s, cx| s.set_value(stored, window, cx));
        }
        cx.notify();
    }

    pub(crate) fn rename_project(&mut self, id: &str, title: &str) -> bool {
        // Not gated on `storage_busy`: commands queue in order on the worker,
        // and refusing would throw the typed name away (e.g. right after
        // creating the project).
        if !self.demo && !self.storage_ready {
            return false;
        }
        let Some(project) = self.projects.iter_mut().find(|p| p.id == id) else {
            return false;
        };
        project.title = title.to_string();
        let project = project.clone();
        if !self.demo {
            self.send_storage(Command::Project(project));
        }
        self.model_rev += 1;
        true
    }

    /// Opens the project with its heading focused and selected.
    pub(crate) fn edit_project_title(&mut self, id: String) {
        self.project_title_focus = true;
        self.navigate(Route::Project(id));
    }

    /// Deletes the project; its tasks go back to Inbox rather than vanish.
    pub(crate) fn delete_project(&mut self, id: &str) {
        if !self.can_save() {
            return;
        }
        let Some(project) = self.project(id).cloned() else {
            return;
        };
        let todos: Vec<(Todo, Todo)> = self
            .todos
            .iter()
            .filter(|t| t.project_id.as_deref() == Some(id))
            .map(|before| {
                let mut todo = before.clone();
                todo.project_id = None;
                if todo.status == Status::Todo && todo.scheduled_date.is_none() && !todo.is_today {
                    todo.status = Status::Inbox;
                }
                (before.clone(), todo)
            })
            .collect();
        if matches!(&self.route, Route::Project(r) if r == id) {
            self.navigate(Route::Inbox);
        }
        if self.demo {
            self.projects.retain(|p| p.id != id);
            for (_, todo) in todos {
                self.accept_todo(todo);
            }
            self.model_rev += 1;
            return;
        }
        self.send_storage(Command::DeleteProject { project, todos });
    }
}
