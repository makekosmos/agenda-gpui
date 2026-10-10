//! `com.kosmos.project` canonical-object mapping (was src/store/projects.rs
//! in agenda-gpui; moved here so Android builds the same object shape).
use crate::types::Project;
use serde_json::{json, Value};

pub const PROJECT_TYPE: &str = "com.kosmos.project";
const PROJECT_STATUS: [&str; 3] = ["active", "someday", "completed"];

pub fn project_object(current: &Value, project: &Project, now: &str) -> Value {
    let mut object = if current.is_null() {
        json!({"id":project.id,"typeId":PROJECT_TYPE,"typeVersion":"1.1.0","createdAt":now,
            "propsJson":{"scheduledAt":null,"color":null,"extensions":{}},
            "contentJson":{"type":"doc","content":[{"type":"paragraph"}]},"deletedAt":null})
    } else {
        current.clone()
    };
    object["title"] = json!(project.title);
    object["updatedAt"] = json!(now);
    let props = &mut object["propsJson"];
    props["status"] = json!(PROJECT_STATUS
        .get(usize::from(project.status))
        .unwrap_or(&"active"));
    props["dueAt"] = json!(project.deadline);
    if !props["extensions"].is_object() {
        props["extensions"] = json!({});
    }
    props["extensions"]["sortOrder"] = json!(project.sort_order);
    props["extensions"]["areaId"] = json!(project.area_id);
    object
}

pub fn read_project(object: &Value) -> Project {
    let props = &object["propsJson"];
    let ext = &props["extensions"];
    Project {
        id: object["id"].as_str().unwrap_or_default().to_string(),
        title: object["title"].as_str().unwrap_or_default().to_string(),
        status: PROJECT_STATUS
            .iter()
            .position(|s| props["status"] == *s)
            .unwrap_or(0) as u8,
        deadline: props["dueAt"].as_str().map(str::to_string),
        sort_order: ext["sortOrder"].as_i64().unwrap_or(0) as i32,
        area_id: ext["areaId"].as_str().map(str::to_string),
    }
}

/// `com.kosmos.agenda.references` propsJson: legacy projects + the tag list.
/// Returns `(projects, tags)`; callers filter legacy projects against the
/// canonical-project id set exactly like desktop (`store.rs`).
pub fn read_references(object: &Value) -> Result<(Vec<Project>, Vec<crate::Tag>), String> {
    let props = &object["propsJson"];
    if props["model_version"] != 1 {
        return Err("unknown references model_version".into());
    }
    let projects = serde_json::from_value(props.get("projects").cloned().unwrap_or(json!([])))
        .map_err(|e| format!("references projects: {e}"))?;
    let tags = serde_json::from_value(props.get("tags").cloned().unwrap_or(json!([])))
        .map_err(|e| format!("references tags: {e}"))?;
    Ok((projects, tags))
}

/// The references object as agenda writes it: one `agenda:references` doc
/// whose propsJson carries the shared tag list (and legacy projects).
pub fn references_object(projects: &[Project], tags: &[crate::Tag], now: &str) -> Value {
    json!({
        "id": "agenda:references",
        "typeId": "com.kosmos.agenda.references",
        "typeVersion": "1.0.0",
        "title": "Agenda references",
        "createdAt": now,
        "updatedAt": now,
        "propsJson": {
            "model_version": 1,
            "projects": projects,
            "tags": tags,
        },
        "contentJson": null,
        "deletedAt": null,
    })
}
