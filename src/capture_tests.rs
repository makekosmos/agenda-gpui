//! Capture and project flows found by the e2e run (`scripts/e2e-macos.sh`):
//! back-to-back capture, the «Эта неделя» chip, project heading and delete.

use gpui::TestAppContext;

use crate::app::{MenuAction, Route};
use crate::model::{is_due_this_week, is_inbox, new_todo, today_key, week_bounds};
use crate::ui_tests::{launch, redraw, type_text};

/// Capturing tasks back to back: the second Ctrl+N must land typing in the
/// title again, not in a stale focus left by the first save.
#[gpui::test]
fn capture_twice_in_a_row(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    let before = agenda.read_with(cx, |a, _| a.todos.len());
    for title in ["Первая подряд", "Вторая подряд"] {
        cx.simulate_keystrokes("ctrl-n");
        redraw(cx);
        type_text(cx, title);
        cx.simulate_keystrokes("enter");
        redraw(cx);
    }
    agenda.read_with(cx, |a, _| {
        assert_eq!(a.todos.len(), before + 2, "both captures must be saved");
        assert!(a.todos.iter().any(|t| t.title == "Вторая подряд"));
    });
}

/// Quick entry opened in «Эта неделя» files the task into this week, and
/// the chip does not follow the user into the next capture elsewhere.
#[gpui::test]
fn week_capture_lands_in_week_and_chip_resets(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    let week = week_bounds(&today_key()).unwrap();
    agenda.update(cx, |a, _| a.navigate(Route::Week));
    for (route, title) in [(Route::Week, "Неделя"), (Route::Inbox, "Входящая")] {
        agenda.update(cx, |a, _| a.navigate(route));
        redraw(cx);
        cx.simulate_keystrokes("ctrl-n");
        redraw(cx);
        type_text(cx, title);
        cx.simulate_keystrokes("enter");
        redraw(cx);
    }
    agenda.read_with(cx, |a, _| {
        let find = |title| a.todos.iter().find(|t| t.title == title).unwrap();
        assert!(is_due_this_week(find("Неделя"), &week));
        assert!(is_inbox(find("Входящая")) && !find("Входящая").is_today);
    });
}

/// A new project opens with its heading selected: typing replaces the
/// placeholder name and Enter saves it.
#[gpui::test]
fn new_project_is_named_from_its_heading(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    agenda.update(cx, |a, _| a.create_project());
    redraw(cx);
    type_text(cx, "Ремонт");
    cx.simulate_keystrokes("enter");
    redraw(cx);
    agenda.read_with(cx, |a, _| {
        let Route::Project(id) = &a.route else {
            panic!("project page expected");
        };
        assert_eq!(a.project(id).unwrap().title, "Ремонт");
    });
}

/// Deleting a project keeps its tasks: they return to Inbox.
#[gpui::test]
fn deleting_project_moves_tasks_to_inbox(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    agenda.update(cx, |a, _| a.create_project());
    let pid = agenda.read_with(cx, |a, _| a.projects.last().unwrap().id.clone());
    agenda.update(cx, |a, _| {
        let mut todo = new_todo("in-project", "Задача проекта");
        todo.project_id = Some(pid.clone());
        todo.status = crate::model::Status::Todo;
        a.todos.push(todo);
        a.run_menu_action(MenuAction::DeleteProject(pid.clone()));
    });
    agenda.read_with(cx, |a, _| {
        assert!(a.project(&pid).is_none());
        assert_eq!(a.route, Route::Inbox);
        let todo = a.todo("in-project").unwrap();
        assert!(todo.project_id.is_none() && is_inbox(todo));
    });
}
