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
pub struct RecurrenceRule {
    /// 0=daily 1=weekly 2=monthly 3=yearly (matches Frequency enum order)
    pub frequency: u8,
    pub interval: u32,
    /// 0=fixed (по расписанию) 1=after completion (после выполнения)
    #[serde(alias = "recurrenceType", default)]
    pub recurrence_type: u8,
    #[serde(alias = "daysOfWeek", default)]
    pub days_of_week: Vec<u8>,
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
    pub priority: u8, // 0 none 1 low 2 medium 3 high
    pub scheduled_date: Option<String>,
    pub deadline: Option<String>,
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
    pub recurrence: Option<RecurrenceRule>,
    pub billable: bool,
    pub price: Option<f64>,
    pub significance: Option<u8>,
    pub fuel_cost: Option<f64>,
    pub sort_order: i32,
}

impl Default for Todo {
    fn default() -> Self {
        new_todo(String::new(), "")
    }
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub title: String,
    pub status: u8, // 0 active, 1 someday, 2 completed
    pub deadline: Option<String>,
    pub sort_order: i32,
    pub area_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
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
        priority: 0,
        scheduled_date: None,
        deadline: None,
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
        recurrence: None,
        billable: false,
        price: None,
        significance: None,
        fuel_cost: None,
        sort_order: 0,
    }
}
