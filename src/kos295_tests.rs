//! Regression tests for KOS-295: Engine error classes reach the UI, a failed
//! write no longer wedges storage, the error banner is a dismissible toast
//! that auto-hides and never covers the FAB, and quick entry has no
//! significance card.
//! Reuses the headless helpers from `ui_tests`.

use std::sync::mpsc;
use std::time::Duration;

use gpui::TestAppContext;

use crate::app::storage::StorageFault;
use crate::app::Route;
use crate::model::{new_todo, Status, Todo};
use crate::store::{Command, EngineError, ErrorKind, Mutation, Reply, Snapshot, Worker};
use crate::ui_tests::{click, launch, redraw, route_of, todo_of, type_text};

fn engine_error(kind: ErrorKind) -> EngineError {
    EngineError {
        kind,
        detail: "test:engine-code".into(),
    }
}

fn saved(todo: Todo, result: Result<(), EngineError>) -> Reply {
    Reply::Saved(
        Box::new(Mutation {
            before: None,
            todo,
            next: None,
        }),
        result,
    )
}

/// An inert worker channel pair: commands are accepted (and inspectable via
/// the returned receiver) but nothing ever replies — no thread, no Engine.
fn stub_worker() -> (Worker, mpsc::Receiver<Command>) {
    let (commands, rx) = mpsc::channel();
    let (_tx, replies) = mpsc::channel();
    (Worker { commands, replies }, rx)
}

/// A failed write must not drop `storage_ready` or require «Обновить»: the
/// next save attempt is still dispatched, and its success clears the toast.
#[gpui::test]
fn write_error_does_not_block_later_saves(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    let (worker, commands) = stub_worker();
    agenda.update(cx, |a, _| {
        a.demo = false;
        a.storage = Some(worker);
        a.storage_ready = true;
    });
    let task = new_todo("t-fail", "Падающая задача");

    assert!(agenda.update(cx, |a, _| a.save_todo(None, task.clone())));
    agenda.update(cx, |a, cx| {
        a.on_storage_reply(
            saved(task.clone(), Err(engine_error(ErrorKind::Unavailable))),
            cx,
        )
    });
    agenda.read_with(cx, |a, _| {
        assert!(a.storage_ready, "a failed write must not unready storage");
        let error = a.storage_error.as_ref().expect("toast missing");
        assert_eq!(error.fault, StorageFault::Write);
    });

    // The next write goes straight through — no manual refresh needed.
    let ok_task = new_todo("t-ok", "Задача после ошибки");
    assert!(agenda.update(cx, |a, _| a.save_todo(None, ok_task.clone())));
    assert!(matches!(commands.try_recv(), Ok(Command::Save(_))));
    agenda.update(cx, |a, cx| a.on_storage_reply(saved(ok_task, Ok(())), cx));
    agenda.read_with(cx, |a, _| {
        assert!(a.storage_error.is_none(), "success must clear the toast");
        assert!(a.todos.iter().any(|t| t.id == "t-ok"));
    });
}

/// A `conflict` write error resyncs from Engine on its own and reports with a
/// clear message — no manual «Обновить».
#[gpui::test]
fn conflict_write_reloads_automatically(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    let (worker, commands) = stub_worker();
    agenda.update(cx, |a, _| {
        a.demo = false;
        a.storage = Some(worker);
        a.storage_ready = true;
    });
    let task = new_todo("t-conflict", "Устаревшая правка");

    agenda.update(cx, |a, _| {
        a.save_todo(None, task.clone());
    });
    commands.try_recv().unwrap(); // the Save
    agenda.update(cx, |a, cx| {
        a.on_storage_reply(saved(task, Err(engine_error(ErrorKind::Conflict))), cx)
    });
    agenda.read_with(cx, |a, _| {
        let error = a.storage_error.as_ref().expect("conflict toast missing");
        assert_eq!(error.fault, StorageFault::Write);
        assert!(
            error.text.contains("обнов"),
            "conflict text must tell the user the list was refreshed: {}",
            error.text
        );
    });
    assert!(
        matches!(commands.try_recv(), Ok(Command::Load)),
        "conflict must trigger an automatic reload"
    );
}

/// A load failure is a persistent banner with «Обновить», not a toast: it
/// does not auto-hide, and pressing «Обновить» retries the load.
#[gpui::test]
fn load_error_stays_until_refresh(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    let (worker, commands) = stub_worker();
    agenda.update(cx, |a, _| {
        a.demo = false;
        a.storage = Some(worker);
        a.storage_ready = false;
    });

    agenda.update(cx, |a, cx| {
        a.on_storage_reply(Reply::Loaded(Err(engine_error(ErrorKind::NotRunning))), cx)
    });
    redraw(cx);
    agenda.read_with(cx, |a, _| {
        let error = a.storage_error.as_ref().expect("load banner missing");
        assert_eq!(error.fault, StorageFault::Load);
        assert!(error.text.contains("Mundus"));
    });
    assert!(cx.debug_bounds("reload-engine").is_some());
    assert!(cx.debug_bounds("storage-toast").is_some());
    assert!(cx.debug_bounds("fab-new").is_some());

    // Load errors do not auto-hide…
    cx.executor().advance_clock(Duration::from_secs(30));
    cx.run_until_parked();
    agenda.read_with(cx, |a, _| assert!(a.storage_error.is_some()));

    // …but «Обновить» retries, and a successful load clears the banner.
    click(cx, "reload-engine");
    assert!(matches!(commands.try_recv(), Ok(Command::Load)));
    agenda.update(cx, |a, cx| {
        a.on_storage_reply(Reply::Loaded(Ok(Snapshot::default())), cx)
    });
    agenda.read_with(cx, |a, _| {
        assert!(a.storage_ready);
        assert!(a.storage_error.is_none());
    });
}

/// The write-error toast paints bottom-left and never intersects the FAB.
#[gpui::test]
fn write_toast_does_not_cover_fab(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    let (worker, _commands) = stub_worker();
    agenda.update(cx, |a, _| {
        a.demo = false;
        a.storage = Some(worker);
        a.storage_ready = true;
    });
    agenda.update(cx, |a, cx| {
        a.on_storage_reply(
            saved(
                new_todo("t", "x"),
                Err(engine_error(ErrorKind::InvalidRequest)),
            ),
            cx,
        )
    });
    redraw(cx);

    let toast = cx.debug_bounds("storage-toast").expect("toast must paint");
    let fab = cx.debug_bounds("fab-new").expect("fab must paint");
    let overlap = f32::from(toast.origin.x + toast.size.width) - f32::from(fab.origin.x);
    assert!(
        overlap <= 1.0,
        "toast overlaps the FAB by {overlap}px (toast {toast:?}, fab {fab:?})"
    );

    // × dismisses immediately…
    click(cx, "storage-toast-close");
    agenda.read_with(cx, |a, _| assert!(a.storage_error.is_none()));
    redraw(cx);
    assert!(cx.debug_bounds("storage-toast").is_none());
}

/// The toast auto-hides after ~7s, and a repeated error restarts the timer.
#[gpui::test]
fn write_toast_auto_hides_and_restarts(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    let (worker, _commands) = stub_worker();
    agenda.update(cx, |a, _| {
        a.demo = false;
        a.storage = Some(worker);
        a.storage_ready = true;
    });
    let raise = |a: &mut crate::app::Agenda, cx: &mut gpui::Context<crate::app::Agenda>| {
        a.on_storage_reply(
            saved(new_todo("t", "x"), Err(engine_error(ErrorKind::Timeout))),
            cx,
        )
    };

    agenda.update(cx, |a, cx| raise(a, cx));
    redraw(cx);
    assert!(cx.debug_bounds("storage-toast").is_some());

    cx.executor().advance_clock(Duration::from_secs(5));
    cx.run_until_parked();
    agenda.read_with(cx, |a, _| assert!(a.storage_error.is_some()));

    // A second error restarts the countdown — 5s later it must still show.
    agenda.update(cx, |a, cx| raise(a, cx));
    cx.executor().advance_clock(Duration::from_secs(5));
    cx.run_until_parked();
    agenda.read_with(cx, |a, _| {
        assert!(a.storage_error.is_some(), "repeated error reset the timer")
    });

    cx.executor().advance_clock(Duration::from_secs(3));
    cx.run_until_parked();
    agenda.read_with(cx, |a, _| {
        assert!(a.storage_error.is_none(), "toast must auto-hide")
    });
}

/// Quick entry has no significance card, and a task created from it has
/// `significance == None` — significance lives only in task properties.
#[gpui::test]
fn quick_entry_has_no_significance_card(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);

    cx.simulate_keystrokes("ctrl-n");
    redraw(cx);
    assert!(cx.debug_bounds("qe-sig-slider").is_none());

    type_text(cx, "Купить молоко");
    cx.simulate_keystrokes("enter");
    agenda.read_with(cx, |a, _| {
        let t = a
            .todos
            .iter()
            .find(|t| t.title == "Купить молоко")
            .expect("created task missing");
        assert_eq!(t.significance, None);
        assert_eq!(t.status, Status::Inbox);
    });
}

/// Significance still works from task properties (chip → dropdown → value).
#[gpui::test]
fn task_props_set_significance(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);

    click(cx, "tr-dev-inbox-1");
    assert_eq!(route_of(cx, &agenda), Route::Task("dev-inbox-1".into()));
    click(cx, "chip-tp-sig");
    click(cx, "dd-7");

    let t = todo_of(cx, &agenda, "dev-inbox-1");
    assert_eq!(t.significance, Some(7));
}
