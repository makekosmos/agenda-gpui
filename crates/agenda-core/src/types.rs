// Task domain types (port of the Engine task shape from taskLifecycle.ts).
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Inbox,
    Todo,
    Started,
    Deferred,
    Done,
    Canceled,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChecklistItem {
    #[serde(alias = "isCompleted", default)]
    pub is_completed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub notes: Option<String>,
    pub scheduled_date: Option<String>,
    pub reminder_date: Option<String>,
    pub is_today: bool,
    pub is_evening: bool,
    pub is_someday: bool,
    pub status: Status,
    pub system_kind: Option<String>,
    pub is_completed: bool,
    pub completed_at: Option<String>,
    pub is_cancelled: bool,
    pub is_trashed: bool,
    pub created_at: String,
    pub heading_id: Option<String>,
    pub project_id: Option<String>,
    pub area_id: Option<String>,
    pub tag_ids: Vec<String>,
    pub checklist: Vec<ChecklistItem>,
    pub billable: bool,
    pub price: Option<f64>,
    pub sort_order: i32,
}

impl Default for Todo {
    fn default() -> Self {
        new_todo(String::new(), "")
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub title: String,
    pub status: u8, // 0 active, 1 someday, 2 completed
    pub sort_order: i32,
    pub area_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    pub title: String,
    pub color: String,
}

pub fn new_todo(id: impl Into<String>, title: &str) -> Todo {
    Todo {
        id: id.into(),
        title: title.into(),
        notes: None,
        scheduled_date: None,
        reminder_date: None,
        is_today: false,
        is_evening: false,
        is_someday: false,
        status: Status::Inbox,
        system_kind: None,
        is_completed: false,
        completed_at: None,
        is_cancelled: false,
        is_trashed: false,
        created_at: String::new(),
        heading_id: None,
        project_id: None,
        area_id: None,
        tag_ids: vec![],
        checklist: vec![],
        billable: false,
        price: None,
        sort_order: 0,
    }
}
