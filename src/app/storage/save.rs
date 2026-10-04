use super::*;
use crate::store::Mutation;

impl Agenda {
    pub(crate) fn prepare_close(&mut self, cx: &mut Context<Self>) -> bool {
        if self.storage_busy {
            // A round-trip is already in flight — queue the close so the
            // reply handler can finish it instead of dropping the click.
            self.close_pending = true;
            return false;
        }
        let Some(id) = self
            .current_task_id()
            .filter(|id| self.input_task.as_ref() == Some(id))
        else {
            return true;
        };
        let title = self
            .inputs
            .get("task-title")
            .map(|s| s.read(cx).value().trim().to_string());
        let notes = self
            .notes_input
            .as_ref()
            .map(|s| s.read(cx).value().trim().to_string());
        let Some(before) = self.todo(&id).cloned() else {
            return true;
        };
        let mut todo = before.clone();
        if let Some(title) = title.filter(|s| !s.is_empty()) {
            todo.title = title;
        }
        if let Some(notes) = notes {
            todo.notes = (!notes.is_empty()).then_some(notes);
        }
        if todo == before {
            return true;
        }
        let flushed = self.save_todo(Some(before), todo);
        cx.notify();
        if flushed && !self.demo {
            // The write is in flight — veto this close and let the Saved(Ok)
            // reply re-trigger it (see on_storage_reply). When Engine is
            // unreachable there is nothing to wait for: close anyway rather
            // than wedging the window on an edited field.
            self.close_pending = true;
            return false;
        }
        true
    }

    pub(crate) fn save_todo(&mut self, before: Option<Todo>, todo: Todo) -> bool {
        if !self.can_save() {
            return false;
        }
        if before.as_ref() == Some(&todo) {
            return true;
        }
        if let Some(before) = before.as_ref().filter(|t| t.system_kind.is_some()) {
            let mut allowed = before.clone();
            allowed.significance = todo.significance;
            if allowed != todo {
                self.raise(StorageError::notice(
                    StorageFault::Write,
                    "Эта служебная задача управляется Agenda автоматически.",
                ));
                return false;
            }
        }
        if !self.demo {
            return self.send_storage(Command::Save(Box::new(Mutation {
                before,
                todo,
                next: None,
            })));
        }
        self.accept_todo(todo);
        true
    }

    pub(super) fn accept_todo(&mut self, todo: Todo) {
        if let Some(stored) = self.todos.iter_mut().find(|t| t.id == todo.id) {
            *stored = todo;
        } else {
            self.todos.push(todo);
        }
        self.model_rev += 1;
    }

    pub(crate) fn save_completion(&mut self, before: Todo, todo: Todo, next: Option<Todo>) {
        if before.system_kind.is_some() {
            return;
        }
        if !self.can_save() {
            return;
        }
        if !self.demo {
            self.send_storage(Command::Save(Box::new(Mutation {
                before: Some(before),
                todo,
                next,
            })));
        } else {
            self.accept_todo(todo);
            if let Some(next) = next {
                self.accept_todo(next);
            }
        }
    }

    pub(crate) fn set_project_status(&mut self, id: &str, status: u8) {
        if !self.can_save() {
            return;
        }
        let Some(mut project) = self.projects.iter().find(|p| p.id == id).cloned() else {
            return;
        };
        project.status = status;
        if self.demo {
            if let Some(stored) = self.projects.iter_mut().find(|p| p.id == id) {
                *stored = project;
            }
        } else {
            self.send_storage(Command::Project(project));
        }
        self.model_rev += 1;
    }
}
