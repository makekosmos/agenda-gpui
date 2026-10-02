use super::toggle_left;
use crate::ui_tests::{click, launch, redraw};
use gpui::TestAppContext;

#[test]
fn macos_toggle_clears_traffic_lights_and_reclaims_fullscreen_space() {
    assert_eq!(toggle_left(true, false), 88.0);
    assert_eq!(toggle_left(true, true), 12.0);
    assert_eq!(toggle_left(false, false), 12.0);
    assert_eq!(toggle_left(false, true), 12.0);
}

#[gpui::test]
fn empty_projects_cannot_expand_or_animate(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    agenda.update(cx, |a, cx| {
        a.projects.clear();
        a.kanban_group_open = true;
        a.group_open_t = 1.0;
        cx.notify();
    });
    redraw(cx);
    click(cx, "sb-projects-head");
    redraw(cx);
    agenda.read_with(cx, |a, _| {
        assert!(!a.kanban_group_open);
        assert_eq!(a.group_open_t, 0.0);
    });
}

#[gpui::test]
fn archived_only_projects_cannot_expand(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    agenda.update(cx, |a, cx| {
        assert!(!a.projects.is_empty());
        for project in &mut a.projects {
            project.status = 1;
        }
        cx.notify();
    });
    redraw(cx);
    click(cx, "sb-projects-head");
    redraw(cx);
    agenda.read_with(cx, |a, _| {
        assert!(!a.kanban_group_open);
        assert_eq!(a.group_open_t, 0.0);
    });
}

#[gpui::test]
fn populated_projects_still_toggle(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    assert!(agenda.read_with(cx, |a, _| a.projects.iter().any(|p| p.status == 0)));
    let before = agenda.read_with(cx, |a, _| a.kanban_group_open);
    click(cx, "sb-projects-head");
    assert_eq!(agenda.read_with(cx, |a, _| a.kanban_group_open), !before);
    click(cx, "sb-projects-head");
    assert_eq!(agenda.read_with(cx, |a, _| a.kanban_group_open), before);
}
