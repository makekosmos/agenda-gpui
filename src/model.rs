// Port of src/dev/seed.ts + task lifecycle/filter logic to Rust.
use chrono::{Datelike, Duration, Local, NaiveDate};
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
    // Schema fields seeded for Agenda parity; not rendered yet.
    #[allow(dead_code)]
    pub deadline: Option<String>,
    #[allow(dead_code)]
    pub sort_order: i32,
    #[allow(dead_code)]
    pub area_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Area {
    // Seeded for Agenda parity; areas are not rendered yet.
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub title: String,
    #[allow(dead_code)]
    pub sort_order: i32,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    pub title: String,
    pub color: String,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Heading {
    // Seeded for Agenda parity; headings are not rendered yet.
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub title: String,
    #[allow(dead_code)]
    pub sort_order: i32,
    #[allow(dead_code)]
    #[serde(default)]
    pub project_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalEvent {
    #[allow(dead_code)]
    pub id: &'static str,
    pub title: &'static str,
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub location: Option<&'static str>,
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

/// dateOnly(): accepts "YYYY-MM-DD" or ISO datetime → "YYYY-MM-DD"
pub fn date_only(value: &Option<String>) -> Option<String> {
    let v = value.as_ref()?;
    v.get(..10).map(str::to_owned)
}

/// taskDate(): deadline wins over scheduled; different dates = conflict.
pub fn task_date(t: &Todo) -> (Option<String>, bool) {
    let scheduled = date_only(&t.scheduled_date);
    let deadline = date_only(&t.deadline);
    if scheduled.is_some() && deadline.is_some() && scheduled != deadline {
        return (None, true);
    }
    (deadline.or(scheduled), false)
}

// ---------------------------------------------------------------------------
// Status / predicates (taskLifecycle.ts + todoFilterService.ts)
// ---------------------------------------------------------------------------

pub fn task_status(t: &Todo) -> Status {
    if t.is_cancelled || t.status == Status::Canceled {
        return Status::Canceled;
    }
    if t.is_completed || t.status == Status::Done {
        return Status::Done;
    }
    match t.status {
        Status::Started => Status::Started,
        Status::Deferred => Status::Deferred,
        Status::Todo => Status::Todo,
        _ => Status::Inbox,
    }
}

pub fn is_active(t: &Todo) -> bool {
    let s = task_status(t);
    s != Status::Done && s != Status::Canceled && !t.is_trashed
}

pub fn is_inbox(t: &Todo) -> bool {
    task_status(t) == Status::Inbox && !t.is_trashed
}

pub fn is_deferred(t: &Todo) -> bool {
    (task_status(t) == Status::Deferred || t.is_someday) && !t.is_trashed
}

pub fn is_overdue(t: &Todo, today: &str) -> bool {
    if !is_active(t) || is_inbox(t) || is_deferred(t) || t.system_kind.is_some() {
        return false;
    }
    let (date, conflict) = task_date(t);
    !conflict && date.as_deref().is_some_and(|d| d < today)
}

pub fn is_due_today(t: &Todo, today: &str) -> bool {
    if !is_active(t) || is_inbox(t) || is_deferred(t) {
        return false;
    }
    let (date, conflict) = task_date(t);
    !conflict && (date.as_deref() == Some(today) || (date.is_none() && t.is_today))
}

pub fn completed_on(t: &Todo, today: &str) -> bool {
    task_status(t) == Status::Done
        && !t.is_trashed
        && date_only(&t.completed_at).as_deref() == Some(today)
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

pub fn overdue_todos(todos: &[Todo], today: &str) -> Vec<Todo> {
    let mut v: Vec<Todo> = todos
        .iter()
        .filter(|t| is_overdue(t, today))
        .cloned()
        .collect();
    v.sort_by(|a, b| {
        let da = task_date(a).0.unwrap_or_default();
        let db = task_date(b).0.unwrap_or_default();
        da.cmp(&db).then(a.sort_order.cmp(&b.sort_order))
    });
    v
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SmartList {
    Inbox,
    Today,
    Plans,
    Someday,
    Logbook,
    Trash,
}

/// Index-based twin of [`filter_todos`]: returns positions into `todos`
/// without cloning any `Todo` — the hot path for virtualized lists.
pub fn filter_idx(list: SmartList, todos: &[Todo]) -> Vec<usize> {
    let today = today_key();
    if list == SmartList::Today {
        let mut v: Vec<usize> = todos
            .iter()
            .enumerate()
            .filter(|(_, t)| is_overdue(t, &today))
            .map(|(i, _)| i)
            .collect();
        v.sort_by_cached_key(|&i| {
            (
                task_date(&todos[i]).0.unwrap_or_default(),
                todos[i].sort_order,
            )
        });
        let mut due: Vec<usize> = todos
            .iter()
            .enumerate()
            .filter(|(_, t)| is_due_today(t, &today) || completed_on(t, &today))
            .map(|(i, _)| i)
            .collect();
        due.sort_by_key(|&i| todos[i].sort_order);
        v.extend(due);
        return v;
    }
    let mut v: Vec<usize> = todos
        .iter()
        .enumerate()
        .filter(|(_, t)| match list {
            SmartList::Today => unreachable!(),
            SmartList::Inbox => is_inbox(t),
            SmartList::Plans => {
                let (date, _) = task_date(t);
                date.as_deref().is_some_and(|d| d > today.as_str())
                    && is_active(t)
                    && !is_deferred(t)
                    && t.system_kind.is_none()
            }
            SmartList::Someday => is_deferred(t),
            SmartList::Logbook => is_archived(t, &today),
            SmartList::Trash => t.is_trashed,
        })
        .map(|(i, _)| i)
        .collect();
    match list {
        SmartList::Plans => {
            v.sort_by_cached_key(|&i| task_date(&todos[i]).0.unwrap_or_else(|| "\u{ffff}".into()))
        }
        SmartList::Logbook => v.sort_by_cached_key(|&i| {
            std::cmp::Reverse(todos[i].completed_at.clone().unwrap_or_default())
        }),
        SmartList::Trash => {
            v.sort_by_cached_key(|&i| std::cmp::Reverse(todos[i].created_at.clone()))
        }
        _ => v.sort_by_key(|&i| todos[i].sort_order),
    }
    v
}

/// Index-based twin of [`sorted`]; `sort_by_cached_key` so date/title keys
/// are computed once per item instead of once per comparison.
pub fn sort_idx(todos: &[Todo], mut items: Vec<usize>, key: SortKey) -> Vec<usize> {
    match key {
        SortKey::Default => {}
        SortKey::Date => items.sort_by_cached_key(|&i| {
            (
                task_date(&todos[i]).0.unwrap_or_else(|| "9999".into()),
                todos[i].sort_order,
            )
        }),
        SortKey::Priority => items.sort_by(|&a, &b| {
            todos[b]
                .priority
                .cmp(&todos[a].priority)
                .then(todos[a].sort_order.cmp(&todos[b].sort_order))
        }),
        SortKey::Title => {
            items.sort_by_cached_key(|&i| (todos[i].title.to_lowercase(), todos[i].sort_order))
        }
    }
    items
}

pub fn filter_todos(list: SmartList, todos: &[Todo]) -> Vec<Todo> {
    let today = today_key();
    // TodayPage.vue: [...overdueTodos(todos), ...filterTodos(SmartList.Today)]
    if list == SmartList::Today {
        let mut v = overdue_todos(todos, &today);
        let mut due: Vec<Todo> = todos
            .iter()
            .filter(|t| is_due_today(t, &today) || completed_on(t, &today))
            .cloned()
            .collect();
        due.sort_by_key(|a| a.sort_order);
        v.extend(due);
        return v;
    }
    let mut v: Vec<Todo> = todos
        .iter()
        .filter(|t| match list {
            SmartList::Today => unreachable!(),
            SmartList::Inbox => is_inbox(t),
            SmartList::Plans => {
                let (date, _) = task_date(t);
                date.as_deref().is_some_and(|d| d > today.as_str())
                    && is_active(t)
                    && !is_deferred(t)
                    && t.system_kind.is_none()
            }
            SmartList::Someday => is_deferred(t),
            SmartList::Logbook => is_archived(t, &today),
            SmartList::Trash => t.is_trashed,
        })
        .cloned()
        .collect();
    v.sort_by(|a, b| match list {
        SmartList::Plans => {
            let da = task_date(a).0.unwrap_or_else(|| "\u{ffff}".into());
            let db = task_date(b).0.unwrap_or_else(|| "\u{ffff}".into());
            da.cmp(&db)
        }
        SmartList::Logbook => {
            let da = a.completed_at.clone().unwrap_or_default();
            let db = b.completed_at.clone().unwrap_or_default();
            db.cmp(&da)
        }
        SmartList::Trash => b.created_at.cmp(&a.created_at),
        _ => a.sort_order.cmp(&b.sort_order),
    });
    v
}

// ---------------------------------------------------------------------------
// Sorting/grouping for TaskBoard
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Default,
    Date,
    Priority,
    Title,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GroupKey {
    None,
    Project,
    Date,
}

pub fn sorted(items: &[Todo], key: SortKey) -> Vec<Todo> {
    let mut arr = items.to_vec();
    match key {
        SortKey::Default => {}
        SortKey::Date => arr.sort_by(|a, b| {
            let da = task_date(a).0.unwrap_or_else(|| "9999".into());
            let db = task_date(b).0.unwrap_or_else(|| "9999".into());
            da.cmp(&db).then(a.sort_order.cmp(&b.sort_order))
        }),
        SortKey::Priority => arr.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then(a.sort_order.cmp(&b.sort_order))
        }),
        SortKey::Title => arr.sort_by(|a, b| {
            a.title
                .to_lowercase()
                .cmp(&b.title.to_lowercase())
                .then(a.sort_order.cmp(&b.sort_order))
        }),
    }
    arr
}

// ---------------------------------------------------------------------------
// Russian date formatting (Intl.DateTimeFormat ru-RU equivalents)
// ---------------------------------------------------------------------------

pub(crate) const MONTH_SHORT: [&str; 12] = [
    "янв.",
    "февр.",
    "мар.",
    "апр.",
    "мая",
    "июня",
    "июля",
    "авг.",
    "сент.",
    "окт.",
    "нояб.",
    "дек.",
];
pub(crate) const MONTH_LONG: [&str; 12] = [
    "января",
    "февраля",
    "марта",
    "апреля",
    "мая",
    "июня",
    "июля",
    "августа",
    "сентября",
    "октября",
    "ноября",
    "декабря",
];
const WEEKDAY_SHORT: [&str; 7] = ["пн", "вт", "ср", "чт", "пт", "сб", "вс"];
const WEEKDAY_DATIVE: [&str; 7] = [
    "понедельникам",
    "вторникам",
    "средам",
    "четвергам",
    "пятницам",
    "субботам",
    "воскресеньям",
];

/// {day: numeric, month: short} then strip trailing dot → "18 сент"
pub fn fmt_day_month(key: &str) -> String {
    match parse_key(key) {
        Some(d) => {
            let m = MONTH_SHORT[(d.month() - 1) as usize];
            let s = format!("{} {}", d.day(), m);
            s.trim_end_matches('.').to_string()
        }
        None => key.to_string(),
    }
}

/// {weekday: short, day: numeric, month: short} → "чт, 18 сент."
pub fn fmt_cal_day_label(key: &str) -> String {
    match parse_key(key) {
        Some(d) => {
            let wd = WEEKDAY_SHORT[d.weekday().num_days_from_monday() as usize];
            format!(
                "{}, {} {}",
                wd,
                d.day(),
                MONTH_SHORT[(d.month() - 1) as usize]
            )
        }
        None => key.to_string(),
    }
}

/// {day: numeric, month: long, year: numeric} → "18 сентября 2025"
pub fn fmt_day_month_year(key: &str) -> String {
    match parse_key(key) {
        Some(d) => format!(
            "{} {} {}",
            d.day(),
            MONTH_LONG[(d.month() - 1) as usize],
            d.year()
        ),
        None => key.to_string(),
    }
}

/// Week period label: "15 сентября — 21 сентября 2025"
pub fn fmt_week_period(first: &str, last: &str) -> String {
    let (f, l) = match (parse_key(first), parse_key(last)) {
        (Some(f), Some(l)) => (f, l),
        _ => return format!("{} — {}", first, last),
    };
    let first_text = format!("{} {}", f.day(), MONTH_LONG[(f.month() - 1) as usize]);
    let mut last_text = format!("{} {}", l.day(), MONTH_LONG[(l.month() - 1) as usize]);
    if f.year() != l.year() {
        last_text = format!("{} {}", last_text, l.year());
    }
    if f.year() == l.year() {
        format!("{} — {} {}", first_text, last_text, f.year())
    } else {
        format!("{} — {}", first_text, last_text)
    }
}

/// "dd.mm.yyyy" for statistics day labels
#[allow(dead_code)]
pub fn fmt_dot_date(key: &str) -> String {
    match parse_key(key) {
        Some(d) => format!("{:02}.{:02}.{}", d.day(), d.month(), d.year()),
        None => key.to_string(),
    }
}

/// iso "YYYY-MM-DDTHH:MM" → "HH:MM"
pub fn fmt_time(iso: &str) -> String {
    if iso.len() >= 16 {
        iso[11..16].to_string()
    } else {
        String::new()
    }
}

// ---------------------------------------------------------------------------
// Recurrence description (describeRecurrence)
// ---------------------------------------------------------------------------

pub fn describe_recurrence(rule: &Option<RecurrenceRule>) -> String {
    let Some(rule) = rule else {
        return "Повторение".into();
    };
    let singular = ["Каждый день", "Каждую неделю", "Каждый месяц", "Каждый год"];
    let plural = ["дня", "недели", "месяца", "года"];
    let i = (rule.frequency as usize).min(3);
    let every = if rule.interval == 1 {
        singular[i].to_string()
    } else {
        format!("Каждые {} {}", rule.interval, plural[i])
    };
    let weekdays = if rule.frequency == 1 && !rule.days_of_week.is_empty() {
        let names: Vec<&str> = rule
            .days_of_week
            .iter()
            .map(|d| WEEKDAY_DATIVE[(*d as usize).saturating_sub(1).min(6)])
            .collect();
        format!(" по {}", names.join(" и "))
    } else {
        String::new()
    };
    let mode = if rule.recurrence_type == 1 {
        " после выполнения"
    } else {
        " по расписанию"
    };
    format!("{}{}{}", every, weekdays, mode)
}

// ---------------------------------------------------------------------------
// Quick-entry parsing (parseProjectMention + simplified parseQuickEntryCapture)
// ---------------------------------------------------------------------------

/// taskLifecycle.parseProjectMention: find "@<project title>" word-bounded mention.
pub fn parse_project_mention(title: &str, projects: &[Project]) -> (String, Option<String>) {
    let lower = title.to_lowercase();
    let mut sorted: Vec<&Project> = projects.iter().collect();
    sorted.sort_by_key(|p| std::cmp::Reverse(p.title.len()));
    for p in sorted {
        let marker = format!("@{}", p.title.to_lowercase());
        let Some(idx) = lower.find(&marker) else {
            continue;
        };
        let prev_ok = lower[..idx]
            .chars()
            .last()
            .is_none_or(|c| c.is_whitespace());
        let next_ok = lower[idx + marker.len()..]
            .chars()
            .next()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        if !(prev_ok && next_ok) {
            continue;
        }
        // Remove the mention at the same byte range in the original string.
        let byte_idx = title.to_lowercase().find(&marker).unwrap_or(idx);
        let end = byte_idx + marker.len();
        if !title.is_char_boundary(byte_idx) || !title.is_char_boundary(end) {
            return (title.to_string(), Some(p.id.to_string()));
        }
        let clean = format!("{}{}", &title[..byte_idx], &title[end..]);
        let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
        return (
            if clean.is_empty() {
                title.to_string()
            } else {
                clean
            },
            Some(p.id.to_string()),
        );
    }
    (title.to_string(), None)
}

/// Simplified parseQuickEntryCapture: handles Russian date keywords
/// (сегодня/завтра/послезавтра, weekday names → next occurrence, "через N дн/нед").
/// Full chrono-node parity is out of scope (documented gap).
pub fn parse_quick_entry_capture(
    title: &str,
    selected: Option<String>,
) -> (String, Option<String>) {
    if selected.is_some() {
        return (title.to_string(), selected);
    }
    let lower = title.to_lowercase();
    let today = Local::now().date_naive();
    let words: Vec<(usize, &str)> = lower
        .split_whitespace()
        .scan(0usize, |pos, w| {
            let start = *pos;
            *pos += w.len() + 1;
            Some((start, w))
        })
        .collect();
    let mut date: Option<NaiveDate> = None;
    let mut remove_range: Option<(usize, usize)> = None;
    let ru_weekdays = [
        ("понедельник", 0u32),
        ("вторник", 1),
        ("сред", 2),
        ("четверг", 3),
        ("пятниц", 4),
        ("суббот", 5),
        ("воскресень", 6),
    ];
    for (i, (start, w)) in words.iter().enumerate() {
        let end = *start + w.len();
        let trimmed = w.trim_matches(|c: char| !c.is_alphanumeric());
        let d = match trimmed {
            "сегодня" => Some(today),
            "завтра" => Some(today + Duration::days(1)),
            "послезавтра" => Some(today + Duration::days(2)),
            _ if trimmed.starts_with("через") || trimmed == "через" => None,
            _ => ru_weekdays
                .iter()
                .find(|(name, _)| trimmed.starts_with(name))
                .map(|(_, wd)| {
                    let cur = today.weekday().num_days_from_monday();
                    let delta = (7 + wd - cur) % 7;
                    today + Duration::days(if delta == 0 { 7 } else { delta } as i64)
                }),
        };
        if let Some(d) = d {
            date = Some(d);
            remove_range = Some((*start, end));
            break;
        }
        if trimmed == "через" {
            if let Some((s2, w2)) = words.get(i + 1) {
                if let Ok(n) = w2
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .parse::<i64>()
                {
                    let unit = words
                        .get(i + 2)
                        .map(|(_, u)| u.trim_matches(|c: char| !c.is_alphanumeric()))
                        .unwrap_or("дня");
                    let days = if unit.starts_with("недел") {
                        n * 7
                    } else {
                        n
                    };
                    date = Some(today + Duration::days(days));
                    let e = words
                        .get(i + 2)
                        .map(|(s, u)| s + u.len())
                        .unwrap_or(s2 + w2.len());
                    remove_range = Some((*start, e));
                    break;
                }
            }
        }
    }
    let Some(d) = date else {
        return (title.to_string(), None);
    };
    let clean = if let Some((s, e)) = remove_range {
        let mut t = String::new();
        if s <= title.len()
            && e <= title.len()
            && title.is_char_boundary(s)
            && title.is_char_boundary(e)
        {
            t.push_str(&title[..s]);
            t.push_str(&title[e..]);
        } else {
            t = title.to_string();
        }
        t.split_whitespace().collect::<Vec<_>>().join(" ")
    } else {
        title.to_string()
    };
    (
        if clean.is_empty() {
            title.to_string()
        } else {
            clean
        },
        Some(key_of(d)),
    )
}

/// Next occurrence for a completed recurring task (createNextRecurrence subset).
pub fn next_recurrence_date(rule: &RecurrenceRule, t: &Todo) -> String {
    let base = task_date(t)
        .0
        .and_then(|d| parse_key(&d))
        .unwrap_or_else(|| Local::now().date_naive());
    let next = match rule.frequency {
        0 => base + Duration::days(rule.interval as i64),
        1 => {
            if rule.days_of_week.is_empty() {
                base + Duration::weeks(rule.interval as i64)
            } else {
                // Next selected weekday strictly after base.
                let mut d = base + Duration::days(1);
                for _ in 0..8 {
                    let wd = d.weekday().num_days_from_monday() as u8 + 1;
                    if rule.days_of_week.contains(&wd) {
                        break;
                    }
                    d += Duration::days(1);
                }
                d
            }
        }
        2 => {
            let mut d = base;
            for _ in 0..rule.interval {
                d = d + chrono::Months::new(1);
            }
            d
        }
        _ => {
            let mut d = base;
            for _ in 0..rule.interval {
                d = d + chrono::Months::new(12);
            }
            d
        }
    };
    key_of(next)
}

// ---------------------------------------------------------------------------
// Seed data (src/dev/seed.ts + src/dev/pseudoCalendar.ts)
// ---------------------------------------------------------------------------

pub fn new_todo(id: impl Into<String>, title: &str) -> Todo {
    todo(id, title)
}

fn todo(id: impl Into<String>, title: &str) -> Todo {
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

pub fn seed_todos() -> Vec<Todo> {
    let mut v: Vec<Todo> = vec![
        todo("dev-inbox-1", "Перечитать бриф по лендингу"),
        {
            let mut t = todo("dev-inbox-2", "Записаться к стоматологу");
            t.priority = 1;
            t
        },
        todo("dev-inbox-3", "Идея: виджет погоды в сайдбаре"),
        {
            let mut t = todo("dev-today-1", "Созвон по дизайну");
            t.status = Status::Todo;
            t.is_today = true;
            t.priority = 3;
            t.tag_ids = vec!["dev-tag-urgent".into(), "dev-tag-focus".into()];
            t.fuel_cost = Some(35.0);
            t
        },
        {
            let mut t = todo("dev-today-2", "Отправить инвойс клиенту");
            t.status = Status::Todo;
            t.scheduled_date = Some(day_key(0));
            t.deadline = Some(day_key(0));
            t.billable = true;
            t.price = Some(15000.0);
            t.project_id = Some("dev-proj-release".into());
            t
        },
        {
            let mut t = todo("dev-today-evening", "Почитать главу книги");
            t.status = Status::Todo;
            t.is_today = true;
            t.is_evening = true;
            t.area_id = Some("dev-area-life".into());
            t
        },
        {
            let mut t = todo("dev-overdue-1", "Вернуть правки по макетам");
            t.status = Status::Todo;
            t.scheduled_date = Some(day_key(-1));
            t.priority = 3;
            t.tag_ids = vec!["dev-tag-urgent".into()];
            t.fuel_cost = Some(20.0);
            t
        },
        {
            let mut t = todo("dev-checklist-1", "Подготовить демо для команды");
            t.status = Status::Todo;
            t.scheduled_date = Some(day_key(1));
            t.project_id = Some("dev-proj-release".into());
            t.heading_id = Some("dev-head-prep".into());
            t.checklist = vec![
                ChecklistItem { is_completed: true },
                ChecklistItem {
                    is_completed: false,
                },
                ChecklistItem {
                    is_completed: false,
                },
            ];
            t
        },
        {
            let mut t = todo("dev-week-2", "Ревью PR по навигации");
            t.status = Status::Started;
            t.scheduled_date = Some(day_key(2));
            t.project_id = Some("dev-proj-release".into());
            t.heading_id = Some("dev-head-polish".into());
            t.significance = Some(7);
            t.fuel_cost = Some(15.0);
            t
        },
        {
            let mut t = todo("dev-week-3", "Оплатить интернет");
            t.status = Status::Todo;
            t.deadline = Some(day_key(3));
            t.reminder_date = Some(iso_at(3, 9, 0));
            t.project_id = Some("dev-proj-home".into());
            t
        },
        {
            let mut t = todo("dev-week-4", "Купить подарок Маше");
            t.status = Status::Todo;
            t.scheduled_date = Some(day_key(4));
            t.area_id = Some("dev-area-life".into());
            t.tag_ids = vec!["dev-tag-home".into()];
            t
        },
        {
            let mut t = todo("dev-recurring-1", "Поливать цветы");
            t.status = Status::Todo;
            t.scheduled_date = Some(day_key(0));
            t.area_id = Some("dev-area-life".into());
            t.recurrence = Some(RecurrenceRule {
                frequency: 1,
                interval: 1,
                recurrence_type: 0,
                days_of_week: vec![1, 3, 5],
            });
            t
        },
        {
            let mut t = todo("dev-next-week-1", "Запланировать отпуск");
            t.status = Status::Todo;
            t.scheduled_date = Some(day_key(8));
            t.priority = 2;
            t
        },
        {
            let mut t = todo("dev-someday-1", "Собрать домашний кинотеатр");
            t.status = Status::Deferred;
            t.is_someday = true;
            t.project_id = Some("dev-proj-someday".into());
            t
        },
        {
            let mut t = todo("dev-done-today", "Утренняя планёрка");
            t.status = Status::Done;
            t.is_completed = true;
            t.completed_at = Some(iso_at(0, 9, 30));
            t.scheduled_date = Some(day_key(0));
            t
        },
        {
            let mut t = todo("dev-done-yesterday", "Позвонить в банк");
            t.status = Status::Done;
            t.is_completed = true;
            t.completed_at = Some(iso_at(-1, 14, 0));
            t
        },
        {
            let mut t = todo("dev-canceled-1", "Встреча с брокером");
            t.status = Status::Canceled;
            t.is_cancelled = true;
            t.completed_at = Some(iso_at(0, 11, 0));
            t.scheduled_date = Some(day_key(1));
            t
        },
        {
            let mut t = todo("dev-trash-1", "Черновик: старый план переезда");
            t.is_trashed = true;
            t
        },
    ];
    v[0].notes = Some("Обратить внимание на секцию про ценообразование.".into());
    let created = Local::now().to_rfc3339();
    for (i, t) in v.iter_mut().enumerate() {
        t.created_at = created.clone();
        t.sort_order = i as i32;
    }
    v
}

/// Dev tool: `n` todos with random-character titles and randomized fields
/// (xorshift64*, no external deps). ~55% are completed, spread over the past
/// year so the statistics heatmap has data to show. Ids are namespaced as
/// `dev-gen-{batch}-{i}` so batches never collide.
pub fn gen_random_todos(n: usize, seed: u64, batch: u64, first_sort: i32) -> Vec<Todo> {
    const PROJECTS: [&str; 3] = ["dev-proj-release", "dev-proj-home", "dev-proj-someday"];
    const TAGS: [&str; 3] = ["dev-tag-urgent", "dev-tag-focus", "dev-tag-home"];

    let mut s = (seed | 1).wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut rng = move || {
        s ^= s >> 12;
        s ^= s << 25;
        s ^= s >> 27;
        s = s.wrapping_mul(0x2545_F491_4F6C_DD1D);
        s
    };
    fn rand_chars(rng: &mut impl FnMut() -> u64, words_max: u64) -> String {
        let words = 1 + rng() % words_max;
        (0..words)
            .map(|_| {
                let len = 2 + rng() % 9;
                (0..len)
                    .map(|_| (b'a' + (rng() % 26) as u8) as char)
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    (0..n)
        .map(|i| {
            let mut t = todo(format!("dev-gen-{batch}-{i}"), &rand_chars(&mut rng, 4));
            t.created_at = iso_at(
                -((rng() % 365) as i64),
                (rng() % 24) as u32,
                (rng() % 60) as u32,
            );
            t.sort_order = first_sort + i as i32;
            if rng() % 100 < 55 {
                t.status = Status::Done;
                t.is_completed = true;
                t.completed_at = Some(iso_at(
                    -((rng() % 365) as i64),
                    (rng() % 24) as u32,
                    (rng() % 60) as u32,
                ));
            } else {
                t.status = match rng() % 100 {
                    0..=19 => Status::Inbox,
                    20..=59 => Status::Todo,
                    60..=74 => Status::Started,
                    75..=89 => Status::Deferred,
                    _ => Status::Canceled,
                };
                match t.status {
                    Status::Canceled => {
                        t.is_cancelled = true;
                        t.completed_at = Some(iso_at(-((rng() % 180) as i64), 12, 0));
                    }
                    Status::Deferred => t.is_someday = rng() % 100 < 60,
                    _ => {}
                }
                if rng() % 100 < 6 {
                    t.is_today = true;
                }
                if rng() % 100 < 40 {
                    t.scheduled_date = Some(day_key((rng() % 76) as i64 - 30));
                }
                if rng() % 100 < 15 {
                    t.deadline = Some(day_key((rng() % 61) as i64 - 10));
                }
                if rng() % 100 < 3 {
                    t.is_trashed = true;
                }
            }
            if rng() % 100 < 30 {
                t.priority = 1 + (rng() % 3) as u8;
            }
            if rng() % 100 < 35 {
                t.project_id = Some(PROJECTS[(rng() % 3) as usize].into());
            }
            if rng() % 100 < 20 {
                t.tag_ids = vec![TAGS[(rng() % 3) as usize].into()];
            }
            if rng() % 100 < 15 {
                t.notes = Some(rand_chars(&mut rng, 12));
            }
            if rng() % 100 < 30 {
                t.fuel_cost = Some(5.0 + (rng() % 60) as f64);
                t.significance = Some(1 + (rng() % 10) as u8);
            }
            t
        })
        .collect()
}

pub fn seed_projects() -> Vec<Project> {
    vec![
        Project {
            id: "dev-proj-release".into(),
            title: "Релиз Agenda 1.0".into(),
            status: 0,
            deadline: Some(day_key(10)),
            sort_order: 0,
            area_id: Some("dev-area-work".into()),
        },
        Project {
            id: "dev-proj-home".into(),
            title: "Дом и быт".into(),
            status: 0,
            deadline: None,
            sort_order: 1,
            area_id: Some("dev-area-life".into()),
        },
        Project {
            id: "dev-proj-someday".into(),
            title: "Путешествие в Японию".into(),
            status: 1,
            deadline: None,
            sort_order: 2,
            area_id: Some("dev-area-life".into()),
        },
    ]
}

pub fn seed_areas() -> Vec<Area> {
    vec![
        Area {
            id: "dev-area-work".into(),
            title: "Работа".into(),
            sort_order: 0,
        },
        Area {
            id: "dev-area-life".into(),
            title: "Личное".into(),
            sort_order: 1,
        },
    ]
}

pub fn seed_tags() -> Vec<Tag> {
    vec![
        Tag {
            id: "dev-tag-urgent".into(),
            title: "срочно".into(),
            color: "red".into(),
        },
        Tag {
            id: "dev-tag-focus".into(),
            title: "фокус".into(),
            color: "blue".into(),
        },
        Tag {
            id: "dev-tag-home".into(),
            title: "дом".into(),
            color: "green".into(),
        },
    ]
}

pub fn seed_headings() -> Vec<Heading> {
    vec![
        Heading {
            id: "dev-head-prep".into(),
            title: "Подготовка".into(),
            sort_order: 0,
            project_id: Some("dev-proj-release".into()),
        },
        Heading {
            id: "dev-head-polish".into(),
            title: "Полировка".into(),
            sort_order: 1,
            project_id: Some("dev-proj-release".into()),
        },
    ]
}

pub fn seed_events() -> Vec<CalEvent> {
    vec![
        CalEvent {
            id: "dev-event-standup",
            title: "Ежедневный стендап",
            starts_at: iso_at(0, 10, 0),
            ends_at: Some(iso_at(0, 10, 30)),
            location: Some("Zoom"),
        },
        CalEvent {
            id: "dev-event-lunch",
            title: "Обед с Аней",
            starts_at: iso_at(0, 13, 0),
            ends_at: Some(iso_at(0, 14, 0)),
            location: Some("Кафе на углу"),
        },
        CalEvent {
            id: "dev-event-gym",
            title: "Спортзал",
            starts_at: iso_at(0, 18, 30),
            ends_at: Some(iso_at(0, 19, 30)),
            location: None,
        },
        CalEvent {
            id: "dev-event-design-review",
            title: "Дизайн-ревью",
            starts_at: iso_at(1, 11, 0),
            ends_at: Some(iso_at(1, 12, 30)),
            location: Some("Переговорка 2"),
        },
        CalEvent {
            id: "dev-event-one-on-one",
            title: "1:1 с руководителем",
            starts_at: iso_at(2, 15, 0),
            ends_at: Some(iso_at(2, 16, 0)),
            location: None,
        },
        CalEvent {
            id: "dev-event-planning",
            title: "Планирование спринта",
            starts_at: iso_at(3, 9, 0),
            ends_at: Some(iso_at(3, 10, 30)),
            location: Some("Zoom"),
        },
        CalEvent {
            id: "dev-event-demo",
            title: "Демо заказчику",
            starts_at: iso_at(4, 17, 0),
            ends_at: Some(iso_at(4, 18, 0)),
            location: None,
        },
        CalEvent {
            id: "dev-event-webinar",
            title: "Вебинар по Rust",
            starts_at: iso_at(5, 12, 0),
            ends_at: Some(iso_at(5, 13, 0)),
            location: None,
        },
        CalEvent {
            id: "dev-event-strategy",
            title: "Стратегическая сессия",
            starts_at: iso_at(8, 10, 0),
            ends_at: Some(iso_at(8, 16, 0)),
            location: Some("Офис"),
        },
    ]
}
