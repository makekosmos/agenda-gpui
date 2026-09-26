//! Accessibility-tree tests (KOS-142).
//!
//! The vendored gpui `TestWindow` activates accessibility immediately
//! (vendor/gpui-pre-0.3.5-patched/PATCH.md), so every drawn frame's accesskit
//! nodes are readable through `Window::debug_a11y_tree_json`. These tests
//! render the real Agenda shell on the test platform and assert that every
//! interactive node carries a Russian accessible name, plus a checked-in
//! snapshot of the whole inbox tree.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{AppContext, Entity, Styled, TestAppContext, VisualTestContext};
use serde_json::Value;

use crate::app::{Agenda, AgendaShell};

/// Roles an assistive user can activate or edit — the union that must never
/// appear in the tree without a name.
const INTERACTIVE_ROLES: &[&str] = &[
    "Button",
    "CheckBox",
    "ComboBox",
    "Link",
    "ListBoxOption",
    "MenuItem",
    "MenuItemCheckBox",
    "MenuItemRadio",
    "RadioButton",
    "ScrollBar",
    "SearchBox",
    "Slider",
    "SpinButton",
    "Switch",
    "Tab",
    "TextBox",
    "TreeItem",
];

/// Builds the same view tree as `main.rs` — Agenda inside AgendaShell inside
/// `gpui_component::Root` — on the deterministic test platform. `AGENDA_DEMO`
/// seeds the in-memory dataset so no Engine worker is spawned.
fn launch(cx: &mut TestAppContext) -> (Entity<Agenda>, &mut VisualTestContext) {
    std::env::set_var("AGENDA_DEMO", "1");
    std::env::remove_var("AGENDA_DEVTASKS");
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

/// Draw a pending frame (product handlers don't always `notify`, so force a
/// redraw), then read the accesskit tree captured at end of frame.
fn a11y_tree(cx: &mut VisualTestContext) -> Value {
    cx.update(|_, cx| cx.refresh_windows());
    cx.run_until_parked();
    let json = cx
        .update(|window, _| window.debug_a11y_tree_json())
        .expect("vendored TestWindow activates a11y; a frame must capture a tree");
    serde_json::from_str(&json).expect("a11y tree is valid JSON")
}

/// `role` / `label` / state flags for each node, keyed by node id.
fn nodes(tree: &Value) -> Vec<(String, Value)> {
    tree["nodes"]
        .as_object()
        .expect("nodes object")
        .values()
        .map(|n| {
            (
                n["aria"]["role"].as_str().unwrap_or_default().to_string(),
                n.clone(),
            )
        })
        .collect()
}

/// Human-readable dump used for the checked-in snapshot. Node ids are
/// ephemeral per frame, so the snapshot keeps role/name/state and the
/// parent-child nesting (depth) instead.
fn snapshot_lines(tree: &Value) -> Vec<String> {
    let nodes = tree["nodes"].as_object().expect("nodes object");
    let children_of = |id: &str| -> Vec<String> {
        nodes[id]["children"]
            .as_array()
            .map(|c| {
                c.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut lines = Vec::new();
    let mut stack: Vec<(String, usize)> =
        vec![(tree["root"].as_str().expect("root id").to_string(), 0)];
    while let Some((id, depth)) = stack.pop() {
        let Some(node) = nodes.get(&id) else { continue };
        let aria = &node["aria"];
        let role = aria["role"].as_str().unwrap_or("?");
        let label = aria["label"].as_str().unwrap_or("");
        let mut extra = String::new();
        if let Some(sel) = aria["selected"].as_bool() {
            extra += &format!(" selected={sel}");
        }
        if let Some(t) = aria["toggled"].as_str() {
            extra += &format!(" toggled={t}");
        }
        if let Some(v) = aria["numeric_value"].as_f64() {
            extra += &format!(" value={v}");
        }
        lines.push(format!("{}{role} \"{label}\"{extra}", "  ".repeat(depth)));
        // Children arrays are ordered paint order; push reversed so the DFS
        // prints front-to-back.
        for child in children_of(&id).into_iter().rev() {
            stack.push((child, depth + 1));
        }
    }
    lines
}

#[gpui::test]
async fn a11y_tree_has_no_unnamed_interactive_nodes(cx: &mut TestAppContext) {
    let (_agenda, cx) = launch(cx);
    let tree = a11y_tree(cx);

    let nodes = nodes(&tree);
    assert!(
        nodes.len() > 10,
        "expected a populated a11y tree, got {} nodes",
        nodes.len()
    );

    // The core invariant: no interactive node may be nameless.
    let unnamed: Vec<String> = nodes
        .iter()
        .filter(|(role, n)| {
            INTERACTIVE_ROLES.contains(&role.as_str())
                && n["aria"]["label"]
                    .as_str()
                    .map(|l| l.trim().is_empty())
                    .unwrap_or(true)
        })
        .map(|(role, n)| format!("{role} node {}", n["accesskit_id"]))
        .collect();
    assert!(unnamed.is_empty(), "unnamed interactive nodes: {unnamed:?}");
}

#[gpui::test]
async fn a11y_tree_inbox_exposes_russian_names(cx: &mut TestAppContext) {
    let (_agenda, cx) = launch(cx);
    let tree = a11y_tree(cx);
    let nodes = nodes(&tree);

    let labels: BTreeMap<String, String> = nodes
        .iter()
        .filter_map(|(role, n)| {
            n["aria"]["label"]
                .as_str()
                .map(|l| (l.to_string(), role.clone()))
        })
        .collect();

    // Sidebar navigation, window chrome and the FAB — stable Russian names.
    for expected in [
        "Входящие",
        "Сегодня",
        "Календарь",
        "Проекты",
        "Другое",
        "Боковая панель",
        "Свернуть",
        "Развернуть",
        "Закрыть",
        "Новая задача",
    ] {
        assert!(
            labels.contains_key(expected),
            "expected a11y name '{expected}' in the tree; got labels: {:?}",
            labels.keys().collect::<Vec<_>>()
        );
    }
    // Window controls and nav rows are buttons.
    assert_eq!(labels["Свернуть"], "Button");
    assert_eq!(labels["Закрыть"], "Button");
    assert_eq!(labels["Входящие"], "Button");
    // Sidebar toggle is a switch with its on/off state.
    assert_eq!(labels["Боковая панель"], "Switch");
    // Demo data seeds tasks — the inbox must expose task rows as named
    // buttons plus per-task checkbox nodes named by task title.
    assert!(
        nodes.iter().any(|(role, _)| role == "CheckBox"),
        "expected task status checkboxes in the tree"
    );
    assert!(
        nodes.iter().filter(|(role, _)| role == "Button").count() >= 10,
        "expected >=10 named buttons (nav + rows + chips), tree: {labels:?}"
    );
}

#[gpui::test]
async fn a11y_tree_inbox_matches_snapshot(cx: &mut TestAppContext) {
    let (_agenda, cx) = launch(cx);
    let tree = a11y_tree(cx);
    let actual = snapshot_lines(&tree).join("\n") + "\n";

    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/snapshots/a11y-inbox.txt");
    if std::env::var("A11Y_BLESS").is_ok() {
        std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap())
            .expect("create snapshots dir");
        std::fs::write(path, &actual).expect("write snapshot");
        return;
    }
    let expected = std::fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("missing snapshot {path}; run with A11Y_BLESS=1 to create it"));
    if actual != expected {
        let expected_lines: Vec<&str> = expected.lines().collect();
        let actual_lines: Vec<&str> = actual.lines().collect();
        let mut diff = String::new();
        for i in 0..expected_lines.len().max(actual_lines.len()) {
            let e = expected_lines.get(i).copied().unwrap_or("<missing>");
            let a = actual_lines.get(i).copied().unwrap_or("<missing>");
            if e != a {
                diff.push_str(&format!(
                    "line {}:\n  expected: {e}\n  actual:   {a}\n",
                    i + 1
                ));
            }
        }
        panic!(
            "a11y tree differs from snapshot {path}:\n{diff}\n(run with A11Y_BLESS=1 to accept)"
        );
    }
}
