//! Regression tests for KOS-171 fixes: quick-entry draft lifecycle and
//! task-page input ownership. Reuses the headless helpers from `ui_tests`.

use gpui::TestAppContext;

use crate::app::Route;
use crate::model::{day_key, RecurrenceRule, Status};
use crate::ui_tests::{click, launch, redraw, route_of, type_text};

/// A saved quick-entry draft must not survive reopening: the overlay kept the
/// previous title/notes text, so Enter on a reopened form silently duplicated
/// the last task (and, because `qe_focused` stayed true, the field was not
/// even focused).
#[gpui::test]
fn quick_entry_reopens_with_empty_draft(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    let before = agenda.read_with(cx, |a, _| a.todos.len());

    cx.simulate_keystrokes("ctrl-n");
    redraw(cx);
    type_text(cx, "Первая задача завтра");
    cx.simulate_keystrokes("enter");
    // One painted frame: the closed overlay releases focus back to the shell.
    redraw(cx);

    cx.simulate_keystrokes("ctrl-n");
    redraw(cx);
    agenda.read_with(cx, |a, cx| {
        assert!(
            a.quick_entry_open,
            "ctrl-n after save must reopen quick entry"
        );
        let title = a.inputs["qe-title"].read(cx).value().to_string();
        let notes = a.inputs["qe-notes"].read(cx).value().to_string();
        assert!(title.is_empty(), "reopened quick entry kept: {title:?}");
        assert!(notes.is_empty(), "reopened quick entry kept: {notes:?}");
        assert_eq!(a.qe_date, None);
        assert!(!a.qe_billable);
    });

    // Enter on the empty form must not recreate the just-saved task.
    cx.simulate_keystrokes("enter");
    agenda.read_with(cx, |a, _| {
        assert_eq!(
            a.todos.len(),
            before + 1,
            "reopen must not duplicate a task"
        );
    });
}

/// The FAB (+) is a second entry point into quick entry and used to skip the
/// draft reset that Ctrl+N performs, leaking chips from a previous draft.
#[gpui::test]
fn quick_entry_fab_resets_stale_chips(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);

    agenda.update(cx, |a, _| {
        a.qe_date = Some(day_key(3));
        a.qe_date_touched = true;
        a.qe_billable = true;
    });
    click(cx, "fab-new");
    redraw(cx);

    agenda.read_with(cx, |a, _| {
        assert!(a.quick_entry_open);
        assert_eq!(a.qe_date, None);
        assert!(!a.qe_billable);
    });
}

/// Clearing a task field mid-edit must stick: task_page reseeded the inputs
/// from the model on every repaint where the field was empty, so deleted text
/// resurrected and the next keystrokes were appended to it.
#[gpui::test]
fn task_fields_stay_empty_after_user_clears_them(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    agenda.update(cx, |a, _| {
        a.update_todo("dev-inbox-1", |t| t.notes = Some("заметка".into()));
    });
    click(cx, "tr-dev-inbox-1");
    assert_eq!(route_of(cx, &agenda), Route::Task("dev-inbox-1".into()));

    cx.update(|window, cx| {
        let title_state = agenda.read(cx).inputs["task-title"].clone();
        let notes_state = agenda.read(cx).notes_input.clone().unwrap();
        title_state.update(cx, |s, cx| s.set_value("", window, cx));
        notes_state.update(cx, |s, cx| s.set_value("", window, cx));
    });
    redraw(cx);

    agenda.read_with(cx, |a, cx| {
        let title = a.inputs["task-title"].read(cx).value().to_string();
        let notes = a.notes_input.as_ref().unwrap().read(cx).value().to_string();
        assert!(title.is_empty(), "cleared title refilled with {title:?}");
        assert!(notes.is_empty(), "cleared notes refilled with {notes:?}");
    });
}

/// Typing "@<project>" in quick entry assigns the project via the Change
/// handler, but the mention text must also be stripped from the saved title —
/// the cleaned string was computed and then discarded, so the task kept the
/// raw "@project" markup in its name.
#[gpui::test]
fn quick_entry_strips_project_mention_on_save(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);

    cx.simulate_keystrokes("ctrl-n");
    redraw(cx);
    type_text(cx, "Купить молоко @Дом и быт");
    agenda.read_with(cx, |a, _| {
        assert_eq!(
            a.qe_project.as_deref(),
            Some("dev-proj-home"),
            "typing the mention must select the project chip"
        );
    });

    cx.simulate_keystrokes("enter");
    agenda.read_with(cx, |a, _| {
        let t = a
            .todos
            .iter()
            .find(|t| t.title.contains("молоко"))
            .expect("saved task missing");
        assert_eq!(
            t.project_id.as_deref(),
            Some("dev-proj-home"),
            "mention-selected project lost"
        );
        assert_eq!(t.title, "Купить молоко", "mention leaked into the title");
    });
}

/// Trash rows are read-only (`editable = false`), but the status ring ignored
/// that flag and ran CompleteTodo anyway: a click wrote a Done mutation to
/// Engine, and on a recurring trashed task it spawned the next occurrence —
/// also flagged trashed — straight into the bin.
#[gpui::test]
fn trashed_task_ring_does_not_toggle(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    agenda.update(cx, |a, _| {
        a.update_todo("dev-trash-1", |t| {
            t.recurrence = Some(RecurrenceRule {
                frequency: 0,
                interval: 1,
                recurrence_type: 0,
                days_of_week: vec![],
            });
        });
    });
    let count = agenda.read_with(cx, |a, _| a.todos.len());

    click(cx, "sb-more");
    // The accordion animates on the wall clock; fast-forward it so the menu
    // rows' hitboxes are fully unclipped before the click.
    agenda.update(cx, |a, cx| {
        a.more_menu_t = 1.0;
        cx.notify();
    });
    click(cx, "more-item-trash");
    assert_eq!(route_of(cx, &agenda), Route::Trash);

    click(cx, "trs-dev-trash-1");
    redraw(cx);
    agenda.read_with(cx, |a, _| {
        let t = a
            .todos
            .iter()
            .find(|t| t.id == "dev-trash-1")
            .expect("trashed task missing");
        assert_ne!(t.status, Status::Done, "trashed task got completed");
        assert!(!t.is_completed, "trashed task got completed");
        assert_eq!(
            a.todos.len(),
            count,
            "completing a trashed recurring task spawned a next occurrence"
        );
    });
}
