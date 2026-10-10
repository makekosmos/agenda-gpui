//! Projects as canonical `com.kosmos.project` objects.
use super::{Engine, EngineError};
use agenda_core::projects::{project_object, read_project, PROJECT_TYPE};
use agenda_core::{Project, Todo};
use serde_json::{json, Value};
use std::collections::HashSet;

impl Engine {
    /// Projects are canonical `com.kosmos.project` objects; Agenda-only
    /// fields live in `extensions`. Creating and saving are the same upsert.
    pub(super) fn save_project(&self, project: &Project) -> Result<(), EngineError> {
        let current = self.rpc("get_object", json!({"id":project.id}))?;
        let now = chrono::Utc::now().to_rfc3339();
        self.upsert(project_object(&current, project, &now))
    }

    /// Soft delete: the object stays in ARK with `deletedAt`, like trash.
    pub(super) fn delete_project(
        &self,
        project: &Project,
        todos: Vec<(Todo, Todo)>,
    ) -> Result<(String, Vec<Todo>), EngineError> {
        let mut moved = Vec::with_capacity(todos.len());
        for (before, todo) in todos {
            self.save(Some(&before), &todo)?;
            moved.push(todo);
        }
        let current = self.rpc("get_object", json!({"id":project.id}))?;
        let now = chrono::Utc::now().to_rfc3339();
        let mut object = project_object(&current, project, &now);
        object["deletedAt"] = json!(now);
        self.upsert(object)?;
        Ok((project.id.clone(), moved))
    }

    /// Live projects plus the ids of every canonical project, deleted or not.
    pub(super) fn projects(&self) -> Result<(Vec<Project>, HashSet<String>), EngineError> {
        let list = self.rpc("list_objects_by_type", json!({"type_id":PROJECT_TYPE}))?;
        let list = list
            .as_array()
            .ok_or_else(|| Self::mapping_err("project list not an array".into()))?;
        let all: Vec<&Value> = list
            .iter()
            .filter(|v| v["typeId"] == PROJECT_TYPE)
            .collect();
        let known = all
            .iter()
            .filter_map(|v| v["id"].as_str().map(str::to_string))
            .collect();
        let live = all
            .into_iter()
            .filter(|v| !v["deletedAt"].is_string())
            .map(read_project)
            .collect();
        Ok((live, known))
    }
}
