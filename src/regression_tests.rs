//! Regression tests for KOS-171 fixes: quick-entry draft lifecycle and
//! task-page input ownership. Reuses the headless helpers from `ui_tests`.

use gpui::TestAppContext;

use crate::model::day_key;
use crate::ui_tests::{click, launch, redraw, type_text};

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
        assert_eq!(a.qe_sig, None);
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
        a.qe_sig = Some(7);
        a.qe_date = Some(day_key(3));
        a.qe_date_touched = true;
        a.qe_billable = true;
    });
    click(cx, "fab-new");
    redraw(cx);

    agenda.read_with(cx, |a, _| {
        assert!(a.quick_entry_open);
        assert_eq!(a.qe_sig, None);
        assert_eq!(a.qe_date, None);
        assert!(!a.qe_billable);
    });
}
