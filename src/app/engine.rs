use gpui::Context;

use super::Agenda;
use crate::model::Todo;
use crate::store::{Command, EngineError, ErrorKind, Reply, Snapshot, Worker};

impl Agenda {
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
            let Ok(alive) = this.update(cx, |this, cx| {
                let reply = match this
                    .storage
                    .as_ref()
                    .map(|worker| worker.replies.try_recv())
                {
                    Some(Ok(reply)) => Some(reply),
                    Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                        this.storage = None;
                        this.storage_busy = false;
                        this.storage_ready = false;
                        this.storage_error =
                            Some("Соединение с Engine завершено. Перезапустите приложение.".into());
                        cx.notify();
                        return false;
                    }
                    _ => None,
                };
                if let Some(reply) = reply {
                    this.on_storage_reply(reply, cx);
                }
                true
            }) else {
                break;
            };
            if !alive {
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

    pub(crate) fn send_storage(&mut self, command: Command) -> bool {
        if self
            .storage
            .as_ref()
            .is_some_and(|worker| worker.commands.send(command).is_ok())
        {
            self.storage_busy = true;
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
            self.storage_error.get_or_insert_with(|| {
                format!("Подключитесь к {} и обновите список.", crate::brand::NAME)
            });
            return false;
        }
        true
    }

    fn on_storage_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        self.storage_busy = false;
        match reply {
            Reply::Loaded(Ok(snapshot)) => self.load_snapshot(snapshot),
            Reply::Loaded(Err(error)) => self.fail_load(error),
            Reply::Saved(mutation, Ok(())) => {
                self.accept_todo(mutation.todo);
                if let Some(next) = mutation.next {
                    self.accept_todo(next);
                }
                self.storage_error = None;
            }
            Reply::Saved(_, Err(error)) => self.fail_write(error),
        }
        cx.notify();
    }

    fn load_snapshot(&mut self, snapshot: Snapshot) {
        self.todos = snapshot.todos;
        self.projects = snapshot.projects;
        self.storage_ready = true;
        self.storage_error = None;
    }

    fn fail_load(&mut self, error: EngineError) {
        eprintln!("[storage] engine error: {error}");
        self.storage_error = Some(error.message());
    }

    fn fail_write(&mut self, error: EngineError) {
        eprintln!("[storage] engine error: {error}");
        self.storage_error = Some(error.message());
        if error.kind == ErrorKind::Conflict {
            self.reload_storage();
        }
    }

    pub(super) fn accept_todo(&mut self, todo: Todo) {
        if let Some(stored) = self.todos.iter_mut().find(|stored| stored.id == todo.id) {
            *stored = todo;
        } else {
            self.todos.push(todo);
        }
    }
}
