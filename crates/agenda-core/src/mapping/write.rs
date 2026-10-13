use super::recurrence::write_recurrence;
use super::{TASK_TYPE, TASK_VERSION};
use crate::dates::{date_only, LocalDay};
use crate::filters::task_status;
use crate::types::{ChecklistItem, Status, Todo};
use serde_json::{json, Value};

/// Fields `write_at` never copies into `extensions`: canonical props, the
/// project link, or the object row itself own them.
const NOT_EXTENSION: [&str; 19] = [
    "id",
    "title",
    "notes",
    "checklist",
    "recurrence",
    "priority",
    "deadline",
    "reminder_date",
    "project_id",
    "created_at",
    "status",
    "is_today",
    "is_someday",
    "is_evening",
    "is_trashed",
    "is_completed",
    "is_cancelled",
    "completed_at",
    "scheduled_date",
];

const PRIORITIES: [&str; 4] = ["none", "low", "medium", "high"];

/// Patch a fresh Engine object, retaining fields this UI does not understand.
/// The caller owns the clock (`now` stamps `updatedAt`, `deletedAt` and new
/// completion or cancellation instants; fixtures pin it) and `day` supplies
/// the local offset `date_only` needs. Project links are not written here:
/// the caller diffs `project_id` against the stored link.
pub fn write_at(
    mut object: Value,
    before: Option<&Todo>,
    todo: &Todo,
    now: &str,
    day: &LocalDay,
) -> Result<Value, String> {
    if object.is_null() {
        object = json!({"id":todo.id,"typeId":TASK_TYPE,"typeVersion":TASK_VERSION,
            "title":todo.title,"createdAt":todo.created_at,"propsJson":{},
            "contentJson":{"type":"doc","content":[{"type":"paragraph"}]},"deletedAt":null});
    }
    if !object.is_object() {
        return Err("Engine object is not an object".into());
    }
    if !object["propsJson"].is_object() {
        object["propsJson"] = json!({});
    }
    fill_required_props(&mut object["propsJson"]);
    let old = before
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| e.to_string())?;
    let next = serde_json::to_value(todo).map_err(|e| e.to_string())?;
    let changed = |key: &str| old.as_ref().is_none_or(|old| old[key] != next[key]);
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
        "is_evening",
        "is_completed",
        "is_cancelled",
        "completed_at",
        "scheduled_date",
    ];
    if planning_inputs.iter().any(|key| changed(key)) {
        write_state_and_planning(&mut object, todo, now, day);
    }
    if changed("priority") {
        object["propsJson"]["priority"] = json!(PRIORITIES[todo.priority.min(3) as usize]);
    }
    if changed("deadline") {
        object["propsJson"]["deadline"] = json!(date_only(&todo.deadline, day));
    }
    if changed("reminder_date") {
        // A bare day is not a core.task instant: keep the stored reminder.
        match todo.reminder_date.as_deref() {
            Some(at) if !is_instant(at) => {}
            reminder => write_reminder(&mut object["propsJson"], reminder),
        }
    }
    if changed("checklist") {
        write_checklist(&mut object["propsJson"], &todo.checklist, &todo.id);
    }
    if changed("recurrence") {
        write_recurrence(&mut object["propsJson"], todo);
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

/// `core.task` requires every canonical prop. Objects built empty or written
/// before the cutover get the contract's default for each missing one.
fn fill_required_props(props: &mut Value) {
    let defaults = [
        ("state", json!({"kind":"open","at":null})),
        (
            "planning",
            json!({"kind":"inbox","period":"any","date":null}),
        ),
        ("deadline", Value::Null),
        ("reminders", json!([])),
        ("recurrence", Value::Null),
        ("priority", json!("none")),
        ("checklist", json!([])),
        ("extensions", json!({})),
    ];
    for (key, value) in defaults {
        if props.get(key).is_none() {
            props[key] = value;
        }
    }
    if !props["extensions"].is_object() {
        props["extensions"] = json!({});
    }
}

fn write_state_and_planning(object: &mut Value, todo: &Todo, now: &str, day: &LocalDay) {
    let status = task_status(todo);
    let scheduled = date_only(&todo.scheduled_date, day);
    let stored = object["propsJson"]["state"].clone();
    let planning_kind = if status == Status::Inbox {
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
    let state = match status {
        Status::Done => {
            let at = todo
                .completed_at
                .as_deref()
                .filter(|at| is_instant(at))
                .map(str::to_owned)
                .or_else(|| stored_instant(&stored, "done"))
                .unwrap_or_else(|| now.to_owned());
            json!({"kind":"done","at":at})
        }
        Status::Canceled => json!({"kind":"canceled",
            "at":stored_instant(&stored, "canceled").unwrap_or_else(|| now.to_owned())}),
        Status::Started => json!({"kind":"inProgress","at":null}),
        _ => json!({"kind":"open","at":null}),
    };
    let props = &mut object["propsJson"];
    props["state"] = state;
    props["planning"] = json!({
        "kind": planning_kind,
        "period": if todo.is_evening { "evening" } else { "any" },
        "date": if planning_kind == "date" { json!(scheduled.clone()) } else { Value::Null },
    });
    // A scheduled day the planning kind cannot hold stays in extensions, so
    // it is never lost; `read` falls back to it.
    if planning_kind == "date" {
        if let Some(ext) = props["extensions"].as_object_mut() {
            ext.remove("scheduled_date");
        }
    } else {
        props["extensions"]["scheduled_date"] = json!(scheduled);
    }
    props["extensions"]["status"] = json!(todo.status);
}

/// The instant a state keeps when it is already `kind`, if it is a real instant.
fn stored_instant(state: &Value, kind: &str) -> Option<String> {
    Some(state)
        .filter(|s| s["kind"] == kind)
        .and_then(|s| s["at"].as_str())
        .filter(|at| is_instant(at))
        .map(str::to_owned)
}

fn is_instant(value: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(value).is_ok()
}

fn write_reminder(props: &mut Value, reminder: Option<&str>) {
    let mut reminders = props["reminders"].as_array().cloned().unwrap_or_default();
    match reminder {
        Some(at) => {
            let entry = json!({"kind":"floating","at":at,"timeZone":null});
            match reminders.first_mut() {
                Some(first) => *first = entry,
                None => reminders.push(entry),
            }
        }
        None => {
            if !reminders.is_empty() {
                reminders.remove(0);
            }
        }
    }
    props["reminders"] = json!(reminders);
}

/// Items keep the id, text and rank the Engine gave them; Agenda only toggles `done`.
fn write_checklist(props: &mut Value, items: &[ChecklistItem], task: &str) {
    let stored = props["checklist"].as_array().cloned().unwrap_or_default();
    let next: Vec<Value> = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let mut entry = stored.get(index).cloned().unwrap_or_else(|| {
                json!({"id":format!("{task}:item:{index}"),"text":"","rank":format!("{index:06}")})
            });
            entry["done"] = json!(item.is_completed);
            entry
        })
        .collect();
    props["checklist"] = json!(next);
}
