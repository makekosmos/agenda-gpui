use crate::model::{Status, Todo};
use serde_json::{json, Value};

/// core.task 0.1.0 (KOS-397). The project is an object link, not a prop; the
/// Engine returns links separately, and `store.rs` diffs the link on save.
/// Tags stay in `extensions.tag_ids`: Agenda does not write `com.kosmos.tag`
/// objects yet, so there is nothing for a tag link to point at.
pub const TASK_TYPE: &str = "core.task";
pub const TASK_VERSION: &str = "0.1.0";
pub const PROJECT_LINK: &str = "project";

/// An outbound task link as the Engine lists it: `kind` is the link type and
/// `target` the linked object id.
#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    pub id: String,
    pub kind: String,
    pub target: String,
}

/// Fields the write path never copies into `extensions`: they live in
/// canonical props, links, or the object row itself.
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
const FREQUENCIES: [&str; 4] = ["daily", "weekly", "monthly", "yearly"];

mod write;
pub use write::write;
// Only the fixture and mapping tests pin a clock; app code always uses `write`.
#[cfg(test)]
pub use write::write_at;

pub fn read(object: &Value, links: &[Link]) -> Result<Todo, String> {
    if object["typeId"] != TASK_TYPE || !object["id"].is_string() {
        return Err("Engine вернул некорректную задачу".into());
    }
    let props = &object["propsJson"];
    let mut fields = props["extensions"].as_object().cloned().unwrap_or_default();
    fields.insert("id".into(), object["id"].clone());
    fields.insert(
        "title".into(),
        json!(object["title"].as_str().unwrap_or_default()),
    );
    let notes = fields
        .get("description")
        .filter(|v| v.is_string())
        .cloned()
        .unwrap_or_else(|| json!(plain_text(&object["contentJson"])));
    fields.insert("notes".into(), notes);
    fields.insert("created_at".into(), object["createdAt"].clone());
    fields.insert("is_trashed".into(), json!(object["deletedAt"].is_string()));

    let state = props["state"]["kind"].as_str().unwrap_or("open");
    let planning = props["planning"]["kind"].as_str().unwrap_or("inbox");
    let ext_status = fields
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let status = match state {
        "done" => Status::Done,
        "canceled" => Status::Canceled,
        "inProgress" => Status::Started,
        _ if planning == "inbox" => Status::Inbox,
        _ if ext_status == "deferred" => Status::Deferred,
        _ => Status::Todo,
    };
    fields.insert("status".into(), json!(status));
    fields.insert("is_completed".into(), json!(state == "done"));
    fields.insert("is_cancelled".into(), json!(state == "canceled"));
    fields.insert("is_today".into(), json!(planning == "today"));
    fields.insert("is_someday".into(), json!(planning == "someday"));
    fields.insert(
        "is_evening".into(),
        json!(props["planning"]["period"] == "evening"),
    );
    fields.insert(
        "completed_at".into(),
        if state == "done" {
            props["state"]["at"].clone()
        } else {
            Value::Null
        },
    );
    let scheduled = if planning == "date" {
        props["planning"]["date"].clone()
    } else {
        fields.get("scheduled_date").cloned().unwrap_or(Value::Null)
    };
    fields.insert("scheduled_date".into(), scheduled);
    fields.insert("deadline".into(), props["deadline"].clone());
    fields.insert("reminder_date".into(), props["reminders"][0]["at"].clone());
    fields.insert(
        "priority".into(),
        json!(match props["priority"].as_str() {
            Some("low") => 1,
            Some("medium") => 2,
            Some("high" | "urgent") => 3,
            _ => 0,
        }),
    );
    fields.insert(
        "project_id".into(),
        json!(links
            .iter()
            .find(|l| l.kind == PROJECT_LINK)
            .map(|l| l.target.clone())),
    );
    let checklist: Vec<Value> = props["checklist"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|item| json!({"is_completed": item["done"] == true}))
        .collect();
    fields.insert("checklist".into(), json!(checklist));
    fields.insert("recurrence".into(), read_recurrence(&props["recurrence"]));
    serde_json::from_value(Value::Object(fields)).map_err(|e| e.to_string())
}

fn read_recurrence(value: &Value) -> Value {
    let Some(rule) = value.as_object() else {
        return Value::Null;
    };
    let frequency = FREQUENCIES
        .iter()
        .position(|f| Some(*f) == rule.get("frequency").and_then(Value::as_str))
        .unwrap_or(0);
    json!({
        "frequency": frequency,
        "interval": rule.get("interval").cloned().unwrap_or(json!(1)),
        "recurrence_type": u8::from(rule.get("kind") == Some(&json!("afterCompletion"))),
        "days_of_week": rule.get("daysOfWeek").cloned().unwrap_or(json!([])),
    })
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
