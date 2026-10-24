use chrono::{Datelike, Duration, Local, NaiveDate};
use serde_json::{json, Value};

use super::Agenda;
use crate::model::{completed_day, Todo};

impl Agenda {
    pub(crate) fn nsh_activity(&self) -> Vec<Value> {
        let today = Local::now().date_naive();
        let end = today - Duration::days(today.weekday().num_days_from_monday() as i64);
        let first = end - Duration::weeks(51);
        (0..52)
            .map(|index| {
                let start = first + Duration::weeks(index);
                let done = completed_in_week(&self.todos, start);
                json!({"label": format!("{} {}", start.day(), start.month()), "done": done})
            })
            .collect()
    }
}

fn completed_in_week(todos: &[Todo], start: NaiveDate) -> usize {
    let end = start + Duration::days(7);
    todos
        .iter()
        .filter(|todo| !todo.is_trashed)
        .filter_map(completed_day)
        .filter(|day| *day >= start && *day < end)
        .count()
}
