mod banner;
mod save;

use super::*;
use crate::store::{Command, EngineError, ErrorKind, Reply, Worker};

/// How long a write-error toast stays on screen. A repeated error restarts
/// the timer (see `set_storage_error`); × dismisses immediately.
const TOAST_TTL: std::time::Duration = std::time::Duration::from_secs(7);

/// Which surface a `StorageError` belongs to: load failures block until data
/// arrives and offer «Обновить»; write failures are transient toasts.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum StorageFault {
    Load,
    Write,
}

pub(crate) struct StorageError {
    pub fault: StorageFault,
    /// User-facing Russian text, derived from the Engine error class.
    pub text: String,
}

impl StorageError {
    /// A failure reported by Engine: text comes from the error class, the raw
    /// code stays in the log (see `fail_write` / the `Loaded(Err)` arm).
    fn engine(fault: StorageFault, error: &EngineError) -> Self {
        Self {
            fault,
            text: error.message(),
        }
    }

    pub(crate) fn notice(fault: StorageFault, text: impl Into<String>) -> Self {
        Self {
            fault,
            text: text.into(),
        }
    }
}

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
            if this
                .update(cx, |this, cx| {
                    let received = this.storage.as_ref().map(|s| s.replies.try_recv());
                    let reply = match received {
                        Some(Ok(reply)) => Some(reply),
                        Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                            this.storage = None;
                            this.storage_busy = false;
                            this.storage_ready = false;
                            this.raise_storage_error(
                                StorageError::notice(
                                    StorageFault::Load,
                                    "Соединение с Engine завершено. Перезапустите приложение.",
                                ),
                                cx,
                            );
                            cx.notify();
                            None
                        }
                        _ => None,
                    };
                    if let Some(reply) = reply {
                        this.on_storage_reply(reply, cx);
                    }
                })
                .is_err()
            {
                break;
            }
        })
        .detach();
    }

    /// One Engine reply from the worker thread. A failed *write* never drops
    /// `storage_ready` — the toast is informational and the next save may
    /// succeed; only a failed *load* (or a dead worker) keeps writes gated.
    pub(crate) fn on_storage_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        self.storage_busy = false;
        let succeeded = matches!(
            &reply,
            Reply::Loaded(Ok(_)) | Reply::Saved(_, Ok(())) | Reply::Project(Ok(_))
        );
        match reply {
            Reply::Loaded(Ok(data)) => {
                self.todos = data.todos;
                self.projects = data.projects;
                self.areas = data.areas;
                self.tags = data.tags;
                self.headings = data.headings;
                self.storage_ready = true;
                if self
                    .storage_error
                    .as_ref()
                    .is_some_and(|e| e.fault == StorageFault::Load)
                {
                    self.storage_error = None;
                }
                self.input_task = None;
            }
            Reply::Loaded(Err(error)) => {
                eprintln!("[storage] engine error: {error}");
                self.raise_storage_error(StorageError::engine(StorageFault::Load, &error), cx);
            }
            Reply::Saved(mutation, result) => match result {
                Ok(()) => {
                    self.accept_todo(mutation.todo);
                    if let Some(next) = mutation.next {
                        self.accept_todo(next);
                    }
                    // A confirmed write clears any earlier write toast.
                    if self
                        .storage_error
                        .as_ref()
                        .is_some_and(|e| e.fault == StorageFault::Write)
                    {
                        self.storage_error = None;
                    }
                }
                Err(error) => self.fail_write(error, cx),
            },
            Reply::Project(result) => match result {
                Ok(project) => {
                    if let Some(p) = self.projects.iter_mut().find(|p| p.id == project.id) {
                        *p = project;
                    }
                    if self
                        .storage_error
                        .as_ref()
                        .is_some_and(|e| e.fault == StorageFault::Write)
                    {
                        self.storage_error = None;
                    }
                }
                Err(error) => self.fail_write(error, cx),
            },
        }
        self.model_rev += 1;
        if self.close_pending {
            // A close was vetoed while this round-trip was in flight. It can
            // complete now that the reply landed — unless it failed, in which
            // case the toast explains why the write did not go through and
            // the user decides whether to retry or close anyway.
            self.close_pending = false;
            if succeeded {
                for w in cx.windows() {
                    let _ = w.update(cx, |_, window, _| window.remove_window());
                }
            }
        }
        cx.notify();
    }

    /// A rejected write: toast it (never gate later saves) and, on `conflict`,
    /// resync from Engine — the local model is stale.
    fn fail_write(&mut self, error: EngineError, cx: &mut Context<Self>) {
        eprintln!("[storage] engine error: {error}");
        if error.kind == ErrorKind::Conflict {
            self.reload_storage();
        }
        self.raise_storage_error(StorageError::engine(StorageFault::Write, &error), cx);
    }

    /// Set the error and invalidate any pending auto-hide timer. For paths
    /// without a `Context` — the notice then simply stays until dismissed.
    pub(crate) fn raise(&mut self, error: StorageError) {
        self.toast_seq += 1;
        self.storage_error = Some(error);
    }

    /// Show an error; for write toasts also arm the auto-hide timer.
    /// `toast_seq` invalidates stale timers: a repeated error restarts the
    /// countdown and a manual close (or any newer error) must not be cleared
    /// by an older timer.
    fn raise_storage_error(&mut self, error: StorageError, cx: &mut Context<Self>) {
        let timed = error.fault == StorageFault::Write;
        self.raise(error);
        if timed {
            let seq = self.toast_seq;
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(TOAST_TTL).await;
                this.update(cx, |this, cx| {
                    if this.toast_seq == seq {
                        this.storage_error = None;
                        cx.notify();
                    }
                })
                .ok();
            })
            .detach();
        }
    }

    /// Dismiss the toast/banner; bumps the seq so a pending timer can't wipe
    /// a newer error later.
    pub(crate) fn dismiss_storage_error(&mut self) {
        self.toast_seq += 1;
        self.storage_error = None;
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
            true
        } else {
            self.raise(StorageError::notice(
                StorageFault::Load,
                "Соединение с Engine завершено. Перезапустите приложение.",
            ));
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
            if self.storage_error.is_none() {
                self.raise(StorageError::notice(
                    StorageFault::Load,
                    format!("Подключитесь к {} и обновите список.", crate::brand::NAME),
                ));
            }
            return false;
        }
        true
    }
}
