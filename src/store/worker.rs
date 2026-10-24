//! The background thread that owns the Engine client. The UI sends a `Command`
//! and reads a `Reply`; no Engine call runs on the UI thread.
use super::{Engine, EngineError};
use crate::model::{Project, Todo};
use std::sync::mpsc::{self, Receiver, Sender};

#[derive(Default)]
pub struct Snapshot {
    pub todos: Vec<Todo>,
    pub projects: Vec<Project>,
}

pub struct Mutation {
    pub before: Option<Todo>,
    pub todo: Todo,
    pub next: Option<Todo>,
}

pub enum Command {
    Load,
    Save(Box<Mutation>),
}

pub enum Reply {
    Loaded(Result<Snapshot, EngineError>),
    Saved(Box<Mutation>, Result<(), EngineError>),
}

pub struct Worker {
    pub commands: Sender<Command>,
    pub replies: Receiver<Reply>,
}

impl Worker {
    pub fn start() -> Self {
        let (commands, requests) = mpsc::channel();
        let (results, replies) = mpsc::channel();
        std::thread::spawn(move || {
            let engine = Engine::default();
            for request in requests {
                let reply = match request {
                    Command::Load => Reply::Loaded(engine.load()),
                    Command::Save(mutation) => {
                        let result = engine.save_with_next(
                            mutation.before.as_ref(),
                            &mutation.todo,
                            mutation.next.as_ref(),
                        );
                        Reply::Saved(mutation, result)
                    }
                };
                if results.send(reply).is_err() {
                    break;
                }
            }
        });
        Self { commands, replies }
    }
}
