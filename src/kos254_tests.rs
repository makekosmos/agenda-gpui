//! Regression tests for KOS-254 residual bug-hunt fixes.
//! Reuses the headless helpers from `ui_tests`.

use gpui::TestAppContext;
use serde_json::json;

use crate::ui_tests::{click, launch, redraw, type_text};
use agenda_core::mapping;
use agenda_core::{filter_idx, is_deferred, task_status, LocalDay, SmartList, Status};

/// «Входящие» is an explicit project choice, not "no choice": the
/// `qe_project_touched` flag distinguishes the two, but `quick_entry_save`
/// only read `qe_project` — `None.or(parsed)` let a parsed @mention override
/// the chip the user just cleared, so the task silently landed in the very
/// project they removed.
#[gpui::test]
fn quick_entry_inbox_choice_beats_mention(cx: &mut TestAppContext) {
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

    click(cx, "qe-proj");
    click(cx, "dd-0"); // «Входящие»
    redraw(cx);
    cx.simulate_keystrokes("enter");

    agenda.read_with(cx, |a, _| {
        let t = a
            .todos
            .iter()
            .find(|t| t.title.contains("молоко"))
            .expect("saved task missing");
        assert_eq!(
            t.project_id, None,
            "explicit «Входящие» pick was overridden by the @mention"
        );
    });
}

/// The recurrence editor is page-local draft state seeded on chip click, but
/// `navigate` never closed it: opening the editor on task A and routing to
/// task B left the panel mounted on B's page with A's values — «Применить»
/// would then write that draft into B.
/// Completing a «Потом» task must remove it from the deferred list: the ring
/// path (`complete_todo`) left `is_someday` set, so `is_deferred` stayed true
/// and the done task kept a row in «Потом» forever — right next to its
/// «Сегодня» completion row.
#[gpui::test]
fn completed_someday_task_leaves_someday(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    click(cx, "sb-someday");
    redraw(cx);
    assert!(agenda.read_with(cx, |a, _| {
        filter_idx(SmartList::Someday, &a.todos, &LocalDay::now())
            .iter()
            .any(|&i| a.todos[i].id == "dev-someday-1")
    }));

    click(cx, "trs-dev-someday-1"); // status ring completes the task
    redraw(cx);

    agenda.read_with(cx, |a, _| {
        let t = a.todo("dev-someday-1").expect("seed missing");
        assert_eq!(task_status(t), Status::Done);
        assert!(!is_deferred(t), "a done task must not count as deferred");
        assert!(
            !filter_idx(SmartList::Someday, &a.todos, &LocalDay::now())
                .iter()
                .any(|&i| a.todos[i].id == "dev-someday-1"),
            "completed task still listed in «Потом»"
        );
    });
}

/// `write` stores canonical `priority` as a string ("none"/"low"/"medium"/
/// "high"), but `read` only tried `as_u64` on it — tasks written by clients
/// that keep only the canonical field (no `extensions.priority` mirror) came
/// back with priority 0: silent data loss of a user-visible field.
#[test]
fn engine_canonical_priority_string_maps_to_level() {
    for (priority, level) in [("none", 0), ("low", 1), ("medium", 2), ("high", 3)] {
        let object = json!({
            "id": "prio-task",
            "typeId": mapping::TASK_TYPE,
            "typeVersion": "1.0.0",
            "title": "x",
            "createdAt": "2026-09-20T10:00:00Z",
            "propsJson": {"status": "todo", "priority": priority},
            "deletedAt": null,
        });
        let t = mapping::read(&object).unwrap();
        assert_eq!(
            t.priority, level,
            "priority '{priority}' must map to {level}"
        );
    }
}
