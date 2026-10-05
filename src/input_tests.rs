use crate::a11y_tests::{a11y_tree, launch as launch_a11y};
use crate::ui_tests::{launch, redraw};
use gpui::TestAppContext;

/// Cmd/Ctrl+B toggles the sidebar via the global keymap action — it must fire
/// even when a text field holds focus (inputs swallow raw key_down events).
#[gpui::test]
fn cmd_b_toggles_sidebar(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    cx.simulate_keystrokes("cmd-b");
    agenda.read_with(cx, |a, _| assert_eq!(a.sidebar_target, 0.0));
    cx.simulate_keystrokes("cmd-b");
    agenda.read_with(cx, |a, _| assert_eq!(a.sidebar_target, 1.0));
}

#[gpui::test]
fn cmd_b_toggles_sidebar_with_text_focus(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    agenda.update(cx, |a, cx| {
        a.set_quick_entry(true, cx);
        cx.notify();
    });
    redraw(cx);
    cx.simulate_input("draft");
    cx.simulate_keystrokes("cmd-b");
    agenda.read_with(cx, |a, cx| {
        assert_eq!(a.sidebar_target, 0.0);
        let title = a.qe_title.as_ref().unwrap();
        assert_eq!(title.read(cx).value(), "draft");
    });
    cx.simulate_keystrokes("cmd-b");
    agenda.read_with(cx, |a, _| assert_eq!(a.sidebar_target, 1.0));
}

#[gpui::test]
async fn task_panel_exposes_focused_editable_text_field(cx: &mut TestAppContext) {
    let (agenda, cx) = launch_a11y(cx);
    agenda.update(cx, |app, cx| {
        app.set_quick_entry(true, cx);
        cx.notify();
    });
    let tree = a11y_tree(cx);
    let focused = tree["gpui_focus"]
        .as_str()
        .unwrap_or_else(|| panic!("focused node: {tree}"));
    let node = &tree["nodes"][focused];
    assert_eq!(node["aria"]["role"], "MultilineTextInput", "{tree}");
    assert_eq!(node["aria"]["label"], "Название задачи", "{tree}");
    assert!(node["aria"]["on_action"]
        .as_array()
        .unwrap()
        .iter()
        .any(|a| a == "SetValue"));

    let notes = agenda.read_with(cx, |a, _| a.qe_notes.as_ref().unwrap().clone());
    cx.update(|window, cx| {
        notes.update(cx, |state, cx| state.focus(window, cx));
    });
    cx.simulate_input("notes");
    let tree = a11y_tree(cx);
    let node = &tree["nodes"][tree["gpui_focus"].as_str().unwrap()];
    assert_eq!(node["aria"]["role"], "MultilineTextInput");
    assert_eq!(node["aria"]["label"], "Заметки");
    assert_eq!(node["aria"]["value"], "notes");

    agenda.update(cx, |app, cx| {
        app.quick_open = true;
        cx.notify();
    });
    // The search creates and focuses its input during its first render.
    a11y_tree(cx);
    let tree = a11y_tree(cx);
    let node = &tree["nodes"][tree["gpui_focus"].as_str().unwrap()];
    assert_eq!(node["aria"]["role"], "TextInput");
    assert_eq!(node["aria"]["label"], "Поиск задач и проектов");
}
