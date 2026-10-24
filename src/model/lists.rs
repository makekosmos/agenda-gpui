use super::*;
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
