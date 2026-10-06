use super::{cluster_vertical_geometry, toggle_left, CLUSTER_BUTTON_SIZE, CLUSTER_ICON_SIZE};
use crate::ui_tests::{click, launch, redraw};
use gpui::TestAppContext;

#[test]
fn macos_toggle_clears_traffic_lights_and_reclaims_fullscreen_space() {
    assert_eq!(toggle_left(true, false), 88.0);
    assert_eq!(toggle_left(true, true), 12.0);
    assert_eq!(toggle_left(false, false), 12.0);
    assert_eq!(toggle_left(false, true), 12.0);
}

#[test]
fn cluster_matches_zeron_geometry() {
    assert_eq!(CLUSTER_BUTTON_SIZE, 24.0);
    assert_eq!(CLUSTER_ICON_SIZE, 16.0);
    for step in 0..=20 {
        let (top, height) = cluster_vertical_geometry(step as f32 / 20.0);
        assert_eq!(top + height / 2.0, 21.0);
    }
    assert_eq!(cluster_vertical_geometry(1.0), (5.0, 32.0));
}

#[gpui::test]
fn empty_projects_cannot_expand_or_animate(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    agenda.update(cx, |a, cx| {
        a.projects.clear();
        a.projects_group_open = true;
        a.group_open_t = 1.0;
        cx.notify();
    });
    redraw(cx);
    click(cx, "sb-projects-head");
    redraw(cx);
    agenda.read_with(cx, |a, _| {
        assert!(!a.projects_group_open);
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
        assert!(!a.projects_group_open);
        assert_eq!(a.group_open_t, 0.0);
    });
}

#[gpui::test]
fn populated_projects_still_toggle(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    assert!(agenda.read_with(cx, |a, _| a.projects.iter().any(|p| p.status == 0)));
    let before = agenda.read_with(cx, |a, _| a.projects_group_open);
    click(cx, "sb-projects-head");
    assert_eq!(agenda.read_with(cx, |a, _| a.projects_group_open), !before);
    click(cx, "sb-projects-head");
    assert_eq!(agenda.read_with(cx, |a, _| a.projects_group_open), before);
}
