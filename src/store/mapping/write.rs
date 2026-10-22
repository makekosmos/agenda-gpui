//! Write side of the core.task mapping: patches a stored Engine object with the
//! edited task and keeps every field this app does not own.
use super::{NOT_EXTENSION, TASK_TYPE, TASK_VERSION};
use crate::model::{date_only, Status, Todo};
use serde_json::{json, Value};

/// Patch a fresh Engine object, retaining fields this UI does not understand.
/// The project link is not written here: `store.rs` diffs it.
pub fn write(object: Value, before: Option<&Todo>, todo: &Todo) -> Result<Value, String> {
    write_at(object, before, todo, &chrono::Utc::now().to_rfc3339())
}

/// `write` with an explicit clock, so Engine payload fixtures stay deterministic.
pub fn write_at(
    mut object: Value,
    before: Option<&Todo>,
    todo: &Todo,
    now: &str,
) -> Result<Value, String> {
    if object.is_null() {
        object = json!({"id":todo.id,"typeId":TASK_TYPE,"typeVersion":TASK_VERSION,
            "title":todo.title,"createdAt":todo.created_at,
            "propsJson":{"state":{"kind":"open","at":null},
                "planning":{"kind":"inbox","period":"any","date":null},
                "deadline":null,"reminders":[],"recurrence":null,"priority":"none",
                "checklist":[],"extensions":{}},
            "contentJson":{"type":"doc","content":[{"type":"paragraph"}]},"deletedAt":null});
    }
    let old = before
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| e.to_string())?;
    let next = serde_json::to_value(todo).map_err(|e| e.to_string())?;
    let changed = |key: &str| old.as_ref().is_none_or(|old| old[key] != next[key]);
    if !object["propsJson"]["extensions"].is_object() {
        object["propsJson"]["extensions"] = json!({});
    }
    for (key, value) in next.as_object().ok_or("task is not an object")? {
        if changed(key) && !NOT_EXTENSION.contains(&key.as_str()) {
            object["propsJson"]["extensions"][key] = value.clone();
        }
    }
    if changed("title") {
        object["title"] = json!(todo.title);
    }
    if changed("notes") {
        object["propsJson"]["extensions"]["description"] = json!(todo.notes);
        object["contentJson"] = json!({"type":"doc","content":[{"type":"paragraph",
            "content":[{"type":"text","text":todo.notes.as_deref().unwrap_or_default()}]}]});
    }
    let planning_inputs = [
        "status",
        "is_today",
        "is_someday",
        "is_completed",
        "completed_at",
        "scheduled_date",
    ];
    if planning_inputs.iter().any(|key| changed(key)) {
        write_state_and_planning(&mut object, todo, now);
    }
    if changed("is_trashed") {
        object["deletedAt"] = if todo.is_trashed {
            json!(now)
        } else {
            Value::Null
        };
    }
    object["updatedAt"] = json!(now);
    object["typeVersion"] = json!(TASK_VERSION);
    Ok(object)
}

fn is_instant(value: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(value).is_ok()
}

fn write_state_and_planning(object: &mut Value, todo: &Todo, now: &str) {
    let scheduled = date_only(&todo.scheduled_date);
    // A malformed completion instant keeps the stored one; a new completion stamps now.
    let stored_done_at = Some(&object["propsJson"]["state"])
        .filter(|state| state["kind"] == "done")
        .and_then(|state| state["at"].as_str())
        .map(str::to_owned);
    let planning_kind = if todo.status == Status::Inbox {
        "inbox"
    } else if todo.is_someday {
        "someday"
    } else if todo.is_today {
        "today"
    } else if scheduled.is_some() {
        "date"
    } else {
        "anytime"
    };
    let stored_state = object["propsJson"]["state"].clone();
    let state = match todo.status {
        // Legacy states keep their kind while the done/not-done bit holds.
        Status::Done if stored_state["kind"] == "canceled" => stored_state,
        Status::Inbox | Status::Todo if stored_state["kind"] == "inProgress" => stored_state,
        Status::Done => {
            let at = todo
                .completed_at
                .as_deref()
                .filter(|at| is_instant(at))
                .map(str::to_owned)
                .or(stored_done_at)
                .unwrap_or_else(|| now.to_owned());
            json!({"kind":"done","at":at})
        }
        Status::Inbox | Status::Todo => json!({"kind":"open","at":null}),
    };
    let props = &mut object["propsJson"];
    // Agenda does not edit the period; keep whatever the object carries.
    let period = props["planning"]["period"]
        .as_str()
        .unwrap_or("any")
        .to_owned();
    props["state"] = state;
    props["planning"] = json!({
        "kind": planning_kind,
        "period": period,
        "date": if planning_kind == "date" { scheduled.clone() } else { None },
    });
    // A scheduled day that the planning kind cannot hold stays in extensions,
    // so it is never lost (`read` falls back to it).
    if planning_kind == "date" {
        if let Some(ext) = props["extensions"].as_object_mut() {
            ext.remove("scheduled_date");
        }
    } else {
        props["extensions"]["scheduled_date"] = json!(scheduled);
    }
    props["extensions"]["status"] = json!(todo.status);
}
