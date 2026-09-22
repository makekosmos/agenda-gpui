//! Engine is the only owner of task persistence. No database or local task mirror.
pub mod mapping;
#[cfg(test)]
mod tests;
mod transport;

use crate::model::{Area, Heading, Project, Tag, Todo};
use serde_json::{json, Value};
use std::sync::mpsc::{self, Receiver, Sender};
pub use transport::Engine;

#[derive(Default)]
pub struct Snapshot {
    pub todos: Vec<Todo>,
    pub projects: Vec<Project>,
    pub areas: Vec<Area>,
    pub tags: Vec<Tag>,
    pub headings: Vec<Heading>,
}

pub struct Mutation {
    pub before: Option<Todo>,
    pub todo: Todo,
    pub next: Option<Todo>,
}

pub enum Command {
    Load,
    Save(Box<Mutation>),
    Project(Project),
}

pub enum Reply {
    Loaded(Result<Snapshot, String>),
    Saved(Box<Mutation>, Result<(), String>),
    Project(Result<Project, String>),
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
                    Command::Project(project) => {
                        Reply::Project(engine.save_project(&project).map(|()| project))
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

impl Engine {
    fn save_with_next(
        &self,
        before: Option<&Todo>,
        todo: &Todo,
        next: Option<&Todo>,
    ) -> Result<(), String> {
        self.save(before, todo)?;
        let Some(next) = next else {
            return Ok(());
        };
        if self.save(None, next).is_ok() {
            return Ok(());
        }
        // A lost acknowledgement may still mean the next occurrence was saved.
        let stored = self.rpc("get_object", json!({"id":next.id}))?;
        if !stored.is_null() && mapping::read(&stored)? == *next {
            return Ok(());
        }
        if stored.is_null() {
            if let Some(before) = before {
                self.save(Some(todo), before)?;
            }
        }
        Err("Не удалось сохранить повторение. Обновите список для проверки состояния.".into())
    }

    pub fn load(&self) -> Result<Snapshot, String> {
        let list = self.rpc(
            "list_objects_by_type",
            json!({"type_id":mapping::TASK_TYPE}),
        )?;
        let list = list.as_array().ok_or("Некорректный список задач Engine")?;
        let mut snapshot = Snapshot::default();
        for object in list {
            if object["typeId"] == mapping::TASK_TYPE {
                snapshot.todos.push(mapping::read(object)?);
            }
        }
        snapshot.todos.sort_by_key(|t| t.sort_order);
        let object = self.references()?;
        if object.is_null() {
            return Ok(snapshot);
        }
        let props = &object["propsJson"];
        if props["model_version"] != 1 {
            return Err("Неизвестная версия справочников Agenda".into());
        }
        snapshot.projects = collection(props, "projects")?;
        snapshot.areas = collection(props, "areas")?;
        snapshot.tags = collection(props, "tags")?;
        snapshot.headings = collection(props, "headings")?;
        Ok(snapshot)
    }

    fn references(&self) -> Result<Value, String> {
        let list = self.rpc(
            "list_objects_by_type",
            json!({"type_id":"com.kosmos.agenda.references"}),
        )?;
        let list = list
            .as_array()
            .ok_or("Некорректный список справочников Engine")?;
        if let Some(object) = list
            .iter()
            .filter(|v| !v["deletedAt"].is_string())
            .max_by_key(|v| v["updatedAt"].as_str().unwrap_or_default())
        {
            return Ok(object.clone());
        }
        self.rpc("get_object", json!({"id":"agenda:references"}))
    }

    pub fn save(&self, before: Option<&Todo>, todo: &Todo) -> Result<(), String> {
        let current = self.rpc("get_object", json!({"id":todo.id}))?;
        if let Some(before) = before {
            if current.is_null() || mapping::read(&current)? != *before {
                return Err("Задача изменилась в другом приложении. Обновите список.".into());
            }
        } else if !current.is_null() {
            return Err("Задача с таким ID уже существует. Обновите список.".into());
        }
        let object = mapping::write(current, before, todo)?;
        self.upsert(object)
    }

    fn upsert(&self, object: Value) -> Result<(), String> {
        let result = self.rpc("upsert_object", json!({"object":object}))?;
        if result != true {
            return Err("Engine не подтвердил сохранение. Обновите список.".into());
        }
        Ok(())
    }

    fn save_project(&self, project: &Project) -> Result<(), String> {
        let mut object = self.references()?;
        let projects = object["propsJson"]["projects"]
            .as_array_mut()
            .ok_or("Нет проектов в Engine")?;
        let stored = projects
            .iter_mut()
            .find(|p| p["id"] == project.id)
            .ok_or("Проект не найден")?;
        stored["status"] = json!(project.status);
        object["updatedAt"] = json!(chrono::Utc::now().to_rfc3339());
        self.upsert(object)
    }
}

fn collection<T: serde::de::DeserializeOwned>(props: &Value, key: &str) -> Result<Vec<T>, String> {
    serde_json::from_value(props.get(key).cloned().unwrap_or_else(|| json!([])))
        .map_err(|_| format!("Некорректный справочник {key}"))
}
