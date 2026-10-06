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
    // The someday flag is only meaningful on a live task: `complete_todo`
    // leaves it set on the finished copy, and Engine data from other clients
    // can arrive as Done/Canceled + is_someday — either way a closed task
    // must not keep a row in «Потом».
    is_active(t) && (task_status(t) == Status::Deferred || t.is_someday)
}

pub fn is_overdue(t: &Todo, today: &str) -> bool {
    if !is_active(t) || is_inbox(t) || is_deferred(t) || t.system_kind.is_some() {
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

/// The local day a task was completed, for statistics aggregation. Only
/// genuinely-done tasks count: canceled tasks carry a `completed_at` stamp
/// (it doubles as "closed at") but are not completions, and trashed tasks
/// are excluded from every count.
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
    Week,
    NextWeek,
    Someday,
    Logbook,
    Trash,
}

/// Index-based twin of [`filter_todos`]: returns positions into `todos`
/// without cloning any `Todo` — the hot path for virtualized lists.
pub fn filter_idx(list: SmartList, todos: &[Todo]) -> Vec<usize> {
    let today = today_key();
    if list == SmartList::Week {
        let week = week_bounds(&today).unwrap_or_else(|| (today.clone(), today.clone()));
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
            .filter(|(_, t)| in_week_list(t, &week, &today))
            .map(|(i, _)| i)
            .collect();
        due.sort_by_key(|&i| todos[i].sort_order);
        v.extend(due);
        return v;
    }
    if list == SmartList::NextWeek {
        let week = next_week_bounds(&today).unwrap_or_else(|| (today.clone(), today.clone()));
        let mut v: Vec<usize> = todos
            .iter()
            .enumerate()
            .filter(|(_, t)| is_due_in_bounds(t, &week))
            .map(|(i, _)| i)
            .collect();
        v.sort_by_key(|&i| todos[i].sort_order);
        return v;
    }
    let mut v: Vec<usize> = todos
        .iter()
        .enumerate()
        .filter(|(_, t)| match list {
            SmartList::Week | SmartList::NextWeek => unreachable!(),
            SmartList::Inbox => is_inbox(t),
            SmartList::Someday => is_deferred(t),
            SmartList::Logbook => is_archived(t, &today),
            SmartList::Trash => t.is_trashed,
        })
        .map(|(i, _)| i)
        .collect();
    match list {
        SmartList::Logbook => v.sort_by_cached_key(|&i| {
            std::cmp::Reverse(instant_key(todos[i].completed_at.as_deref()))
        }),
        SmartList::Trash => v.sort_by_cached_key(|&i| {
            std::cmp::Reverse(instant_key(Some(todos[i].created_at.as_str())))
        }),
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
    if list == SmartList::Week {
        let week = week_bounds(&today).unwrap_or_else(|| (today.clone(), today.clone()));
        let mut v = overdue_todos(todos, &today);
        let mut due: Vec<Todo> = todos
            .iter()
            .filter(|t| in_week_list(t, &week, &today))
            .cloned()
            .collect();
        due.sort_by_key(|a| a.sort_order);
        v.extend(due);
        return v;
    }
    if list == SmartList::NextWeek {
        let week = next_week_bounds(&today).unwrap_or_else(|| (today.clone(), today.clone()));
        let mut v: Vec<Todo> = todos
            .iter()
            .filter(|t| is_due_in_bounds(t, &week))
            .cloned()
            .collect();
        v.sort_by_key(|a| a.sort_order);
        return v;
    }
    let mut v: Vec<Todo> = todos
        .iter()
        .filter(|t| match list {
            SmartList::Week | SmartList::NextWeek => unreachable!(),
            SmartList::Inbox => is_inbox(t),
            SmartList::Someday => is_deferred(t),
            SmartList::Logbook => is_archived(t, &today),
            SmartList::Trash => t.is_trashed,
        })
        .cloned()
        .collect();
    v.sort_by(|a, b| match list {
        SmartList::Logbook => {
            instant_key(b.completed_at.as_deref()).cmp(&instant_key(a.completed_at.as_deref()))
        }
        SmartList::Trash => {
            instant_key(Some(b.created_at.as_str())).cmp(&instant_key(Some(a.created_at.as_str())))
        }
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

// ---------------------------------------------------------------------------
// Quick-entry parsing (parseProjectMention + simplified parseQuickEntryCapture)
// ---------------------------------------------------------------------------

/// taskLifecycle.parseProjectMention: find "@<project title>" word-bounded mention.
pub fn parse_project_mention(title: &str, projects: &[Project]) -> (String, Option<String>) {
    let mut sorted: Vec<&Project> = projects.iter().collect();
    sorted.sort_by_key(|p| std::cmp::Reverse(p.title.len()));
    for p in sorted {
        let marker = format!("@{}", p.title.to_lowercase());
        // Search in `title`'s own coordinates: a lowercased copy can differ in
        // byte length ('İ' → "i̇"), which would shift the cut range and corrupt
        // the remaining text.
        for (start, _) in title.char_indices() {
            if !title[..start]
                .chars()
                .last()
                .is_none_or(|c| c.is_whitespace())
            {
                continue;
            }
            let mut acc = String::new();
            let mut end = start;
            for (off, ch) in title[start..].char_indices() {
                acc.extend(ch.to_lowercase());
                end = start + off + ch.len_utf8();
                if acc == marker || !marker.starts_with(acc.as_str()) {
                    break;
                }
            }
            if acc != marker {
                continue;
            }
            if !title[end..]
                .chars()
                .next()
                .is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
            {
                continue;
            }
            let clean = format!("{}{}", &title[..start], &title[end..]);
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
    let today = Local::now().date_naive();
    // Word offsets must index `title` itself: separator runs can be wider than
    // one byte, and lowercasing may change byte length, so positions derived
    // from `title.to_lowercase()` do not map back onto `title`.
    let mut words: Vec<(usize, &str)> = Vec::new();
    let mut pos = 0usize;
    for w in title.split_whitespace() {
        let start = pos + title[pos..].find(w).unwrap_or(0);
        pos = start + w.len();
        words.push((start, w));
    }
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
    // Russian weekday declensions: nominative + case endings accepted after
    // each stem above (среда/среду/средой, пятницу/пятницей, воскресенья...).
    const WEEKDAY_SUFFIXES: &[&str] = &[
        "", "а", "у", "е", "и", "ы", "ю", "я", "ой", "ей", "ом", "ем", "ам", "ям",
    ];
    for (i, (start, w)) in words.iter().enumerate() {
        let end = *start + w.len();
        let lw = w.to_lowercase();
        let trimmed = lw.trim_matches(|c: char| !c.is_alphanumeric());
        let d = match trimmed {
            "сегодня" => Some(today),
            "завтра" => Some(today + Duration::days(1)),
            "послезавтра" => Some(today + Duration::days(2)),
            _ if trimmed.starts_with("через") || trimmed == "через" => None,
            _ => ru_weekdays
                .iter()
                .find(|(name, _)| {
                    // Bare prefix matching treats "средний"/"субботник" as
                    // weekday names; only declension endings may follow a stem.
                    // "среди" is the exception: a common preposition, not a
                    // declension of "среда" — the "и" ending is valid only for
                    // the other stems.
                    trimmed.strip_prefix(name).is_some_and(|s| {
                        WEEKDAY_SUFFIXES.contains(&s) && !(*name == "сред" && s == "и")
                    })
                })
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
                let numeral = w2
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .parse::<i64>()
                    .ok();
                // A bare unit word reads as n=1 ("через неделю", "через
                // час"); anything else after «через» is not a capture.
                let bare = w2.to_lowercase();
                let parsed = match numeral {
                    Some(n) => Some((n, i + 2)),
                    None if through_stem(&bare).is_some() => Some((1, i + 1)),
                    None => None,
                };
                if let Some((n, unit_i)) = parsed {
                    let unit = words
                        .get(unit_i)
                        .map(|(_, u)| u.to_lowercase())
                        .unwrap_or_default();
                    let months = |m: i64| {
                        u32::try_from(m)
                            .ok()
                            .and_then(|m| today.checked_add_months(chrono::Months::new(m)))
                    };
                    // The model stores dates only: sub-day units ("через 2
                    // часа") can only mean today. Month/year units used to
                    // fall into the days branch and scheduled N days out.
                    let target = match through_stem(&unit) {
                        Some(0) => n
                            .checked_mul(7)
                            .and_then(Duration::try_days)
                            .and_then(|d| today.checked_add_signed(d)),
                        Some(1) => months(n),
                        Some(2) => n.checked_mul(12).and_then(months),
                        Some(3) => Some(today),
                        // день/дня/дней and anything unrecognized after a
                        // numeral keep the original days fallback.
                        _ => Duration::try_days(n).and_then(|d| today.checked_add_signed(d)),
                    };
                    // `NaiveDate + duration` panics on out-of-range values —
                    // an unrepresentable offset is simply "no date captured".
                    if let Some(d) = target {
                        date = Some(d);
                        let e = words
                            .get(unit_i)
                            .map(|(s, u)| s + u.len())
                            .unwrap_or(s2 + w2.len());
                        remove_range = Some((*start, e));
                        break;
                    }
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

/// Unit stems after «через»: 0=неделя 1=месяц 2=год/лет 3=час/мин/сек 4=день.
fn through_stem(unit: &str) -> Option<u8> {
    let u = unit.trim_matches(|c: char| !c.is_alphanumeric());
    Some(match u {
        _ if u.starts_with("недел") => 0,
        _ if u.starts_with("месяц") => 1,
        _ if u.starts_with("год") || u.starts_with("лет") => 2,
        _ if u.starts_with("час") || u.starts_with("мин") || u.starts_with("секунд") => {
            3
        }
        _ if u.starts_with("дн") || u.starts_with("день") => 4,
        _ => return None,
    })
}

/// Next occurrence for a completed recurring task (createNextRecurrence
/// subset). Rules come from Engine data, so interval/weekday values are
/// untrusted: checked arithmetic keeps absurd rules from panicking or
/// looping for billions of months — they simply produce no next occurrence.
pub fn next_recurrence_date(rule: &RecurrenceRule, t: &Todo) -> Option<String> {
    // recurrenceType=1 ("после выполнения") counts the interval from the
    // completion day; type=0 ("по расписанию") chains the fixed schedule
    // regardless of when the task was actually completed.
    let base = if rule.recurrence_type == 1 {
        date_only(&t.completed_at)
            .and_then(|d| parse_key(&d))
            .unwrap_or_else(|| Local::now().date_naive())
    } else {
        task_date(t)
            .0
            .and_then(|d| parse_key(&d))
            .unwrap_or_else(|| Local::now().date_naive())
    };
    let interval = i64::from(rule.interval);
    let next = match rule.frequency {
        0 => Duration::try_days(interval).and_then(|d| base.checked_add_signed(d)),
        1 => {
            if rule.days_of_week.is_empty() {
                interval
                    .checked_mul(7)
                    .and_then(Duration::try_days)
                    .and_then(|d| base.checked_add_signed(d))
            } else {
                // Next selected weekday strictly after base.
                let mut d = base;
                let mut found = None;
                for _ in 0..8 {
                    d = d.checked_add_signed(Duration::days(1))?;
                    let wd = d.weekday().num_days_from_monday() as u8 + 1;
                    if rule.days_of_week.contains(&wd) {
                        found = Some(d);
                        break;
                    }
                }
                found
            }
        }
        2 => base.checked_add_months(chrono::Months::new(rule.interval)),
        _ => base.checked_add_months(chrono::Months::new(rule.interval.checked_mul(12)?)),
    };
    next.map(key_of)
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

#[cfg(test)]
mod tests;
