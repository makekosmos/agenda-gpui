// Smart-list membership, ordering and @project mention tests.
use agenda_core::*;
use chrono::{FixedOffset, NaiveDate};

fn day() -> LocalDay {
    LocalDay::now()
}

fn project(id: &str, title: &str) -> Project {
    Project {
        id: id.into(),
        title: title.into(),
        status: 0,
        deadline: None,
        sort_order: 0,
        area_id: None,
    }
}

#[test]
fn project_mention_basic() {
    let (clean, pid) = parse_project_mention("Купить молоко @Дом", &[project("p1", "Дом")]);
    assert_eq!(pid.as_deref(), Some("p1"));
    assert_eq!(clean, "Купить молоко");
}

#[test]
fn project_mention_handles_lowercase_expansion() {
    // 'İ' lowercases to "i̇" (2 chars / 3 UTF-8 bytes): byte offsets taken
    // from `title.to_lowercase()` do not map back onto `title`, so the cut
    // range lands mid-mention and leaves a dangling '@' (or skips the strip).
    let (clean, pid) = parse_project_mention("İX @ab!", &[project("p1", "ab")]);
    assert_eq!(pid.as_deref(), Some("p1"));
    assert_eq!(clean, "İX !");

    let (clean, pid) = parse_project_mention("İX @ab", &[project("p1", "ab")]);
    assert_eq!(pid.as_deref(), Some("p1"));
    assert_eq!(clean, "İX");
}

#[test]
fn logbook_sorts_completed_instants_not_strings() {
    // RFC 3339 instants with different offsets do not sort like the wall clock
    // as raw strings: "+03:00" compares above "Z" suffixes even though it marks
    // an *earlier* instant. The Logbook must order by the parsed instant.
    let mut earlier = new_todo("a", "done earlier (larger local clock)");
    earlier.is_completed = true;
    earlier.status = Status::Done;
    earlier.completed_at = Some("2020-01-10T23:30:00+03:00".into()); // 20:30Z
    let mut later = new_todo("b", "done later (smaller local clock)");
    later.is_completed = true;
    later.status = Status::Done;
    later.completed_at = Some("2020-01-10T21:00:00Z".into());

    let got: Vec<usize> = filter_idx(
        SmartList::Logbook,
        &[earlier.clone(), later.clone()],
        &day(),
    );
    assert_eq!(got, [1, 0], "filter_idx must order Logbook by instant");
}

/// A task dated earlier this week is overdue AND inside the week bounds; the
/// Week list used to show it twice (Tuesday through Sunday), and the two rows
/// shared one element id, so hovering them flapped forever.
#[test]
fn week_list_has_no_duplicate_tasks() {
    let day = day();
    let todos: Vec<Todo> = (-6..=6)
        .map(|offset| {
            let mut t = new_todo(format!("t{offset}"), "task");
            t.status = Status::Todo;
            t.scheduled_date = Some(day.day_key(offset));
            t
        })
        .collect();
    let mut seen = filter_idx(SmartList::Week, &todos, &day);
    assert!(!seen.is_empty());
    let listed = seen.len();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), listed, "Week list repeats a task");
}

/// Pinned-day smart list membership: on 2026-10-10 (Saturday) the week is
/// Oct 5–11, next week Oct 12–18.
#[test]
fn smart_lists_on_a_fixed_date() {
    let day = LocalDay::new(
        NaiveDate::parse_from_str("2026-10-10", "%Y-%m-%d").unwrap(),
        FixedOffset::east_opt(0).unwrap(),
    );
    let mut inbox = new_todo("i", "inbox");
    inbox.status = Status::Inbox;

    let mut overdue = new_todo("o", "overdue");
    overdue.status = Status::Todo;
    overdue.scheduled_date = Some("2026-10-09".into());

    let mut in_week = new_todo("w", "this week");
    in_week.status = Status::Todo;
    in_week.scheduled_date = Some("2026-10-11".into());

    let mut next_week = new_todo("n", "next week");
    next_week.status = Status::Todo;
    next_week.scheduled_date = Some("2026-10-13".into());

    let mut someday = new_todo("s", "someday");
    someday.status = Status::Deferred;
    someday.is_someday = true;

    let mut done = new_todo("d", "archived");
    done.status = Status::Done;
    done.is_completed = true;
    done.completed_at = Some("2026-10-08T10:00:00Z".into());

    let mut trash = new_todo("t", "trash");
    trash.is_trashed = true;

    let todos = vec![inbox, overdue, in_week, next_week, someday, done, trash];
    let names = |list| {
        filter_idx(list, &todos, &day)
            .iter()
            .map(|&i| todos[i].id.as_str())
            .collect::<Vec<_>>()
    };
    assert_eq!(names(SmartList::Inbox), ["i"]);
    assert_eq!(names(SmartList::Week), ["o", "w", "d"]);
    assert_eq!(names(SmartList::NextWeek), ["n"]);
    assert_eq!(names(SmartList::Someday), ["s"]);
    assert_eq!(names(SmartList::Logbook), ["d"]);
    assert_eq!(names(SmartList::Trash), ["t"]);
}
