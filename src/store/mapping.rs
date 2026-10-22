use crate::model::{Status, Todo};
use serde_json::{json, Value};

/// core.task 0.1.0 (KOS-397). The project is an object link, not a prop; the
/// Engine returns links separately, and `store.rs` diffs the link on save.
/// Fields Agenda does not show (deadline, reminders, priority, checklist,
/// recurrence, tags, evening period) stay on the object untouched.
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
const NOT_EXTENSION: [&str; 12] = [
    "id",
    "title",
    "notes",
    "project_id",
    "created_at",
    "status",
    "is_today",
    "is_someday",
    "is_trashed",
    "is_completed",
    "completed_at",
    "scheduled_date",
];

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
    let legacy_deferred = fields.get("status") == Some(&json!("deferred"));
    // Agenda has two states. Legacy states other clients wrote still open:
    // `canceled` reads as done (closed, but no completion stamp, so it stays
    // out of statistics), `inProgress` as not done. `write` keeps them stored
    // until the user toggles the task.
    let closed = matches!(state, "done" | "canceled");
    let status = match state {
        _ if closed => Status::Done,
        _ if planning == "inbox" => Status::Inbox,
        _ => Status::Todo,
    };
    fields.insert("status".into(), json!(status));
    fields.insert("is_completed".into(), json!(closed));
    fields.insert("is_today".into(), json!(planning == "today"));
    fields.insert(
        "is_someday".into(),
        json!(planning == "someday" || legacy_deferred),
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
    fields.insert(
        "project_id".into(),
        json!(links
            .iter()
            .find(|l| l.kind == PROJECT_LINK)
            .map(|l| l.target.clone())),
    );
    serde_json::from_value(Value::Object(fields)).map_err(|e| e.to_string())
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
