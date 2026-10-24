//! Engine is the only owner of task persistence. No database or local task mirror.
pub mod mapping;
#[cfg(test)]
mod mapping_tests;
#[cfg(test)]
mod payload_fixtures;
mod projects;
#[cfg(test)]
mod restart_tests;
#[cfg(test)]
mod tests;
mod transport;
#[cfg(test)]
mod transport_tests;

use crate::model::{Project, Todo};
pub use imago_nsh::mundus_gpui_kit::engine_error::{EngineError, ErrorKind};
use serde_json::{json, Value};
use std::collections::HashMap;
pub use transport::Engine;
mod worker;
pub use worker::{Command, Mutation, Reply, Snapshot, Worker};

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
        let (stored, links) = self.stored_task(&next.id)?;
        if !stored.is_null() && mapping::read(&stored, &links).map_err(Self::mapping_err)? == *next
        {
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
        let mut links = self.project_links()?;
        let mut snapshot = Snapshot::default();
        for object in list {
            if object["typeId"] == mapping::TASK_TYPE {
                let id = object["id"].as_str().unwrap_or_default();
                let own = links.remove(id).unwrap_or_default();
                snapshot
                    .todos
                    .push(mapping::read(object, &own).map_err(Self::mapping_err)?);
            }
        }
        snapshot.todos.sort_by_key(|t| t.sort_order);
        let (live, known) = self.projects()?;
        snapshot.projects = live;
        let object = self.references()?;
        if object.is_null() {
            snapshot.projects.sort_by_key(|p| p.sort_order);
            return Ok(snapshot);
        }
        let props = &object["propsJson"];
        if props["model_version"] != 1 {
            return Err(Self::mapping_err("unknown references model_version".into()));
        }
        // Legacy references-only projects stay visible until their next save
        // writes them as canonical objects.
        for legacy in collection::<Project>(props, "projects")? {
            // `known` includes soft-deleted canonical projects: their stale
            // references entry must not bring them back.
            if !known.contains(&legacy.id) {
                snapshot.projects.push(legacy);
            }
        }
        snapshot.projects.sort_by_key(|p| p.sort_order);
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

    /// The stored task object and its project link, read the way `load` reads them.
    fn stored_task(&self, id: &str) -> Result<(Value, Vec<mapping::Link>), EngineError> {
        let object = self.rpc("get_object", json!({"id":id}))?;
        if object.is_null() {
            return Ok((object, Vec::new()));
        }
        let links = self.project_links()?.remove(id).unwrap_or_default();
        Ok((object, links))
    }

    /// Project links of every task, keyed by task id.
    fn project_links(&self) -> Result<HashMap<String, Vec<mapping::Link>>, EngineError> {
        let list = self.rpc("list_object_links", json!({}))?;
        let list = list
            .as_array()
            .ok_or_else(|| Self::mapping_err("links not an array".into()))?;
        let mut by_task: HashMap<String, Vec<mapping::Link>> = HashMap::new();
        for link in list
            .iter()
            .filter(|l| l["linkType"] == mapping::PROJECT_LINK)
        {
            let (Some(id), Some(source), Some(target)) = (
                link["id"].as_str(),
                link["sourceObjectId"].as_str(),
                link["targetObjectId"].as_str(),
            ) else {
                return Err(Self::mapping_err("link without id or endpoints".into()));
            };
            by_task
                .entry(source.to_owned())
                .or_default()
                .push(mapping::Link {
                    id: id.to_owned(),
                    kind: mapping::PROJECT_LINK.to_owned(),
                    target: target.to_owned(),
                });
        }
        Ok(by_task)
    }

    pub fn save(&self, before: Option<&Todo>, todo: &Todo) -> Result<(), EngineError> {
        let (current, stored) = self.stored_task(&todo.id)?;
        if let Some(before) = before {
            if current.is_null()
                || mapping::read(&current, &stored).map_err(Self::mapping_err)? != *before
            {
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
        let linked = stored.iter().find(|l| l.kind == mapping::PROJECT_LINK);
        let project = todo.project_id.as_deref();
        let relink = linked.map(|l| l.target.as_str()) != project;
        if let (true, Some(project)) = (relink, project) {
            // The Engine enforces the link's foreign keys: the project must be an object first.
            if self.rpc("get_object", json!({"id":project}))?.is_null() {
                return Err(EngineError {
                    kind: ErrorKind::InvalidRequest,
                    detail: "local: project is not saved yet".into(),
                });
            }
        }
        let object = mapping::write(current, before, todo).map_err(Self::mapping_err)?;
        self.upsert(object)?;
        if relink {
            self.relink_project(&todo.id, linked, project)?;
        }
        Ok(())
    }

    fn relink_project(
        &self,
        task: &str,
        linked: Option<&mapping::Link>,
        project: Option<&str>,
    ) -> Result<(), EngineError> {
        let Some(project) = project else {
            // Leaving the project has no new link to write, so the old one is removed alone.
            if let Some(link) = linked {
                self.rpc("delete_object_link", json!({"id":link.id}))?;
            }
            return Ok(());
        };
        // Id from the endpoints, so a link added twice (or from two devices) is one row.
        // One Engine transaction drops the other project links of this task and writes
        // this one, so a sync device never sees the task in no project or in two.
        let link = json!({
            "id": format!("lnk:{task}:{}:{project}", mapping::PROJECT_LINK),
            "sourceObjectId": task,
            "targetObjectId": project,
            "linkType": mapping::PROJECT_LINK,
            "createdAt": chrono::Utc::now().to_rfc3339(),
        });
        self.rpc("replace_object_link", json!({"object_link":link}))?;
        Ok(())
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
