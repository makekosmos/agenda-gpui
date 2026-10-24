// Port of src/dev/seed.ts + task lifecycle/filter logic to Rust.
use chrono::{Datelike, Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Not done, not yet placed (the Inbox list).
    Inbox,
    /// Not done, placed in a list, a date or a project.
    Todo,
    Done,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub notes: Option<String>,
    pub scheduled_date: Option<String>,
    pub is_today: bool,
    pub is_someday: bool,
    pub status: Status,
    pub is_completed: bool,
    pub completed_at: Option<String>,
    pub is_trashed: bool,
    pub created_at: String,
    pub project_id: Option<String>,
    pub sort_order: i32,
}

impl Default for Todo {
    fn default() -> Self {
        new_todo(String::new(), "")
    }
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub title: String,
    pub status: u8, // 0 active, 1 someday, 2 completed
    pub sort_order: i32,
}

// ---------------------------------------------------------------------------
// Date helpers (mirror taskLifecycle localDateKey / taskDate)
// ---------------------------------------------------------------------------

pub fn today_key() -> String {
    let d = Local::now().date_naive();
    format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day())
}

pub fn key_of(d: NaiveDate) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day())
}

pub fn parse_key(key: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(key, "%Y-%m-%d").ok()
}

pub fn day_key(offset: i64) -> String {
    key_of(Local::now().date_naive() + Duration::days(offset))
}

fn iso_at(offset: i64, h: u32, m: u32) -> String {
    let d = Local::now().date_naive() + Duration::days(offset);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:00",
        d.year(),
        d.month(),
        d.day(),
        h,
        m
    )
}

/// The day an RFC 3339 stamp expresses at `offset` — pure, so tests pin a
/// zone instead of depending on the machine's `Local`. The local-time rule is
/// deliberate: old writers emitted `toISOString()` of local midnights, and
/// ark-core's `canonical_types::normalize` applies the same rule on read.
pub(crate) fn stamp_day_in(v: &str, offset: chrono::FixedOffset) -> Option<String> {
    chrono::DateTime::parse_from_rfc3339(v)
        .ok()
        .map(|dt| key_of(dt.with_timezone(&offset).date_naive()))
}

/// dateOnly(): accepts "YYYY-MM-DD" or ISO datetime → "YYYY-MM-DD".
/// Stamps carrying an explicit offset are instants: interpret them in
/// local time (dayjs parity) instead of truncating the UTC representation,
/// which shifts the day for anyone outside UTC around midnight.
pub fn date_only(value: &Option<String>) -> Option<String> {
    let v = value.as_ref()?;
    let now = Local::now();
    if let Some(day) = stamp_day_in(v, *now.offset()) {
        return Some(day);
    }
    // A ≥10-char prefix is not proof of a date: malformed stamps from other
    // clients became lexicographic "date" keys, hiding tasks from Inbox and
    // sorting them into phantom groups. Keep only parseable YYYY-MM-DD.
    v.get(..10)
        .filter(|s| parse_key(s).is_some())
        .map(str::to_owned)
}

/// Day the board uses: the scheduled day, if it parses.
pub fn task_date(t: &Todo) -> (Option<String>, bool) {
    (date_only(&t.scheduled_date), false)
}

// ---------------------------------------------------------------------------
// Status / predicates (taskLifecycle.ts + todoFilterService.ts)
// ---------------------------------------------------------------------------

pub fn task_status(t: &Todo) -> Status {
    if t.is_completed || t.status == Status::Done {
        Status::Done
    } else {
        t.status
    }
}

pub fn is_active(t: &Todo) -> bool {
    task_status(t) != Status::Done && !t.is_trashed
}

pub fn is_inbox(t: &Todo) -> bool {
    task_status(t) == Status::Inbox && !t.is_trashed
}

pub fn is_deferred(t: &Todo) -> bool {
    // The someday flag is only meaningful on a live task: `complete_todo`
    // leaves it set on the finished copy, and Engine data from other clients
    // can arrive as Done + is_someday — either way a closed task must not
    // keep a row in «Потом».
    is_active(t) && t.is_someday
}

pub fn is_overdue(t: &Todo, today: &str) -> bool {
    if !is_active(t) || is_inbox(t) || is_deferred(t) {
        return false;
    }
    let (date, conflict) = task_date(t);
    !conflict && date.as_deref().is_some_and(|d| d < today)
}

/// Next ISO week bounds (Monday..=Sunday) as day keys.
pub fn next_week_bounds(today: &str) -> Option<(String, String)> {
    let (mon, sun) = week_bounds(today)?;
    let mon = parse_key(&mon)? + Duration::days(7);
    let sun = parse_key(&sun)? + Duration::days(7);
    Some((key_of(mon), key_of(sun)))
}

/// Monday of next week as a day key — «следующая неделя» as a real date.
pub fn next_monday_key(today: &str) -> Option<String> {
    let (mon, _) = week_bounds(today)?;
    let mon = parse_key(&mon)? + Duration::days(7);
    Some(key_of(mon))
}

/// Current ISO week bounds (Monday..=Sunday) as day keys.
pub fn week_bounds(today: &str) -> Option<(String, String)> {
    let d = parse_key(today)?;
    let mon = d - Duration::days(d.weekday().num_days_from_monday() as i64);
    Some((key_of(mon), key_of(mon + Duration::days(6))))
}

/// Dorofeev-style week membership: no precise day needed — a task belongs to
/// the week when its date falls inside it or it carries the `is_today`
/// ("this week") flag with no date at all.
/// Active, non-deferred task whose date falls inside the given week bounds.
fn is_due_in_bounds(t: &Todo, week: &(String, String)) -> bool {
    let (date, _) = task_date(t);
    date.as_deref()
        .is_some_and(|d| d >= week.0.as_str() && d <= week.1.as_str())
        && is_active(t)
        && !is_deferred(t)
}

pub fn is_due_this_week(t: &Todo, week: &(String, String)) -> bool {
    if !is_active(t) || is_inbox(t) || is_deferred(t) {
        return false;
    }
    let (date, conflict) = task_date(t);
    !conflict
        && (date
            .as_deref()
            .is_some_and(|d| d >= week.0.as_str() && d <= week.1.as_str())
            || (date.is_none() && t.is_today))
}

/// Week list body below the overdue block: a task dated earlier this week is
/// already listed as overdue, so it must not appear a second time.
fn in_week_list(t: &Todo, week: &(String, String), today: &str) -> bool {
    (is_due_this_week(t, week) && !is_overdue(t, today)) || completed_in_week(t, week)
}

pub fn completed_in_week(t: &Todo, week: &(String, String)) -> bool {
    task_status(t) == Status::Done
        && !t.is_trashed
        && date_only(&t.completed_at)
            .as_deref()
            .is_some_and(|d| d >= week.0.as_str() && d <= week.1.as_str())
}

/// The local day a task was completed, for statistics aggregation. Trashed
/// tasks are excluded from every count.
pub fn completed_day(t: &Todo) -> Option<NaiveDate> {
    if task_status(t) != Status::Done || t.is_trashed {
        return None;
    }
    date_only(&t.completed_at).and_then(|k| parse_key(&k))
}

pub fn is_archived(t: &Todo, today: &str) -> bool {
    if task_status(t) != Status::Done || t.is_trashed {
        return false;
    }
    match date_only(&t.completed_at) {
        Some(d) => d.as_str() < today,
        None => false,
    }
}

/// Ordering key for RFC 3339 instants. Raw strings do not sort correctly when
/// offsets differ ("+03:00" compares above "Z" although it marks an earlier
/// instant), so Logbook/Trash order by the parsed instant; unparseable values
/// sink to the bottom.
fn instant_key(value: Option<&str>) -> i64 {
    value
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.timestamp_micros())
        .unwrap_or(i64::MIN)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod capture;
mod lists;
mod seed;
#[cfg(test)]
pub use capture::{parse_project_mention, parse_quick_entry_capture};
pub use lists::{filter_idx, SmartList};
#[cfg(test)]
pub(crate) use seed::todo;
pub use seed::{new_todo, seed_projects, seed_todos};
