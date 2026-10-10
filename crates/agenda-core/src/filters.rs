// Status / predicates (taskLifecycle.ts + todoFilterService.ts).
use crate::dates::{date_only, parse_key, task_date, LocalDay};
use crate::types::{Status, Todo};
use chrono::NaiveDate;

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

pub fn is_overdue(t: &Todo, day: &LocalDay) -> bool {
    if !is_active(t) || is_inbox(t) || is_deferred(t) || t.system_kind.is_some() {
        return false;
    }
    let (date, conflict) = task_date(t, day);
    !conflict
        && date
            .as_deref()
            .is_some_and(|d| d < day.today_key().as_str())
}

/// Dorofeev-style week membership: no precise day needed — a task belongs to
/// the week when its date falls inside it or it carries the `is_today`
/// ("this week") flag with no date at all.
/// Active, non-deferred task whose date falls inside the given week bounds.
fn is_due_in_bounds(t: &Todo, week: &(String, String), day: &LocalDay) -> bool {
    let (date, _) = task_date(t, day);
    date.as_deref()
        .is_some_and(|d| d >= week.0.as_str() && d <= week.1.as_str())
        && is_active(t)
        && !is_deferred(t)
}

pub fn is_due_this_week(t: &Todo, week: &(String, String), day: &LocalDay) -> bool {
    if !is_active(t) || is_inbox(t) || is_deferred(t) {
        return false;
    }
    let (date, conflict) = task_date(t, day);
    !conflict
        && (date
            .as_deref()
            .is_some_and(|d| d >= week.0.as_str() && d <= week.1.as_str())
            || (date.is_none() && t.is_today))
}

/// Week list body below the overdue block: a task dated earlier this week is
/// already listed as overdue, so it must not appear a second time.
fn in_week_list(t: &Todo, week: &(String, String), day: &LocalDay) -> bool {
    (is_due_this_week(t, week, day) && !is_overdue(t, day)) || completed_in_week(t, week, day)
}

pub fn completed_in_week(t: &Todo, week: &(String, String), day: &LocalDay) -> bool {
    task_status(t) == Status::Done
        && !t.is_trashed
        && date_only(&t.completed_at, day)
            .as_deref()
            .is_some_and(|d| d >= week.0.as_str() && d <= week.1.as_str())
}

/// The local day a task was completed, for statistics aggregation. Only
/// genuinely-done tasks count: canceled tasks carry a `completed_at` stamp
/// (it doubles as "closed at") but are not completions, and trashed tasks
/// are excluded from every count.
pub fn completed_day(t: &Todo, day: &LocalDay) -> Option<NaiveDate> {
    if task_status(t) != Status::Done || t.is_trashed {
        return None;
    }
    date_only(&t.completed_at, day).and_then(|k| parse_key(&k))
}

pub fn is_archived(t: &Todo, day: &LocalDay) -> bool {
    if task_status(t) != Status::Done || t.is_trashed {
        return false;
    }
    match date_only(&t.completed_at, day) {
        Some(d) => d.as_str() < day.today_key().as_str(),
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SmartList {
    Inbox,
    Week,
    NextWeek,
    Someday,
    Logbook,
    Trash,
}

/// Index-based twin of `filter_todos`: returns positions into `todos`
/// without cloning any `Todo` — the hot path for virtualized lists.
pub fn filter_idx(list: SmartList, todos: &[Todo], day: &LocalDay) -> Vec<usize> {
    if list == SmartList::Week {
        let week = day
            .week_bounds()
            .unwrap_or_else(|| (day.today_key(), day.today_key()));
        let mut v: Vec<usize> = todos
            .iter()
            .enumerate()
            .filter(|(_, t)| is_overdue(t, day))
            .map(|(i, _)| i)
            .collect();
        v.sort_by_cached_key(|&i| {
            (
                task_date(&todos[i], day).0.unwrap_or_default(),
                todos[i].sort_order,
            )
        });
        let mut due: Vec<usize> = todos
            .iter()
            .enumerate()
            .filter(|(_, t)| in_week_list(t, &week, day))
            .map(|(i, _)| i)
            .collect();
        due.sort_by_key(|&i| todos[i].sort_order);
        v.extend(due);
        return v;
    }
    if list == SmartList::NextWeek {
        let week = day
            .next_week_bounds()
            .unwrap_or_else(|| (day.today_key(), day.today_key()));
        let mut v: Vec<usize> = todos
            .iter()
            .enumerate()
            .filter(|(_, t)| is_due_in_bounds(t, &week, day))
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
            SmartList::Logbook => is_archived(t, day),
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

/// Index-based twin of `sorted`; `sort_by_cached_key` so date/title keys
/// are computed once per item instead of once per comparison.
pub fn sort_idx(todos: &[Todo], mut items: Vec<usize>, key: SortKey, day: &LocalDay) -> Vec<usize> {
    match key {
        SortKey::Default => {}
        SortKey::Date => items.sort_by_cached_key(|&i| {
            (
                task_date(&todos[i], day).0.unwrap_or_else(|| "9999".into()),
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

/// «Сегодня» board: the Week pipeline narrowed to what the day actually
/// holds — the overdue block first (same sort), then tasks dated today.
/// Android shows this as its own list; desktop folds it into «Эта неделя».
pub fn today_idx(todos: &[Todo], day: &LocalDay) -> Vec<usize> {
    let week = filter_idx(SmartList::Week, todos, day);
    let overdue: Vec<usize> = week
        .iter()
        .copied()
        .filter(|&i| is_overdue(&todos[i], day))
        .collect();
    let due: Vec<usize> = week
        .iter()
        .copied()
        .filter(|&i| {
            !is_overdue(&todos[i], day)
                && task_date(&todos[i], day).0.as_deref() == Some(day.today_key().as_str())
        })
        .collect();
    overdue.into_iter().chain(due).collect()
}

/// Project page membership (desktop `project_page`): tasks in the project,
/// not trashed, not archived — `sort_order` board order.
pub fn project_page_idx(project_id: &str, todos: &[Todo], day: &LocalDay) -> Vec<usize> {
    let mut v: Vec<usize> = todos
        .iter()
        .enumerate()
        .filter(|(_, t)| {
            t.project_id.as_deref() == Some(project_id) && !t.is_trashed && !is_archived(t, day)
        })
        .map(|(i, _)| i)
        .collect();
    v.sort_by_key(|&i| todos[i].sort_order);
    v
}

/// Tag filter membership: live tasks carrying the tag (trash hidden),
/// `sort_order` board order.
pub fn tag_page_idx(tag_id: &str, todos: &[Todo]) -> Vec<usize> {
    let mut v: Vec<usize> = todos
        .iter()
        .enumerate()
        .filter(|(_, t)| !t.is_trashed && t.tag_ids.iter().any(|id| id == tag_id))
        .map(|(i, _)| i)
        .collect();
    v.sort_by_key(|&i| todos[i].sort_order);
    v
}
