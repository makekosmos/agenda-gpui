use super::{Agenda, Route};
use crate::model::{new_todo, next_monday_key, today_key, Status, Todo};
use crate::store::{Command, Mutation};

impl Agenda {
    pub(crate) fn toggle_sidebar(&mut self) {
        self.sidebar_visible = !self.sidebar_visible;
    }

    pub(crate) fn complete_todo(&mut self, id: &str) {
        let Some(before) = self.todos.iter().find(|todo| todo.id == id).cloned() else {
            return;
        };
        if before.is_completed || before.is_trashed {
            return;
        }
        let mut todo = before.clone();
        todo.status = Status::Done;
        todo.is_completed = true;
        todo.completed_at = Some(chrono::Utc::now().to_rfc3339());
        self.save_todo(Some(before), todo);
    }

    pub(crate) fn create_nsh_todo(&mut self, title: &str) {
        let title = title.trim();
        if title.is_empty() {
            return;
        }
        let mut todo = new_todo(uuid::Uuid::new_v4().to_string(), title);
        todo.sort_order = self.todos.len() as i32;
        todo.created_at = chrono::Utc::now().to_rfc3339();
        match &self.route {
            Route::Week => {
                todo.status = Status::Todo;
                todo.is_today = true;
            }
            Route::NextWeek => {
                todo.status = Status::Todo;
                todo.scheduled_date = next_monday_key(&today_key());
            }
            Route::Someday => {
                todo.status = Status::Todo;
                todo.is_someday = true;
            }
            Route::Project(id) => {
                todo.status = Status::Todo;
                todo.project_id = Some(id.clone());
            }
            _ => {}
        }
        self.save_todo(None, todo);
    }

    pub(super) fn save_todo(&mut self, before: Option<Todo>, todo: Todo) {
        if !self.can_save() {
            return;
        }
        if self.demo {
            self.accept_todo(todo);
            return;
        }
        self.send_storage(Command::Save(Box::new(Mutation {
            before,
            todo,
            next: None,
        })));
    }
}
