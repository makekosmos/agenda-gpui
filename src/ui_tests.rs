//! Headless UI tests for the Agenda shell (KOS-141).
//!
//! Test builds always take the in-memory demo store (`cfg!(test)` in
//! `Agenda::new`), so these tests drive the real GPUI event pipeline —
//! keystroke dispatch, hitbox clicks, routing — without an Engine worker.
//! Interactions are located by `debug_selector` markers, not fixed layout
//! coordinates, so layout changes don't silently break the suite.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    AppContext, Bounds, Entity, Keystroke, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent,
    Pixels, PlatformInput, Styled, TestAppContext, VisualTestContext,
};

use crate::app::{Agenda, AgendaShell, Route};
use crate::model::{task_status, Status, Todo};

/// Builds the same view tree as `main.rs` — Agenda inside AgendaShell inside
/// `gpui_component::Root` — on the deterministic test platform. The returned
/// `VisualTestContext` must be shadowed over `cx`.
fn launch(cx: &mut TestAppContext) -> (Entity<Agenda>, &mut VisualTestContext) {
    // Keep the env-based QA hooks identical for every test process.
    for var in [
        "AGENDA_OVERLAY",
        "AGENDA_ROUTE",
        "AGENDA_FPS",
        "AGENDA_DEVTASKS",
    ] {
        std::env::remove_var(var);
    }
    cx.update(gpui_component::init);
    let slot: Rc<RefCell<Option<Entity<Agenda>>>> = Rc::new(RefCell::new(None));
    let slot2 = slot.clone();
    let (_root, cx) = cx.add_window_view(move |window, cx| {
        let agenda = cx.new(Agenda::new);
        *slot2.borrow_mut() = Some(agenda.clone());
        let view = cx.new(|_| AgendaShell { agenda });
        let mut root = gpui_component::Root::new(view, window, cx);
        root.style().background = Some(gpui::transparent_black().into());
        root
    });
    let agenda = slot.borrow_mut().take().expect("window builder ran");
    (agenda, cx)
}

/// Force a repaint. Several product handlers mutate `Agenda` state without
/// `cx.notify()` (e.g. `on_key` opening quick entry), so nothing marks the
/// window dirty and the test platform never redraws. Flushing a
/// `RefreshWindows` effect draws the pending frame synchronously.
fn redraw(cx: &mut VisualTestContext) {
    cx.update(|_, cx| cx.refresh_windows());
}

/// Left-click the center of the painted bounds of `selector`.
///
/// Mouse down and up are dispatched inside one update so the frame cannot be
/// redrawn in between — the same atomicity a real click has. Split
/// `simulate_click` events would let the post-mousedown redraw drop rows whose
/// backdrop already closed the menu (ctx menu / property dropdowns), and their
/// `on_click` would never fire.
fn click(cx: &mut VisualTestContext, selector: &'static str) {
    let bounds: Bounds<Pixels> = cx
        .debug_bounds(selector)
        .unwrap_or_else(|| panic!("'{selector}' has no painted bounds"));
    let position = bounds.center();
    let modifiers = Modifiers::default();
    cx.update(|window, cx| {
        window.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                position,
                modifiers,
                button: MouseButton::Left,
                click_count: 1,
                first_mouse: false,
            }),
            cx,
        );
        window.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                position,
                modifiers,
                button: MouseButton::Left,
                click_count: 1,
            }),
            cx,
        );
    });
    cx.run_until_parked();
}

/// Type text into the focused input. `simulate_input` parses each char via
/// `Keystroke::parse`, which leaves `key_char` empty, so the input handler
/// path is skipped entirely — this sets `key_char` explicitly.
fn type_text(cx: &mut VisualTestContext, text: &str) {
    cx.update(|window, cx| {
        for ch in text.chars() {
            window.dispatch_keystroke(
                Keystroke {
                    modifiers: Modifiers::default(),
                    key: ch.to_lowercase().to_string(),
                    key_char: Some(ch.to_string()),
                },
                cx,
            );
        }
    });
    cx.run_until_parked();
}

fn route_of(cx: &VisualTestContext, agenda: &Entity<Agenda>) -> Route {
    agenda.read_with(cx, |a, _| a.route.clone())
}

fn todo_of(cx: &VisualTestContext, agenda: &Entity<Agenda>, id: &str) -> Todo {
    agenda.read_with(cx, |a, _| {
        a.todos
            .iter()
            .find(|t| t.id == id)
            .unwrap_or_else(|| panic!("todo '{id}' missing"))
            .clone()
    })
}

/// Capture: Ctrl+N opens quick entry, typed text + Enter saves an inbox task.
#[gpui::test]
fn capture_via_quick_entry(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    let title = "Собрать документы для страховой";
    let before = agenda.read_with(cx, |a, _| a.todos.len());

    cx.simulate_keystrokes("ctrl-n");
    redraw(cx);
    assert!(agenda.read_with(cx, |a, _| a.quick_entry_open));

    type_text(cx, title);
    cx.simulate_keystrokes("enter");

    agenda.read_with(cx, |a, _| {
        assert!(!a.quick_entry_open, "quick entry must close after save");
        assert_eq!(a.todos.len(), before + 1);
        let t = a
            .todos
            .iter()
            .find(|t| t.title == title)
            .expect("captured task missing from todos");
        assert_eq!(t.status, Status::Inbox);
        assert!(!t.is_completed);
    });
}

/// Escape closes quick entry without creating a task.
#[gpui::test]
fn escape_dismisses_quick_entry(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    let before = agenda.read_with(cx, |a, _| a.todos.len());

    cx.simulate_keystrokes("ctrl-n");
    redraw(cx);
    type_text(cx, "Черновик");
    cx.simulate_keystrokes("escape");

    agenda.read_with(cx, |a, _| {
        assert!(!a.quick_entry_open, "escape must close quick entry");
        assert_eq!(a.todos.len(), before, "escape must not save");
    });
}

/// Clicking a row's status ring completes the task.
#[gpui::test]
fn status_ring_completes_task(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);

    click(cx, "trs-dev-inbox-1");

    let t = todo_of(cx, &agenda, "dev-inbox-1");
    assert_eq!(task_status(&t), Status::Done);
    assert!(t.is_completed);
    assert!(t.completed_at.is_some());
}

/// Re-toggling a done task back to work: the ring click also opens the task
/// page (both listeners see the hit), where the "…" menu offers
/// «Вернуть в работу» — CompleteTodo on a done task reverts it to Todo.
#[gpui::test]
fn done_task_returns_to_work_via_menu(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);

    click(cx, "trs-dev-inbox-1");
    assert_eq!(route_of(cx, &agenda), Route::Task("dev-inbox-1".into()));

    click(cx, "tp-more-btn");
    click(cx, "ctx-item-0"); // «Вернуть в работу»

    let t = todo_of(cx, &agenda, "dev-inbox-1");
    assert!(!t.is_completed);
    assert_eq!(task_status(&t), Status::Todo);
}

/// The status chip on the task page opens the status dropdown; «Готово»
/// completes, «Сделать» reverts — a full UI toggle round-trip.
#[gpui::test]
fn task_status_dropdown_toggles_round_trip(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);

    click(cx, "tr-dev-inbox-2");
    assert_eq!(route_of(cx, &agenda), Route::Task("dev-inbox-2".into()));

    click(cx, "chip-tp-status");
    click(cx, "dd-4"); // «Готово»
    let t = todo_of(cx, &agenda, "dev-inbox-2");
    assert_eq!(task_status(&t), Status::Done);
    assert!(t.is_completed);

    click(cx, "chip-tp-status");
    click(cx, "dd-1"); // «Сделать»
    let t = todo_of(cx, &agenda, "dev-inbox-2");
    assert!(!t.is_completed);
    assert_eq!(task_status(&t), Status::Todo);
}

/// Sidebar nav rows route the content view.
#[gpui::test]
fn sidebar_navigates_lists(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    for (selector, want) in [
        ("sb-today", Route::Today),
        ("sb-someday", Route::Someday),
        ("sb-plans", Route::Plans),
        ("sb-inbox", Route::Inbox),
    ] {
        click(cx, selector);
        assert_eq!(route_of(cx, &agenda), want, "sidebar '{selector}'");
    }
}

/// «Другое» → «Настройки» opens Settings; the settings sidebar's «Назад»
/// returns to the previous route.
#[gpui::test]
fn settings_open_and_back(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);

    click(cx, "sb-more");
    // The accordion animates on the wall clock; fast-forward it so the menu
    // rows' hitboxes are fully unclipped before the click.
    agenda.update(cx, |a, cx| {
        a.more_menu_t = 1.0;
        cx.notify();
    });
    click(cx, "more-item-settings");
    assert_eq!(route_of(cx, &agenda), Route::Settings);

    click(cx, "sb-back");
    assert_eq!(route_of(cx, &agenda), Route::Inbox);
}
