//! Regression tests for KOS-216: recurrence-editor panic on out-of-range
//! Engine data, overlay click-through/dismissal leaks, and the detached
//! dropdown anchor. (The quick-entry significance slider whose geometry this
//! suite also covered was removed in KOS-295.)
//! Reuses the headless helpers from `ui_tests`.

use gpui::{px, TestAppContext};

use crate::app::Route;
use crate::ui_tests::{click, click_position, launch, redraw};

/// Engine owns the recurrence rule and only string frequencies are validated
/// on read — a numeric `frequency` outside 0..=3 (e.g. written by another
/// Mundus UI) is stored verbatim. The editor's label lookup then indexed
/// `["День","Неделя","Месяц","Год"]` with the raw value: merely opening the
/// recurrence editor panicked the whole app.
/// The dropdown overlay was mounted inside the page column while its anchor
/// (`DropState::x/y`, captured from `ClickEvent::position()`) is in window
/// coordinates — every dropdown rendered offset by the page origin (≈240px
/// right of the sidebar, ≈40px below the titlebar). The overlay now mounts at
/// the window root, so the panel must paint exactly at the press point.
#[gpui::test]
fn dropdown_anchors_at_window_press_point(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    agenda.update(cx, |a, _| {
        a.navigate(Route::Project("dev-proj-home".into()))
    });
    cx.simulate_keystrokes("ctrl-n");
    redraw(cx);

    let chip = cx.debug_bounds("qe-proj").expect("qe-proj must paint");
    click(cx, "qe-proj"); // press lands at the chip center

    let dd = cx.debug_bounds("dd-panel").expect("dd-panel must paint");
    let press_x = f32::from(chip.origin.x) + f32::from(chip.size.width) / 2.;
    let press_y = f32::from(chip.origin.y) + f32::from(chip.size.height) / 2.;
    // Anchored at the press point, or clamped back inside the window when the
    // press sits near the edge (quick-entry chips live in the right panel).
    let dd_right = f32::from(dd.origin.x) + f32::from(dd.size.width);
    assert!(
        f32::from(dd.origin.x) <= press_x + 1. && dd_right >= press_x - 1.,
        "dd panel {dd:?} must horizontally cover the press x {press_x}"
    );
    let dd_bottom = f32::from(dd.origin.y) + f32::from(dd.size.height);
    assert!(
        f32::from(dd.origin.y) <= press_y + 5. && dd_bottom >= press_y + 3.,
        "dd panel {dd:?} must vertically cover the press y {press_y} + 4"
    );
}

/// Two defects stacked here. First, the click-away backdrop
/// (`qe-backdrop`, `on_mouse_down`) received every click inside the overlay —
/// the panel and significance card never stopped propagation — so clicking a
/// chip dismissed quick entry while also opening the dropdown. Second, on a
/// project route `render_quick_entry` re-applied the route's project whenever
/// `qe_project` was None, so picking «Входящие» was silently undone on the
/// next frame. `qe_date` already has a `touched` flag; the project chip did
/// not.
#[gpui::test]
fn quick_entry_project_chip_stays_cleared_on_project_route(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    agenda.update(cx, |a, _| {
        a.navigate(Route::Project("dev-proj-home".into()))
    });

    cx.simulate_keystrokes("ctrl-n");
    redraw(cx);
    agenda.read_with(cx, |a, _| {
        assert_eq!(
            a.qe_project.as_deref(),
            Some("dev-proj-home"),
            "project route must seed the project chip"
        );
    });

    click(cx, "qe-proj"); // opens the QeProject dropdown
    agenda.read_with(cx, |a, _| {
        assert!(
            a.quick_entry_open,
            "clicking a quick-entry chip dismissed the whole overlay"
        );
        assert!(
            a.dropdown.is_some(),
            "project chip must open the QeProject dropdown"
        );
    });
    click(cx, "dd-0"); // «Входящие»
    redraw(cx);

    agenda.read_with(cx, |a, _| {
        assert!(
            a.quick_entry_open,
            "picking «Входящие» dismissed the overlay"
        );
        assert_eq!(
            a.qe_project, None,
            "«Входящие» pick must survive the next render"
        );
    });
}

/// Same click-away backdrop defect in quick search: `qs-backdrop`'s
/// `on_mouse_down` fired for clicks anywhere inside the panel, so clicking
/// the results body (or the empty state) dismissed the overlay mid-gesture.
#[gpui::test]
fn quick_search_panel_click_does_not_dismiss(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    cx.simulate_keystrokes("ctrl-k");
    redraw(cx);
    assert!(agenda.read_with(cx, |a, _| a.quick_open));

    let bounds = cx.debug_bounds("qs-panel").expect("qs-panel must paint");
    // A point inside the panel but off every interactive child (bottom-right
    // corner of the empty-state body).
    click_position(
        cx,
        gpui::point(
            bounds.origin.x + bounds.size.width - px(10.),
            bounds.origin.y + bounds.size.height - px(10.),
        ),
    );

    agenda.read_with(cx, |a, _| {
        assert!(
            a.quick_open,
            "a click inside the panel dismissed quick search"
        );
    });
}
