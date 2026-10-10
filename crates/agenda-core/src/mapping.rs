use crate::dates::{date_only, LocalDay};
use crate::filters::task_status;
use crate::types::{Status, Todo};
use serde_json::{json, Value};

pub const TASK_TYPE: &str = "com.kosmos.task";
/// com.kosmos.task 1.1.0: `scheduledAt`/`dueAt`/`recurrence.endDate` are
/// `format: "date"` — a bare `YYYY-MM-DD` day, enforced by the Engine. The
/// instant fields (`reminderAt`/`completedAt`/`canceledAt`) stay `date-time`.
pub const TASK_VERSION: &str = "1.1.0";

pub fn read(object: &Value) -> Result<Todo, String> {
    if object["typeId"] != TASK_TYPE || !object["id"].is_string() {
        return Err("Engine вернул некорректную задачу".into());
    }
    let props = &object["propsJson"];
    let mut fields = props["extensions"].as_object().cloned().unwrap_or_default();
    if let Some(props) = props.as_object() {
        fields.extend(props.clone());
    }
    fields.insert("id".into(), object["id"].clone());
    fields.insert(
        "title".into(),
        json!(object["title"].as_str().unwrap_or_default()),
    );
    fields.insert(
        "notes".into(),
        fields
            .get("description")
            .filter(|v| v.is_string())
            .cloned()
            .unwrap_or_else(|| json!(plain_text(&object["contentJson"]))),
    );
    for (local, canonical) in [
        ("scheduled_date", "scheduledAt"),
        ("deadline", "dueAt"),
        ("reminder_date", "reminderAt"),
        ("completed_at", "completedAt"),
    ] {
        if fields.get(local).is_none_or(Value::is_null) && props[canonical].is_string() {
            fields.insert(local.into(), props[canonical].clone());
        }
    }
    fields
        .entry("created_at")
        .or_insert_with(|| object["createdAt"].clone());
    let canonical = props["status"].as_str();
    let extension = props["extensions"]["status"].as_str();
    let status = if matches!(canonical, Some("todo" | "inbox"))
        && matches!(
            extension,
            Some("started" | "in_progress" | "deferred" | "backlog")
        ) {
        extension
    } else if matches!(canonical, Some("todo" | "inbox"))
        && props["extensions"]["kosmos"]["taskBucket"] == "backlog"
    {
        Some("deferred")
    } else {
        canonical.or(extension)
    };
    let status = match status {
        Some("inProgress" | "in_progress" | "started") => "started",
        Some("done" | "completed") => "done",
        Some("canceled" | "cancelled" | "duplicate") => "canceled",
        Some("deferred" | "backlog") => "deferred",
        Some("todo") => "todo",
        _ => "inbox",
    };
    fields.insert("status".into(), json!(status));
    if canonical.is_some() || extension.is_some() {
        fields.insert("is_completed".into(), json!(status == "done"));
        fields.insert("is_cancelled".into(), json!(status == "canceled"));
    }
    fields.insert(
        "is_trashed".into(),
        json!(fields.get("is_trashed") == Some(&json!(true)) || object["deletedAt"].is_string()),
    );
    fields.insert(
        "priority".into(),
        json!(props["extensions"]["priority"]
            .as_u64()
            .or_else(|| props["priority"].as_u64())
            .or_else(|| {
                // Canonical `priority` is written as a string
                // ("high"/"medium"/"low"/"none"): clients that keep only the
                // canonical field would otherwise lose the level on read.
                ["none", "low", "medium", "high"]
                    .iter()
                    .position(|v| props["priority"].as_str() == Some(*v))
                    .map(|i| i as u64)
            })
            .unwrap_or(0)
            .min(3)),
    );
    fields.insert(
        "checklist".into(),
        props
            .get("checklist")
            .or_else(|| fields.get("checklist_items"))
            .filter(|v| v.is_array())
            .cloned()
            .unwrap_or_else(|| json!([])),
    );
    let mut recurrence = fields
        .get("recurrence_rule")
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or_else(|| props["recurrence"].clone());
    if recurrence.is_object() {
        if let Some(frequency) = recurrence["frequency"].as_str() {
            let index = ["daily", "weekly", "monthly", "yearly"]
                .iter()
                .position(|v| *v == frequency)
                .ok_or("Некорректная периодичность задачи")?;
            recurrence["frequency"] = json!(index);
        }
        if recurrence["recurrenceType"].is_string() {
            recurrence["recurrenceType"] =
                json!(u8::from(recurrence["recurrenceType"] == "afterCompletion"));
        }
        if recurrence["daysOfWeek"].is_null() {
            recurrence.as_object_mut().unwrap().remove("daysOfWeek");
        }
    }
    fields.insert("recurrence".into(), recurrence);
    serde_json::from_value(Value::Object(fields))
        .map_err(|_| "Некорректные поля задачи Engine".into())
}

fn plain_text(value: &Value) -> Option<String> {
    fn visit(value: &Value, text: &mut String) {
        if let Some(s) = value["text"].as_str() {
            text.push_str(s);
        }
        if let Some(children) = value["content"].as_array() {
            for child in children {
                visit(child, text);
            }
            if matches!(
                value["type"].as_str(),
                Some("paragraph" | "heading" | "blockquote" | "listItem")
            ) {
                text.push('\n');
            }
        }
    }
    let mut text = String::new();
    visit(value, &mut text);
    (!text.trim().is_empty()).then(|| text.trim().to_owned())
}

/// Patch a fresh Engine object, retaining fields this UI does not
/// understand. The caller owns the clock: `now` stamps
/// `updatedAt`/`canceledAt` (fixtures pin it for determinism) and `day`
/// supplies the local offset `date_only` needs to normalize day fields.
pub fn write_at(
    mut object: Value,
    before: Option<&Todo>,
    todo: &Todo,
    now: &str,
    day: &LocalDay,
) -> Result<Value, String> {
    if object.is_null() {
        object = json!({"id":todo.id,"typeId":TASK_TYPE,"typeVersion":TASK_VERSION,
            "title":todo.title,"createdAt":todo.created_at,"propsJson":{"extensions":{}},
            "contentJson":{"type":"doc","content":[{"type":"paragraph"}]},"deletedAt":null});
    }
    let old = before
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| e.to_string())?;
    let next = serde_json::to_value(todo).map_err(|e| e.to_string())?;
    let changed = |key: &str| old.as_ref().is_none_or(|old| old[key] != next[key]);
    if !object["propsJson"].is_object() {
        object["propsJson"] = json!({});
    }
    if !object["propsJson"]["extensions"].is_object() {
        object["propsJson"]["extensions"] = json!({});
    }
    for (key, value) in next.as_object().unwrap() {
        if !changed(key)
            || matches!(
                key.as_str(),
                "id" | "title" | "notes" | "checklist" | "recurrence"
            )
        {
            continue;
        }
        object["propsJson"]["extensions"][key] = value.clone();
    }
    if changed("title") {
        object["title"] = json!(todo.title);
    }
    if changed("notes") {
        object["propsJson"]["extensions"]["description"] = json!(todo.notes);
        object["contentJson"] = json!({"type":"doc","content":[{"type":"paragraph",
            "content":[{"type":"text","text":todo.notes.as_deref().unwrap_or_default()}]}]});
    }
    let props = &mut object["propsJson"];
    if changed("status") || changed("is_completed") || changed("is_cancelled") {
        props["status"] = json!(match task_status(todo) {
            Status::Inbox => "inbox",
            Status::Todo | Status::Deferred => "todo",
            Status::Started => "inProgress",
            Status::Done => "done",
            Status::Canceled => "canceled",
        });
        props["extensions"]["status"] = json!(todo.status);
        if props["extensions"]["kosmos"].is_object() {
            props["extensions"]["kosmos"]
                .as_object_mut()
                .unwrap()
                .remove("taskBucket");
        }
        if todo.status == Status::Deferred {
            if !props["extensions"]["kosmos"].is_object() {
                props["extensions"]["kosmos"] = json!({});
            }
            props["extensions"]["kosmos"]["taskBucket"] = json!("backlog");
        }
        props["canceledAt"] = if todo.is_cancelled {
            json!(now)
        } else {
            Value::Null
        };
        props["extensions"]["cancelled_at"] = props["canceledAt"].clone();
    }
    if changed("priority") {
        props["priority"] = json!(["none", "low", "medium", "high"][todo.priority.min(3) as usize]);
    }
    // Day fields are `format: "date"` under 1.1.0 — normalize through
    // `date_only` so a stray RFC 3339 stamp read back from older data can
    // never be re-persisted into the canonical field.
    if changed("scheduled_date") {
        props["scheduledAt"] = json!(date_only(&todo.scheduled_date, day));
    }
    if changed("deadline") {
        props["dueAt"] = json!(date_only(&todo.deadline, day));
    }
    for (local, canonical) in [
        ("reminder_date", "reminderAt"),
        ("completed_at", "completedAt"),
    ] {
        if changed(local) {
            let value = next[local].clone();
            // These canonical fields are `date-time` instants: a bare day or
            // other non-instant string copied verbatim fails Engine
            // validation and rejects the whole upsert. Keep the stored
            // canonical value then; the local value still lands in
            // `extensions` via the loop above.
            if value.is_null()
                || value
                    .as_str()
                    .is_some_and(|s| chrono::DateTime::parse_from_rfc3339(s).is_ok())
            {
                props[canonical] = value;
            }
        }
    }
    // Existing checklist IDs/titles and recurrence end conditions belong to Engine.
    if before.is_none() {
        props["checklist"] = json!([]);
        props["extensions"]["checklist_items"] = json!([]);
    }
    if changed("recurrence") {
        // `dayOfMonth` is Engine-owned state that the canonical `recurrence`
        // schema does not allow (`additionalProperties: false`) — it survives
        // only inside `extensions.recurrence_rule`, never at the schema level.
        let end_date = props["recurrence"]["endDate"].clone();
        let day_of_month = props["recurrence"]["dayOfMonth"].clone();
        props["extensions"]["recurrence_rule"] = json!(todo.recurrence.as_ref().map(|r| json!({
            "frequency":r.frequency,"interval":r.interval,"recurrenceType":r.recurrence_type,"daysOfWeek":r.days_of_week,
            "endDate":end_date,"dayOfMonth":day_of_month
        })));
        props["recurrence"] = json!(todo.recurrence.as_ref().map(|r| json!({
            "frequency":(["daily","weekly","monthly","yearly"][r.frequency.min(3) as usize]),
            "interval":r.interval,"recurrenceType":if r.recurrence_type == 1 {"afterCompletion"} else {"fixed"},
            "daysOfWeek":r.days_of_week,"endDate":end_date
        })));
    }
    if changed("is_trashed") {
        object["deletedAt"] = if todo.is_trashed {
            json!(now)
        } else {
            Value::Null
        };
    }
    object["updatedAt"] = json!(now);
    // The payload now conforms to the current contract; claim it so the
    // Engine validates against 1.1.0, not the superseded 1.0.0.
    object["typeVersion"] = json!(TASK_VERSION);
    Ok(object)
}
