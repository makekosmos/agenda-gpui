use super::*;
use crate::store::{Command, Mutation, Reply, Worker};
use crate::widgets::A11y;

impl Agenda {
    pub(crate) fn prepare_close(&mut self, cx: &mut Context<Self>) -> bool {
        if self.storage_busy {
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
        self.save_todo(Some(before), todo);
        cx.notify();
        self.demo
    }

    pub(crate) fn start_storage(&mut self, cx: &mut Context<Self>) {
        if self.demo {
            return;
        }
        self.storage = Some(Worker::start());
        self.reload_storage();
        cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(100))
                .await;
            if this
                .update(cx, |this, cx| {
                    let received = this.storage.as_ref().map(|s| s.replies.try_recv());
                    let reply = match received {
                        Some(Ok(reply)) => Some(reply),
                        Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                            this.storage = None;
                            this.storage_busy = false;
                            this.storage_ready = false;
                            this.storage_error = Some(
                                "Соединение с Engine завершено. Перезапустите приложение.".into(),
                            );
                            cx.notify();
                            None
                        }
                        _ => None,
                    };
                    if let Some(reply) = reply {
                        this.storage_busy = false;
                        match reply {
                            Reply::Loaded(Ok(data)) => {
                                this.todos = data.todos;
                                this.projects = data.projects;
                                this.areas = data.areas;
                                this.tags = data.tags;
                                this.headings = data.headings;
                                this.storage_ready = true;
                                this.storage_error = None;
                                this.input_task = None;
                            }
                            Reply::Loaded(Err(error)) => this.storage_error = Some(error),
                            Reply::Saved(mutation, result) => {
                                if let Err(error) = result {
                                    this.storage_error = Some(error);
                                    this.storage_ready = false;
                                } else {
                                    this.accept_todo(mutation.todo);
                                    if let Some(next) = mutation.next {
                                        this.accept_todo(next);
                                    }
                                }
                            }
                            Reply::Project(result) => match result {
                                Ok(project) => {
                                    if let Some(p) =
                                        this.projects.iter_mut().find(|p| p.id == project.id)
                                    {
                                        *p = project;
                                    }
                                }
                                Err(error) => {
                                    this.storage_error = Some(error);
                                    this.storage_ready = false;
                                }
                            },
                        }
                        this.model_rev += 1;
                        cx.notify();
                    }
                })
                .is_err()
            {
                break;
            }
        })
        .detach();
    }

    pub(crate) fn reload_storage(&mut self) {
        if self.storage_busy {
            return;
        }
        self.storage_ready = false;
        self.storage_error = None;
        self.send_storage(Command::Load);
    }

    fn send_storage(&mut self, command: Command) -> bool {
        if self
            .storage
            .as_ref()
            .is_some_and(|s| s.commands.send(command).is_ok())
        {
            self.storage_busy = true;
            self.storage_error = None;
            true
        } else {
            self.storage_error =
                Some("Соединение с Engine завершено. Перезапустите приложение.".into());
            self.storage_ready = false;
            false
        }
    }

    pub(crate) fn can_save(&mut self) -> bool {
        if self.demo {
            return true;
        }
        if self.storage_busy {
            return false;
        }
        if !self.storage_ready {
            self.storage_error
                .get_or_insert_with(|| "Подключитесь к Engine и обновите список.".into());
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
                self.storage_error =
                    Some("Эта служебная задача управляется Agenda автоматически.".into());
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

    fn accept_todo(&mut self, todo: Todo) {
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

    pub(crate) fn storage_banner(&self, cx: &mut Context<Self>) -> AnyElement {
        let text = if self.storage_busy {
            "Сохранение / загрузка…".to_string()
        } else {
            self.storage_error.clone().unwrap_or_default()
        };
        div()
            .absolute()
            .bottom_3()
            .left_3()
            .right_3()
            .p_3()
            .rounded_md()
            .bg(c(BG()))
            .text_color(c(FG()))
            .text_sm()
            .flex()
            .gap_3()
            .child(text)
            .when(!self.storage_busy, |d| {
                d.child(
                    div()
                        .id("reload-engine")
                        .cursor_pointer()
                        .child("Обновить")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.reload_storage();
                            cx.notify();
                        }))
                        .a11y_button("Обновить"),
                )
            })
            .into_any_element()
    }
}
