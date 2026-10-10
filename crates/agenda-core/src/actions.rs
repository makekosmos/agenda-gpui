//! Write actions shared by the desktop app (`app.rs`) and the Android port:
//! every mutation a task can take lives here once, so both platforms apply
//! identical field-level rules. Functions mutate a `Todo` in place; the
//! caller owns persistence (`save` with a `before` precondition).

use crate::dates::LocalDay;
use crate::filters::task_status;
use crate::quick_entry::{parse_project_mention, parse_quick_entry_capture};
use crate::recurrence::next_recurrence_date;
use crate::types::{Project, Status, Todo};

/// `set_todo_status` non-Done arm: status plus every derived flag.
/// `now` is an RFC3339 instant — Canceled still stamps `completed_at`.
pub fn set_status(t: &mut Todo, status: Status, now: &str) {
    t.status = status;
    t.is_completed = false;
    t.completed_at = if status == Status::Canceled {
        Some(now.to_string())
    } else {
        None
    };
    t.is_cancelled = status == Status::Canceled;
    t.is_someday = status == Status::Deferred;
    t.is_today = false;
}

/// `complete_todo`'s mutation of the completed task itself.
pub fn complete_fields(t: &mut Todo, now: &str) {
    t.status = Status::Done;
    t.is_completed = true;
    t.is_cancelled = false;
    t.completed_at = Some(now.to_string());
}

/// The next instance `complete_todo` spawns for a recurring task, or None
/// when the rule yields no date. `before` is the pre-completion snapshot —
/// the clone source, exactly as desktop builds it.
pub fn next_recurrence_todo(
    before: &Todo,
    completed: &Todo,
    day: &LocalDay,
    id: String,
    sort_order: i32,
    now: &str,
) -> Option<Todo> {
    let rule = completed.recurrence.as_ref()?;
    let mut next = before.clone();
    next.id = id;
    next.status = Status::Todo;
    next.scheduled_date = Some(next_recurrence_date(rule, completed, day)?);
    next.completed_at = None;
    next.is_completed = false;
    next.is_cancelled = false;
    next.is_today = false;
    next.is_evening = false;
    next.is_someday = false;
    next.deadline = None;
    next.reminder_date = None;
    next.checklist.clear();
    next.created_at = now.to_string();
    next.sort_order = sort_order;
    Some(next)
}

/// «На неделю» — into the week list, no precise date (Dorofeev week).
pub fn move_to_week(t: &mut Todo) {
    t.status = Status::Todo;
    t.scheduled_date = None;
    t.deadline = None;
    t.is_today = true;
    t.is_someday = false;
}

/// `move_to_project`: a project is a commitment, so an Inbox task becomes
/// Todo and someday is cleared.
pub fn move_to_project(t: &mut Todo, project_id: Option<String>) {
    t.project_id = project_id;
    if task_status(t) == Status::Inbox {
        t.status = Status::Todo;
    }
    t.is_someday = false;
}

/// `SetDate`: pick or clear the scheduled date; deadline resets with it.
pub fn set_scheduled_date(t: &mut Todo, date: Option<String>) {
    t.scheduled_date = date;
    t.deadline = None;
}

/// Trash / restore (`TrashTodo` / `RestoreTodo`).
pub fn set_trashed(t: &mut Todo, trashed: bool) {
    t.is_trashed = trashed;
}

/// Title/notes edit from the task panel (`panel_persist`): an emptied title
/// keeps the old one, emptied notes clear them.
pub fn edit_title_notes(t: &mut Todo, title: &str, notes: Option<String>) {
    let title = title.trim();
    if !title.is_empty() {
        t.title = title.to_string();
    }
    t.notes = notes;
}

/// What `quick_entry_save` derived from the draft before stamping a Todo:
/// parsed title/date, resolved project and the lifecycle fields.
pub struct QuickEntryDecision {
    pub title: String,
    pub scheduled_date: Option<String>,
    pub project_id: Option<String>,
    pub status: Status,
    pub is_today: bool,
    pub is_someday: bool,
}

/// `quick_entry_save` field logic (app.rs): date words → `scheduled_date`,
/// `@mention` → project unless a chip was explicitly picked, and the status
/// rule — any commitment (date, project, week flag) leaves Inbox; someday
/// wins over everything.
// Positional args mirror the desktop draft fields one-to-one and are the
// UniFFI-facing signature agenda-android's rust/ calls; bundling them into a
// struct is a cross-repo API change, not part of this gate fix.
#[allow(clippy::too_many_arguments)]
pub fn quick_entry_decision(
    title: &str,
    selected_date: Option<String>,
    week: bool,
    someday: bool,
    project: Option<String>,
    project_touched: bool,
    projects: &[Project],
    day: &LocalDay,
) -> QuickEntryDecision {
    let captured = parse_quick_entry_capture(title, selected_date, day);
    let mut scheduled = captured.1;
    let (clean_title, parsed_pid) = parse_project_mention(&captured.0, projects);
    let project = if project_touched {
        project
    } else {
        project.or(parsed_pid)
    };
    let mut status = if scheduled.is_some() || project.is_some() || week {
        Status::Todo
    } else {
        Status::Inbox
    };
    let mut is_someday = false;
    if someday {
        status = Status::Deferred;
        is_someday = true;
        scheduled = None;
    }
    QuickEntryDecision {
        title: clean_title,
        scheduled_date: scheduled,
        project_id: project,
        status,
        is_today: week,
        is_someday,
    }
}
