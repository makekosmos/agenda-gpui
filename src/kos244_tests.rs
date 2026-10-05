//! Regression tests for KOS-244 residual fixes: nested controls whose clicks
//! leaked into ancestor `on_click` handlers (row status chip, row actions,
//! the date-clear ×), the un-toggleable tags dropdown, the ghost-project page
//! listing unprojected tasks, and malformed Engine date stamps treated as
//! real date keys. Reuses the headless helpers from `ui_tests`.

use gpui::TestAppContext;

use crate::app::Route;
use crate::model::date_only;
use crate::ui_tests::{click, launch, redraw};

/// The status chip on a task row did open its dropdown — but the click then
/// «На неделю» moves the task into the week list — but the bubbled click also fired the
/// row's `on_click` and teleported the user into the task page. A hover
/// action must not change the route.
#[gpui::test]
fn row_action_does_not_navigate(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    agenda.update(cx, |a, _| a.navigate(Route::Week));
    redraw(cx);
    click(cx, "act-ra-week-dev-overdue-1");
    redraw(cx);
    agenda.read_with(cx, |a, _| {
        let t = a
            .todos
            .iter()
            .find(|t| t.id == "dev-overdue-1")
            .expect("dev-overdue-1 missing");
        assert_eq!(
            t.scheduled_date.as_deref(),
            None,
            "«На неделю» clears the precise date"
        );
        assert!(
            a.todos
                .iter()
                .find(|t| t.id == "dev-overdue-1")
                .is_some_and(|t| t.is_today),
            "«На неделю» must flag the task for this week"
        );
        assert!(
            matches!(a.route, Route::Week),
            "row action navigated to {:?}",
            a.route
        );
    });
}

/// The × inside the quick-entry date chip cleared the draft date AND opened
/// the date dropdown — the click bubbled into the chip's own `on_click`.
#[gpui::test]
fn qe_date_clear_does_not_open_dropdown(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    agenda.update(cx, |a, _| a.navigate(Route::Week));
    cx.simulate_keystrokes("ctrl-n");
    redraw(cx);
    // On the week route the draft seeds the «Неделя» flag, not a date.
    assert!(agenda.read_with(cx, |a, _| a.qe_week));

    click(cx, "qe-date-clear");
    redraw(cx);
    agenda.read_with(cx, |a, _| {
        assert_eq!(a.qe_date, None, "× must clear the draft date");
        assert!(!a.qe_week, "× must clear the week flag");
        assert!(a.qe_date_touched);
        assert!(
            a.dropdown.is_none(),
            "clearing the date must not open the picker"
        );
    });
}

/// Same defect on the task page: the × inside the date chip cleared the
/// date and immediately reopened the date picker.
#[gpui::test]
fn task_date_clear_does_not_reopen_dropdown(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    agenda.update(cx, |a, _| a.navigate(Route::Week));
    redraw(cx);
    click(cx, "tr-dev-today-2");
    agenda.read_with(cx, |a, _| {
        assert!(
            a.quick_entry_open && a.qe_task.as_deref() == Some("dev-today-2"),
            "row click must open the task in the right panel"
        );
    });

    // Edit panel has no × on the date row — clear via the dropdown's
    // «Без даты» row instead.
    click(cx, "qe-date");
    redraw(cx);
    click(cx, "dd-2");
    redraw(cx);
    agenda.read_with(cx, |a, _| {
        let t = a
            .todos
            .iter()
            .find(|t| t.id == "dev-today-2")
            .expect("dev-today-2 missing");
        assert!(t.scheduled_date.is_none() && t.deadline.is_none());
        assert!(
            a.dropdown.is_none(),
            "clearing the date must not reopen the picker"
        );
    });
}

/// A project route whose id resolves to nothing matched
/// `t.project_id == None`, so the ghost page listed every unprojected task
/// under a «Проект» header instead of an empty state.
#[gpui::test]
fn unknown_project_shows_no_tasks(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    agenda.update(cx, |a, _| a.navigate(Route::Project("ghost".into())));
    redraw(cx);
    assert!(
        cx.debug_bounds("tr-dev-inbox-1").is_none(),
        "a nonexistent project must not list unprojected tasks"
    );
}

/// `date_only` took the first 10 chars of any ≥10-char stamp, so a malformed
/// `scheduledAt`/`deadline` written by another client became a real date
/// key: the task left Inbox and landed in a phantom lexicographic group.
/// Only a parseable YYYY-MM-DD prefix may count as a date.
#[test]
fn malformed_date_stamps_do_not_parse() {
    let some = |v: &str| Some(v.to_string());
    assert_eq!(
        date_only(&some("2026-06-01")).as_deref(),
        Some("2026-06-01")
    );
    assert_eq!(
        date_only(&some("2026-06-01T10:00:00")).as_deref(),
        Some("2026-06-01")
    );
    assert_eq!(date_only(&some("понедельник")), None);
    assert_eq!(date_only(&some("not-a-date!!")), None);
    assert_eq!(date_only(&some("2026-02-30")), None);
    assert_eq!(date_only(&some("коротко")), None);
    assert_eq!(date_only(&some("")), None);
}
