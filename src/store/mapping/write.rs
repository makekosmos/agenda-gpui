//! Write side of the core.task mapping: patches a stored Engine object with the
//! edited task and keeps every field this app does not own.
use super::{FREQUENCIES, NOT_EXTENSION, PRIORITIES, TASK_TYPE, TASK_VERSION};
use crate::model::{date_only, ChecklistItem, Status, Todo};
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
        "is_evening",
        "is_completed",
        "is_cancelled",
        "completed_at",
        "scheduled_date",
    ];
    if planning_inputs.iter().any(|key| changed(key)) {
        write_state_and_planning(&mut object, todo, now);
    }
    if changed("priority") {
        object["propsJson"]["priority"] = json!(PRIORITIES[todo.priority.min(3) as usize]);
    }
    if changed("deadline") {
        object["propsJson"]["deadline"] = json!(date_only(&todo.deadline));
    }
    if changed("reminder_date") {
        // A bare day is not a core.task instant: keep the stored reminder instead.
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

fn is_instant(value: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(value).is_ok()
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
    let state = match todo.status {
        Status::Canceled => json!({"kind":"canceled","at":now}),
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
        Status::Started => json!({"kind":"inProgress","at":null}),
        _ => json!({"kind":"open","at":null}),
    };
    let props = &mut object["propsJson"];
    props["state"] = state;
    props["planning"] = json!({
        "kind": planning_kind,
        "period": if todo.is_evening { "evening" } else { "any" },
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

fn write_recurrence(props: &mut Value, todo: &Todo) {
    let current = props["recurrence"].clone();
    props["recurrence"] = match todo.recurrence.as_ref() {
        None => Value::Null,
        Some(rule) => json!({
            "kind": if rule.recurrence_type == 1 { "afterCompletion" } else { "calendar" },
            "frequency": FREQUENCIES[rule.frequency.min(3) as usize],
            "interval": rule.interval.max(1),
            "daysOfWeek": rule.days_of_week,
            "timeZone": current["timeZone"].clone(),
            "end": current["end"].clone(),
        }),
    };
}
