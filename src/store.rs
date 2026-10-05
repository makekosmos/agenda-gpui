//! Engine is the only owner of task persistence. No database or local task mirror.
pub mod mapping;
#[cfg(test)]
mod mapping_tests;
#[cfg(test)]
mod payload_fixtures;
#[cfg(test)]
mod tests;
mod transport;
#[cfg(test)]
mod transport_tests;

use crate::model::{Area, Heading, Project, Tag, Todo};
pub use mundus_gpui_kit::engine_error::{EngineError, ErrorKind};
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
    CreateProject(Project),
}

pub enum Reply {
    Loaded(Result<Snapshot, EngineError>),
    Saved(Box<Mutation>, Result<(), EngineError>),
    Project(Result<Project, EngineError>),
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
                    Command::CreateProject(project) => {
                        Reply::Project(engine.create_project(&project).map(|()| project))
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
    ) -> Result<(), EngineError> {
        self.save(before, todo)?;
        let Some(next) = next else {
            return Ok(());
        };
        if self.save(None, next).is_ok() {
            return Ok(());
        }
        // A lost acknowledgement may still mean the next occurrence was saved.
        let stored = self.rpc("get_object", json!({"id":next.id}))?;
        if !stored.is_null() && mapping::read(&stored).map_err(Self::mapping_err)? == *next {
            return Ok(());
        }
        if stored.is_null() {
            if let Some(before) = before {
                self.save(Some(todo), before)?;
            }
        }
        Err(EngineError {
            kind: ErrorKind::Unavailable,
            detail: "local: save_with_next unconfirmed".into(),
        })
    }

    fn mapping_err(error: String) -> EngineError {
        EngineError {
            kind: ErrorKind::InvalidRequest,
            detail: format!("mapping: {error}"),
        }
    }

    pub fn load(&self) -> Result<Snapshot, EngineError> {
        let list = self.rpc(
            "list_objects_by_type",
            json!({"type_id":mapping::TASK_TYPE}),
        )?;
        let list = list
            .as_array()
            .ok_or_else(|| Self::mapping_err("list not an array".into()))?;
        let mut snapshot = Snapshot::default();
        for object in list {
            if object["typeId"] == mapping::TASK_TYPE {
                snapshot
                    .todos
                    .push(mapping::read(object).map_err(Self::mapping_err)?);
            }
        }
        snapshot.todos.sort_by_key(|t| t.sort_order);
        let object = self.references()?;
        if object.is_null() {
            return Ok(snapshot);
        }
        let props = &object["propsJson"];
        if props["model_version"] != 1 {
            return Err(Self::mapping_err("unknown references model_version".into()));
        }
        snapshot.projects = collection(props, "projects")?;
        snapshot.areas = collection(props, "areas")?;
        snapshot.tags = collection(props, "tags")?;
        snapshot.headings = collection(props, "headings")?;
        Ok(snapshot)
    }

    fn references(&self) -> Result<Value, EngineError> {
        let list = self.rpc(
            "list_objects_by_type",
            json!({"type_id":"com.kosmos.agenda.references"}),
        )?;
        let list = list
            .as_array()
            .ok_or_else(|| Self::mapping_err("references list not an array".into()))?;
        if let Some(object) = list
            .iter()
            .filter(|v| !v["deletedAt"].is_string())
            .max_by_key(|v| v["updatedAt"].as_str().unwrap_or_default())
        {
            return Ok(object.clone());
        }
        self.rpc("get_object", json!({"id":"agenda:references"}))
    }

    pub fn save(&self, before: Option<&Todo>, todo: &Todo) -> Result<(), EngineError> {
        let current = self.rpc("get_object", json!({"id":todo.id}))?;
        if let Some(before) = before {
            if current.is_null() || mapping::read(&current).map_err(Self::mapping_err)? != *before {
                return Err(EngineError {
                    kind: ErrorKind::Conflict,
                    detail: "local: precondition failed (stale or missing)".into(),
                });
            }
        } else if !current.is_null() {
            return Err(EngineError {
                kind: ErrorKind::Conflict,
                detail: "local: object already exists".into(),
            });
        }
        let object = mapping::write(current, before, todo).map_err(Self::mapping_err)?;
        self.upsert(object)
    }

    fn upsert(&self, object: Value) -> Result<(), EngineError> {
        let result = self.rpc("upsert_object", json!({"object":object}))?;
        if result != true {
            return Err(EngineError {
                kind: ErrorKind::Transport,
                detail: "upsert_object returned non-true data".into(),
            });
        }
        Ok(())
    }

    fn create_project(&self, project: &Project) -> Result<(), EngineError> {
        let mut object = self.references()?;
        let projects = object["propsJson"]["projects"]
            .as_array_mut()
            .ok_or_else(|| Self::mapping_err("no projects in references".into()))?;
        projects.push(json!({
            "id": project.id,
            "title": project.title,
            "status": project.status,
            "deadline": project.deadline,
            "sortOrder": project.sort_order,
            "areaId": project.area_id,
        }));
        object["updatedAt"] = json!(chrono::Utc::now().to_rfc3339());
        self.upsert(object)
    }

    fn save_project(&self, project: &Project) -> Result<(), EngineError> {
        let mut object = self.references()?;
        let projects = object["propsJson"]["projects"]
            .as_array_mut()
            .ok_or_else(|| Self::mapping_err("no projects in references".into()))?;
        let stored = projects
            .iter_mut()
            .find(|p| p["id"] == project.id)
            .ok_or_else(|| EngineError {
                kind: ErrorKind::NotFound,
                detail: format!("project {} not in references", project.id),
            })?;
        stored["status"] = json!(project.status);
        object["updatedAt"] = json!(chrono::Utc::now().to_rfc3339());
        self.upsert(object)
    }
}

fn collection<T: serde::de::DeserializeOwned>(
    props: &Value,
    key: &str,
) -> Result<Vec<T>, EngineError> {
    serde_json::from_value(props.get(key).cloned().unwrap_or_else(|| json!([]))).map_err(|e| {
        EngineError {
            kind: ErrorKind::InvalidRequest,
            detail: format!("references {key}: {e}"),
        }
    })
}
