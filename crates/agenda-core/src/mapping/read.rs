use super::{Link, PROJECT_LINK, TASK_TYPE};
use crate::types::{Status, Todo};
use serde_json::{json, Value};

/// Read a `core.task` object and the links the Engine lists for it.
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
    fields.insert(
        "created_at".into(),
        json!(object["createdAt"].as_str().unwrap_or_default()),
    );
    fields.insert("is_trashed".into(), json!(object["deletedAt"].is_string()));

    let state = props["state"]["kind"].as_str().unwrap_or("open");
    let planning = props["planning"]["kind"].as_str().unwrap_or("inbox");
    let stored_status = fields
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let status = match state {
        "done" => Status::Done,
        "canceled" => Status::Canceled,
        "inProgress" => Status::Started,
        _ if planning == "inbox" => Status::Inbox,
        _ if stored_status == "deferred" => Status::Deferred,
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
    fields.insert("reminder_date".into(), props["reminders"][0]["at"].clone());
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
